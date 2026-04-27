// src/config/settings.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub default: Settings,

    #[serde(default)]
    pub profile: HashMap<String, Settings>,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    #[serde(default)]
    pub watcher: WatcherConfig,

    #[serde(default)]
    pub file_filter: FileFilterConfig,

    #[serde(skip)]
    pub active_profile: Option<String>,
}

impl Config {
    pub fn settings(&self) -> &Settings {
        if let Some(profile_name) = &self.active_profile {
            self.profile.get(profile_name).unwrap_or(&self.default)
        } else {
            &self.default
        }
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        if let Some(profile_name) = &self.active_profile {
            self.profile.entry(profile_name.clone()).or_default()
        } else {
            &mut self.default
        }
    }

    pub fn apply_env_overrides(&mut self) {
        let settings = self.settings_mut();

        if let Ok(val) = std::env::var("RAG_CLI_CHAT_MODEL") {
            settings.chat_model = val;
        }
        if let Ok(val) = std::env::var("RAG_CLI_EMBED_MODEL") {
            settings.embed_model = val;
        }
        if let Ok(val) = std::env::var("RAG_CLI_BASE_URL") {
            settings.base_url = val;
        }
        if let Ok(val) = std::env::var("RAG_CLI_INDEX_FILE") {
            settings.index_file = val;
        }
        if let Ok(val) = std::env::var("RAG_CLI_TOP_K") {
            if let Ok(num) = val.parse() {
                settings.top_k = num;
            }
        }
        if let Ok(val) = std::env::var("RAG_CLI_RELEVANCE_THRESHOLD") {
            if let Ok(num) = val.parse() {
                settings.relevance_threshold = num;
            }
        }
        if let Ok(val) = std::env::var("RAG_CLI_CHUNK_SIZE") {
            if let Ok(num) = val.parse() {
                settings.chunk_size = num;
            }
        }
        if let Ok(val) = std::env::var("RAG_CLI_CHUNK_OVERLAP") {
            if let Ok(num) = val.parse() {
                settings.chunk_overlap = num;
            }
        }
    }

    pub fn with_profile(mut self, profile: &str) -> Self {
        self.active_profile = Some(profile.to_string());
        self
    }

    pub fn available_profiles(&self) -> Vec<&String> {
        self.profile.keys().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_chat_model")]
    pub chat_model: String,

    #[serde(default = "default_embed_model")]
    pub embed_model: String,

    #[serde(default = "default_base_url")]
    pub base_url: String,

    #[serde(default = "default_index_file")]
    pub index_file: String,

    #[serde(default = "default_top_k")]
    pub top_k: usize,

    #[serde(default)]
    pub relevance_threshold: f32,

    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,

    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,

    #[serde(default = "default_cache_size")]
    pub cache_size: usize,

    #[serde(default = "default_max_history")]
    pub max_history_turns: usize,
}

fn default_chat_model() -> String { "llama3".to_string() }
fn default_embed_model() -> String { "nomic-embed-text".to_string() }
fn default_base_url() -> String { "http://localhost:11434".to_string() }
fn default_index_file() -> String { ".semantic-index.json".to_string() }
fn default_top_k() -> usize { 5 }
fn default_chunk_size() -> usize { 500 }
fn default_chunk_overlap() -> usize { 10 }
fn default_cache_size() -> usize { 100 }
fn default_max_history() -> usize { 5 }

impl Default for Settings {
    fn default() -> Self {
        Self {
            chat_model: default_chat_model(),
            embed_model: default_embed_model(),
            base_url: default_base_url(),
            index_file: default_index_file(),
            top_k: default_top_k(),
            relevance_threshold: 0.0,
            chunk_size: default_chunk_size(),
            chunk_overlap: default_chunk_overlap(),
            cache_size: default_cache_size(),
            max_history_turns: default_max_history(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_syntax_highlight")]
    pub syntax_highlight: bool,

    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_syntax_highlight() -> bool { true }
fn default_theme() -> String { "base16-eighties.dark".to_string() }

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            syntax_highlight: default_syntax_highlight(),
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_rps")]
    pub requests_per_second: u32,

    #[serde(default = "default_burst")]
    pub burst_size: u32,
}

fn default_rps() -> u32 { 10 }
fn default_burst() -> u32 { 20 }

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_second: default_rps(),
            burst_size: default_burst(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,

    #[serde(default = "default_reindex_delay_ms")]
    pub reindex_delay_ms: u64,
}

fn default_debounce_ms() -> u64 { 500 }
fn default_reindex_delay_ms() -> u64 { 1000 }

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            debounce_ms: default_debounce_ms(),
            reindex_delay_ms: default_reindex_delay_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFilterConfig {
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,

    #[serde(default = "default_ignore_dirs")]
    pub ignore_dirs: Vec<String>,
}

fn default_extensions() -> Vec<String> {
    vec![
        ".rs".to_string(), ".toml".to_string(), ".json".to_string(),
        ".yaml".to_string(), ".yml".to_string(), ".md".to_string(),
        ".py".to_string(), ".pyi".to_string(),
        ".js".to_string(), ".ts".to_string(), ".jsx".to_string(), ".tsx".to_string(),
        ".go".to_string(),
        ".java".to_string(), ".kt".to_string(),
        ".c".to_string(), ".cpp".to_string(), ".h".to_string(), ".hpp".to_string(),
        ".sh".to_string(), ".bash".to_string(),
        ".txt".to_string(), ".sql".to_string(),
    ]
}

fn default_ignore_dirs() -> Vec<String> {
    vec![
        "models".to_string(), "node_modules".to_string(),
        "target".to_string(), "dist".to_string(), ".git".to_string(),
    ]
}

impl Default for FileFilterConfig {
    fn default() -> Self {
        Self {
            extensions: default_extensions(),
            ignore_dirs: default_ignore_dirs(),
        }
    }
}
