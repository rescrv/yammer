use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::io::{BufRead, Write};
use std::time::{Instant, SystemTime};

use arrrg::{CommandLine, NoExitCommandLine};
use rustyline::config::EditMode;
use rustyline::error::ReadlineError;
use rustyline::hint::HistoryHinter;
use rustyline::{CompletionType, Config, Editor, EventHandler, KeyEvent};
use utf8path::Path;

use crate::cli::{CommandHint, ShellHelper, TabEventHandler};
use crate::types::{ChatMessage, ChatRequest};
use crate::{Error, Parameters, Spinner, WordWrap};

//////////////////////////////////////////// ChatLogLine ///////////////////////////////////////////

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "type")]
pub enum ChatLogLine {
    #[serde(rename = "message")]
    Message {
        created_at: chrono::DateTime<chrono::Local>,
        message: ChatMessage,
    },
    #[serde(rename = "options")]
    Options {
        created_at: chrono::DateTime<chrono::Local>,
        options: ChatOptions,
    },
    #[serde(rename = "tag")]
    Tag {
        created_at: chrono::DateTime<chrono::Local>,
        tag: String,
    },
    #[serde(rename = "untag")]
    Untag {
        created_at: chrono::DateTime<chrono::Local>,
        tag: String,
    },
    #[serde(rename = "retry")]
    Retry {
        created_at: chrono::DateTime<chrono::Local>,
    },
}

//////////////////////////////////////////// ChatSummary ///////////////////////////////////////////

#[derive(Clone, Debug)]
pub struct ChatSummary {
    pub slug: String,
    pub modified: SystemTime,
    pub model: String,
    pub summary: String,
    pub tags: HashSet<String>,
}

impl ChatSummary {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let basename = path.basename().as_str().to_string();
        let Some(slug) = basename.strip_suffix(".ndjson") else {
            return Err(Error::InvalidArgument(
                "must provide a path ending in .ndjson".to_string(),
            ));
        };
        let slug = slug.to_string();
        let md = path.into_std().metadata()?;
        let modified = md.modified()?;
        let transcript = std::fs::read_to_string(path)?;
        let mut model = None;
        let mut first_message = None;
        let mut tags: HashSet<String> = HashSet::default();
        for line in transcript.lines() {
            let log_line = serde_json::from_str(line)?;
            match log_line {
                ChatLogLine::Message {
                    created_at: _,
                    message,
                } if first_message.is_none() => {
                    first_message = Some(message);
                }
                ChatLogLine::Options {
                    created_at: _,
                    options,
                } => {
                    model = Some(options.model.clone());
                }
                ChatLogLine::Message {
                    created_at: _,
                    message: _,
                } => {}
                ChatLogLine::Tag { created_at: _, tag } => {
                    tags.insert(tag);
                }
                ChatLogLine::Untag { created_at: _, tag } => {
                    tags.remove(&tag);
                }
                ChatLogLine::Retry { created_at: _ } => {}
            }
        }
        let summary = match first_message {
            Some(message) => message
                .content
                .lines()
                .take(1)
                .next()
                .map(String::from)
                .unwrap_or("<empty message>".to_string()),
            None => "<empty conversation>".to_string(),
        };
        let model = model.unwrap_or("<unknown>".to_string());
        Ok(Self {
            slug,
            modified,
            model,
            summary,
            tags,
        })
    }
}

impl std::fmt::Display for ChatSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let modified = chrono::DateTime::<chrono::Local>::from(self.modified);
        let format_string = "%Y-%m-%dT%H:%M".to_string();
        write!(
            f,
            "{} {} {:20}{}",
            modified.format(&format_string),
            self.slug,
            self.model,
            self.summary.trim()
        )
    }
}

//////////////////////////////////////////// ChatOptions ///////////////////////////////////////////

/// Options for the `chat` command.
#[derive(
    Clone, Debug, Eq, PartialEq, arrrg_derive::CommandLine, serde::Deserialize, serde::Serialize,
)]
pub struct ChatOptions {
    /// The host to connect to.
    #[arrrg(optional, "The host to connect to.")]
    pub ollama_host: Option<String>,
    /// The model to use from the ollama library.
    #[arrrg(optional, "The model to use from the ollama library.")]
    pub model: String,
    /// The duration to keep the model in memory for after the call.
    #[arrrg(optional, "Duration to keep the model in memory for after the call.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
    /// The parameters to pass to the model.
    #[arrrg(nested)]
    pub param: Parameters,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            ollama_host: None,
            // TODO(rescrv):  Don't hardcode this.
            model: "gemma2".to_string(),
            keep_alive: None,
            param: Parameters::default(),
        }
    }
}

/////////////////////////////////////////////// Chat ///////////////////////////////////////////////

/// The `chat` command.
#[derive(Clone, Debug)]
pub struct Chat {
    changelog: Path<'static>,
    messages: Vec<ChatMessage>,
    options: ChatOptions,
}

impl Chat {
    /// Create a new chat from a changelog file.  Options will only be used if changelog is None.
    pub fn new(changelog: Option<Path>, options: ChatOptions) -> Result<Self, Error> {
        if let Some(changelog) = changelog.as_ref() {
            std::fs::create_dir_all(changelog.dirname())?;
            let changelog = changelog.clone().into_owned();
            let messages = vec![];
            // Intentionally override so defaults get default value.
            //
            // The contract is that options will only be consulted if the changelog is None, so to
            // not override the options would be incorrect.  This is less incorrect.
            let options = ChatOptions::default();
            let mut this = Self {
                changelog,
                messages,
                options,
            };
            this.load()?;
            Ok(this)
        } else {
            loop {
                let chat_id = chat_id()?;
                let changelog = super::chat_path(&chat_id)?;
                std::fs::create_dir_all(changelog.dirname())?;
                let res = std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&changelog);
                let file = match res {
                    Ok(file) => file,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::AlreadyExists {
                            continue;
                        }
                        return Err(err.into());
                    }
                };
                drop(file);
                let messages = vec![];
                let mut this = Self {
                    changelog,
                    messages,
                    options,
                };
                this.save_options()?;
                break Ok(this);
            }
        }
    }

    /// Apply a chat log line to the chat.
    pub fn apply(&mut self, log_line: ChatLogLine) {
        match log_line {
            ChatLogLine::Message {
                created_at: _,
                message,
            } => self.push(message),
            ChatLogLine::Options {
                created_at: _,
                options,
            } => self.options = options,
            ChatLogLine::Tag {
                created_at: _,
                tag: _,
            } => {}
            ChatLogLine::Untag {
                created_at: _,
                tag: _,
            } => {}
            ChatLogLine::Retry { created_at: _ } => {
                if !self.messages.is_empty() && self.messages.last().unwrap().role == "assistant" {
                    self.truncate(self.messages.len() - 1)
                }
            }
        }
    }

    /// Get the chat messages.
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Truncate the chat.
    pub fn truncate(&mut self, index: usize) {
        self.messages.truncate(index);
    }

    /// Push a ChatMessage onto the chat.
    pub fn push(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Assemble the assistant response into a ChatMessage from the pieces.
    pub fn assemble_assistant_response(pieces: Vec<serde_json::Value>) -> ChatMessage {
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
        ChatMessage {
            role: "assistant".to_string(),
            content,
            images: None,
            tool_calls: None,
        }
    }

    /// Convert the chat into a chat request.
    pub fn into_request(self) -> ChatRequest {
        ChatRequest {
            model: self.options.model,
            messages: self.messages,
            stream: Some(true),
            tools: None,
            format: None,
            keep_alive: self.options.keep_alive,
            options: serde_json::json!({}),
        }
    }

    /// Load the chat log from the log file.
    pub fn load(&mut self) -> Result<(), super::Error> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&self.changelog)?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let log_line = serde_json::from_str(&line)?;
            self.apply(log_line);
        }
        Ok(())
    }

    /// Save the chat log line to the log.
    pub fn log(&mut self, log_line: &ChatLogLine) -> Result<(), super::Error> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.changelog)?;
        let mut line = serde_json::to_string(log_line)?;
        line.push('\n');
        file.write_all(line.as_bytes())?;
        Ok(())
    }

    /// Save the options to the log.
    pub fn save_options(&mut self) -> Result<(), super::Error> {
        self.log(&ChatLogLine::Options {
            created_at: chrono::Local::now(),
            options: self.options.clone(),
        })
    }

    /// Run the interactive chat shell.
    pub async fn shell(mut self) -> Result<(), super::Error> {
        let config = Config::builder()
            .auto_add_history(true)
            .edit_mode(EditMode::Vi)
            .completion_type(CompletionType::List)
            .check_cursor_position(true)
            .max_history_size(1_000_000)
            .expect("this should always work")
            .history_ignore_dups(true)
            .expect("this should always work")
            .history_ignore_space(true)
            .build();
        let history = rustyline::history::FileHistory::new();
        let mut rl = Editor::with_history(config, history).expect("this should always work");
        const PROMPT: &str = ">>> ";
        let commands = vec![
            CommandHint::new(":help", ":help"),
            CommandHint::new(":exit", ":exit"),
            CommandHint::new(":quit", ":quit"),
            CommandHint::new(":edit", ":edit"),
            CommandHint::new(":retry", ":retry"),
            CommandHint::new(":param", ":param"),
        ];
        let h = ShellHelper {
            commands: commands.clone(),
            hinter: HistoryHinter::new(),
            hints: commands.clone(),
        };
        rl.set_helper(Some(h));
        rl.bind_sequence(
            KeyEvent::from('\t'),
            EventHandler::Conditional(Box::new(TabEventHandler)),
        );
        loop {
            let line = rl.readline(PROMPT);
            let (args, line) = match line {
                Ok(line) => {
                    let args = match shvar::split(&line) {
                        Ok(args) => args,
                        Err(err) => {
                            eprintln!("could not split line: {:?}", err);
                            continue;
                        }
                    };
                    if args.is_empty() {
                        continue;
                    }
                    (args, line)
                }
                Err(ReadlineError::Interrupted) => {
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    break Ok(());
                }
                Err(err) => {
                    eprintln!("could not read line: {}", err);
                    continue;
                }
            };
            let log_line = match args[0].as_str() {
                ":exit" | ":quit" | ":wq" | ":q" => {
                    break Ok(());
                }
                ":help" => {
                    eprintln!(
                        r#"chat
====

Commands:

:model <model>  Set the model to use.
:edit           Edit and send the next message.
:retry          Retry the last message.
:param          Set parameters for the model (e.g. --temperature 0.5).

Anything else will be interpreted as a message.
"#
                    );
                    continue;
                }
                ":model" => {
                    if args.len() < 2 {
                        eprintln!("expected model name");
                        continue;
                    }
                    if args.len() > 2 {
                        eprintln!("expected only model name");
                        continue;
                    }
                    self.options.model = args[1].clone();
                    self.save_options()?;
                    continue;
                }
                ":edit" => {
                    let promptfile = match crate::editor() {
                        Ok(promptfile) => promptfile,
                        Err(err) => {
                            eprintln!("could not edit: {:?}", err);
                            continue;
                        }
                    };
                    let prompt = std::fs::read_to_string(promptfile.as_ref())?;
                    writeln!(
                        std::io::stdout(),
                        "{}",
                        prompt
                            .split_terminator('\n')
                            .map(|x| "... ".to_string() + x)
                            .collect::<Vec<_>>()
                            .join("\n")
                    )?;
                    ChatLogLine::Message {
                        created_at: chrono::Local::now(),
                        message: ChatMessage {
                            role: "user".to_string(),
                            content: prompt,
                            images: None,
                            tool_calls: None,
                        },
                    }
                }
                ":retry" => ChatLogLine::Retry {
                    created_at: chrono::Local::now(),
                },
                ":param" => {
                    let args = args.iter().map(|x| x.as_str()).collect::<Vec<_>>();
                    if args.len() < 2 {
                        println!("{:#?}", self.options.param);
                        continue;
                    }
                    let (param, free) = NoExitCommandLine::<Parameters>::from_arguments_relaxed(
                        ":param --key value",
                        &args[1..],
                    );
                    let (param, errors, status) = param.into_parts();
                    if status != 0 {
                        for error in errors {
                            eprintln!("{}", error);
                        }
                        continue;
                    }
                    if !free.is_empty() {
                        eprintln!("command takes no positional arguments");
                        continue;
                    }
                    self.options.param.apply(param);
                    self.save_options()?;
                    continue;
                }
                _ => {
                    if !args[0].starts_with(':') {
                        ChatLogLine::Message {
                            created_at: chrono::Local::now(),
                            message: ChatMessage {
                                role: "user".to_string(),
                                content: line,
                                images: None,
                                tool_calls: None,
                            },
                        }
                    } else {
                        eprintln!("unknown command: {}", args[0]);
                        continue;
                    }
                }
            };
            self.log(&log_line)?;
            self.apply(log_line);
            let req = self.clone().into_request();
            let req = req.make_request(&super::ollama_host(self.options.ollama_host.clone()));
            let spinner = Spinner::new();
            spinner.start();
            let mut pieces = vec![];
            let mut ww = WordWrap::new(100);
            let res = crate::stream(req, |resp| {
                spinner.inhibit();
                if let Some(serde_json::Value::Object(message)) = resp.get("message") {
                    if let Some(serde_json::Value::String(content)) = message.get("content") {
                        ww.push(content.clone(), &mut std::io::stdout())?;
                    }
                }
                pieces.push(resp);
                Ok(())
            })
            .await;
            spinner.inhibit();
            println!();
            if let Err(Error::Signal) = res {
                continue;
            } else if let Err(err) = res {
                eprintln!("could not chat: {:?}", err);
                continue;
            };
            let log_line = ChatLogLine::Message {
                created_at: chrono::Local::now(),
                message: Self::assemble_assistant_response(pieces),
            };
            self.log(&log_line)?;
            self.apply(log_line);
        }
    }
}

////////////////////////////////////////////// chat_id /////////////////////////////////////////////

pub fn chat_id() -> Result<String, Error> {
    let s = RandomState::new();
    let mut random = s.hash_one(Instant::now());
    const BASE20: [char; 20] = [
        '2', '3', '4', '5', '6', '7', '8', '9', 'C', 'F', 'G', 'H', 'J', 'M', 'P', 'Q', 'R', 'V',
        'W', 'X',
    ];
    let mut id = String::new();
    for _ in 0..5 {
        let idx = random % 20;
        random /= 20;
        id.push(BASE20[idx as usize]);
    }
    Ok(id)
}
