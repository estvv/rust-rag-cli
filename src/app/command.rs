// src/app/command.rs

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Command {
    Models,
    Switch { model: String },
    SwitchEmbed { model: String },
    Index { path: PathBuf },
    Reindex,
    Save,
    Clear,
    Help,
    Quit,
}

pub fn parse(input: &str) -> Option<Command> {
    let input = input.trim();
    if !input.starts_with('/') {
        return None;
    }

    let input = input.trim_start_matches('/');
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        "models" => Some(Command::Models),
        "switch" => {
            parts.get(1)
                .map(|m| Command::Switch { model: m.to_string() })
        }
        "switch-embed" => {
            parts.get(1)
                .map(|m| Command::SwitchEmbed { model: m.to_string() })
        }
        "index" => {
            let path = parts.get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            Some(Command::Index { path })
        }
        "reindex" => Some(Command::Reindex),
        "save" => Some(Command::Save),
        "clear" => Some(Command::Clear),
        "help" | "h" | "?" => Some(Command::Help),
        "quit" | "q" | "exit" => Some(Command::Quit),
        _ => None,
    }
}

pub fn help_text() -> &'static str {
    "/models          - List models
    /switch <model>  - Change chat model
    /switch-embed <model> - Change embed model
    /index [path]     - Index directory
    /reindex          - Reindex current project
    /save             - Save index
    /clear            - Clear chat
    /help             - Show help
    /quit             - Exit

    @file or @dir/    - Include files
    Tab               - Complete @path"
}
