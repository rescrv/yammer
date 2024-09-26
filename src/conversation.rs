//! A conversation captures an exchange of messages between a user and an assistant.
//!
//! This module provides a `Conversation` struct that can be used to build a conversation between
//! a user and an assistant. The conversation can be used to generate a `ChatRequest` to work with
//! the core yammer library.

use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Config, Editor};

use super::{ChatMessage, ChatRequest};

//////////////////////////////////////// ConversationOptions ///////////////////////////////////////

#[derive(Clone, Debug, Eq, PartialEq, arrrg_derive::CommandLine)]
pub struct ConversationOptions {
    #[arrrg(required, "Model to run.")]
    pub model: String,
    #[arrrg(optional, "System prompt to load in advance.")]
    pub system: Option<String>,
    #[arrrg(optional, "File to write the ndjson logs to.")]
    pub log: Option<String>,
    #[arrrg(optional, "HISTFILE for the shell.")]
    pub histfile: Option<String>,
    #[arrrg(flag, "Ignore duplicate history entries.")]
    pub history_ignore_dups: bool,
    #[arrrg(flag, "Ignore history entries starting with space.")]
    pub history_ignore_space: bool,
    #[arrrg(optional, "PS1 for the conversation shell")]
    pub ps1: String,
    #[arrrg(optional, "Load chat history from a file previously created by log")]
    pub load: Option<String>,
}

impl Default for ConversationOptions {
    fn default() -> Self {
        Self {
            model: "mistral-nemo".to_string(),
            system: None,
            log: None,
            histfile: None,
            history_ignore_dups: false,
            history_ignore_space: false,
            ps1: "yammer> ".to_string(),
            load: None,
        }
    }
}

/////////////////////////////////////////// Conversation ///////////////////////////////////////////

/// Conversation captures an exchange of messages between a user and an assistant.
#[derive(Clone, Debug, Default)]
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
                if let Some(serde_json::Value::Object(x)) = x.get("message") {
                    if let Some(serde_json::Value::String(x)) = x.get("content") {
                        Some(x.clone())
                    } else {
                        None
                    }
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
        let mut spinner = Spinner::new();
        let mut log = if let Some(log_path) = options.log.as_ref() {
            let log = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)?;
            let mut log = BufWriter::new(log);
            writeln!(
                log,
                "{}",
                serde_json::to_string(&std::env::args().collect::<Vec<_>>())?
            )?;
            let _ = log.flush();
            Some(BufWriter::new(log))
        } else {
            None
        };
        if let Some(load) = options.load.as_ref() {
            for msg in super::load(load)? {
                self.push(msg);
            }
        }
        loop {
            let line = rl.readline(&options.ps1);
            match line {
                Ok(line) => {
                    if line.trim().starts_with("/") {
                        if self.command(line).is_break() {
                            return Ok(());
                        }
                        continue;
                    }
                    self.push(ChatMessage {
                        role: "user".to_string(),
                        content: line,
                        images: None,
                        tool_calls: None,
                    });
                    if let Some(log) = log.as_mut() {
                        writeln!(
                            log,
                            "{}",
                            serde_json::to_string(&self.messages[self.messages.len() - 1])?
                        )?;
                        let _ = log.flush();
                    }
                    let cr = self.clone().request(&options.model);
                    let req = match super::Request::chat(global.clone(), cr) {
                        Ok(req) => req,
                        Err(err) => {
                            eprintln!("could not chat: {}", err);
                            continue;
                        }
                    };
                    let mut printer = super::ChatAccumulator::default();
                    let mut acc = self.accumulator();
                    spinner.start();
                    let resp =
                        super::accumulate(req, &mut (&mut spinner, &mut acc, &mut printer)).await;
                    spinner.inhibit();
                    if let Err(err) = resp {
                        eprintln!("could not chat: {:?}", err);
                    } else {
                        println!();
                    }
                    drop(acc);
                    // FENCE: drop acc above here; log below here.
                    if let Some(log) = log.as_mut() {
                        writeln!(
                            log,
                            "{}",
                            serde_json::to_string(&self.messages[self.messages.len() - 1])?
                        )?;
                        let _ = log.flush();
                    }
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

    fn command(&mut self, line: String) -> std::ops::ControlFlow<()> {
        match line.as_str() {
            "/exit" => std::ops::ControlFlow::Break(()),
            _ => {
                eprintln!("unknown command: {}", line);
                std::ops::ControlFlow::Continue(())
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
    fn accumulate(&mut self, message: serde_json::Value) -> std::ops::ControlFlow<()> {
        self.pieces.push(message);
        std::ops::ControlFlow::Continue(())
    }
}

impl<'a> Drop for ConversationAccumulator<'a> {
    fn drop(&mut self) {
        self.convo
            .add_assistant_response(std::mem::take(&mut self.pieces));
    }
}

////////////////////////////////////////////// Spinner /////////////////////////////////////////////

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug)]
pub struct Spinner {
    done: Arc<AtomicBool>,
    inhibited: Arc<Mutex<bool>>,
    background: Option<std::thread::JoinHandle<()>>,
}

impl Spinner {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let done_p = Arc::clone(&done);
        let inhibited = Arc::new(Mutex::new(true));
        let inhibited_p = Arc::clone(&inhibited);
        let background = std::thread::spawn(move || {
            let mut i = 0;
            while !done_p.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(50));
                let inhibited_p = inhibited_p.lock().unwrap();
                if *inhibited_p {
                    continue;
                }
                let mut stdout = std::io::stdout().lock();
                let _ = stdout.write(b"\x1b[1D");
                let _ = stdout.write(b"\x1b[1D");
                let _ = stdout.write(SPINNER[i % SPINNER.len()].as_bytes());
                let _ = stdout.write(" ".as_bytes());
                let _ = stdout.flush();
                i += 1;
            }
        });
        Self {
            done,
            inhibited,
            background: Some(background),
        }
    }

    pub fn start(&self) {
        *self.inhibited.lock().unwrap() = false;
    }

    pub fn inhibit(&self) {
        let mut inhibited = self.inhibited.lock().unwrap();
        if !*inhibited {
            *inhibited = true;
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write(b"\x1b[1D");
            let _ = stdout.write(b"\x1b[1D");
        }
    }
}

impl super::Accumulator for Spinner {
    fn accumulate(&mut self, _: serde_json::Value) -> std::ops::ControlFlow<()> {
        self.inhibit();
        std::ops::ControlFlow::Continue(())
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Relaxed);
        self.inhibit();
        if let Some(background) = self.background.take() {
            background.join().unwrap();
        }
    }
}
