# FEATURES

## Core Functionality
- [x] SEMANTIC SEARCH - Cosine similarity-based code retrieval using embeddings.
- [x] RAG CHAT - Context-aware chat with local LLM via Ollama.
- [x] FILE REFERENCES - Include files/directories in prompts with @path syntax.
- [x] STREAMING RESPONSES - Real-time response streaming from LLM.
- [x] INDEXING - Automatic chunking and embedding of code files.
- [x] PERSISTENCE - Save/load semantic index to JSON file.
- [ ] CONVERSATION HISTORY - Multi-turn conversation context retention.
- [ ] FOLLOW-UP QUESTIONS - Automatic follow-up detection and context reuse.

## File Processing
- [x] DIRECTORY SCANNING - Recursive file discovery with ignore patterns.
- [x] FILE FILTERING - Support for .rs, .toml, .json, .yaml, .md files.
- [x] IGNORE DIRECTORIES - Skips models/, node_modules/, target/, dist/, .git/.
- [x] TEXT CHUNKING - Configurable chunk size and overlap.
- [ ] FILE WATCHING - Auto-reindex on file changes.
- [ ] CODE PARSING - AST-aware chunk boundaries.
- [ ] LANGUAGE SUPPORT - Additional file types (Python, JS, Go, etc.).

## User Interface
- [x] TUI INTERFACE - Terminal UI with ratatui.
- [x] SPLIT VIEW - Chat panel and context panel side-by-side.
- [x] COMMAND INPUT - Slash commands for configuration and actions.
- [x] AUTO-COMPLETION - Tab completion for commands and @file paths.
- [x] MODEL SELECTION - Switch between available Ollama models.
- [x] STATUS BAR - Real-time status and progress indicators.
- [x] MOUSE SUPPORT - Text selection with mouse drag.
- [ ] SYNTAX HIGHLIGHTING - Code highlighting in responses.
- [ ] THEME SUPPORT - Customizable color schemes.
- [ ] KEYBINDINGS - Configurable keyboard shortcuts.

## Indexing & Search
- [x] COSINE SIMILARITY - Vector similarity scoring for retrieval.
- [x] TOP-K RETRIEVAL - Configurable number of context chunks.
- [x] STREAMING INDEX - Progressive indexing with progress display.
- [x] INDEX SAVE/LOAD - Persist index to avoid re-indexing.
- [ ] HNSW INDEX - Approximate nearest neighbor for faster search.
- [ ] THRESHOLD FILTERING - Relevance-based chunk filtering.
- [ ] METADATA STORE - Track file modification times for incremental updates.

## Model Integration
- [x] OLLAMA CLIENT - HTTP client for local LLM inference.
- [x] EMBEDDING API - Generate embeddings via Ollama API.
- [x] CHAT API - Query LLM with context-augmented prompts.
- [x] MODEL LISTING - Discover available Ollama models.
- [x] MODEL SWITCHING - Runtime model selection for chat and embed.
- [ ] OPENAI COMPAT - Support for OpenAI-compatible endpoints.
- [ ] MULTI-PROVIDER - Support for multiple LLM providers.
- [ ] API KEY MANAGEMENT - Secure storage for remote API keys.

## Configuration
- [x] CLI ARGUMENTS - Command-line arguments for path and index file.
- [x] DEFAULT MODELS - Configurable chat and embed model defaults.
- [x] BASE URL - Configurable Ollama server endpoint.
- [ ] CONFIG FILES - TOML/YAML configuration files.
- [ ] ENV VARIABLES - Environment-based configuration.
- [ ] PROFILES - Multiple configuration profiles.

## Performance & UX
- [x] ASYNC RUNTIME - Tokio-based async operations.
- [x] PROGRESS INDICATORS - Real-time indexing progress.
- [x] LOADING STATES - Visual feedback during operations.
- [x] CLIPBOARD SUPPORT - Copy selected text to clipboard.
- [ ] CACHING - Cache embeddings for repeated queries.
- [ ] BACKGROUND INDEXING - Non-blocking index operations.
- [ ] RATE LIMITING - Throttle API requests.

## Developer Tools
- [ ] DEBUG MODE - Verbose logging and diagnostics.
- [ ] EXPORT CONTEXT - Save retrieved context to file.
- [ ] IMPORT QUESTIONS - Load questions from file.
- [ ] BENCHMARK MODE - Performance metrics collection.
