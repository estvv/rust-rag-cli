mod app;
mod cli;
mod client;
mod clients;
mod db;
mod input;
mod scrapper;
mod service;
mod ui;

use app::{App, Action, reduce, Message, Command, parse_command, help_text, IndexingProgress};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{self, Event as CrosstermEvent, MouseButton, MouseEventKind},
};
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
    LlmStreamDone,
    LlmError(String),
    ModelsList(Vec<String>),
    IndexProgress(IndexingProgress),
    IndexComplete { files: usize, chunks: usize },
    Tick,
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::parse();

    match args.command {
        cli::Commands::Chat { path, reindex, index_file } => {
            run_chat(path, reindex, index_file).await
        }
        cli::Commands::Index { path, index_file } => {
            run_index(path, index_file).await
        }
    }
}

async fn run_index(path: PathBuf, index_file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Indexing {}...", path.display());

    let service = service::ChatService::new(service::Config::default());
    let mut files_processed = 0;
    let mut chunks_processed = 0;

    service.index_directory_streaming(&path, |done, total, chunks, file| {
        files_processed = done;
        chunks_processed = chunks;
        print!("\r  [{}/{}] files, {} chunks - {}", done, total, chunks, file);
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }).await?;

    println!();

    service.save_index(index_file.to_str().unwrap()).await?;
    println!("Saved to {}", index_file.display());
    println!("Done: {} files, {} chunks", files_processed, chunks_processed);

    Ok(())
}

async fn run_chat(path: PathBuf, reindex: bool, index_file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let event_loop = EventLoop::new(tick_rate);
    let event_sender = event_loop.sender();

    let service = Arc::new(tokio::sync::Mutex::new(service::ChatService::new(service::Config::default())));
    
    let auto_index = !index_file.exists() && path.exists() && path.is_dir();
    
    if index_file.exists() && !reindex && !auto_index {
        if service.lock().await.load_index(index_file.to_str().unwrap()).await.is_ok() {
            let chunks = service.lock().await.get_chunks().await;
            let status = if chunks.is_empty() {
                "No index. /index <path>".to_string()
            } else {
                format!("Loaded {} chunks", chunks.len())
            };
            let app = App::new().with_status(status);
            let res = run(&mut terminal, app, service.clone(), event_loop, event_sender, index_file, path).await;
            cleanup(&mut terminal, res)
        } else {
            cleanup(&mut terminal, Err("Failed to load index".into()))
        }
    } else if reindex || auto_index {
        let mut app = App::new().with_status(format!("Indexing {}", path.display()));
        app.indexing_progress = Some(IndexingProgress::default());
        
        let service_clone = service.clone();
        let sender_clone = event_sender.clone();
        let path_clone = path.clone();
        let index_file_clone = index_file.clone();
        
        tokio::spawn(async move {
            let mut total = 0;
            let _ = service_clone.lock().await.index_directory_streaming(&path_clone, |done, total_count, chunks, file| {
                total = total_count;
                let _ = sender_clone.send(Event::IndexProgress(IndexingProgress {
                    files_done: done,
                    files_total: total_count,
                    chunks_done: chunks,
                    current_file: file.to_string(),
                }));
            }).await;
            let chunks = service_clone.lock().await.get_chunks().await;
            let _ = sender_clone.send(Event::IndexComplete {
                files: total,
                chunks: chunks.len(),
            });
            let _ = service_clone.lock().await.save_index(index_file_clone.to_str().unwrap()).await;
        });
        
        let res = run(&mut terminal, app, service.clone(), event_loop, event_sender, index_file, path).await;
        cleanup(&mut terminal, res)
    } else {
        let app = App::new().with_status("Ready".to_string());
        let res = run(&mut terminal, app, service.clone(), event_loop, event_sender, index_file, path).await;
        cleanup(&mut terminal, res)
    }
}

fn cleanup(terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>, res: Result<(), Box<dyn std::error::Error>>) -> Result<(), Box<dyn std::error::Error>> {
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
) -> Result<(), Box<dyn std::error::Error>> {
    app.current_model = service.lock().await.get_chat_model().to_string();

    while app.running {
        terminal.draw(|f| ui::render(f, &app))?;

        if let Some(event) = event_loop.next().await {
            match event {
                Event::Key(key) => {
                    if app.is_loading || app.streaming_message.is_some() {
                        continue;
                    }
                    if let Some(action) = input::handle(key, &app.input, app.show_suggestions) {
                        match &action {
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
                                                        .map(|m| m.name.split(':').next().unwrap_or(&m.name).to_string())
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
                                        Command::Reindex => {
                                            app.status = "Use /index <path>".to_string();
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
                                            Ok(_) => {
                                                let _ = sender_clone.send(Event::LlmStreamDone);
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
                    if app.is_loading || app.streaming_message.is_some() {
                        continue;
                    }
                    match mouse.kind {
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
                Event::LlmStreamDone => {
                    if let Some(msg) = app.streaming_message.take() {
                        app.messages.push(Message::assistant(msg));
                    }
                    app.is_loading = false;
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
                Event::Resize(_, _) | Event::Tick => {
                    if app.is_loading || app.streaming_message.is_some() {
                        app.thinking_dots = (app.thinking_dots + 1) % 4;
                    }
                }
            }
        }
    }
    Ok(())
}