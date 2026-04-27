use crate::clients::ollama::{ModelInfo, OllamaClient};
use crate::db::store::{CodeChunk, SemanticIndex};
use crate::app::FileReference;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
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

#[derive(Clone)]
pub struct ChatService {
    client: OllamaClient,
    config: Config,
    index: Arc<Mutex<SemanticIndex>>,
}

impl ChatService {
    pub fn new(config: Config) -> Self {
        let client = OllamaClient::new(&config.base_url);
        let index = Arc::new(Mutex::new(SemanticIndex::new()));

        Self { client, config, index }
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        self.client.list_models().await
    }

    pub fn set_chat_model(&mut self, model: String) {
        self.config.chat_model = model;
    }

    pub fn set_embed_model(&mut self, model: String) {
        self.config.embed_model = model;
    }

    pub fn get_chat_model(&self) -> &str {
        &self.config.chat_model
    }

    pub async fn query(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        self.client.get_embedding(&self.config.embed_model, text).await
    }

    pub async fn retrieve_context(&self, query_embedding: &[f32], top_k: usize) -> Vec<CodeChunk> {
        let index = self.index.lock().await;
        if index.chunks.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<_> = index.chunks.iter()
            .map(|chunk| {
                let score = cosine_similarity(query_embedding, &chunk.embedding);
                (score, chunk.clone())
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        scored.into_iter().take(top_k).map(|(_, chunk)| chunk).collect()
    }

    pub fn build_prompt_with_refs(&self, query: &str, file_refs: &[FileReference], rag_context: &[CodeChunk]) -> String {
        let mut prompt_parts = Vec::new();

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
                prompt_parts.push(format!("\n--- {} ---\n{}", chunk.file_path, chunk.content));
            }
        }

        prompt_parts.push(format!("\nQuestion: {}", query));

        prompt_parts.join("\n")
    }

    pub async fn chat_streaming<F>(&self, prompt: &str, file_refs: &[FileReference], on_chunk: F) -> Result<String, Box<dyn std::error::Error>>
        where
            F: FnMut(&str) + Send,
    {
        let embedding = self.query(prompt).await?;
        let rag_context = self.retrieve_context(&embedding, 5).await;
        let full_prompt = self.build_prompt_with_refs(prompt, file_refs, &rag_context);

        self.client.ask_question_streaming(&self.config.chat_model, &full_prompt, on_chunk).await
    }

    pub async fn chat_with_refs(&self, prompt: &str, file_refs: &[FileReference]) -> Result<String, Box<dyn std::error::Error>> {
        let embedding = self.query(prompt).await?;
        let rag_context = self.retrieve_context(&embedding, 5).await;
        let full_prompt = self.build_prompt_with_refs(prompt, file_refs, &rag_context);

        self.client.ask_question(&self.config.chat_model, &full_prompt).await
    }

    fn build_prompt(&self, query: &str, context: &[CodeChunk]) -> String {
        if context.is_empty() {
            return query.to_string();
        }

        let context_text: String = context.iter().map(|c| format!("--- {} ---\n{}", c.file_path, c.content)).collect::<Vec<_>>().join("\n\n");

        format!(
            "Use the following context to answer the question.\n\n\
            Context:\n{}\n\n\
            Question: {}",
            context_text, query
        )
    }

    pub async fn get_chunks(&self) -> Vec<CodeChunk> {
        self.index.lock().await.chunks.clone()
    }

    pub async fn index_directory_streaming<F>(&self, path: &Path, mut on_progress: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(usize, usize, usize, &str) + Send,
    {
        let files = crate::scrapper::scrapper(path);
        let total_files = files.len();
        let mut chunks_done = 0;

        for (idx, (file_path, content)) in files.into_iter().enumerate() {
            let file_name = file_path.display().to_string();
            on_progress(idx, total_files, chunks_done, &file_name);

            let chunks = crate::scrapper::chunk_text(&content, 50, 10);

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

    pub async fn save_index(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let index = self.index.lock().await;
        index.save(path)
    }

    pub async fn load_index(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let loaded = SemanticIndex::load(path)?;
        let mut index = self.index.lock().await;
        *index = loaded;
        Ok(())
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
