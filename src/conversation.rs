use super::{ChatMessage, ChatRequest};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Conversation {
    messages: Vec<ChatMessage>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn push(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn truncate(&mut self, index: usize) {
        self.messages.truncate(index);
    }

    pub fn add_assistant_response(&mut self, pieces: Vec<serde_json::Value>) {
        println!("{:?}", pieces);
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
}
