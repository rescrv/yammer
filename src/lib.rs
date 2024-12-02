#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use reqwest::RequestBuilder;
use utf8path::Path;

mod chat;
mod chats;
mod cli;
mod types;
mod wrap;

pub use chat::{Chat, ChatOptions};
pub use chats::{Chats, ChatsOptions};
pub use types::{ChatMessage, ChatRequest, ChatResponse, GenerateRequest, GenerateResponse};
pub use wrap::WordWrap;

///////////////////////////////////////////// constants ////////////////////////////////////////////

/// The default host to connect to.
pub const OLLAMA_HOST: &str = "http://localhost:11434";

/////////////////////////////////////////////// Error //////////////////////////////////////////////

/// An error that can occur when interacting with the ollama API.
#[derive(Debug)]
pub enum Error {
    /// An Internal error occurred.
    Internal,
    /// A signal interrupted the call.
    Signal,
    /// The EDITOR environment variable is not set.
    EditorNotSet,
    /// EDITOR failed.
    EditorFailed(Option<i32>),
    /// The YAMMER_CHAT environment variable is not set.
    ChatNotSet,
    /// An invalid argument was passed.
    InvalidArgument(String),
    /// An error occurred in the ollama service.
    Ollama(String),
    /// An I/O error occurred.
    Io(std::io::Error),
    /// A UTF-8 error occurred.
    Utf8Error(std::str::Utf8Error),
    /// A JSON error occurred.
    Json(serde_json::Error),
    /// A Reqwest error occurred.
    Reqwest(reqwest::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Internal => write!(f, "Internal error"),
            Self::Signal => write!(f, "Signal received"),
            Self::EditorNotSet => write!(f, "EDITOR not set"),
            Self::EditorFailed(Some(code)) => write!(f, "Editor failed with exit code {}", code),
            Self::EditorFailed(None) => write!(f, "Editor failed without exit code"),
            Self::ChatNotSet => write!(f, "YAMMER_CHAT not set"),
            Self::InvalidArgument(message) => write!(f, "invalid argument: {}", message),
            Self::Ollama(s) => write!(f, "Ollama error: {}", s),
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
            Self::Json(e) => write!(f, "JSON error: {}", e),
            Self::Reqwest(e) => write!(f, "Reqwest error: {}", e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

//////////////////////////////////////////// Parameters ////////////////////////////////////////////

/// Parameters for the model.
///
/// These correspond to the same name as PARAMETER options in Ollama.
#[derive(
    Clone, Debug, Default, arrrg_derive::CommandLine, serde::Deserialize, serde::Serialize,
)]
pub struct Parameters {
    /// Enable Mirostat sampling for controlling perplexity. (default: 0, 0 = disabled, 1 = Mirostat, 2 = Mirostat 2.0)
    #[arrrg(optional, "Enable Mirostat sampling for controlling perplexity. (default: 0, 0 = disabled, 1 = Mirostat, 2 = Mirostat 2.0)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirostat: Option<i32>,

    /// Influences how quickly the algorithm responds to feedback from the generated text.
    ///
    /// A lower learning rate will result in slower adjustments, while a higher learning rate will
    /// make the algorithm more responsive. (Default: 0.1)
    #[arrrg(
        optional,
        "Influences how quickly the algorithm responds to feedback from the generated text."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirostat_eta: Option<f64>,

    /// Controls the balance between coherence and diversity of the output.
    ///
    /// A lower value will result in more focused and coherent text. (Default: 5.0)
    #[arrrg(
        optional,
        "Controls the balance between coherence and diversity of the output."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirostat_tau: Option<f64>,

    /// The number of tokens worth of context to allocate.
    #[arrrg(optional, "The number of tokens worth of context to allocate.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<u32>,

    /// Sets how far back for the model to look back to prevent repetition.
    ///
    /// (Default: 64, 0 = disabled, -1 = num_ctx)
    #[arrrg(
        optional,
        "Sets how far back for the model to look back to prevent repetition."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_last_n: Option<i32>,

    /// Sets how strongly to penalize repetitions.
    ///
    /// A higher value (e.g., 1.5) will penalize repetitions more strongly, while a lower value
    /// (e.g., 0.9) will be more lenient. (Default: 1.1)
    #[arrrg(optional, "Sets how strongly to penalize repetitions.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f64>,

    /// The temperature of the model.
    ///
    /// Increasing the temperature will make the model answer more creatively. (Default: 0.8)
    #[arrrg(optional, "The temperature of the model.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Sets the random number seed to use for generation.
    ///
    /// Setting this to a specific number will make the model generate the same text for the same
    /// prompt.  (Default: 0)
    #[arrrg(optional, "Sets the random number seed to use for generation.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,

    /// Tail free sampling is used to reduce the impact of less probable tokens from the output.
    ///
    /// A higher value (e.g., 2.0) will reduce the impact more, while a value of 1.0 disables this
    /// setting. (default: 1)
    #[arrrg(
        optional,
        "Tail free sampling is used to reduce the impact of less probable tokens from the output."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tfs_z: Option<f64>,

    /// Maximum number of tokens to predict when generating text.
    ///
    /// (Default: 128, -1 = infinite generation, -2 = fill context)
    #[arrrg(optional, "Maximum number of tokens to predict when generating text.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,

    /// Reduces the probability of generating nonsense.
    ///
    /// A higher value (e.g. 100) will give more diverse answers, while a lower value (e.g. 10)
    /// will be more conservative. (Default: 40)
    #[arrrg(optional, "Reduces the probability of generating nonsense.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,

    /// Works together with top-k.
    ///
    /// A higher value (e.g., 0.95) will lead to more diverse text, while a lower value (e.g., 0.5)
    /// will generate more focused and conservative text. (Default: 0.9)
    #[arrrg(optional, "Works together with top-k.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Alternative to the top_p, and aims to ensure a balance of quality and variety.
    ///
    /// The parameter p represents the minimum probability for a token to be considered, relative
    /// to the probability of the most likely token. For example, with p=0.05 and the most likely
    /// token having a probability of 0.9, logits with a value less than 0.045 are filtered out.
    /// (Default: 0.0)
    #[arrrg(
        optional,
        "Alternative to the top_p, and aims to ensure a balance of quality and variety."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f64>,
}

impl Parameters {
    /// Overlay the parameters from another set of parameters.
    pub fn apply(&mut self, from: Self) {
        if let Some(mirostat) = from.mirostat {
            self.mirostat = Some(mirostat);
        }
        if let Some(mirostat_eta) = from.mirostat_eta {
            self.mirostat_eta = Some(mirostat_eta);
        }
        if let Some(mirostat_tau) = from.mirostat_tau {
            self.mirostat_tau = Some(mirostat_tau);
        }
        if let Some(num_ctx) = from.num_ctx {
            self.num_ctx = Some(num_ctx);
        }
        if let Some(repeat_last_n) = from.repeat_last_n {
            self.repeat_last_n = Some(repeat_last_n);
        }
        if let Some(repeat_penalty) = from.repeat_penalty {
            self.repeat_penalty = Some(repeat_penalty);
        }
        if let Some(temperature) = from.temperature {
            self.temperature = Some(temperature);
        }
        if let Some(seed) = from.seed {
            self.seed = Some(seed);
        }
        if let Some(tfs_z) = from.tfs_z {
            self.tfs_z = Some(tfs_z);
        }
        if let Some(num_predict) = from.num_predict {
            self.num_predict = Some(num_predict);
        }
        if let Some(top_k) = from.top_k {
            self.top_k = Some(top_k);
        }
        if let Some(top_p) = from.top_p {
            self.top_p = Some(top_p);
        }
        if let Some(min_p) = from.min_p {
            self.min_p = Some(min_p);
        }
    }
}

impl From<Parameters> for serde_json::Value {
    fn from(p: Parameters) -> serde_json::Value {
        let mut json = serde_json::json!({});
        if let Some(mirostat) = p.mirostat {
            json["mirostat"] = serde_json::json!(mirostat);
        }
        if let Some(mirostat_eta) = p.mirostat_eta {
            json["mirostat_eta"] = serde_json::json!(mirostat_eta);
        }
        if let Some(mirostat_tau) = p.mirostat_tau {
            json["mirostat_tau"] = serde_json::json!(mirostat_tau);
        }
        if let Some(num_ctx) = p.num_ctx {
            json["num_ctx"] = serde_json::json!(num_ctx);
        }
        if let Some(repeat_last_n) = p.repeat_last_n {
            json["repeat_last_n"] = serde_json::json!(repeat_last_n);
        }
        if let Some(repeat_penalty) = p.repeat_penalty {
            json["repeat_penalty"] = serde_json::json!(repeat_penalty);
        }
        if let Some(temperature) = p.temperature {
            json["temperature"] = serde_json::json!(temperature);
        }
        if let Some(seed) = p.seed {
            json["seed"] = serde_json::json!(seed);
        }
        if let Some(tfs_z) = p.tfs_z {
            json["tfs_z"] = serde_json::json!(tfs_z);
        }
        if let Some(num_predict) = p.num_predict {
            json["num_predict"] = serde_json::json!(num_predict);
        }
        if let Some(top_k) = p.top_k {
            json["top_k"] = serde_json::json!(top_k);
        }
        if let Some(top_p) = p.top_p {
            json["top_p"] = serde_json::json!(top_p);
        }
        if let Some(min_p) = p.min_p {
            json["min_p"] = serde_json::json!(min_p);
        }
        json
    }
}

impl Eq for Parameters {}

impl PartialEq for Parameters {
    fn eq(&self, other: &Self) -> bool {
        self.mirostat == other.mirostat
            && self.mirostat_eta == other.mirostat_eta
            && self.mirostat_tau == other.mirostat_tau
            && self.num_ctx == other.num_ctx
            && self.repeat_last_n == other.repeat_last_n
            && self.repeat_penalty == other.repeat_penalty
            && self.temperature == other.temperature
            && self.seed == other.seed
            && self.tfs_z == other.tfs_z
            && self.num_predict == other.num_predict
            && self.top_k == other.top_k
            && self.top_p == other.top_p
            && self.min_p == other.min_p
    }
}

////////////////////////////////////////////// Shellm //////////////////////////////////////////////

/// Options for the `shellm` command.
#[derive(Clone, Debug, Eq, PartialEq, arrrg_derive::CommandLine)]
pub struct ShellmOptions {
    /// The host to connect to.
    #[arrrg(optional, "The host to connect to.")]
    pub ollama_host: Option<String>,
    /// The model to use from the ollama library.
    #[arrrg(optional, "The model to use from the ollama library.")]
    pub model: String,
    /// The suffix to append to the response.
    #[arrrg(optional, "The suffix to append to the response.")]
    pub suffix: String,
    /// The system to use in the template.
    #[arrrg(optional, "The system to use in the template.")]
    pub system: Option<String>,
    /// The template to use for the prompt.
    #[arrrg(optional, "The template to use for the prompt.")]
    pub template: Option<String>,
    /// Format the response in JSON.  You must also ask the model to do so.
    #[arrrg(
        flag,
        "Format the response in JSON.  You must also ask the model to do so."
    )]
    pub json: bool,
    /// Whether to pass bypass formatting of the prompt.
    #[arrrg(optional, "Whether to pass bypass formatting of the prompt.")]
    pub raw: Option<bool>,
    /// Duration to keep the model in memory for after the call.
    #[arrrg(optional, "Duration to keep the model in memory for after the call.")]
    pub keep_alive: Option<String>,
    /// Additional options to pass to the model.
    #[arrrg(nested)]
    pub param: Parameters,
}

impl Default for ShellmOptions {
    fn default() -> Self {
        ShellmOptions {
            ollama_host: None,
            // TODO(rescrv):  Don't hard-code the default model.
            model: "gemma2".to_string(),
            suffix: "".to_string(),
            system: None,
            template: None,
            json: false,
            raw: None,
            keep_alive: None,
            param: Parameters::default(),
        }
    }
}

////////////////////////////////////////////// shellm //////////////////////////////////////////////

/// The `shellm` command.
pub async fn shellm(
    options: ShellmOptions,
    promptfiles: &[impl AsRef<str>],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin: Option<String> = None;
    for promptfile in promptfiles {
        let promptfile = promptfile.as_ref();
        let prompt = if promptfile == "-" {
            if let Some(stdin) = stdin.as_ref() {
                stdin.clone()
            } else {
                let mut s = String::new();
                std::io::stdin().read_to_string(&mut s)?;
                stdin = Some(s.clone());
                s
            }
        } else {
            match std::fs::read_to_string(promptfile) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("shellm: {}: {}", promptfile, e);
                    continue;
                }
            }
        };
        let gen = GenerateRequest {
            model: options.model.clone(),
            prompt,
            suffix: options.suffix.clone(),
            images: None,
            format: if options.json {
                Some("json".to_string())
            } else {
                None
            },
            system: options.system.clone(),
            template: options.template.clone(),
            stream: Some(true),
            raw: options.raw,
            keep_alive: None,
            options: options.param.clone().into(),
        };
        let req = gen.make_request(&ollama_host(options.ollama_host.clone()));
        let mut ww = WordWrap::new(100);
        let res = stream(req, |v| {
            if let Some(serde_json::Value::String(message)) = v.get("response") {
                ww.push(message.clone(), &mut std::io::stdout())?;
            }
            Ok(())
        })
        .await;
        if let Err(Error::Signal) = res {
            break;
        } else if let Err(err) = res {
            return Err(err.into());
        }
        writeln!(std::io::stdout())?;
    }
    Ok(())
}

////////////////////////////////////////// OneShotOptions //////////////////////////////////////////

/// Options for the `oneshot` command.
#[derive(Clone, Debug, Eq, PartialEq, arrrg_derive::CommandLine)]
pub struct OneshotOptions {
    /// The host to connect to.
    #[arrrg(optional, "The host to connect to.")]
    pub ollama_host: Option<String>,
    /// The suffix to append to the response.
    #[arrrg(optional, "The suffix to append to the response.")]
    pub suffix: String,
    /// The system to use in the template.
    #[arrrg(optional, "The system to use in the template.")]
    pub system: Option<String>,
    /// The template to use for the prompt.
    #[arrrg(optional, "The template to use for the prompt.")]
    pub template: Option<String>,
    /// Format the response in JSON.  You must also ask the model to do so.
    #[arrrg(
        flag,
        "Format the response in JSON.  You must also ask the model to do so."
    )]
    pub json: bool,
    /// Whether to pass bypass formatting of the prompt.
    #[arrrg(optional, "Whether to pass bypass formatting of the prompt.")]
    pub raw: Option<bool>,
    /// Duration to keep the model in memory for after the call.
    #[arrrg(optional, "Duration to keep the model in memory for after the call.")]
    pub keep_alive: Option<String>,
    /// Additional options to pass to the model.
    #[arrrg(nested)]
    pub param: Parameters,
}

impl Default for OneshotOptions {
    fn default() -> Self {
        OneshotOptions {
            ollama_host: None,
            suffix: "".to_string(),
            system: None,
            template: None,
            json: false,
            raw: None,
            keep_alive: None,
            param: Parameters::default(),
        }
    }
}

////////////////////////////////////////////// editor //////////////////////////////////////////////

fn editor() -> Result<impl AsRef<String>, Error> {
    let path = format!(
        ".yammer.{}.{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_micros()
    );
    let editor = std::env::var("EDITOR").map_err(|_| Error::EditorNotSet)?;
    struct UnlinkOnDrop(String);
    impl Drop for UnlinkOnDrop {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    impl AsRef<String> for UnlinkOnDrop {
        fn as_ref(&self) -> &String {
            &self.0
        }
    }
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    let unlink = UnlinkOnDrop(path.clone());
    file.flush()?;
    file.sync_all()?;
    drop(file);
    let status = std::process::Command::new(editor).arg(&path).status()?;
    if Some(0) != status.code() {
        return Err(Error::EditorFailed(status.code()));
    }
    Ok(unlink)
}

////////////////////////////////////////////// oneshot /////////////////////////////////////////////

/// The `oneshot` command.
pub async fn oneshot(
    options: OneshotOptions,
    models: &[impl AsRef<str>],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = editor()?;
    for model in models {
        let options = ShellmOptions {
            ollama_host: options.ollama_host.clone(),
            model: model.as_ref().to_string(),
            suffix: options.suffix.clone(),
            system: options.system.clone(),
            template: options.template.clone(),
            json: options.json,
            raw: options.raw,
            keep_alive: options.keep_alive.clone(),
            param: options.param.clone(),
        };
        shellm(options, &[path.as_ref()]).await?;
    }
    Ok(())
}

/////////////////////////////////////////// PromptOptions //////////////////////////////////////////

/// Options for the `prompt` command.
#[derive(Clone, Debug, Eq, PartialEq, arrrg_derive::CommandLine)]
pub struct PromptOptions {
    /// The host to connect to.
    #[arrrg(optional, "The host to connect to.")]
    pub ollama_host: Option<String>,
    /// The model to use from the ollama library.
    #[arrrg(optional, "The model to use from the ollama library.")]
    pub model: String,
    /// The suffix to append to the response.
    #[arrrg(optional, "The suffix to append to the response.")]
    pub suffix: String,
    /// The system to use in the template.
    #[arrrg(optional, "The system to use in the template.")]
    pub system: Option<String>,
    /// The template to use for the prompt.
    #[arrrg(optional, "The template to use for the prompt.")]
    pub template: Option<String>,
    /// Format the response in JSON.  You must also ask the model to do so.
    #[arrrg(
        flag,
        "Format the response in JSON.  You must also ask the model to do so."
    )]
    pub json: bool,
    /// Whether to pass bypass formatting of the prompt.
    #[arrrg(optional, "Whether to pass bypass formatting of the prompt.")]
    pub raw: Option<bool>,
    /// Duration to keep the model in memory for after the call.
    #[arrrg(optional, "Duration to keep the model in memory for after the call.")]
    pub keep_alive: Option<String>,
    /// Additional options to pass to the model.
    #[arrrg(nested)]
    pub param: Parameters,
}

impl Default for PromptOptions {
    fn default() -> Self {
        PromptOptions {
            ollama_host: None,
            model: "gemma2".to_string(),
            suffix: "".to_string(),
            system: None,
            template: None,
            json: false,
            raw: None,
            keep_alive: None,
            param: Parameters::default(),
        }
    }
}

////////////////////////////////////////////// Prompt //////////////////////////////////////////////

/// The `prompt` command.
pub async fn prompt(
    options: PromptOptions,
    prompts: &[impl AsRef<str>],
) -> Result<(), Box<dyn std::error::Error>> {
    for prompt in prompts {
        let gen = GenerateRequest {
            model: options.model.clone(),
            prompt: prompt.as_ref().to_string(),
            suffix: options.suffix.clone(),
            images: None,
            format: if options.json {
                Some("json".to_string())
            } else {
                None
            },
            system: options.system.clone(),
            template: options.template.clone(),
            stream: Some(true),
            raw: options.raw,
            keep_alive: None,
            options: options.param.clone().into(),
        };
        let req = gen.make_request(&ollama_host(options.ollama_host.clone()));
        let mut ww = WordWrap::new(100);
        let res = stream(req, |v| {
            if let Some(serde_json::Value::String(message)) = v.get("response") {
                ww.push(message.clone(), &mut std::io::stdout())?;
            }
            Ok(())
        })
        .await;
        if let Err(Error::Signal) = res {
            break;
        } else if let Err(err) = res {
            return Err(err.into());
        }
        writeln!(std::io::stdout())?;
    }
    Ok(())
}

//////////////////////////////////////////// chat_shell ////////////////////////////////////////////

/// Start the `chat` shell.
pub async fn chat_shell(changelog: Option<Path<'_>>, options: ChatOptions) -> Result<(), Error> {
    let chat = Chat::new(changelog, options)?;
    chat.shell().await
}

//////////////////////////////////////////// chats_shell ///////////////////////////////////////////

/// Start the `chats` shell.
pub async fn chats_shell(options: ChatsOptions) -> Result<(), Error> {
    let chats = Chats::new(options)?;
    chats.shell().await
}

////////////////////////////////////////////// stream //////////////////////////////////////////////

/// Stream the response of a request, calling `for_each` on each JSON object in the response.
pub async fn stream(
    req: RequestBuilder,
    for_each: impl FnMut(serde_json::Value) -> Result<(), Error>,
) -> Result<(), Error> {
    let sns = stream_no_signal(req, for_each);
    let sig = async {
        loop {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if minimal_signals::pending()
                .iter()
                .filter(|s| *s != minimal_signals::SIGCHLD)
                .count()
                > 0
            {
                break;
            }
        }
    };
    tokio::select! {
        res = sns => res,
        _ = sig => Err(Error::Signal),
    }
}

async fn stream_no_signal(
    req: RequestBuilder,
    mut for_each: impl FnMut(serde_json::Value) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut resp = req.send().await?;
    if resp.status() != 200 {
        return if let Some(chunk) = resp.chunk().await? {
            #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
            struct ErrorResponse {
                pub error: String,
            }
            let err = serde_json::from_slice::<ErrorResponse>(&chunk)?;
            Err(Error::Ollama(err.error))
        } else {
            Err(Error::Internal)
        };
    }
    let mut leftovers: Vec<u8> = vec![];
    while let Some(chunk) = resp.chunk().await? {
        leftovers.extend(&chunk);
        if let Ok(value) = serde_json::from_slice(&leftovers) {
            for_each(value)?;
            leftovers.clear();
        }
    }
    if !leftovers.is_empty() {
        let Ok(value) = serde_json::from_slice(&leftovers) else {
            return Err(Error::Ollama(format!(
                "Host returned invalid JSON chunk {leftovers:?}"
            )));
        };
        for_each(value)?;
    }
    Ok(())
}

//////////////////////////////////////////// ollama_host ///////////////////////////////////////////

/// Return the Ollama host, preferring the value passed in, falling back to the env var, falling
/// back to the hard-coded default.
pub fn ollama_host(host: Option<String>) -> String {
    host.unwrap_or_else(|| std::env::var("OLLAMA_HOST").unwrap_or_else(|_| OLLAMA_HOST.to_string()))
}

///////////////////////////////////////////// chat_root ////////////////////////////////////////////

fn chat_root() -> Result<Path<'static>, Error> {
    let root = std::env::var("YAMMER_CHAT").map_err(|_| Error::ChatNotSet)?;
    Ok(Path::from(root))
}

///////////////////////////////////////////// chat_path ////////////////////////////////////////////

fn chat_path(chat_id: &str) -> Result<Path<'static>, Error> {
    let root = chat_root()?;
    Ok(root.join("chats").join(format!("{}.ndjson", chat_id)))
}

////////////////////////////////////////////// Spinner /////////////////////////////////////////////

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// A spinner widget.
#[derive(Debug)]
pub struct Spinner {
    done: Arc<AtomicBool>,
    inhibited: Arc<Mutex<bool>>,
    background: Option<std::thread::JoinHandle<()>>,
}

impl Spinner {
    /// Create a new spinner.
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
                let _ = stdout.write(b"\x1b[2K\r");
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

    /// Start the spinner.
    pub fn start(&self) {
        *self.inhibited.lock().unwrap() = false;
    }

    /// Inhibit the spinner.
    pub fn inhibit(&self) {
        let mut inhibited = self.inhibited.lock().unwrap();
        if !*inhibited {
            *inhibited = true;
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write(b"\x1b[2K\r");
        }
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
