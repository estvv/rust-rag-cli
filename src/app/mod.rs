// src/app/mod.rs

mod action;
mod command;
mod state;

pub use action::{Action, reduce};
pub use command::{Command, parse as parse_command, help_text};
pub use state::{App, Message, MessageSource, Mode, IndexingProgress, FileReference, MouseSelection};
