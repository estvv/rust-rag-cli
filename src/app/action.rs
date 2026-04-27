// src/app/action.rs

use crate::app::state::App;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    InsertChar(char),
    DeleteChar,
    Backspace,
    CursorLeft,
    CursorRight,
    CursorTo(usize),
    SendMessage,
    ExecuteCommand,
    ScrollChatUp,
    ScrollChatDown,
    ScrollContextUp,
    ScrollContextDown,
    NextCompletion,
    PrevCompletion,
    ApplyCompletion,
    HideSuggestions,
    SetModels(Vec<String>),
}

pub fn reduce(app: &mut App, action: Action) {
    match action {
        Action::Quit => {
            app.running = false;
        }
        Action::InsertChar(c) => {
            app.insert_char(c);
        }
        Action::DeleteChar => {
            app.delete_char_at_cursor();
        }
        Action::Backspace => {
            app.delete_char_before_cursor();
        }
        Action::CursorLeft => {
            app.move_cursor_left();
        }
        Action::CursorRight => {
            app.move_cursor_right();
        }
        Action::CursorTo(pos) => {
            app.move_cursor_to(pos);
        }
        Action::SendMessage => {
            if !app.input.is_empty() && !app.input.trim().starts_with('/') {
                app.messages.push(crate::app::state::Message::user(app.input.clone()));
                app.input.clear();
                app.cursor_pos = 0;
                app.is_loading = true;
                app.show_suggestions = false;
            }
        }
        Action::ExecuteCommand => {
            if app.input.trim().starts_with('/') {
                app.status = format!("Executed: {}", app.input);
            }
        }
        Action::ScrollChatUp => {
            if app.chat_scroll > 0 {
                app.chat_scroll -= 1;
            }
        }
        Action::ScrollChatDown => {
            app.chat_scroll += 1;
        }
        Action::ScrollContextUp => {
            if app.context_scroll > 0 {
                app.context_scroll -= 1;
            }
        }
        Action::ScrollContextDown => {
            app.context_scroll += 1;
        }
        Action::NextCompletion => {
            if app.show_suggestions {
                if app.suggestion_index < app.suggestions.len().saturating_sub(1) {
                    app.suggestion_index += 1;
                } else {
                    app.suggestion_index = 0;
                }
            }
        }
        Action::PrevCompletion => {
            if app.show_suggestions {
                if app.suggestion_index > 0 {
                    app.suggestion_index -= 1;
                } else {
                    app.suggestion_index = app.suggestions.len().saturating_sub(1);
                }
            }
        }
        Action::ApplyCompletion => {
            if app.show_suggestions && !app.suggestions.is_empty() {
                if let Some(suggestion) = app.suggestions.get(app.suggestion_index) {
                    let input = app.input.trim();
                    if input.starts_with('/') {
                        if suggestion.starts_with('/') {
                            app.input = suggestion.clone() + " ";
                            app.cursor_pos = app.input.len();
                        } else {
                            let space_pos = app.input.find(' ').unwrap_or(app.input.len());
                            let before_space = &app.input[..space_pos];
                            app.input = format!("{} {}", before_space, suggestion);
                            app.cursor_pos = app.input.len();
                        }
                    } else if let Some((start, _)) = app.get_word_at_cursor() {
                        app.input.replace_range(start..app.cursor_pos, suggestion);
                        app.cursor_pos = start + suggestion.len();
                    }
                }
                app.show_suggestions = false;
            }
        }
        Action::HideSuggestions => {
            app.show_suggestions = false;
        }
        Action::SetModels(models) => {
            app.available_models = models;
        }
    }
}
