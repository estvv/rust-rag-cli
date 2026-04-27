// src/cli.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rust-rag-cli")]
#[command(about = "Semantic code search with local LLM")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Chat {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        #[arg(short, long, help = "Force re-index before starting")]
        reindex: bool,

        #[arg(long, default_value = ".semantic-index.json", help = "Index file path")]
        index_file: PathBuf,
    },

    Index {
        #[arg(help = "Directory to index")]
        path: PathBuf,

        #[arg(long, default_value = ".semantic-index.json", help = "Index file path")]
        index_file: PathBuf,
    },
}

pub fn parse() -> Cli {
    Cli::parse()
}
