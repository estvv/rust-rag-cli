// src/watcher/handler.rs

use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum WatchEvent {
    FilesChanged(Vec<PathBuf>),
    FilesRemoved(Vec<PathBuf>),
    Error(String),
}

pub struct FileWatcher {
    events_rx: mpsc::Receiver<WatchEvent>,
}

impl FileWatcher {
    pub fn new(watch_path: PathBuf, debounce_ms: u64) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<WatchEvent>(100);
        let tx_clone = tx.clone();

        std::thread::spawn(move || {
            let result = run_watcher_blocking(watch_path, tx_clone, debounce_ms);
            if let Err(e) = result {
                tracing::error!("File watcher error: {}", e);
            }
        });

        Ok(Self {
            events_rx: rx,
        })
    }

    pub async fn next_event(&mut self) -> Option<WatchEvent> {
        self.events_rx.recv().await
    }
}

fn run_watcher_blocking(
    path: PathBuf,
    tx: mpsc::Sender<WatchEvent>,
    debounce_ms: u64,
) -> Result<(), String> {
    let mut debouncer = new_debouncer(
        Duration::from_millis(debounce_ms),
        move |result: notify_debouncer_mini::DebounceEventResult| {
            match result {
                Ok(events) => {
                    let changed: Vec<PathBuf> = events.into_iter()
                        .map(|e| e.path.clone())
                        .collect();

                    if !changed.is_empty() {
                        let _ = tx.blocking_send(WatchEvent::FilesChanged(changed));
                    }
                }
                Err(e) => {
                    tracing::error!("Watch error: {}", e);
                }
            }
        },
    ).map_err(|e| e.to_string())?;

    debouncer.watcher().watch(&path, RecursiveMode::Recursive).map_err(|e| e.to_string())?;
    tracing::info!("File watcher started for: {:?}", path);

    loop {
        std::thread::park();
    }
}
