// src/main.rs

mod app;
mod cli;
mod client;
mod clients;
mod config;
mod db;
mod dev;
mod input;
mod scrapper;
mod secrets;
mod service;
mod ui;
mod watcher;

use app::{App, Action, reduce, Message, Command, parse_command, help_text, IndexingProgress, IndexingStatus};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event as CrosstermEvent, MouseButton, MouseEventKind},
};
use dev::init_debug;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};

#[derive(Debug, Clone)]
pub enum Event {
    Key(event::KeyEvent),
    Mouse(event::MouseEvent),
    Resize(u16, u16),
    LlmResponse(String),
    LlmStreamChunk(String),
    LlmStreamDone { timing: service::TimingMetrics },
    LlmError(String),
    ModelsList(Vec<String>),
    IndexProgress(IndexingProgress),
    IndexComplete { files: usize, chunks: usize },
    IndexError(String),
    Tick,
    FileChanged(Vec<PathBuf>),
    FileRemoved(Vec<PathBuf>),
    WarmupComplete,
}

struct EventLoop {
    receiver: UnboundedReceiver<Event>,
    sender: Arc<UnboundedSender<Event>>,
}

impl EventLoop {
    fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let sender = Arc::new(sender);

        let event_sender = sender.clone();
        tokio::spawn(async move {
            loop {
                if event::poll(tick_rate).unwrap() {
                    match event::read().unwrap() {
                        CrosstermEvent::Key(key) => {
                            let _ = event_sender.send(Event::Key(key));
                        }
                        CrosstermEvent::Mouse(mouse) => {
                            let _ = event_sender.send(Event::Mouse(mouse));
                        }
                        CrosstermEvent::Resize(w, h) => {
                            let _ = event_sender.send(Event::Resize(w, h));
                        }
                        _ => {}
                    }
                } else {
                    let _ = event_sender.send(Event::Tick);
                }
            }
        });

        Self { receiver, sender }
    }

    fn sender(&self) -> Arc<UnboundedSender<Event>> {
        self.sender.clone()
    }

    async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = cli::parse();

    match args.command {
        cli::Commands::Chat {
            path,
            reindex,
            index_file,
            config,
            profile,
            debug,
            debug_file,
            benchmark,
            benchmark_output: _,
            no_syntax,
            theme,
            watch,
        } => {
            run_chat(
                path,
                reindex,
                index_file,
                config,
                profile,
                debug,
                debug_file,
                benchmark,
                no_syntax,
                theme,
                watch,
            ).await
        }
        cli::Commands::Index {
            path,
            index_file,
            config,
            incremental,
        } => {
            run_index(path, index_file, config, incremental).await
        }
        cli::Commands::Config { show_path, generate, output } => {
            run_config(show_path, generate, output)
        }
    }
}

fn run_config(show_path: bool, generate: bool, output: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if show_path {
        println!("{}", config::default_config_path().display());
        return Ok(());
    }

    if generate {
        let output_path = output.unwrap_or_else(config::default_config_path);
        let default_config = config::Config::default();
        config::save_config(&default_config, Some(&output_path))?;
        println!("Generated default config at {}", output_path.display());
    }

    Ok(())
}

async fn run_index(
    path: PathBuf,
    index_file: PathBuf,
    _config_path: Option<PathBuf>,
    incremental: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Indexing {}...", path.display());

    let app_config = config::load_config(None);
    let service = service::ChatService::new(app_config);

    if incremental {
        service.incremental_index(&path, |done, total, chunks, file| {
            print!("\r  [{}/{}] files, {} chunks - {}", done, total, chunks, file);
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }).await?;
    } else {
        service.index_directory_streaming(&path, |done, total, chunks, file| {
            print!("\r  [{}/{}] files, {} chunks - {}", done, total, chunks, file);
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }).await?;
    }

    println!();

    service.save_index(index_file.to_str().unwrap()).await?;
    println!("Saved to {}", index_file.display());

    let stats = service.get_index_stats().await;
    println!("Done: {} files, {} chunks", stats.1, stats.0);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_chat(
    project_path: PathBuf,
    reindex: bool,
    index_file: PathBuf,
    config_path: Option<PathBuf>,
    profile: Option<String>,
    debug: bool,
    debug_file: Option<PathBuf>,
    _benchmark_mode: bool,
    no_syntax: bool,
    theme: Option<String>,
    _enable_watcher: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_debug(debug, debug_file.as_deref());

    let mut app_config = config::load_config(config_path.as_ref());

    if let Some(profile_name) = &profile {
        app_config = app_config.with_profile(profile_name);
    }

    let settings = app_config.settings().clone();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let event_loop = EventLoop::new(tick_rate);
    let event_sender = event_loop.sender();

    let service = Arc::new(tokio::sync::Mutex::new(service::ChatService::new(app_config.clone())));

    {
        let service_clone = service.clone();
        let sender_clone = event_sender.clone();
        tokio::spawn(async move {
            if service_clone.lock().await.warmup().await.is_ok() {
                let _ = sender_clone.send(Event::WarmupComplete);
            }
        });
    }

    let auto_index = false;

    let mut app = App::new()
        .with_status("Ready".to_string())
        .with_syntax_highlight(!no_syntax && app_config.ui.syntax_highlight)
        .with_theme(theme.unwrap_or(app_config.ui.theme.clone()));

    app.current_model = settings.chat_model.clone();
    app.current_embed_model = settings.embed_model.clone();

    if index_file.exists() && !reindex && !auto_index {
        if service.lock().await.load_index(index_file.to_str().unwrap()).await.is_ok() {
            let stats = service.lock().await.get_index_stats().await;
            let status = if stats.0 == 0 {
                "No index. /index <path>".to_string()
            } else {
                format!("Loaded {} chunks from {} files", stats.0, stats.1)
            };
            app = app.with_status(status);
            let res = run(&mut terminal, app, service.clone(), event_loop, event_sender, index_file, project_path).await;
            cleanup(&mut terminal, res)
        } else {
            cleanup(&mut terminal, Err("Failed to load index".into()))
        }
    } else if reindex || auto_index {
        app.indexing_progress = Some(IndexingProgress::default());
        app.indexing_status = IndexingStatus::InProgress {
            current: 0,
            total: 0,
            file: String::new(),
        };
        app.status = format!("Indexing {}", project_path.display());

        let service_clone = service.clone();
        let sender_clone = event_sender.clone();
        let path_clone = project_path.clone();
        let index_file_clone = index_file.clone();

        tokio::spawn(async move {
            let mut total = 0;
            let result = service_clone.lock().await.index_directory_streaming(&path_clone, |done, total_count, chunks, file| {
                total = total_count;
                let _ = sender_clone.send(Event::IndexProgress(IndexingProgress {
                    files_done: done,
                    files_total: total_count,
                    chunks_done: chunks,
                    current_file: file.to_string(),
                }));
            }).await;

            match result {
                Ok(_) => {
                    let stats = service_clone.lock().await.get_index_stats().await;
                    let _ = sender_clone.send(Event::IndexComplete {
                        files: total,
                        chunks: stats.0,
                    });
                    let _ = service_clone.lock().await.save_index(index_file_clone.to_str().unwrap()).await;
                }
                Err(e) => {
                    let _ = sender_clone.send(Event::IndexError(e.to_string()));
                }
            }
        });

        let res = run(&mut terminal, app, service.clone(), event_loop, event_sender, index_file, project_path).await;
        cleanup(&mut terminal, res)
    } else {
        let res = run(&mut terminal, app, service.clone(), event_loop, event_sender, index_file, project_path).await;
        cleanup(&mut terminal, res)
    }
}

fn cleanup(terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>, res: Result<(), Box<dyn std::error::Error + Send + Sync>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    res
}

async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    service: Arc<tokio::sync::Mutex<service::ChatService>>,
    mut event_loop: EventLoop,
    sender: Arc<UnboundedSender<Event>>,
    index_file: PathBuf,
    project_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    while app.running {
        terminal.draw(|f| ui::render(f, &mut app))?;

        if let Some(event) = event_loop.next().await {
            match event {
                Event::Key(key) => {
                    if let Some(action) = input::handle(key, &app.input, app.show_suggestions) {
                        match &action {
                            Action::Quit => {
                                app.running = false;
                            }
                            Action::Cancel => {
                                if app.is_loading || app.streaming_message.is_some() {
                                    app.is_loading = false;
                                    app.streaming_message = None;
                                    if let Some(msg) = app.messages.last_mut() {
                                        msg.content.push_str("\n[cancelled]");
                                    }
                                }
                            }
                            Action::ScrollChatUp | Action::ScrollChatDown => {
                                reduce(&mut app, action);
                            }
                            _ if app.is_loading || app.streaming_message.is_some() => {
                                continue;
                            }
                            Action::ExecuteCommand => {
                                if let Some(cmd) = parse_command(&app.input) {
                                    match cmd {
                                        Command::Index { path } => {
                                            let service = service.clone();
                                            let sender = sender.clone();
                                            let path = path.clone();
                                            let index_file = index_file.clone();
                                            app.indexing_progress = Some(IndexingProgress::default());
                                            app.status = format!("Indexing {}", path.display());
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;

                                            tokio::spawn(async move {
                                                let mut total = 0;
                                                let _ = service.lock().await.index_directory_streaming(&path, |done, total_count, chunks, file| {
                                                    total = total_count;
                                                    let _ = sender.send(Event::IndexProgress(IndexingProgress {
                                                        files_done: done,
                                                        files_total: total_count,
                                                        chunks_done: chunks,
                                                        current_file: file.to_string(),
                                                    }));
                                                }).await;
                                                let chunks = service.lock().await.get_chunks().await;
                                                let _ = sender.send(Event::IndexComplete {
                                                    files: total,
                                                    chunks: chunks.len(),
                                                });
                                                let _ = service.lock().await.save_index(index_file.to_str().unwrap()).await;
                                            });
                                        }
                                        Command::Save => {
                                            let service = service.clone();
                                            let index_file = index_file.clone();
                                            app.status = format!("Saved to {}", index_file.display());
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;

                                            tokio::spawn(async move {
                                                let _ = service.lock().await.save_index(index_file.to_str().unwrap()).await;
                                            });
                                        }
                                        Command::Help => {
                                            app.status = help_text().to_string();
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::Quit => {
                                            app.running = false;
                                        }
                                        Command::Models => {
                                            app.is_loading = true;
                                            app.thinking_dots = 0;
                                            app.messages.push(Message::user(app.input.clone()));
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;

                                            let service = service.clone();
                                            let sender = sender.clone();

                                            tokio::spawn(async move {
                                                if let Ok(models) = service.lock().await.list_models().await {
                                                    let names: Vec<String> = models.into_iter()
                                                        .map(|m| m.name.clone())
                                                        .collect();
                                                    let _ = sender.send(Event::ModelsList(names));
                                                }
                                            });
                                        }
                                        Command::Switch { model } => {
                                            service.lock().await.set_chat_model(model.clone());
                                            app.current_model = model.clone();
                                            app.status = format!("Switched to {}", model);
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::SwitchEmbed { model } => {
                                            service.lock().await.set_embed_model(model.clone());
                                            app.status = format!("Switched embed to {}", model);
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::Clear => {
                                            app.messages.clear();
                                            app.status = "Cleared".to_string();
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::ClearHistory => {
                                            let service_clone = service.clone();
                                            tokio::spawn(async move {
                                                service_clone.lock().await.clear_history().await;
                                            });
                                            app.status = "Conversation history cleared".to_string();
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::IncrementalIndex { path } => {
                                            app.status = format!("Incremental indexing {}", path.display());
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;

                                            let service_clone = service.clone();
                                            let sender_clone = sender.clone();
                                            let index_file_clone = index_file.clone();

                                            tokio::spawn(async move {
                                                let mut total = 0;
                                                let result = service_clone.lock().await.incremental_index(&path, |done, total_count, chunks, file| {
                                                    total = total_count;
                                                    let _ = sender_clone.send(Event::IndexProgress(IndexingProgress {
                                                        files_done: done,
                                                        files_total: total_count,
                                                        chunks_done: chunks,
                                                        current_file: file.to_string(),
                                                    }));
                                                }).await;

                                                match result {
                                                    Ok(_) => {
                                                        let stats = service_clone.lock().await.get_index_stats().await;
                                                        let _ = sender_clone.send(Event::IndexComplete {
                                                            files: total,
                                                            chunks: stats.0,
                                                        });
                                                        let _ = service_clone.lock().await.save_index(index_file_clone.to_str().unwrap()).await;
                                                    }
                                                    Err(e) => {
                                                        let _ = sender_clone.send(Event::IndexError(e.to_string()));
                                                    }
                                                }
                                            });
                                        }
                                        Command::Export { path } => {
                                            let export_path = path.unwrap_or_else(|| PathBuf::from("context.json"));
                                            app.status = format!("Exported context to {}", export_path.display());
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::Import { path } => {
                                            if let Ok(content) = std::fs::read_to_string(&path) {
                                                for question in content.lines() {
                                                    if !question.trim().is_empty() {
                                                        app.messages.push(Message::user(question.to_string()));
                                                    }
                                                }
                                                app.status = format!("Imported {} questions", content.lines().count());
                                            } else {
                                                app.status = format!("Failed to import {}", path.display());
                                            }
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::Benchmark => {
                                            let service_clone = service.clone();
                                            let sender_clone = sender.clone();

                                            tokio::spawn(async move {
                                                if let Some(avg) = service_clone.lock().await.get_average_metrics().await {
                                                    let _ = sender_clone.send(Event::LlmResponse(format!(
                                                        "Avg times: embed={:.2}s, retrieval={:.2}s, llm={:.2}s, total={:.2}s",
                                                        avg.embedding_time_ms as f64 / 1000.0,
                                                        avg.retrieval_time_ms as f64 / 1000.0,
                                                        avg.llm_time_ms as f64 / 1000.0,
                                                        avg.total_time_ms as f64 / 1000.0
                                                    )));
                                                }
                                            });

                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::Profiles => {
                                            let profiles: Vec<String> = app.available_profiles.iter()
                                                .map(|p| {
                                                    if Some(p) == app.current_profile.as_ref() {
                                                        format!("* {}", p)
                                                    } else {
                                                        format!("  {}", p)
                                                    }
                                                })
                                                .collect();
                                            app.messages.push(Message::assistant(format!("Profiles:\n{}", profiles.join("\n"))));
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::Profile { name } => {
                                            if app.available_profiles.contains(&name) {
                                                app.current_profile = Some(name.clone());
                                                app.status = format!("Switched to profile: {}", name);
                                            } else {
                                                app.status = format!("Profile '{}' not found", name);
                                            }
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::SyntaxToggle => {
                                            app.syntax_highlight = !app.syntax_highlight;
                                            app.status = format!("Syntax highlighting: {}", if app.syntax_highlight { "ON" } else { "OFF" });
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                        Command::Reindex => {
                                            app.status = "Use /index <path> or /incremental-index <path>".to_string();
                                            app.input.clear();
                                            app.cursor_pos = 0;
                                            app.show_suggestions = false;
                                        }
                                    }
                                } else {
                                    app.status = format!("Unknown: {}", app.input);
                                    app.input.clear();
                                    app.cursor_pos = 0;
                                    app.show_suggestions = false;
                                }
                            }
                            Action::InsertChar(_) => {
                                reduce(&mut app, action);
                                app.update_suggestions(&project_path);
                            }
                            Action::Backspace => {
                                reduce(&mut app, action);
                                app.update_suggestions(&project_path);
                            }
                            Action::SendMessage => {
                                if !app.input.is_empty() && !app.input.trim().starts_with('/') {
                                    let input = app.input.clone();
                                    app.parse_references(&input, &project_path);

                                    let prompt = input.split_whitespace()
                                        .filter(|w| !w.starts_with('@'))
                                        .map(|s| s.to_string())
                                        .collect::<Vec<_>>()
                                        .join(" ");

                                    let file_refs = app.file_references.clone();
                                    app.messages.push(Message::user(app.input.clone()));
                                    app.input.clear();
                                    app.cursor_pos = 0;
                                     app.is_loading = true;
                                     app.thinking_dots = 0;
                                     app.streaming_message = Some(String::new());
                                     app.show_suggestions = false;
                                     app.follow_bottom = true;

                                    tracing::info!("User: {}", prompt);

                                    let sender_clone = sender.clone();
                                    let service_clone = service.clone();

                                    tokio::spawn(async move {
                                        let result = service_clone.lock().await.chat_streaming(
                                            &prompt,
                                            &file_refs,
                                            |chunk| {
                                                let _ = sender_clone.send(Event::LlmStreamChunk(chunk.to_string()));
                                            }
                                        ).await;

                                        match result {
                                            Ok((_, _, timing)) => {
                                                let _ = sender_clone.send(Event::LlmStreamDone { timing });
                                            }
                                            Err(e) => {
                                                let _ = sender_clone.send(Event::LlmError(e.to_string()));
                                            }
                                        }
                                    });
                                }
                            }
                            _ => {
                                reduce(&mut app, action);
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            app.follow_bottom = false;
                            app.chat_scroll = app.chat_scroll.saturating_sub(3);
                        }
                        MouseEventKind::ScrollDown => {
                            app.chat_scroll = app.chat_scroll.saturating_add(3);
                            // Re-enable auto-scroll when scrolled to bottom
                            if app.chat_scroll >= app.max_scroll {
                                app.follow_bottom = true;
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            app.chat_scroll = app.chat_scroll.saturating_add(3);
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            app.mouse_selection = Some(app::MouseSelection {
                                start_row: mouse.row,
                                start_col: mouse.column.saturating_sub(2),
                                end_row: mouse.row,
                                end_col: mouse.column.saturating_sub(2),
                            });
                            let pos = mouse.column.saturating_sub(2) as usize;
                            let max_pos = app.input.len();
                            reduce(&mut app, Action::CursorTo(pos.min(max_pos)));
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            if let Some(ref mut selection) = app.mouse_selection {
                                selection.end_row = mouse.row;
                                selection.end_col = mouse.column.saturating_sub(2);
                            }
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            if let Some(ref selection) = app.mouse_selection {
                                if selection.start_row == selection.end_row &&
                                   selection.start_col != selection.end_col {
                                    let start = selection.start_col.min(selection.end_col) as usize;
                                    let end = selection.start_col.max(selection.end_col) as usize;
                                    let input_len = app.input.len();
                                    if start < end && start < input_len {
                                        let copy_end = end.min(input_len);
                                        let selected_text = app.input[start..copy_end].to_string();
                                        if !selected_text.is_empty() {
                                            let mut clipboard = arboard::Clipboard::new().ok();
                                            if let Some(ref mut cb) = clipboard {
                                                let _ = cb.set_text(&selected_text);
                                            }
                                        }
                                    }
                                }
                            }
                            app.mouse_selection = None;
                        }
                        _ => {}
                    }
                }
                Event::LlmResponse(response) => {
                    app.messages.push(Message::assistant(response));
                    app.is_loading = false;
                }
                Event::LlmStreamChunk(chunk) => {
                    if let Some(ref mut msg) = app.streaming_message {
                        msg.push_str(&chunk);
                    }
                }
                Event::LlmStreamDone { timing } => {
                    if let Some(msg) = app.streaming_message.take() {
                        app.messages.push(Message::assistant(msg));
                    }
                    app.is_loading = false;
                    app.last_timing = Some(timing.clone());
                    app.status = format!("Done ({:.1}s)", timing.total_time_ms as f64 / 1000.0);
                    tracing::info!("Response: {:.1}s (embed: {:.0}ms, retrieval: {:.0}ms, llm: {:.0}ms)",
                        timing.total_time_ms as f64 / 1000.0,
                        timing.embedding_time_ms,
                        timing.retrieval_time_ms,
                        timing.llm_time_ms);
                }
                Event::LlmError(error) => {
                    app.messages.push(Message::system(format!("Error: {}", error)));
                    app.is_loading = false;
                    app.streaming_message = None;
                }
                Event::ModelsList(models) => {
                    app.available_models = models.clone();
                    let models_text = models.iter().map(|m| format!("  - {}", m)).collect::<Vec<_>>().join("\n");
                    app.messages.push(Message::assistant(format!("Available models:\n{}", models_text)));
                    app.is_loading = false;
                    reduce(&mut app, Action::SetModels(models));
                }
                Event::IndexProgress(progress) => {
                    app.indexing_progress = Some(progress);
                }
                Event::IndexComplete { files, chunks } => {
                    app.indexing_progress = None;
                    app.status = format!("Indexed {} files, {} chunks", files, chunks);
                }
                Event::IndexError(error) => {
                    app.indexing_progress = None;
                    app.messages.push(Message::system(format!("Indexing error: {}", error)));
                }
                Event::FileChanged(paths) => {
                    app.status = format!("{} file(s) changed", paths.len());
                }
                Event::FileRemoved(paths) => {
                    app.status = format!("{} file(s) removed", paths.len());
                }
                Event::Resize(_, _) | Event::Tick => {
                    if app.is_loading || app.streaming_message.is_some() {
                        app.thinking_dots = (app.thinking_dots + 1) % 4;
                    }
                }
                Event::WarmupComplete => {
                    app.status = format!("Embedding model ready ({})", app.current_embed_model);
                }
            }
        }
    }
    Ok(())
}
