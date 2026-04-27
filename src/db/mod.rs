// src/db/mod.rs

pub mod store;
pub mod metadata;

pub use store::{CodeChunk, SemanticIndex, FileMetadata};
