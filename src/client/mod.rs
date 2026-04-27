// src/client/mod.rs

pub mod rate_limiter;
pub mod types;

pub use rate_limiter::RateLimiter;
pub use types::{EmbedRequest, EmbedResponse, ChatRequest, ChatResponse, ChatStreamResponse};

pub type Client = reqwest::Client;
