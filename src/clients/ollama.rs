// src/clients/ollama.rs

use crate::client::{Client, ChatRequest, ChatResponse, ChatStreamResponse, EmbedRequest, EmbedResponse, RateLimiter};
use crate::config::Settings;
use serde::Deserialize;
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct OllamaClient {
    base_url: String,
    client: Client,
    rate_limiter: Option<RateLimiter>,
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
            rate_limiter: None,
        }
    }

    pub fn with_rate_limiter(mut self, rate_limiter: RateLimiter) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    pub async fn get_embedding(&self, model_name: &str, text: &str) -> Result<Vec<f32>, Box<dyn Error + Send + Sync>> {
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }

        let start = Instant::now();
        tracing::debug!("Getting embedding for {} chars with model {}", text.len(), model_name);

        let url = format!("{}/api/embeddings", self.base_url);

        let request_body = EmbedRequest {
            model: model_name.to_string(),
            prompt: text.to_string(),
        };

        let response = self.client.post(&url).json(&request_body).send().await?;
        let parsed = response.json::<EmbedResponse>().await?;

        tracing::debug!("Got embedding in {}ms (dim: {})", start.elapsed().as_millis(), parsed.embedding.len());

        Ok(parsed.embedding)
    }

    pub async fn ask_question(&self, model_name: &str, prompt: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }

        let start = Instant::now();
        tracing::debug!("Asking question with model {} (prompt: {} chars)", model_name, prompt.len());

        let url = format!("{}/api/generate", self.base_url);

        let request_body = ChatRequest {
            model: model_name.to_string(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let response = self.client.post(&url).json(&request_body).send().await?;
        let parsed = response.json::<ChatResponse>().await?;

        tracing::debug!("Got response in {}ms", start.elapsed().as_millis());

        if let Some(error) = parsed.error {
            Err(error.into())
        } else {
            Ok(parsed.response)
        }
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        let parsed = response.json::<ModelsResponse>().await?;

        Ok(parsed.models)
    }

    pub async fn ask_question_streaming<F>(&self, model_name: &str, prompt: &str, mut on_chunk: F) -> Result<String, Box<dyn Error + Send + Sync>>
        where
            F: FnMut(&str) + Send,
    {
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }

        let start = Instant::now();
        tracing::debug!("Starting streaming request with model {}", model_name);

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

        tracing::debug!("Streaming completed in {}ms", start.elapsed().as_millis());
        Ok(full_response)
    }
}
