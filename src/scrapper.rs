// src/scrapper.rs

use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_IGNORE: [&str; 5] = ["models", "node_modules", "target", "dist", ".git"];

const DEFAULT_FILTERS: &[&str] = &[
    ".rs", ".toml", ".json", ".yaml", ".yml", ".md",
    ".py", ".pyi",
    ".js", ".ts", ".jsx", ".tsx", ".mjs",
    ".go",
    ".java", ".kt", ".scala",
    ".c", ".cpp", ".h", ".hpp",
    ".sh", ".bash", ".zsh",
    ".txt", ".sql",
];

pub struct ScrapperConfig {
    pub ignore_dirs: Vec<String>,
    pub extensions: Vec<String>,
}

impl Default for ScrapperConfig {
    fn default() -> Self {
        Self {
            ignore_dirs: DEFAULT_IGNORE.iter().map(|s| s.to_string()).collect(),
            extensions: DEFAULT_FILTERS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl ScrapperConfig {
    pub fn new(ignore_dirs: Vec<String>, extensions: Vec<String>) -> Self {
        Self { ignore_dirs, extensions }
    }
}

pub fn scrapper(path: &Path) -> Vec<(PathBuf, String)> {
    scrapper_with_config(path, &ScrapperConfig::default())
}

pub fn scrapper_with_config(path: &Path, config: &ScrapperConfig) -> Vec<(PathBuf, String)> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();

    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                let path_str = entry_path.to_string_lossy();

                if config.ignore_dirs.iter().any(|ignore| {
                    path_str.contains(ignore) ||
                    entry_path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n == ignore)
                        .unwrap_or(false)
                }) {
                    continue;
                }

                if entry_path.is_dir() {
                    files.extend(scrapper_with_config(&entry_path, config));
                } else if config.extensions.iter().any(|ext| {
                    entry_path.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| format!(".{}", e) == *ext || entry_path.to_string_lossy().ends_with(ext))
                        .unwrap_or(false) ||
                    entry_path.to_string_lossy().ends_with(ext)
                }) {
                    if let Ok(content) = fs::read_to_string(&entry_path) {
                        files.push((entry_path.clone(), content));
                    }
                }
            }
        }
    } else if path.is_file() {
        if let Ok(content) = fs::read_to_string(path) {
            files.push((path.to_path_buf(), content));
        }
    }

    files
}

pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut chunks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let end = std::cmp::min(i + chunk_size, lines.len());
        let chunk = lines[i..end].join("\n");

        chunks.push(chunk);

        if end == lines.len() {
            break;
        }
        i += chunk_size.saturating_sub(overlap);
    }

    chunks
}
