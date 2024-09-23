//! A conversation captures an exchange of messages between a user and an assistant.
//!
//! This module provides a `Conversation` struct that can be used to build a conversation between
//! a user and an assistant. The conversation can be used to generate a `ChatRequest` to work with
//! the core yammer library.

use super::{ChatMessage, ChatRequest};

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
}
