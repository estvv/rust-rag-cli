// src/service/mod.rs

pub mod chat;

pub use chat::{ChatService, TimingMetrics, ConversationTurn};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub embed_model: String,
    pub chat_model: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            embed_model: "nomic-embed-text".to_string(),
            chat_model: "llama3".to_string(),
        }
    }
}
