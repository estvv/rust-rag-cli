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

        #[arg(long, help = "Path to config file")]
        config: Option<PathBuf>,

        #[arg(long, help = "Configuration profile to use")]
        profile: Option<String>,

        #[arg(long, help = "Enable debug logging")]
        debug: bool,

        #[arg(long, help = "Debug log file path")]
        debug_file: Option<PathBuf>,

        #[arg(long, help = "Enable benchmark mode")]
        benchmark: bool,

        #[arg(long, help = "Benchmark output file")]
        benchmark_output: Option<PathBuf>,

        #[arg(long, help = "Disable syntax highlighting")]
        no_syntax: bool,

        #[arg(long, help = "Theme for syntax highlighting")]
        theme: Option<String>,

        #[arg(long, help = "Enable file watching")]
        watch: bool,
    },

    Index {
        #[arg(help = "Directory to index")]
        path: PathBuf,

        #[arg(long, default_value = ".semantic-index.json", help = "Index file path")]
        index_file: PathBuf,

        #[arg(long, help = "Path to config file")]
        config: Option<PathBuf>,

        #[arg(long, help = "Incremental indexing (only changed files)")]
        incremental: bool,
    },

    Config {
        #[arg(long, help = "Show config path")]
        show_path: bool,

        #[arg(long, help = "Generate default config")]
        generate: bool,

        #[arg(long, help = "Config file path")]
        output: Option<PathBuf>,
    },
}

pub fn parse() -> Cli {
    Cli::parse()
}
