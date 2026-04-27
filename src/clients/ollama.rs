// src/clients/ollama.rs

use crate::client::{Client, ChatRequest, ChatResponse, ChatStreamResponse, EmbedRequest, EmbedResponse};
use serde::Deserialize;
use std::error::Error;

#[derive(Clone)]
pub struct OllamaClient {
    base_url: String,
    client: Client
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

impl OllamaClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    pub async fn get_embedding(&self, model_name: &str, text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
        let url = format!("{}/api/embeddings", self.base_url);

        let request_body = EmbedRequest {
            model: model_name.to_string(),
            prompt: text.to_string(),
        };

        let response = self.client.post(&url).json(&request_body).send().await?;
        let parsed = response.json::<EmbedResponse>().await?;

        Ok(parsed.embedding)
    }

    pub async fn ask_question(&self, model_name: &str, prompt: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/api/generate", self.base_url);

        let request_body = ChatRequest {
            model: model_name.to_string(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self.client.post(&url).json(&request_body).send().await?;
        let parsed = response.json::<ChatResponse>().await?;

        if let Some(error) = parsed.error {
            Err(error.into())
        } else {
            Ok(parsed.response)
        }
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, Box<dyn Error>> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        let parsed = response.json::<ModelsResponse>().await?;

        Ok(parsed.models)
    }

    pub async fn ask_question_streaming<F>(&self, model_name: &str, prompt: &str, mut on_chunk: F) -> Result<String, Box<dyn Error>>
        where
            F: FnMut(&str) + Send,
    {
        use futures_util::StreamExt;

        let url = format!("{}/api/generate", self.base_url);

        let request_body = ChatRequest {
            model: model_name.to_string(),
            prompt: prompt.to_string(),
            stream: true,
        };

        let response = self.client.post(&url).json(&request_body).send().await?;
        let mut full_response = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            if let Ok(bytes) = chunk_result {
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    for line in text.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<ChatStreamResponse>(line) {
                            if let Some(error) = parsed.error {
                                return Err(error.into());
                            }
                            if !parsed.response.is_empty() {
                                on_chunk(&parsed.response);
                                full_response.push_str(&parsed.response);
                            }
                            if parsed.done {
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(full_response)
    }
}
