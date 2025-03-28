use rustyline::config::EditMode;
use rustyline::error::ReadlineError;
use rustyline::hint::HistoryHinter;
use rustyline::{Cmd, CompletionType, Config, Editor, EventHandler, KeyEvent};
use utf8path::Path;

use crate::chat::{Chat, ChatLogLine, ChatOptions, ChatSummary};
use crate::cli::{CommandHint, ShellHelper, TabEventHandler};
use crate::types::ChatMessage;
use crate::{Error, Parameters};

/////////////////////////////////////////// ChatsOptions ///////////////////////////////////////////

/// CommandLine options for the `chats` command.
#[derive(Clone, Debug, Eq, PartialEq, arrrg_derive::CommandLine)]
pub struct ChatsOptions {
    /// The host to connect to.
    #[arrrg(optional, "The host to connect to.")]
    pub ollama_host: Option<String>,
    /// The model to use from the ollama library.
    #[arrrg(optional, "The model to use from the ollama library.")]
    pub model: String,
    /// The duration to keep the model in memory for after the call.
    #[arrrg(optional, "Duration to keep the model in memory for after the call.")]
    pub keep_alive: Option<String>,
    /// The parameters to pass to the model.
    #[arrrg(nested)]
    pub param: Parameters,
    /// Number of results to return per section.
    #[arrrg(optional, "Number of results to return per section.")]
    paginate: usize,
    /// Number of messages to replay when continuing a chat.
    #[arrrg(optional, "Number of messages to replay when continuing a chat.")]
    replay: usize,
}

impl Default for ChatsOptions {
    fn default() -> Self {
        Self {
            ollama_host: None,
            // TODO(rescrv): don't hard-code
            model: "gemma2".to_string(),
            keep_alive: None,
            param: Parameters::default(),
            paginate: 10,
            replay: 5,
        }
    }
}

impl From<ChatsOptions> for ChatOptions {
    fn from(options: ChatsOptions) -> Self {
        Self {
            ollama_host: options.ollama_host,
            model: options.model,
            keep_alive: options.keep_alive,
            param: options.param,
        }
    }
}

/////////////////////////////////////////////// Chats //////////////////////////////////////////////

/// The `chats` command.
pub struct Chats {
    options: ChatsOptions,
}

impl Chats {
    /// Create a new `Chats` command.
    pub fn new(options: ChatsOptions) -> Result<Self, Error> {
        Ok(Self { options })
    }

    /// Run the `chats` interactive shell.
    pub async fn shell(self) -> Result<(), Error> {
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
        const PROMPT: &str = "> ";
        let commands = vec![
            CommandHint::new("help", "help"),
            CommandHint::new("exit", "exit"),
            CommandHint::new("quit", "quit"),
            CommandHint::new("list", "list"),
            CommandHint::new("archive", "archive"),
            CommandHint::new("unarchive", "unarchive"),
            CommandHint::new("archived", "archived"),
            CommandHint::new("pin", "pin"),
            CommandHint::new("unpin", "unpin"),
            CommandHint::new("pinned", "pinned"),
            CommandHint::new("new", "new"),
            CommandHint::new("chat", "chat"),
            CommandHint::new("editor", "editor"),
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
        rl.bind_sequence(KeyEvent::ctrl('l'), EventHandler::Simple(Cmd::ClearScreen));
        self.list();
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
            match args[0].as_str() {
                "exit" | "quit" | ":wq" | ":q" => {
                    break Ok(());
                }
                "help" => {
                    eprintln!(
                        r#"chats
=====

Commands:

list        Show all chats.
archive     Archive a chat.
unarchive   Unarchive a chat.
archived    Show all archived chats.
pin         Pin a chat.
unpin       Unpin a chat.
pinned      Show all pinned chats.
new         Start a new chat.
chat        Continue a chat.
editor      Start a chat with a system message written in EDITOR.
"#
                    );
                    continue;
                }
                "list" => {
                    self.list();
                }
                "archive" => {
                    if args.len() != 2 {
                        eprintln!("USAGE: archive <chat>");
                        continue;
                    }
                    self.archive(&args[1]);
                }
                "unarchive" => {
                    if args.len() != 2 {
                        eprintln!("USAGE: unarchive <chat>");
                        continue;
                    }
                    self.unarchive(&args[1]);
                }
                "archived" => {
                    if args.len() != 1 {
                        eprintln!("USAGE: archived");
                        continue;
                    }
                    self.archived();
                }
                "pin" => {
                    if args.len() != 2 {
                        eprintln!("USAGE: pin <chat>");
                        continue;
                    }
                    self.pin(&args[1]);
                }
                "unpin" => {
                    if args.len() != 2 {
                        eprintln!("USAGE: unpin <chat>");
                        continue;
                    }
                    self.unpin(&args[1]);
                }
                "pinned" => {
                    if args.len() != 1 {
                        eprintln!("USAGE: pinned");
                        continue;
                    }
                    self.pinned();
                }
                "new" => {
                    self.new_chat(&line, args).await;
                }
                "chat" => {
                    if args.len() != 2 {
                        eprintln!("USAGE: open <chat>");
                        continue;
                    }
                    self.continue_chat(&args[1]).await;
                }
                "editor" => {
                    if args.len() != 1 {
                        eprintln!("USAGE: editor");
                        continue;
                    }
                    self.editor_chat().await;
                }
                "copy" => {
                    if args.len() != 2 {
                        eprintln!("USAGE: copy <chat>");
                        continue;
                    }
                    let from = match super::chat_path(&args[1]) {
                        Ok(path) => path,
                        Err(err) => {
                            eprintln!("could not copy: {err}");
                            continue;
                        }
                    };
                    let chat_id = match crate::chat::chat_id() {
                        Ok(chat_id) => chat_id,
                        Err(err) => {
                            eprintln!("could not generate chat id: {err}");
                            continue;
                        }
                    };
                    let to = match super::chat_path(&chat_id) {
                        Ok(path) => path,
                        Err(err) => {
                            eprintln!("could not copy: {err}");
                            continue;
                        }
                    };
                    std::fs::create_dir_all(to.dirname())?;
                    if let Err(err) = std::fs::copy(from, to) {
                        eprintln!("could not copy: {err}");
                        continue;
                    }
                }
                _ => {
                    eprintln!("unknown command: {}", args[0]);
                    continue;
                }
            };
        }
    }

    fn load(&self) -> Result<Vec<ChatSummary>, Error> {
        let chat_root = super::chat_root()?;
        let dirents = std::fs::read_dir(chat_root.join("chats"))?;
        let mut chats = vec![];
        for dirent in dirents {
            let dirent = dirent?;
            let path = match Path::try_from(dirent.path()) {
                Ok(path) => path,
                Err(err) => {
                    eprintln!("could not convert path: {:?}", err);
                    continue;
                }
            };
            if path.as_str().ends_with(".ndjson") {
                chats.push(path);
            }
        }
        let mut summaries = Vec::with_capacity(chats.len());
        for chat in chats {
            summaries.push(ChatSummary::load(&chat)?);
        }
        Ok(summaries)
    }

    fn display(&self, mut summaries: Vec<ChatSummary>) {
        summaries.sort_by_key(|chat| chat.modified);
        summaries.reverse();
        let mut pinned_first = true;
        for summary in summaries
            .iter()
            .filter(|s| s.tags.contains("pinned"))
            .take(self.options.paginate)
        {
            if pinned_first {
                println!("pinned:");
            }
            pinned_first = false;
            println!("{}", summary);
        }
        let mut first = true;
        for summary in summaries
            .iter()
            .filter(|s| !s.tags.contains("pinned"))
            .take(self.options.paginate)
        {
            if first {
                if !pinned_first {
                    println!();
                }
                println!("recent:");
            }
            first = false;
            println!("{}", summary);
        }
    }

    fn list(&self) {
        let mut summaries = match self.load() {
            Ok(summaries) => summaries,
            Err(err) => {
                eprintln!("could not load chat summaries: {:?}", err);
                return;
            }
        };
        summaries.retain(|s| !s.tags.contains("archived"));
        self.display(summaries);
    }

    fn chat_for_slug(&self, slug: &str) -> Result<Chat, Error> {
        let chat_root = super::chat_root()?;
        let chat = chat_root.join("chats").join(format!("{}.ndjson", slug));
        Chat::new(Some(chat), ChatOptions::default())
    }

    fn tag(&self, slug: &str, tag: &str) {
        let mut chat = match self.chat_for_slug(slug) {
            Ok(chat) => chat,
            Err(err) => {
                eprintln!("could not load chat: {:?}", err);
                return;
            }
        };
        if let Err(err) = chat.log(&ChatLogLine::Tag {
            created_at: chrono::Local::now(),
            tag: tag.to_string(),
        }) {
            eprintln!("could not tag chat as {}: {:?}", tag, err);
        }
    }

    fn untag(&self, slug: &str, tag: &str) {
        let mut chat = match self.chat_for_slug(slug) {
            Ok(chat) => chat,
            Err(err) => {
                eprintln!("could not load chat: {:?}", err);
                return;
            }
        };
        if let Err(err) = chat.log(&ChatLogLine::Untag {
            created_at: chrono::Local::now(),
            tag: tag.to_string(),
        }) {
            eprintln!("could not untag chat as {}: {:?}", tag, err);
        }
    }

    fn archive(&self, slug: &str) {
        self.tag(slug, "archived");
    }

    fn unarchive(&self, slug: &str) {
        self.untag(slug, "archived");
    }

    fn archived(&self) {
        let mut summaries = match self.load() {
            Ok(summaries) => summaries,
            Err(err) => {
                eprintln!("could not load chat summaries: {:?}", err);
                return;
            }
        };
        summaries.sort_by_key(|chat| chat.modified);
        summaries.retain(|s| s.tags.contains("archived"));
        self.display(summaries);
    }

    fn pin(&self, slug: &str) {
        self.tag(slug, "pinned");
    }

    fn unpin(&self, slug: &str) {
        self.untag(slug, "pinned");
    }

    fn pinned(&self) {
        let mut summaries = match self.load() {
            Ok(summaries) => summaries,
            Err(err) => {
                eprintln!("could not load chat summaries: {:?}", err);
                return;
            }
        };
        summaries.sort_by_key(|chat| chat.modified);
        summaries.retain(|s| !s.tags.contains("archived") && s.tags.contains("pinned"));
        for summary in summaries.iter().take(self.options.paginate) {
            println!("{}", summary);
        }
    }

    async fn new_chat(&self, line: &str, args: Vec<String>) {
        let mut chat = match Chat::new(None, self.options.clone().into()) {
            Ok(chat) => chat,
            Err(err) => {
                eprintln!("could not create chat: {:?}", err);
                return;
            }
        };
        if args.len() != 1 {
            // SAFETY(rescrv):  This is safe because we know that args[0] is "new".
            let prompt = line.strip_prefix("new").unwrap().trim();
            let system = ChatLogLine::Message {
                created_at: chrono::Local::now(),
                message: ChatMessage {
                    role: "system".to_string(),
                    content: prompt.to_string(),
                    images: None,
                    tool_calls: None,
                },
            };
            if let Err(err) = chat.log(&system) {
                eprintln!("could not log system message: {:?}", err);
                return;
            }
            chat.apply(system);
        }
        if let Err(err) = chat.shell().await {
            eprintln!("could not chat: {:?}", err);
        }
    }

    async fn continue_chat(&self, chat: &str) {
        let chat = match self.chat_for_slug(chat) {
            Ok(chat) => chat,
            Err(err) => {
                eprintln!("could not load chat: {:?}", err);
                return;
            }
        };
        let replay = std::cmp::min(self.options.replay, chat.messages().len());
        for message in chat.messages().iter().rev().take(replay).rev() {
            match message.role.as_str() {
                "system" => {
                    if message.content.contains('\n') {
                        println!("SYSTEM \"\"\"\n{}\n\"\"\"", message.content);
                    } else {
                        println!("SYSTEM {}", message.content);
                    }
                }
                "user" => {
                    let lines = message.content.lines().enumerate().map(|(idx, line)| {
                        if idx == 0 {
                            format!(">>> {}", line)
                        } else {
                            format!("... {}", line)
                        }
                    });
                    println!("{}", lines.collect::<Vec<_>>().join("\n"));
                }
                "assistant" => {
                    println!("{}", message.content);
                }
                _ => {
                    eprintln!("unknown role: {}", message.role);
                    continue;
                }
            }
        }
        if let Err(err) = chat.shell().await {
            eprintln!("could not chat: {:?}", err);
        }
    }

    async fn editor_chat(&self) {
        let mut chat = match Chat::new(None, self.options.clone().into()) {
            Ok(chat) => chat,
            Err(err) => {
                eprintln!("could not create chat: {:?}", err);
                return;
            }
        };
        let system = match crate::editor("This will become the system prompt.") {
            Ok(system) => system,
            Err(err) => {
                eprintln!("could not get system message: {:?}", err);
                return;
            }
        };
        let content = match std::fs::read_to_string(system.as_ref()) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("could not read system message: {:?}", err);
                return;
            }
        };
        let system = ChatLogLine::Message {
            created_at: chrono::Local::now(),
            message: ChatMessage {
                role: "system".to_string(),
                content,
                images: None,
                tool_calls: None,
            },
        };
        if let Err(err) = chat.log(&system) {
            eprintln!("could not log system message: {:?}", err);
            return;
        }
        chat.apply(system);
        if let Err(err) = chat.shell().await {
            eprintln!("could not chat: {:?}", err);
        }
    }
}
