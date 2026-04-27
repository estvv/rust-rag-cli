// src/app/state.rs

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageSource {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub source: MessageSource,
    pub content: String,
}

impl Message {
    pub fn user(content: String) -> Self {
        Self { source: MessageSource::User, content }
    }

    pub fn assistant(content: String) -> Self {
        Self { source: MessageSource::Assistant, content }
    }

    pub fn system(content: String) -> Self {
        Self { source: MessageSource::System, content }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IndexingProgress {
    pub files_done: usize,
    pub files_total: usize,
    pub chunks_done: usize,
    pub current_file: String,
}

#[derive(Debug, Clone)]
pub enum IndexingStatus {
    Idle,
    InProgress {
        current: usize,
        total: usize,
        file: String,
    },
    Complete {
        files: usize,
        chunks: usize,
    },
    Error(String),
}

impl Default for IndexingStatus {
    fn default() -> Self {
        IndexingStatus::Idle
    }
}

#[derive(Debug, Clone)]
pub struct FileReference {
    pub path: PathBuf,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MouseSelection {
    pub start_row: u16,
    pub start_col: u16,
    pub end_row: u16,
    pub end_col: u16,
}

#[derive(Debug, Clone)]
pub struct App {
    pub running: bool,
    pub input: String,
    pub cursor_pos: usize,
    pub messages: Vec<Message>,
    pub context_chunks: Vec<crate::db::store::CodeChunk>,
    pub chat_scroll: usize,
    pub context_scroll: usize,
    pub is_loading: bool,
    pub streaming_message: Option<String>,
    pub status: String,
    pub indexing_progress: Option<IndexingProgress>,
    pub indexing_status: IndexingStatus,
    pub current_model: String,
    pub current_embed_model: String,
    pub available_models: Vec<String>,
    pub file_references: Vec<FileReference>,
    pub suggestions: Vec<String>,
    pub suggestion_index: usize,
    pub show_suggestions: bool,
    pub command_hint: String,
    pub mouse_selection: Option<MouseSelection>,
    pub mouse_dragging: bool,
    pub thinking_dots: usize,
    pub follow_bottom: bool,
    pub max_scroll: usize,
    pub benchmark_mode: bool,
    pub last_timing: Option<crate::service::TimingMetrics>,
    pub last_retrieved_context: Vec<crate::db::store::CodeChunk>,
    pub syntax_highlight: bool,
    pub theme: String,
    pub debug_mode: bool,
    pub available_profiles: Vec<String>,
    pub current_profile: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            input: String::new(),
            cursor_pos: 0,
            messages: Vec::new(),
            context_chunks: Vec::new(),
            chat_scroll: 0,
            context_scroll: 0,
            is_loading: false,
            streaming_message: None,
            status: String::new(),
            indexing_progress: None,
            indexing_status: IndexingStatus::Idle,
            current_model: "llama3".to_string(),
            current_embed_model: "nomic-embed-text".to_string(),
            available_models: Vec::new(),
            file_references: Vec::new(),
            suggestions: Vec::new(),
            suggestion_index: 0,
            show_suggestions: false,
            command_hint: String::new(),
            mouse_selection: None,
            mouse_dragging: false,
            thinking_dots: 0,
            follow_bottom: true,
            max_scroll: 0,
            benchmark_mode: false,
            last_timing: None,
            last_retrieved_context: Vec::new(),
            syntax_highlight: true,
            theme: "base16-eighties.dark".to_string(),
            debug_mode: false,
            available_profiles: Vec::new(),
            current_profile: None,
        }
    }

    pub fn with_status(mut self, status: String) -> Self {
        self.status = status;
        self
    }

    pub fn with_syntax_highlight(mut self, enabled: bool) -> Self {
        self.syntax_highlight = enabled;
        self
    }

    pub fn with_theme(mut self, theme: String) -> Self {
        self.theme = theme;
        self
    }

    pub fn with_benchmark(mut self, enabled: bool) -> Self {
        self.benchmark_mode = enabled;
        self
    }

    pub fn with_debug(mut self, enabled: bool) -> Self {
        self.debug_mode = enabled;
        self
    }

    pub fn parse_references(&mut self, input: &str, base_path: &std::path::Path) {
        self.file_references.clear();

        for word in input.split_whitespace() {
            if word.starts_with('@') {
                let path_str = word.trim_start_matches('@');
                let path = base_path.join(path_str);

                if path.exists() {
                    let content = if path.is_file() {
                        std::fs::read_to_string(&path).ok()
                    } else if path.is_dir() {
                        let files = crate::scrapper::scrapper(&path);
                        let content: String = files.into_iter()
                            .map(|(_, content)| content)
                            .collect::<Vec<_>>()
                            .join("\n\n--- ---\n\n");

                        if content.len() > 50000 {
                            Some(format!("{}\n\n... (truncated, folder too large)",
                                content.chars().take(50000).collect::<String>()))
                        } else {
                            Some(content)
                        }
                    } else {
                        None
                    };

                    self.file_references.push(FileReference { path, content });
                }
            }
        }
    }

    pub fn get_word_at_cursor(&self) -> Option<(usize, String)> {
        if self.input.is_empty() || self.cursor_pos == 0 {
            return None;
        }

        let text_before_cursor = &self.input[..self.cursor_pos];

        let start = text_before_cursor.rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);

        let word = &self.input[start..self.cursor_pos];

        if word.starts_with('@') {
            Some((start, word.to_string()))
        } else {
            None
        }
    }

    pub fn update_suggestions(&mut self, base_path: &std::path::Path) {
        self.suggestions.clear();
        self.suggestion_index = 0;
        self.command_hint.clear();

        let input = self.input.trim().to_string();
        let models = self.available_models.clone();

        if input.starts_with('/') {
            let input_after_slash = &input[1..];

            if let Some(space_pos) = input_after_slash.find(' ') {
                let cmd = &input_after_slash[..space_pos];
                let arg_input = input_after_slash[space_pos + 1..].trim_start();

                self.command_hint = match cmd {
                    "switch" => format!("/switch <model>  - current: {}", self.current_model),
                    "switch-embed" => format!("/switch-embed <model>  - current: {}", self.current_embed_model),
                    "index" => "/index <path>  - directory to index".to_string(),
                    "models" => "/models  - list available models".to_string(),
                    "reindex" => "/reindex  - reindex current project".to_string(),
                    "save" => "/save  - save current index".to_string(),
                    "clear" => "/clear  - clear chat history".to_string(),
                    "help" => "/help  - show help".to_string(),
                    "quit" => "/quit  - exit".to_string(),
                    _ => String::new(),
                };

                match cmd {
                    "switch" | "switch-embed" => {
                        for model in &models {
                            if arg_input.is_empty() || model.starts_with(arg_input) {
                                self.suggestions.push(model.clone());
                            }
                        }
                    }
                    "index" => {
                        if arg_input.starts_with('/') || arg_input.starts_with('.') || arg_input.is_empty() {
                            self.resolve_arg_file_completions(arg_input, base_path);
                        }
                    }
                    _ => {}
                }
            } else {
                let commands: Vec<(&str, &str)> = vec![
                    ("/models", "list available models"),
                    ("/switch", "change chat model"),
                    ("/switch-embed", "change embed model"),
                    ("/index", "index a directory"),
                    ("/reindex", "reindex current project"),
                    ("/save", "save current index"),
                    ("/clear", "clear chat history"),
                    ("/help", "show help"),
                    ("/quit", "exit"),
                ];

                for (cmd, hint) in &commands {
                    if cmd.starts_with(&input) && cmd != &input {
                        self.suggestions.push(cmd.to_string());
                    }
                    if input == *cmd {
                        self.command_hint = format!("{} - {}", cmd, hint);
                    }
                }
            }
            self.show_suggestions = !self.suggestions.is_empty();
        } else if let Some((_, partial)) = self.get_word_at_cursor() {
            if partial.starts_with('@') {
                self.resolve_file_completions(&partial, base_path);
            } else {
                self.show_suggestions = false;
            }
        } else {
            self.show_suggestions = false;
        }
    }

    fn resolve_arg_file_completions(&mut self, partial: &str, base_path: &std::path::Path) {
        let (parent, path_prefix) = if partial.is_empty() {
            (base_path.to_path_buf(), "")
        } else if partial.ends_with('/') {
            let path = base_path.join(partial);
            (path, partial)
        } else if let Some(slash_pos) = partial.rfind('/') {
            let dir_part = &partial[..slash_pos + 1];
            (base_path.join(dir_part), dir_part)
        } else {
            (base_path.to_path_buf(), "")
        };

        let file_prefix = partial.rsplit('/').next().unwrap_or("");

        if let Ok(entries) = std::fs::read_dir(&parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(file_prefix) {
                    let suffix = if entry.path().is_dir() { "/" } else { "" };
                    self.suggestions.push(format!("{}{}{}", path_prefix, name, suffix));
                }
            }
            self.suggestions.sort();
        }
    }

    fn resolve_file_completions(&mut self, partial: &str, base_path: &std::path::Path) {
        let partial_path = partial.trim_start_matches('@');

        let (parent, path_prefix) = if partial_path.is_empty() {
            (base_path.to_path_buf(), "")
        } else if partial_path.ends_with('/') {
            (base_path.join(partial_path), partial_path)
        } else if let Some(slash_pos) = partial_path.rfind('/') {
            let dir_part = &partial_path[..slash_pos + 1];
            (base_path.join(dir_part), dir_part)
        } else {
            (base_path.to_path_buf(), "")
        };

        let file_prefix = partial_path.rsplit('/').next().unwrap_or("");

        if let Ok(entries) = std::fs::read_dir(&parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(file_prefix) {
                    let suffix = if entry.path().is_dir() { "/" } else { "" };
                    self.suggestions.push(format!("@{}{}{}", path_prefix, name, suffix));
                }
            }
            self.suggestions.sort();
        }

        self.show_suggestions = !self.suggestions.is_empty();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn move_cursor_to(&mut self, pos: usize) {
        self.cursor_pos = pos.min(self.input.len());
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn delete_char_at_cursor(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
