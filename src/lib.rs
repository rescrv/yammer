//! yammer is a library for interacting with the ollama API.

use std::io::Write;

use reqwest::Client;

mod conversation;

pub use conversation::{Conversation, ConversationOptions, Spinner};

/////////////////////////////////////////////// Error //////////////////////////////////////////////

/// An error that can occur when interacting with the ollama API.
#[derive(Debug)]
pub enum Error {
    Message(String),
    Io(std::io::Error),
    Request(reqwest::Error),
    Json(serde_json::Error),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Request(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Message(format!("could not parse utf8: {err:?}"))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/////////////////////////////////////////// ErrorResponse //////////////////////////////////////////

/// An error response from the ollama API.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

//////////////////////////////////////////// PullRequest ///////////////////////////////////////////

/// A request to pull a model from the ollama API.
#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    arrrg_derive::CommandLine,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct PullRequest {
    /// The name of the ollama model to use from the ollama library.
    #[arrrg(
        optional,
        "The name of the ollama model to use from the ollama library."
    )]
    pub model: String,
}

impl PullRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

/////////////////////////////////////////// CreateRequest //////////////////////////////////////////

/// A request to pull a model from the ollama API.
#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    arrrg_derive::CommandLine,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct CreateRequest {
    /// The name of the model to create.
    #[arrrg(required, "The name of the model to create.")]
    pub name: String,

    /// The name of the model to create.
    #[arrrg(required, "The contents of the modelfile.")]
    pub modelfile: String,

    /// Whether to stream the results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Whether to stream the results.
    #[arrrg(optional, "An optional quantize specification.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantize: Option<String>,
}

impl CreateRequest {
    pub fn new(name: impl Into<String>, modelfile: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            modelfile: modelfile.into(),
            stream: None,
            quantize: None,
        }
    }
}

////////////////////////////////////////// GenerateRequest /////////////////////////////////////////

/// Generate a response to a prompt.
#[derive(
    Clone, Debug, Eq, PartialEq, arrrg_derive::CommandLine, serde::Deserialize, serde::Serialize,
)]
pub struct GenerateRequest {
    /// The name of the ollama model to use from the ollama library.
    #[arrrg(
        optional,
        "The name of the ollama model to use from the ollama library."
    )]
    pub model: String,

    /// The prompt to provide to the model.  This is the text that the model will use to generate a
    /// response.
    #[arrrg(
        optional,
        "The prompt to provide to the model.  This is the text that the model will use to generate a response."
    )]
    pub prompt: String,

    /// The suffix to append to the prompt.  This is useful for generating a response that is a
    /// continuation of the prompt.
    #[arrrg(
        optional,
        "The suffix to append to the prompt.  This is useful for generating a response that is a continuation of the prompt."
    )]
    pub suffix: String,

    /// A list of base64-encoded images to supply to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,

    /// The format to return the response in.  If provided, this must be "json".
    #[arrrg(
        optional,
        "The format to return the response in.  If provided, this must be \"json\"."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// The system to use for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// The template to use for the response.
    #[arrrg(optional, "The template to use for the response.")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,

    /// Should this response stream?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Should this response be raw?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<bool>,

    /// How long to hold the model in memory for once the request completes.
    #[arrrg(
        optional,
        "How long to hold the model in memory for once the request completes."
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

impl Default for GenerateRequest {
    fn default() -> Self {
        Self {
            model: "mistral-nemo".to_string(),
            prompt: "42".to_string(),
            suffix: "".to_string(),
            images: None,
            format: None,
            system: None,
            template: None,
            stream: None,
            raw: None,
            keep_alive: None,
        }
    }
}

///////////////////////////////////////// GenerateResponse /////////////////////////////////////////

/// A response to a generate request.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GenerateResponse {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
    pub total_duration: Option<f64>,
    pub load_duration: Option<f64>,
    pub prompt_eval_count: Option<f64>,
    pub prompt_eval_duration: Option<f64>,
    pub eval_count: Option<f64>,
    pub eval_duration: Option<f64>,
    pub context: Vec<f64>,
}

/////////////////////////////////////////// EmbedRequest ///////////////////////////////////////////

#[derive(
    Clone, Debug, Eq, PartialEq, arrrg_derive::CommandLine, serde::Deserialize, serde::Serialize,
)]
pub struct EmbedRequest {
    #[arrrg(
        optional,
        "The name of the ollama model to use from the ollama library."
    )]
    pub model: String,
    pub input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

impl Default for EmbedRequest {
    fn default() -> Self {
        Self {
            model: "mistral-nemo".to_string(),
            input: vec![],
            truncate: None,
            keep_alive: None,
        }
    }
}

//////////////////////////////////////////// ShowRequest ///////////////////////////////////////////

/// A request to pull a model from the ollama API.
#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    arrrg_derive::CommandLine,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct ShowRequest {
    /// The name of the ollama model to use from the ollama library.
    #[arrrg(
        optional,
        "The name of the ollama model to use from the ollama library."
    )]
    pub model: String,
}

impl ShowRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

//////////////////////////////////////////// ChatMessage ///////////////////////////////////////////

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

//////////////////////////////////////////// ChatRequest ///////////////////////////////////////////

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

/////////////////////////////////////////// ChatResponse ///////////////////////////////////////////

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatResponse {
    pub created_at: String,
    pub message: ChatMessage,
    pub done: bool,
}

////////////////////////////////////////// RequestOptions //////////////////////////////////////////

#[derive(Clone, Debug, Default, Eq, PartialEq, arrrg_derive::CommandLine)]
pub struct RequestOptions {
    #[arrrg(optional, "The URL of an ollama server.")]
    pub url: Option<String>,
}

impl RequestOptions {
    pub fn url(&self) -> String {
        self.url
            .clone()
            .or_else(|| std::env::var("OLLAMA_HOST").ok())
            .unwrap_or_else(|| "http://localhost:11434".to_string())
    }
}

////////////////////////////////////////////// Request /////////////////////////////////////////////

#[derive(Debug)]
pub struct Request {
    pub url: String,
    pub api: String,
    pub payload: String,
    pub streaming: bool,
}

impl Request {
    pub fn pull(options: RequestOptions, pull: PullRequest) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_string(&pull)?;
        Ok(Self {
            url: options.url(),
            api: "pull".to_string(),
            payload,
            streaming: true,
        })
    }

    pub fn create(
        options: RequestOptions,
        create: CreateRequest,
    ) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_string(&create)?;
        Ok(Self {
            url: options.url(),
            api: "create".to_string(),
            payload,
            streaming: true,
        })
    }

    pub fn generate(
        options: RequestOptions,
        generate: GenerateRequest,
    ) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_string(&generate)?;
        Ok(Self {
            url: options.url(),
            api: "generate".to_string(),
            payload,
            streaming: true,
        })
    }

    pub fn embed(
        options: RequestOptions,
        embed: EmbedRequest,
        inputs: Vec<impl Into<String>>,
    ) -> Result<Self, serde_json::Error> {
        let model = embed.model;
        let input: Vec<String> = inputs.into_iter().map(|s| s.into()).collect();
        let payload =
            serde_json::to_string(&serde_json::json!({ "model": model, "input": input }))?;
        Ok(Self {
            url: options.url(),
            api: "embed".to_string(),
            payload,
            streaming: false,
        })
    }

    pub fn chat(options: RequestOptions, chat: ChatRequest) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_string(&chat)?;
        Ok(Self {
            url: options.url(),
            api: "chat".to_string(),
            payload,
            streaming: true,
        })
    }

    pub fn tags(options: RequestOptions) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_string(&serde_json::json!({}))?;
        Ok(Self {
            url: options.url(),
            api: "tags".to_string(),
            payload,
            streaming: false,
        })
    }

    pub fn show(options: RequestOptions, show: ShowRequest) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_string(&show)?;
        Ok(Self {
            url: options.url(),
            api: "show".to_string(),
            payload,
            streaming: false,
        })
    }

    pub async fn accumulate(self, acc: &mut impl Accumulator) -> Result<(), Error> {
        accumulate(self, acc).await
    }

    async fn doit(self) -> reqwest::Result<reqwest::Response> {
        let client = Client::new();
        // NOTE(rescrv): This is intentionally match.  I could embed the Method in the Request, but
        // that wouldn't allow me the flexibility to e.g., easily add a new variant with special
        // headers down the line.  This allows me to add methods to where I need them.
        match self.api.as_str() {
            "pull" | "create" | "generate" | "embed" | "chat" | "show" => {
                client
                    .post(&format!("{}/api/{}", self.url, self.api))
                    .header(reqwest::header::ACCEPT, "application/json")
                    .header(reqwest::header::CONTENT_LENGTH, "10485760")
                    .body(self.payload)
                    .send()
                    .await
            }
            "tags" => {
                client
                    .get(&format!("{}/api/{}", self.url, self.api))
                    .header(reqwest::header::ACCEPT, "application/json")
                    .header(reqwest::header::CONTENT_LENGTH, "10485760")
                    .send()
                    .await
            }
            _ => {
                panic!("Unknown API: {}", self.api);
            }
        }
    }
}

//////////////////////////////////////////// Accumulator ///////////////////////////////////////////

pub trait Accumulator: std::fmt::Debug {
    fn accumulate(&mut self, message: serde_json::Value) -> std::ops::ControlFlow<()>;
}

impl<T: Accumulator> Accumulator for &mut T {
    fn accumulate(&mut self, message: serde_json::Value) -> std::ops::ControlFlow<()> {
        (**self).accumulate(message)
    }
}

macro_rules! impl_accumulator {
    ($($name:ident)+) => {
        #[allow(non_snake_case)]
        impl<$($name: Accumulator),+> Accumulator for ($($name,)+)
        where ($($name,)+): std::fmt::Debug,
        {
            fn accumulate(&mut self, message: serde_json::Value)  -> std::ops::ControlFlow<()>{
                let ($($name,)+) = self;
                $($name.accumulate(message.clone())?;)+
                std::ops::ControlFlow::Continue(())
            }
        }
    };
}

impl_accumulator! { A }
impl_accumulator! { A B }
impl_accumulator! { A B C }
impl_accumulator! { A B C D }
impl_accumulator! { A B C D E }
impl_accumulator! { A B C D E F }
impl_accumulator! { A B C D E F G }
impl_accumulator! { A B C D E F G H }
impl_accumulator! { A B C D E F G H I }
impl_accumulator! { A B C D E F G H I J }
impl_accumulator! { A B C D E F G H I J K }
impl_accumulator! { A B C D E F G H I J K L }
impl_accumulator! { A B C D E F G H I J K L M }
impl_accumulator! { A B C D E F G H I J K L M N }
impl_accumulator! { A B C D E F G H I J K L M N O }
impl_accumulator! { A B C D E F G H I J K L M N O P }
impl_accumulator! { A B C D E F G H I J K L M N O P Q }
impl_accumulator! { A B C D E F G H I J K L M N O P Q R }
impl_accumulator! { A B C D E F G H I J K L M N O P Q R S }
impl_accumulator! { A B C D E F G H I J K L M N O P Q R S T }
impl_accumulator! { A B C D E F G H I J K L M N O P Q R S T U }
impl_accumulator! { A B C D E F G H I J K L M N O P Q R S T U V }
impl_accumulator! { A B C D E F G H I J K L M N O P Q R S T U V W }
impl_accumulator! { A B C D E F G H I J K L M N O P Q R S T U V W X }
impl_accumulator! { A B C D E F G H I J K L M N O P Q R S T U V W X Y }
impl_accumulator! { A B C D E F G H I J K L M N O P Q R S T U V W X Y Z }

#[derive(Debug)]
pub struct FieldWriteAccumulator<W: Write + std::fmt::Debug> {
    output: W,
    field: String,
}

impl<W: Write + std::fmt::Debug> FieldWriteAccumulator<W> {
    pub fn new(output: W, field: impl Into<String>) -> Self {
        Self {
            output,
            field: field.into(),
        }
    }
}

impl<W: Write + std::fmt::Debug> Accumulator for FieldWriteAccumulator<W> {
    fn accumulate(&mut self, message: serde_json::Value) -> std::ops::ControlFlow<()> {
        if let Some(serde_json::Value::String(message)) = message.get(&self.field) {
            let _ = write!(self.output, "{}", message);
            let _ = self.output.flush();
        }
        std::ops::ControlFlow::Continue(())
    }
}

#[derive(Debug)]
pub struct JsonAccumulator<W: Write + std::fmt::Debug> {
    output: W,
    pub pretty: bool,
}

impl<W: Write + std::fmt::Debug> JsonAccumulator<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            pretty: false,
        }
    }

    pub fn pretty(output: W) -> Self {
        Self {
            output,
            pretty: true,
        }
    }
}

impl<W: Write + std::fmt::Debug> Accumulator for JsonAccumulator<W> {
    fn accumulate(&mut self, message: serde_json::Value) -> std::ops::ControlFlow<()> {
        if self.pretty {
            let _ = writeln!(
                self.output,
                "{}",
                serde_json::to_string_pretty(&message).unwrap()
            );
        } else {
            let _ = writeln!(self.output, "{}", serde_json::to_string(&message).unwrap());
        }
        std::ops::ControlFlow::Continue(())
    }
}

#[derive(Debug)]
pub struct VecAccumulator<'a> {
    pub output: &'a mut Vec<serde_json::Value>,
}

impl<'a> VecAccumulator<'a> {
    pub fn new(output: &'a mut Vec<serde_json::Value>) -> Self {
        Self { output }
    }
}

impl<'a> Accumulator for VecAccumulator<'a> {
    fn accumulate(&mut self, message: serde_json::Value) -> std::ops::ControlFlow<()> {
        self.output.push(message);
        std::ops::ControlFlow::Continue(())
    }
}

#[derive(Debug, Default)]
pub struct ChatAccumulator {
    seen_non_ws: bool,
}

impl Accumulator for ChatAccumulator {
    fn accumulate(&mut self, msg: serde_json::Value) -> std::ops::ControlFlow<()> {
        let msg = match serde_json::from_value::<ChatResponse>(msg.clone()) {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("could not parse message {msg}: {:?}", err);
                return std::ops::ControlFlow::Break(());
            }
        };
        if self.seen_non_ws || !msg.message.content.trim().is_empty() {
            let mut stdout = std::io::stdout();
            let _ = write!(stdout, "{}", msg.message.content);
            let _ = stdout.flush();
            self.seen_non_ws = true;
        }
        std::ops::ControlFlow::Continue(())
    }
}

//////////////////////////////////////////// accumulate ////////////////////////////////////////////

pub async fn accumulate(req: Request, mut acc: impl Accumulator) -> Result<(), Error> {
    let streaming = req.streaming;
    let mut resp = req.doit().await?;
    if resp.status() != 200 {
        let mut text = String::new();
        while let Some(chunk) = resp.chunk().await? {
            text.push_str(std::str::from_utf8(chunk.as_ref())?);
        }
        return Err(Error::Message(text));
    }
    if streaming {
        let mut leftovers = String::new();
        while let Some(chunk) = resp.chunk().await? {
            let chunk = std::str::from_utf8(chunk.as_ref())?.trim();
            leftovers.push_str(chunk);
            if !chunk.is_empty() {
                if let Ok(err) = serde_json::from_str::<ErrorResponse>(&leftovers) {
                    return Err(Error::Message(err.error));
                }
                let Ok(message): Result<serde_json::Value, _> = serde_json::from_str(&leftovers)
                else {
                    continue;
                };
                if acc.accumulate(message).is_break() {
                    break;
                }
                leftovers.clear();
            }
        }
    } else {
        let mut text = String::new();
        while let Some(chunk) = resp.chunk().await? {
            if !chunk.is_empty() {
                let chunk = std::str::from_utf8(chunk.as_ref())?;
                text.push_str(chunk);
            }
        }
        let message: serde_json::Value = serde_json::from_str(text.trim())?;
        acc.accumulate(message);
    }
    Ok(())
}

/////////////////////////////////////////////// load ///////////////////////////////////////////////

pub fn load(path: impl AsRef<std::path::Path>) -> Result<Vec<ChatMessage>, Error> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    let mut msgs = vec![];
    for line in content.split_terminator('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<ChatMessage>(line) else {
            continue;
        };
        msgs.push(msg);
    }
    Ok(msgs)
}
