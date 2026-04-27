// src/dev/debug.rs

use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

lazy_static::lazy_static! {
    pub static ref LOG_BUFFER: Arc<parking_lot::Mutex<Vec<String>>> = Arc::new(parking_lot::Mutex::new(Vec::new()));
}

#[derive(Clone)]
pub struct LogWriter;

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            let mut logs = LOG_BUFFER.lock();
            let line = s.trim().to_string();
            if !line.is_empty() {
                logs.push(line);
                if logs.len() > 100 {
                    logs.remove(0);
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogWriter {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter
    }
}

pub fn init_debug(debug: bool, debug_file: Option<&std::path::Path>) {
    let filter = if debug {
        EnvFilter::new("rust_rag_cli=debug,info")
    } else {
        EnvFilter::new("rust_rag_cli=info")
    };

    let registry = tracing_subscriber::registry()
        .with(filter);

    if let Some(file) = debug_file {
        use tracing_subscriber::fmt;
        let log_file = std::fs::File::create(file).expect("Failed to create debug log file");
        registry
            .with(fmt::layer().with_writer(std::sync::Arc::new(log_file)).with_ansi(false))
            .init();
    } else if debug {
        let log_file = std::fs::File::create("rag-cli-debug.log").expect("Failed to create debug log file");
        registry
            .with(tracing_subscriber::fmt::layer().with_writer(std::sync::Arc::new(log_file)).with_ansi(false))
            .init();
    } else {
        let writer = LogWriter;
        registry
            .with(tracing_subscriber::fmt::layer().with_writer(writer).with_ansi(false))
            .init();
    }
}

pub fn get_logs() -> Vec<String> {
    LOG_BUFFER.lock().clone()
}
