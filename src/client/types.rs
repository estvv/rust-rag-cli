// src/client.rs

use serde::{Deserialize, Serialize};

pub type Client = reqwest::Client;

#[derive(Serialize)]
pub struct EmbedRequest {
    pub model: String,
    pub prompt: String,
}

#[derive(Deserialize)]
pub struct EmbedResponse {
    pub embedding: Vec<f32>,
}

#[derive(Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
}

#[derive(Deserialize)]
pub struct ChatResponse {
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct ChatStreamResponse {
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub error: Option<String>,
}
