// src/service/chat.rs

use crate::client::RateLimiter;
use crate::clients::ollama::{ModelInfo, OllamaClient};
use crate::db::store::{CodeChunk, SemanticIndex};
use crate::app::FileReference;
use crate::config::{Config, Settings};
use lru::LruCache;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct TimingMetrics {
    pub embedding_time_ms: u64,
    pub retrieval_time_ms: u64,
    pub llm_time_ms: u64,
    pub total_time_ms: u64,
}

impl Default for TimingMetrics {
    fn default() -> Self {
        Self {
            embedding_time_ms: 0,
            retrieval_time_ms: 0,
            llm_time_ms: 0,
            total_time_ms: 0,
        }
    }
}

impl std::fmt::Display for TimingMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "embed:{}ms | search:{}ms | llm:{}ms | total:{}ms",
            self.embedding_time_ms,
            self.retrieval_time_ms,
            self.llm_time_ms,
            self.total_time_ms
        )
    }
}

#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub user: String,
    pub assistant: String,
}

#[derive(Clone)]
pub struct ChatService {
    client: OllamaClient,
    config: Config,
    index: Arc<Mutex<SemanticIndex>>,
    embedding_cache: Arc<Mutex<LruCache<String, Vec<f32>>>>,
    conversation_history: Arc<Mutex<VecDeque<ConversationTurn>>>,
    timing_metrics: Arc<Mutex<Vec<TimingMetrics>>>,
}

impl ChatService {
    pub fn new(config: Config) -> Self {
        let settings = config.settings();
        let client = OllamaClient::new(&settings.base_url);

        let cache_size = NonZeroUsize::new(settings.cache_size).unwrap_or(NonZeroUsize::new(100).unwrap());
        let embedding_cache = LruCache::new(cache_size);

        let index = Arc::new(Mutex::new(SemanticIndex::new()));
        let conversation_history = Arc::new(Mutex::new(VecDeque::new()));
        let timing_metrics = Arc::new(Mutex::new(Vec::new()));

        let client = if config.rate_limit.enabled {
            client.with_rate_limiter(RateLimiter::new(
                config.rate_limit.requests_per_second,
                config.rate_limit.burst_size,
            ))
        } else {
            client
        };

        Self {
            client,
            config,
            index,
            embedding_cache: Arc::new(Mutex::new(embedding_cache)),
            conversation_history,
            timing_metrics,
        }
    }

    pub fn with_settings(settings: Settings) -> Self {
        let mut config = Config::default();
        config.default = settings;
        Self::new(config)
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error + Send + Sync>> {
        self.client.list_models().await
    }

    pub fn set_chat_model(&mut self, model: String) {
        self.config.settings_mut().chat_model = model;
    }

    pub fn set_embed_model(&mut self, model: String) {
        self.config.settings_mut().embed_model = model;
    }

    pub fn get_chat_model(&self) -> &str {
        &self.config.settings().chat_model
    }

    pub fn get_settings(&self) -> &Settings {
        self.config.settings()
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub async fn warmup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("Warming up embedding model: {}", self.config.settings().embed_model);
        let start = std::time::Instant::now();
        self.client.get_embedding(&self.config.settings().embed_model, "warmup").await?;
        tracing::info!("Embedding model ready in {}ms", start.elapsed().as_millis());
        Ok(())
    }

    pub async fn query(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let cache_key = format!("{}:{}", self.config.settings().embed_model, text);

        {
            let mut cache = self.embedding_cache.lock().await;
            if let Some(cached) = cache.get(&cache_key) {
                tracing::info!("Cache hit for query embedding");
                return Ok(cached.clone());
            }
        }

        let embedding = self.client.get_embedding(&self.config.settings().embed_model, text).await?;

        {
            let mut cache = self.embedding_cache.lock().await;
            cache.put(cache_key, embedding.clone());
        }

        Ok(embedding)
    }

    pub async fn retrieve_context_with_threshold(&self, query_embedding: &[f32], top_k: usize, threshold: f32) -> Vec<CodeChunk> {
        let index = self.index.lock().await;
        if index.chunks.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<_> = index.chunks.iter()
            .map(|chunk| {
                let score = cosine_similarity(query_embedding, &chunk.embedding);
                (score, chunk.clone())
            })
            .filter(|(score, _)| *score >= threshold)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        scored.into_iter()
            .take(top_k)
            .map(|(score, mut chunk)| {
                chunk.score = score;
                chunk
            })
            .collect()
    }

    pub fn build_prompt_with_refs(
        &self,
        query: &str,
        file_refs: &[FileReference],
        rag_context: &[CodeChunk],
        history: &[ConversationTurn],
    ) -> String {
        let mut prompt_parts = Vec::new();

        if !history.is_empty() {
            prompt_parts.push("[Previous conversation]".to_string());
            for turn in history.iter().rev().take(self.config.settings().max_history_turns).rev() {
                prompt_parts.push(format!("\nUser: {}", turn.user));
                prompt_parts.push(format!("\nAssistant: {}", turn.assistant));
            }
            prompt_parts.push("".to_string());
        }

        if !file_refs.is_empty() {
            prompt_parts.push("Referenced files:".to_string());
            for (idx, file_ref) in file_refs.iter().enumerate() {
                if let Some(ref content) = file_ref.content {
                    prompt_parts.push(format!(
                        "\n[{}] {}:\n{}\n",
                        idx + 1,
                        file_ref.path.display(),
                        content
                    ));
                }
            }
        }

        if !rag_context.is_empty() {
            prompt_parts.push("\nRelated context from codebase:".to_string());
            for chunk in rag_context {
                prompt_parts.push(format!("\n--- {} (relevance: {:.2}) ---\n{}", chunk.file_path, chunk.score, chunk.content));
            }
        }

        prompt_parts.push(format!("\nQuestion: {}", query));

        prompt_parts.join("\n")
    }

    pub async fn chat_streaming<F>(
        &self,
        prompt: &str,
        file_refs: &[FileReference],
        on_chunk: F,
    ) -> Result<(String, Vec<CodeChunk>, TimingMetrics), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(&str) + Send,
    {
        let total_start = Instant::now();
        let mut metrics = TimingMetrics::default();

        let rag_context = if file_refs.is_empty() {
            let embed_start = Instant::now();
            let embedding = self.query(prompt).await?;
            metrics.embedding_time_ms = embed_start.elapsed().as_millis() as u64;

            let retrieval_start = Instant::now();
            let context = self.retrieve_context_with_threshold(&embedding, self.config.settings().top_k, self.config.settings().relevance_threshold).await;
            metrics.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
            context
        } else {
            Vec::new()
        };

        let history = self.conversation_history.lock().await.iter().cloned().collect::<Vec<_>>();
        let full_prompt = self.build_prompt_with_refs(prompt, file_refs, &rag_context, &history);

        let llm_start = Instant::now();
        let response = self.client.ask_question_streaming(&self.config.settings().chat_model, &full_prompt, on_chunk).await?;
        metrics.llm_time_ms = llm_start.elapsed().as_millis() as u64;

        metrics.total_time_ms = total_start.elapsed().as_millis() as u64;

        {
            let mut metrics_history = self.timing_metrics.lock().await;
            metrics_history.push(metrics.clone());
        }

        {
            let mut history = self.conversation_history.lock().await;
            history.push_back(ConversationTurn {
                user: prompt.to_string(),
                assistant: response.clone(),
            });

            while history.len() > self.config.settings().max_history_turns {
                history.pop_front();
            }
        }

        Ok((response, rag_context, metrics))
    }

    pub async fn get_chunks(&self) -> Vec<CodeChunk> {
        self.index.lock().await.chunks.clone()
    }

    pub async fn get_index_stats(&self) -> (usize, usize) {
        let index = self.index.lock().await;
        (index.chunk_count(), index.file_count())
    }

    pub async fn index_directory_streaming<F>(
        &self,
        path: &Path,
        mut on_progress: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(usize, usize, usize, &str) + Send,
    {
        let config = crate::scrapper::ScrapperConfig::new(
            self.config.file_filter.ignore_dirs.clone(),
            self.config.file_filter.extensions.clone(),
        );

        let files = crate::scrapper::scrapper_with_config(path, &config);
        let total_files = files.len();
        let mut chunks_done = 0;

        for (idx, (file_path, content)) in files.into_iter().enumerate() {
            let file_name = file_path.display().to_string();
            on_progress(idx, total_files, chunks_done, &file_name);

            tracing::debug!("Indexing file: {}", file_name);

            let chunks = crate::scrapper::chunk_text(&content, self.config.settings().chunk_size, self.config.settings().chunk_overlap);

            for chunk in chunks {
                let embedding = self.query(&chunk).await?;
                let mut index = self.index.lock().await;
                index.add_chunk(file_name.clone(), chunk, embedding);
                chunks_done += 1;
            }
        }

        on_progress(total_files, total_files, chunks_done, "done");
        Ok(())
    }

    pub async fn incremental_index<F>(
        &self,
        path: &Path,
        mut on_progress: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(usize, usize, usize, &str) + Send,
    {
        let config = crate::scrapper::ScrapperConfig::new(
            self.config.file_filter.ignore_dirs.clone(),
            self.config.file_filter.extensions.clone(),
        );

        let files = crate::scrapper::scrapper_with_config(path, &config);
        let total_files = files.len();
        let mut chunks_done = 0;
        let mut files_reindexed = 0;

        for (idx, (file_path, content)) in files.into_iter().enumerate() {
            let file_name = file_path.display().to_string();
            on_progress(idx, total_files, chunks_done, &file_name);

            let file_metadata = std::fs::metadata(&file_path).ok();
            let mtime = file_metadata.as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let size = file_metadata.map(|m| m.len()).unwrap_or(0);

            let needs_reindex = {
                let index = self.index.lock().await;
                index.needs_reindex(&file_path, mtime, size)
            };

            if needs_reindex {
                files_reindexed += 1;

                {
                    let mut index = self.index.lock().await;
                    index.remove_chunks_for_file(&file_name);
                }

                let chunks = crate::scrapper::chunk_text(&content, self.config.settings().chunk_size, self.config.settings().chunk_overlap);

                for chunk in chunks {
                    let embedding = self.query(&chunk).await?;
                    let mut index = self.index.lock().await;
                    index.add_chunk(file_name.clone(), chunk, embedding);
                    chunks_done += 1;
                }
            }
        }

        on_progress(total_files, total_files, chunks_done, &format!("done ({} files reindexed)", files_reindexed));
        Ok(())
    }

    pub async fn save_index(&self, path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let index = self.index.lock().await;
        index.save(path)
    }

    pub async fn load_index(&self, path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let loaded = SemanticIndex::load(path)?;
        let mut index = self.index.lock().await;
        *index = loaded;
        Ok(())
    }

    pub async fn clear_history(&self) {
        let mut history = self.conversation_history.lock().await;
        history.clear();
    }

    pub async fn get_average_metrics(&self) -> Option<TimingMetrics> {
        let metrics = self.timing_metrics.lock().await;
        if metrics.is_empty() {
            return None;
        }

        let len = metrics.len() as u64;
        let sum = metrics.iter().fold(TimingMetrics::default(), |acc, m| TimingMetrics {
            embedding_time_ms: acc.embedding_time_ms + m.embedding_time_ms,
            retrieval_time_ms: acc.retrieval_time_ms + m.retrieval_time_ms,
            llm_time_ms: acc.llm_time_ms + m.llm_time_ms,
            total_time_ms: acc.total_time_ms + m.total_time_ms,
        });

        Some(TimingMetrics {
            embedding_time_ms: sum.embedding_time_ms / len,
            retrieval_time_ms: sum.retrieval_time_ms / len,
            llm_time_ms: sum.llm_time_ms / len,
            total_time_ms: sum.total_time_ms / len,
        })
    }

    pub fn is_followup_question(question: &str) -> bool {
        let pronouns = ["it", "that", "this", "they", "them", "those", "these"];
        let continuation_phrases = [
            "expand on", "tell me more", "what about", "how about",
            "can you explain", "what do you mean", "clarify",
            "the above", "the previous", "earlier", "before",
            "you mentioned", "you said",
        ];

        let question_lower = question.to_lowercase();

        let words: std::collections::HashSet<_> = question_lower
            .split_whitespace()
            .collect();

        if pronouns.iter().any(|p| words.contains(p)) {
            return true;
        }

        if continuation_phrases.iter().any(|phrase| question_lower.contains(phrase)) {
            return true;
        }

        false
    }

    pub fn export_context(
        query: &str,
        rag_context: &[CodeChunk],
        prompt: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "query": query,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            "retrieved_chunks": rag_context.iter().map(|c| {
                serde_json::json!({
                    "file_path": c.file_path,
                    "content": c.content,
                    "score": c.score,
                })
            }).collect::<Vec<_>>(),
            "prompt": prompt,
        })
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    dot / (mag_a * mag_b)
}
