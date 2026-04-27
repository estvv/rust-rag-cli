// src/ui/mod.rs

pub mod render;
pub mod syntax;

pub use render::render;
pub use syntax::{highlight_code, highlight_code_lines, detect_language_from_path, extract_code_blocks, CodeBlock};
