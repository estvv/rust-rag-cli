// src/scrapper.rs

const IGNORE: [&str; 5] = ["models", "node_modules", "target", "dist", ".git"];
const FILTERS: [&str; 5] = [".rs", ".toml", ".json", ".yaml", ".md"];

use std::fs;
use std::path::{Path, PathBuf};

pub fn scrapper(path: &Path) -> Vec<(PathBuf, String)> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();

    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if IGNORE.iter().any(|&ignore| path.to_str().unwrap().contains(ignore)) {
                continue;
            }
            if path.is_dir() {
                files.extend(scrapper(&path));
            } else if FILTERS.iter().any(|&filter| path.to_str().unwrap().ends_with(filter)) {
                files.push((path.clone(), fs::read_to_string(&path).unwrap_or_else(|_| String::new())));
            }
        }
    }
    files
}

pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
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
        i += chunk_size - overlap;
    }

    chunks
}
