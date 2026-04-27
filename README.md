# rust-rag-cli

A local-first semantic code search and chat CLI using RAG (Retrieval-Augmented Generation) with Ollama.

[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     User Interface (ratatui)                │
│  ┌──────────────────────┐  ┌──────────────────────────────┐ │
│  │   Chat Panel         │  │   Context Panel              │ │
│  │   (Messages/Draft)   │  │   (Retrieved Code Chunks)    │ │
│  └──────────────────────┘  └──────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Input Bar (Commands / Messages)                      │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    App State (Arc<Mutex<App>>)              │
│  ├─ messages: Vec<Message>                                  │
│  ├─ input: String                                           │
│  ├─ file_references: Vec<FileReference>                     │
│  ├─ indexing_progress: Option<IndexingProgress>             │
│  └─ streaming_message: Option<String>                       │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   ChatService (Arc<Mutex<>>)                │
│  ├─ client: OllamaClient                                    │
│  ├─ config: Config (models, base_url)                       │
│  └─ index: Arc<Mutex<SemanticIndex>>                        │
│      └─ chunks: Vec<CodeChunk>                              │
│          ├─ file_path: String                               │
│          ├─ content: String                                 │
│          └─ embedding: Vec<f32>                             │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Ollama API (HTTP)                         │
│  ├─ /api/embeddings  - Get embeddings                       │
│  ├─ /api/generate    - Chat completion (streaming)          │
│  └─ /api/tags        - List available models                │
└─────────────────────────────────────────────────────────────┘
```

## Query Flow

```
User Input                                  LLM Response
    │                                             ▲
    │  1. Parse @file references                  │
    │  2. Load file contents                      │
    ▼                                             │
┌─────────────────────────────────────┐           │
│         ChatService                 │           │
│  ┌──────────────────────────────┐   │           │
│  │ query(prompt)                │   │           │
│  │ ├─ Get embedding from Ollama │   │           │
│  │ └─ cosine_similarity()       │   │           │
│  └──────────────────────────────┘   │           │
│             │                       │           │
│             ▼                       │           │
│  ┌─────────────────────────────┐    │           │
│  │ retrieve_context(top_k=5)   │    │           │
│  │ └─ Score and sort chunks    │    │           │
│  └─────────────────────────────┘    │           │
│             │                       │           │
│             ▼                       │           │
│  ┌─────────────────────────────┐    │           │
│  │ build_prompt_with_refs()    │    │           │
│  │ ├─ File references          │    │           │
│  │ ├─ Retrieved context        │    │           │
│  │ └─ User question            │    │           │
│  └─────────────────────────────┘    │           │
└─────────────────────────────────────┘           │
             │                                    │
             └────────────────────────────────────┘
                        ask_question_streaming()
```

## Features

### Core Functionality
- **SEMANTIC SEARCH** - Vector similarity search over code chunks
- **RAG CHAT** - Context-aware conversations with local LLM
- **FILE REFERENCES** - Include files/directories with @path syntax
- **STREAMING RESPONSES** - Real-time response streaming
- **INDEX PERSISTENCE** - Save/load index to avoid re-processing

### User Interface
- **TUI INTERFACE** - Beautiful terminal UI with ratatui
- **SPLIT VIEW** - Chat and context panels side-by-side
- **COMMAND INPUT** - Slash commands for actions
- **AUTO-COMPLETION** - Tab completion for @files and commands
- **PROGRESS DISPLAY** - Real-time indexing progress bar

### File Processing
- **DIRECTORY SCANNING** - Recursive file discovery
- **IGNORE PATTERNS** - Skips target/, node_modules/, .git/, etc.
- **FILE FILTERING** - Support for .rs, .toml, .json, .yaml, .md
- **TEXT CHUNKING** - Configurable chunk size and overlap

### Model Integration
- **OLLAMA CLIENT** - HTTP client for local LLM inference
- **MODEL SWITCHING** - Change chat/embed models at runtime
- **MODEL LISTING** - Discover available Ollama models

See [FEATURES.md](FEATURES.md) for the full roadmap with planned features.

## Quick Start

```bash
# Build and run
cargo run -- chat

# Run with specific project path
cargo run -- chat --path /path/to/project

# Run with custom index file
cargo run -- chat --index-file .my-index.json

# Force re-index
cargo run -- chat --reindex

# Index without starting chat
cargo run -- index /path/to/project
```

### Prerequisites

1. Install [Ollama](https://ollama.com/)
2. Pull required models:
   ```bash
   ollama pull llama3
   ollama pull nomic-embed-text
   ```

### Two Modes

**1. Chat Mode (Default)**
```bash
cargo run -- chat
# Starts interactive TUI
# Auto-indexes if no index found
```

**2. Index-Only Mode**
```bash
cargo run -- index /path/to/project
# Creates index file
# Use with --index-file flag
```

## Commands

| Command | Description | Example |
|---------|-------------|---------|
| `/models` | List available Ollama models | `/models` |
| `/switch <model>` | Change chat model | `/switch llama3` |
| `/switch-embed <model>` | Change embed model | `/switch-embed nomic-embed-text` |
| `/index [path]` | Index a directory | `/index ./src` |
| `/reindex` | Re-index current project | `/reindex` |
| `/save` | Save current index | `/save` |
| `/clear` | Clear chat history | `/clear` |
| `/help` | Show help text | `/help` or `/h` or `?` |
| `/quit` | Exit application | `/quit` or `/q` |

### File References

Include specific files or directories in your questions:

```
@src/main.rs How does the event loop work?
@src/db/ Explain the database schema
@README.md What is this project about?
```

Include multiple files:
```
@src/client.rs @src/service/chat.rs How do these modules interact?
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Enter` | Send message or execute command |
| `Tab` | Accept auto-completion or next suggestion |
| `Shift+Tab` | Previous suggestion |
| `Esc` | Cancel suggestions |
| `←/→` | Move cursor |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Home` | Move cursor to start |
| `PageUp` | Scroll chat up |
| `PageDown` | Scroll chat down |
| `Ctrl+C` | Quit |

## Modules

| Module | Description |
|--------|-------------|
| `main.rs` | Application entry point, event loop, TUI setup |
| `cli.rs` | Command-line argument parsing |
| `app/command.rs` | Command parsing and dispatch |
| `app/state.rs` | Application state (messages, input, progress) |
| `app/action.rs` | State reduction actions |
| `service/chat.rs` | RAG service (index, retrieve, chat) |
| `service/mod.rs` | Service configuration |
| `clients/ollama.rs` | Ollama HTTP client (embeddings, chat, models) |
| `client.rs` | HTTP request/response types |
| `db/store.rs` | Semantic index (chunks, save/load) |
| `scrapper.rs` | Directory scanning and text chunking |
| `input/handler.rs` | Keyboard input handling |
| `ui/render.rs` | TUI rendering with ratatui |

## Data Model

```rust
struct CodeChunk {
    file_path: String,
    content: String,
    embedding: Vec<f32>,  // Vector embedding from Ollama
}

struct SemanticIndex {
    chunks: Vec<CodeChunk>,
}

struct FileReference {
    path: PathBuf,
    content: Option<String>,  // Loaded file/directory content
}

struct Message {
    source: MessageSource,  // User, Assistant, System
    content: String,
}

struct App {
    messages: Vec<Message>,
    input: String,
    streaming_message: Option<String>,
    file_references: Vec<FileReference>,
    indexing_progress: Option<IndexingProgress>,
    current_model: String,
    current_embed_model: String,
    available_models: Vec<String>,
    suggestions: Vec<String>,
    // ...
}
```

## Indexing Strategy

1. **Directory Scan**
   - Recursively find files in target directory
   - Skip ignored directories (models/, node_modules/, target/, dist/, .git/)
   - Filter by extension (.rs, .toml, .json, .yaml, .md)

2. **Text Chunking**
   - Split files into overlapping line-based chunks
   - Default: 50 lines per chunk, 10 line overlap
   - Preserves code context across chunk boundaries

3. **Embedding Generation**
   - Send each chunk to Ollama embedding API
   - Store chunk + embedding + file path
   - Streaming progress updates

4. **Persistence**
   - Save index as JSON file
   - Load on startup to avoid re-indexing

## Retrieval Algorithm

```rust
fn retrieve_context(query: &str, top_k: usize) -> Vec<CodeChunk> {
    // 1. Embed the query
    let query_embedding = ollama.get_embedding(query);

    // 2. Score chunks by cosine similarity
    let mut scored: Vec<_> = index.chunks.iter()
        .map(|chunk| {
            let score = cosine_similarity(&query_embedding, &chunk.embedding);
            (score, chunk.clone())
        })
        .collect();

    // 3. Sort by score descending
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    // 4. Return top-k chunks
    scored.into_iter().take(top_k).map(|(_, chunk)| chunk).collect()
}
```

## Prompt Construction

```
[Referenced files (if any @path)]

[Related context from codebase]
--- src/file1.rs ---
[chunk content]

--- src/file2.rs ---
[chunk content]

Question: [user question]
```

## Examples

### Basic Chat

```
> How does the event loop work?
[Assistant responds with context from indexed code]
```

### With File Reference

```
> @src/main.rs Explain the EventLoop struct
[Assistant sees full content of src/main.rs]
```

### Switch Models

```
> /models
Available models:
  - llama3
  - nomic-embed-text
  - codellama

> /switch codellama
Switched to codellama
```

### Index Progress

```
Status: Indexing ./src
[45/120] files, 823 chunks - src/db/store.rs
```

## Project Structure

```
src/
├── main.rs                 # Entry point, event loop
├── cli.rs                  # CLI argument parsing
├── client.rs               # HTTP request types
├── scrapper.rs             # File scanning, chunking
├── app/
│   ├── mod.rs              # App module exports
│   ├── command.rs          # /command parsing
│   ├── state.rs            # App state struct
│   └── action.rs           # State reduction
├── clients/
│   ├── mod.rs              # Client module exports
│   └── ollama.rs           # Ollama HTTP client
├── db/
│   ├── mod.rs              # DB module exports
│   └── store.rs            # SemanticIndex storage
├── service/
│   ├── mod.rs              # Service config
│   └── chat.rs             # RAG service logic
├── input/
│   ├── mod.rs              # Input module exports
│   └── handler.rs          # Keyboard input handler
└── ui/
    ├── mod.rs              # UI module exports
    └── render.rs           # TUI rendering
```

## Dependencies

| Crate | Version | Usage |
|-------|---------|-------|
| `tokio` | 1 | Async runtime |
| `reqwest` | 0.11 | HTTP client for Ollama API |
| `serde` | 1.0 | Serialization |
| `serde_json` | 1.0 | JSON parsing |
| `ratatui` | 0.28 | Terminal UI |
| `crossterm` | 0.28 | Terminal manipulation |
| `clap` | 4 | CLI argument parsing |
| `arboard` | 3 | Clipboard support |
| `tokio-stream` | 0.1 | Stream utilities |
| `futures-util` | 0.3 | Stream extensions |

## Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| Base URL | `http://localhost:11434` | Ollama server endpoint |
| Chat model | `llama3` | Default model for chat |
| Embed model | `nomic-embed-text` | Default model for embeddings |
| Index file | `.semantic-index.json` | Index persistence path |
| Top-K | 5 | Number of chunks to retrieve |
| Chunk size | 50 lines | Lines per chunk |
| Chunk overlap | 10 lines | Overlap between chunks |

## Performance

- **Indexing**: Streaming embeddings, real-time progress
- **Retrieval**: O(n) similarity search (HNSW planned)
- **UI**: Non-blocking async event loop
- **Memory**: Index stored in Arc<Mutex<SemanticIndex>>

## Roadmap

See [FEATURES.md](FEATURES.md) for planned and completed features.

## License

Licensed under [MIT](LICENSE)
