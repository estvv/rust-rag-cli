// src/app/command.rs

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Command {
    Models,
    Switch { model: String },
    SwitchEmbed { model: String },
    Index { path: PathBuf },
    IncrementalIndex { path: PathBuf },
    Reindex,
    Save,
    Export { path: Option<PathBuf> },
    Import { path: PathBuf },
    Clear,
    ClearHistory,
    Benchmark,
    Profiles,
    Profile { name: String },
    Help,
    Quit,
    SyntaxToggle,
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
        "incremental-index" | "smart-index" => {
            let path = parts.get(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            Some(Command::IncrementalIndex { path })
        }
        "reindex" => Some(Command::Reindex),
        "save" => Some(Command::Save),
        "export" => {
            let path = parts.get(1).map(PathBuf::from);
            Some(Command::Export { path })
        }
        "import" => {
            parts.get(1)
                .map(|p| Command::Import { path: PathBuf::from(p) })
        }
        "clear" => Some(Command::Clear),
        "clear-history" => Some(Command::ClearHistory),
        "benchmark" | "bench" => Some(Command::Benchmark),
        "profiles" => Some(Command::Profiles),
        "profile" => {
            parts.get(1)
                .map(|n| Command::Profile { name: n.to_string() })
        }
        "syntax" | "highlight" => Some(Command::SyntaxToggle),
        "help" | "h" | "?" => Some(Command::Help),
        "quit" | "q" | "exit" => Some(Command::Quit),
        _ => None,
    }
}

pub fn help_text() -> &'static str {
    "/models              - List models
    /switch <model>      - Change chat model
    /switch-embed <model> - Change embed model
    /index <path>        - Index directory (full)
    /incremental-index <path> - Smart reindex (only changed files)
    /reindex             - Reindex current project
    /save                - Save index
    /export [file]       - Export last retrieved context
    /import <file>       - Load questions from file
    /clear               - Clear chat
    /clear-history       - Clear conversation context
    /benchmark           - Show timing metrics
    /profiles            - List config profiles
    /profile <name>      - Switch profile
    /syntax              - Toggle syntax highlighting
    /help                - Show help
    /quit                - Exit

    @file or @dir/       - Include files in question
    Tab                  - Complete @path"
}
