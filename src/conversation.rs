//! A conversation captures an exchange of messages between a user and an assistant.
//!
//! This module provides a `Conversation` struct that can be used to build a conversation between
//! a user and an assistant. The conversation can be used to generate a `ChatRequest` to work with
//! the core yammer library.

use std::path::PathBuf;

use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Config, Editor};

use super::{Accumulator, ChatMessage, ChatRequest};

//////////////////////////////////////// ConversationOptions ///////////////////////////////////////

#[derive(Clone, Debug, Eq, PartialEq, arrrg_derive::CommandLine)]
pub struct ConversationOptions {
    #[arrrg(required, "Model to run.")]
    model: String,
    #[arrrg(optional, "HISTFILE for the shell.")]
    histfile: Option<String>,
    #[arrrg(flag, "Ignore duplicate history entries.")]
    history_ignore_dups: bool,
    #[arrrg(flag, "Ignore history entries starting with space.")]
    history_ignore_space: bool,
    #[arrrg(optional, "PS1 for the conversation shell")]
    ps1: String,
}

impl Default for ConversationOptions {
    fn default() -> Self {
        Self {
            model: "mistral-nemo".to_string(),
            histfile: None,
            history_ignore_dups: false,
            history_ignore_space: false,
            ps1: "yammer> ".to_string(),
        }
    }
}

/////////////////////////////////////////// Conversation ///////////////////////////////////////////

/// Conversation captures an exchange of messages between a user and an assistant.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Conversation {
    messages: Vec<ChatMessage>,
}

impl Conversation {
    /// Create a new conversation.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Push the ChatMessage onto the conversation.
    pub fn push(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Get the messages in the conversation.
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Truncate the conversation to at most `index` messages.
    pub fn truncate(&mut self, index: usize) {
        self.messages.truncate(index);
    }

    /// Interpret an assistant response and add it to the conversation.
    pub fn add_assistant_response(&mut self, pieces: Vec<serde_json::Value>) {
        let content = pieces
            .into_iter()
            .flat_map(|x| {
                if let Some(serde_json::Value::String(x)) = x.get("response") {
                    Some(x.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");
        if !content.is_empty() {
            self.push(ChatMessage {
                role: "assistant".to_string(),
                content,
                images: None,
                tool_calls: None,
            });
        }
    }

    /// Return an Accumulator for the conversation.
    pub fn accumulator(&mut self) -> ConversationAccumulator {
        ConversationAccumulator {
            convo: self,
            pieces: Vec::new(),
        }
    }

    /// Consume the conversation and return a ChatRequest for `model`.
    pub fn request(self, model: impl Into<String>) -> ChatRequest {
        ChatRequest {
            model: model.into(),
            messages: self.messages,
            stream: Some(true),
            tools: None,
            format: None,
            keep_alive: None,
        }
    }

    pub async fn shell(
        mut self,
        global: super::RequestOptions,
        options: ConversationOptions,
    ) -> Result<(), super::Error> {
        let config = Config::builder()
            .auto_add_history(true)
            .max_history_size(1_000_000)
            .expect("this should always work")
            .history_ignore_dups(options.history_ignore_dups)
            .expect("this should always work")
            .history_ignore_space(options.history_ignore_space)
            .build();
        let mut rl: Editor<(), FileHistory> = if let Some(histfile) = options.histfile.as_ref() {
            let histfile = PathBuf::from(histfile);
            let history = rustyline::history::FileHistory::new();
            let mut rl = Editor::with_history(config, history).expect("this should always work");
            if histfile.exists() {
                rl.load_history(&histfile).expect("this should always work");
            }
            rl
        } else {
            Editor::with_config(config).expect("this should always work")
        };
        loop {
            let line = rl.readline(&options.ps1);
            match line {
                Ok(line) => {
                    self.push(ChatMessage {
                        role: "user".to_string(),
                        content: line,
                        images: None,
                        tool_calls: None,
                    });
                    let cr = self.clone().request(&options.model);
                    let req = match super::Request::chat(global.clone(), cr) {
                        Ok(req) => req,
                        Err(err) => {
                            eprintln!("could not chat: {}", err);
                            continue;
                        }
                    };
                    let mut printer = super::ChatAccumulator::default();
                    if let Err(err) =
                        super::accumulate(req, &mut (self.accumulator(), &mut printer)).await
                    {
                        eprintln!("could not chat: {:?}", err);
                    }
                    println!();
                }
                Err(ReadlineError::Interrupted) => {}
                Err(ReadlineError::Eof) => {
                    return Ok(());
                }
                Err(err) => {
                    eprintln!("could not read line: {}", err);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ConversationAccumulator<'a> {
    convo: &'a mut Conversation,
    pieces: Vec<serde_json::Value>,
}

impl<'a> super::Accumulator for ConversationAccumulator<'a> {
    fn accumulate(&mut self, message: serde_json::Value) {
        self.pieces.push(message);
    }
}

impl<'a> Drop for ConversationAccumulator<'a> {
    fn drop(&mut self) {
        self.convo
            .add_assistant_response(std::mem::take(&mut self.pieces));
    }
}
