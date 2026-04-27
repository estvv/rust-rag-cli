// src/input/handler.rs

use crate::app::Action;
use crossterm::event::{self, KeyCode, KeyModifiers};

pub fn handle(key: event::KeyEvent, input: &str, show_suggestions: bool) -> Option<Action> {
    if show_suggestions {
        match key.code {
            KeyCode::Down => return Some(Action::NextCompletion),
            KeyCode::Up => return Some(Action::PrevCompletion),
            KeyCode::Enter | KeyCode::Tab => return Some(Action::ApplyCompletion),
            KeyCode::Esc => return Some(Action::HideSuggestions),
            _ => {}
        }
    }

    if input.is_empty() {
        match key.code {
            KeyCode::Up => return Some(Action::ScrollChatUp),
            KeyCode::Down => return Some(Action::ScrollChatDown),
            _ => {}
        }
    }

    match key.code {
        KeyCode::Esc => Some(Action::Cancel),
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Enter => {
            if input.trim().starts_with('/') {
                Some(Action::ExecuteCommand)
            } else {
                Some(Action::SendMessage)
            }
        }
        KeyCode::Backspace => {
            if input.is_empty() {
                None
            } else {
                Some(Action::Backspace)
            }
        }
        KeyCode::Delete => Some(Action::DeleteChar),
        KeyCode::Left => Some(Action::CursorLeft),
        KeyCode::Right => Some(Action::CursorRight),
        KeyCode::Home => Some(Action::CursorTo(0)),
        KeyCode::End => None,
        KeyCode::Tab => Some(Action::ApplyCompletion),
        KeyCode::BackTab => Some(Action::PrevCompletion),
        KeyCode::PageUp => Some(Action::ScrollChatUp),
        KeyCode::PageDown => Some(Action::ScrollChatDown),
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'a' => Some(Action::CursorTo(0)),
                    'e' => None,
                    'b' => Some(Action::CursorLeft),
                    'f' => Some(Action::CursorRight),
                    'p' => Some(Action::ScrollChatUp),
                    'n' => Some(Action::ScrollChatDown),
                    _ => None,
                }
            } else {
                Some(Action::InsertChar(c))
            }
        }
        _ => None,
    }
}
