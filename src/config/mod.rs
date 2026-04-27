// src/config/mod.rs

pub mod settings;

pub use settings::{Config, Settings, UiConfig, RateLimitConfig, WatcherConfig, FileFilterConfig};

use std::path::PathBuf;
use std::fs;
use std::io;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rust-rag-cli")
}

pub fn default_config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load_config(path: Option<&PathBuf>) -> Config {
    let config_path = path.cloned().unwrap_or_else(default_config_path);

    let mut config = if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                match toml::from_str::<Config>(&content) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse config file: {}", e);
                        Config::default()
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to read config file: {}", e);
                Config::default()
            }
        }
    } else {
        Config::default()
    };

    config.apply_env_overrides();
    config
}

pub fn save_config(config: &Config, path: Option<&PathBuf>) -> io::Result<()> {
    let config_path = path.cloned().unwrap_or_else(default_config_path);

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    fs::write(&config_path, content)
}
