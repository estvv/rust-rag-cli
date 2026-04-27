// src/db/store.rs

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::error::Error;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CodeChunk {
    pub file_path: String,
    pub content: String,
    pub embedding: Vec<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SemanticIndex {
    pub chunks: Vec<CodeChunk>,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn add_chunk(&mut self, file_path: String, content: String, embedding: Vec<f32>) {
        let chunk = CodeChunk {
            file_path,
            content,
            embedding,
        };

        self.chunks.push(chunk);
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string_pretty(self)?;

        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &str) -> Result<Self, Box<dyn Error>> {
        if !Path::new(path).exists() {
            println!("Aucun index trouvé à {}. Création d'un nouvel index.", path);
            return Ok(Self::new());
        }

        let data = fs::read_to_string(path)?;
        let index: SemanticIndex = serde_json::from_str(&data)?;

        Ok(index)
    }
}
