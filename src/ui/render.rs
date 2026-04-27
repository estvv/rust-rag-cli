// src/ui/render.rs

use crate::app::{App, MessageSource};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let suggestions_height = if app.show_suggestions {
        (app.suggestions.len().min(7) + 2) as u16
    } else {
        0
    };

    let constraints = if suggestions_height > 0 {
        vec![
            Constraint::Min(5),
            Constraint::Length(suggestions_height),
            Constraint::Length(6),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Min(5),
            Constraint::Length(6),
            Constraint::Length(1),
        ]
    };

    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).split(frame.area());

    let main_area = chunks[0];

    let (input_area, status_area) = if suggestions_height > 0 {
        render_suggestions(frame, app, chunks[1]);
        (chunks[2], chunks[3])
    } else {
        (chunks[1], chunks[2])
    };

    let main_chunks = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(main_area);

    render_chat(frame, app, main_chunks[0]);
    render_context(frame, app, main_chunks[1]);
    render_input(frame, app, input_area);
    render_status(frame, app, status_area);
}

fn render_suggestions(frame: &mut Frame, app: &App, area: Rect) {
    if app.suggestions.is_empty() {
        return;
    }

    let title = if app.input.trim().starts_with('/') {
        " Commands "
    } else {
        " Files "
    };

    let block = Block::default().title(Span::styled(title, Style::default().fg(Color::Black).bg(Color::Cyan))).borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan));

    let visible_count = (area.height.saturating_sub(2)) as usize;
    let start_idx = if app.suggestion_index >= visible_count {
        app.suggestion_index - visible_count.saturating_sub(1)
    } else {
        0
    };

    let lines: Vec<Line> = app.suggestions.iter()
        .skip(start_idx)
        .take(visible_count)
        .enumerate()
        .map(|(i, suggestion)| {
            let actual_idx = start_idx + i;
            if actual_idx == app.suggestion_index {
                Line::from(vec![
                    Span::styled("▶ ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        suggestion.clone(),
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(suggestion.clone(), Style::default().fg(Color::Gray)),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(lines).block(block).style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, area);
}

fn render_chat(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().title(" Chat ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan));

    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let (prefix, color) = match msg.source {
            MessageSource::User => (" You: ", Color::Green),
            MessageSource::Assistant => (" AI: ", Color::Blue),
            MessageSource::System => (" System: ", Color::Yellow),
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        ]));

        for line in msg.content.lines() {
            lines.push(Line::from(highlight_message_line(line)));
        }
        lines.push(Line::from(""));
    }

    if let Some(ref streaming) = app.streaming_message {
        if streaming.is_empty() {
            let dots = match app.thinking_dots % 4 {
                0 => "",
                1 => ".",
                2 => "..",
                3 => "...",
                _ => "...",
            };
            lines.push(Line::from(Span::styled(
                format!(" Thinking{}", dots),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
            )));
        } else {
            lines.push(Line::from(vec![
                Span::styled(" AI: ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            ]));

            for line in streaming.lines() {
                lines.push(Line::from(Span::styled(
                    line,
                    Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC),
                )));
            }
        }
    } else if app.is_loading {
        let dots = match app.thinking_dots % 4 {
            0 => "",
            1 => ".",
            2 => "..",
            3 => "...",
            _ => "...",
        };
        lines.push(Line::from(Span::styled(
            format!(" Thinking{}", dots),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
        )));
    }

    let total_lines = lines.len();
    let visible_lines = area.height.saturating_sub(2) as usize;

    let scroll_offset = app.chat_scroll as usize;
    let max_scroll = total_lines.saturating_sub(visible_lines);
    let scroll_offset = scroll_offset.min(max_scroll);

    let lines: Vec<Line> = lines.into_iter().skip(scroll_offset).take(visible_lines).collect();
    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn highlight_message_line(line: &str) -> Line {
    let mut spans: Vec<Span> = Vec::new();
    let mut current = String::new();
    let mut in_backtick = false;
    let mut in_file_ref = false;

    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '`' {
            if !current.is_empty() {
                if in_file_ref {
                    spans.push(Span::styled(
                        current.clone(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
                    ));
                } else if in_backtick {
                    spans.push(Span::styled(
                        current.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::raw(current.clone()));
                }
                current.clear();
            }

            in_backtick = !in_backtick;
            i += 1;
        } else if c == '@' && !in_backtick {
            if !current.is_empty() {
                spans.push(Span::raw(current.clone()));
                current.clear();
            }
            current.push(c);
            in_file_ref = true;
            i += 1;
        } else if (c.is_whitespace() || c == '\n') && in_file_ref {
            if !current.is_empty() {
                spans.push(Span::styled(
                    current.clone(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
                ));
                current.clear();
            }
            in_file_ref = false;
            current.push(c);
            spans.push(Span::raw(current.clone()));
            current.clear();
            i += 1;
        } else {
            current.push(c);
            i += 1;
        }
    }

    if !current.is_empty() {
        if in_file_ref {
            spans.push(Span::styled(
                current,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
            ));
        } else if in_backtick {
            spans.push(Span::styled(
                current,
                Style::default().add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw(current));
        }
    }

    Line::from(spans)
}

fn render_context(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.file_references.is_empty() {
        " RAG Context "
    } else {
        " Files & Context "
    };

    let block = Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(Color::Magenta));

    let mut lines: Vec<Line> = Vec::new();

    if !app.file_references.is_empty() {
        lines.push(Line::from(Span::styled(
            " Referenced:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        for (idx, r) in app.file_references.iter().enumerate() {
            lines.push(Line::from(Span::styled(
                format!("  [{}] {}", idx + 1, r.path.display()),
                Style::default().fg(Color::Yellow),
            )));
        }
        lines.push(Line::from(""));
    }

    for (idx, chunk) in app.context_chunks.iter().enumerate() {
        lines.push(Line::from(Span::styled(
            format!("[{}] {}", idx + 1, chunk.file_path),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        )));

        for line in chunk.content.lines().take(5) {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::Gray),
            )));
        }
        lines.push(Line::from(""));
    }

    if app.context_chunks.is_empty() && app.file_references.is_empty() {
        lines.push(Line::from(Span::styled(
            " Use @files or /index <path>",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Input ", Style::default().fg(Color::Black).bg(Color::Green)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let mut lines: Vec<Line> = Vec::new();

    let display_text = &app.input;
    let cursor = app.cursor_pos;

    let has_selection = app.mouse_selection.as_ref()
        .map(|s| s.start_row == s.end_row && s.start_col != s.end_col)
        .unwrap_or(false);

    let (sel_start, sel_end) = if has_selection {
        let sel = app.mouse_selection.as_ref().unwrap();
        let start = sel.start_col.min(sel.end_col) as usize;
        let end = sel.start_col.max(sel.end_col) as usize;

        (Some(start), Some(end))
    } else {
        (None, None)
    };

    let mut spans = Vec::new();
    let mut current_ref = String::new();
    let mut in_ref = false;

    for (i, c) in display_text.chars().enumerate() {
        let char_start = i;
        let is_selected = has_selection &&
            sel_start.map(|s| char_start >= s).unwrap_or(false) &&
            sel_end.map(|e| char_start < e).unwrap_or(false);
        let is_cursor = char_start == cursor;

        if c == '@' && !is_selected {
            if !current_ref.is_empty() {
                spans.push(Span::raw(current_ref.clone()));
                current_ref.clear();
            }
            in_ref = true;
            current_ref.push(c);
        } else if in_ref && (c.is_whitespace() || is_selected) {
            if !current_ref.is_empty() && !is_selected {
                spans.push(Span::styled(
                    current_ref.clone(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
                ));
                current_ref.clear();
            }
            in_ref = false;

            if is_cursor {
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::Black).bg(Color::Green),
                ));
            } else if is_selected {
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::White).bg(Color::Blue),
                ));
            } else {
                spans.push(Span::raw(c.to_string()));
            }
        } else if in_ref {
            current_ref.push(c);
        } else {
            if is_cursor {
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::Black).bg(Color::Green),
                ));
            } else if is_selected {
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::White).bg(Color::Blue),
                ));
            } else {
                spans.push(Span::raw(c.to_string()));
            }
        }
    }

    if !current_ref.is_empty() {
        spans.push(Span::styled(
            current_ref,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
        ));
    }

    if cursor == display_text.len() {
        spans.push(Span::styled(" ", Style::default().bg(Color::Green)));
    }

    lines.push(Line::from(spans));
    lines.push(Line::from(""));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(" INS ", Style::default().fg(Color::Black).bg(Color::Green)),
    ];

    if let Some(ref progress) = app.indexing_progress {
        let pct = if progress.files_total > 0 {
            (progress.files_done as f32 / progress.files_total as f32 * 100.0) as usize
        } else {
            0
        };
        spans.push(Span::styled(
            format!(" [{}%] {} ", pct, progress.current_file),
            Style::default().fg(Color::Yellow),
        ));
    } else if !app.command_hint.is_empty() {
        spans.push(Span::styled(
            format!(" {} ", &app.command_hint.chars().take(80).collect::<String>()),
            Style::default().fg(Color::Cyan),
        ));
    } else if !app.status.is_empty() {
        spans.push(Span::styled(
            format!(" {} ", &app.status.chars().take(60).collect::<String>()),
            Style::default().fg(Color::Yellow),
        ));
    }

    spans.push(Span::styled(
        format!(" {} ", app.current_model),
        Style::default().fg(Color::Magenta),
    ));

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(paragraph, area);
}
