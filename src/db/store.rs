// src/db/store.rs

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CodeChunk {
    pub file_path: String,
    pub content: String,
    pub embedding: Vec<f32>,
    #[serde(default)]
    pub score: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SemanticIndex {
    pub chunks: Vec<CodeChunk>,
    #[serde(default)]
    pub metadata_store: HashMap<String, FileMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FileMetadata {
    pub mtime: u64,
    pub size: u64,
    pub hash: u64,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            metadata_store: HashMap::new(),
        }
    }

    pub fn add_chunk(&mut self, file_path: String, content: String, embedding: Vec<f32>) {
        let chunk = CodeChunk {
            file_path,
            content,
            embedding,
            score: 0.0,
        };

        self.chunks.push(chunk);
    }

    pub fn remove_chunks_for_file(&mut self, file_path: &str) {
        self.chunks.retain(|c| c.file_path != file_path);
    }

    pub fn update_metadata(&mut self, file_path: String, metadata: FileMetadata) {
        self.metadata_store.insert(file_path, metadata);
    }

    pub fn needs_reindex(&self, file_path: &Path, current_mtime: u64, current_size: u64) -> bool {
        let path_str = file_path.to_string_lossy().to_string();
        match self.metadata_store.get(&path_str) {
            Some(meta) => {
                meta.mtime < current_mtime || meta.size != current_size
            }
            None => true,
        }
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;

        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if !Path::new(path).exists() {
            return Ok(Self::new());
        }

        let data = fs::read_to_string(path)?;
        let index: SemanticIndex = serde_json::from_str(&data)?;

        Ok(index)
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn file_count(&self) -> usize {
        self.chunks.iter()
            .map(|c| c.file_path.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len()
    }
}
