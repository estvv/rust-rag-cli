// src/ui/render.rs

use crate::app::{App, MessageSource};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use ratatui::widgets::Padding;

pub fn render(frame: &mut Frame, app: &mut App) {
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

    let main_chunks = Layout::default().direction(Direction::Horizontal).constraints([
        Constraint::Percentage(70),
        Constraint::Percentage(30),
    ]).split(main_area);

    let right_chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Percentage(55),
        Constraint::Percentage(45),
    ]).split(main_chunks[1]);

    render_chat(frame, app, main_chunks[0]);
    render_context(frame, app, right_chunks[0]);
    render_logs(frame, right_chunks[1]);
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

fn render_chat(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_color = if app.streaming_message.is_some() {
        Color::Magenta
    } else if app.is_loading {
        Color::Yellow
    } else {
        Color::Blue
    };

    let block = Block::default()
        .title(" Chat ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .padding(Padding::new(1, 1, 0, 0));

    let inner_area = block.inner(area);
    let visible_lines = inner_area.height as usize;
    let content_width = inner_area.width.saturating_sub(2) as usize;

    let mut all_lines: Vec<String> = Vec::new();

    for msg in &app.messages {
        let prefix = match msg.source {
            MessageSource::User => "You:",
            MessageSource::Assistant => "AI:",
            MessageSource::System => "System:",
        };
        all_lines.push(prefix.to_string());

        for line in msg.content.lines() {
            let wrapped = wrap_line(line, content_width);
            all_lines.extend(wrapped);
        }
        all_lines.push(String::new());
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
            all_lines.push(format!("Thinking{}", dots));
        } else {
            all_lines.push("AI:".to_string());
            for line in streaming.lines() {
                let wrapped = wrap_line(line, content_width);
                all_lines.extend(wrapped);
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
        all_lines.push(format!("Thinking{}", dots));
    }

    let total_lines = all_lines.len();
    app.max_scroll = total_lines.saturating_sub(visible_lines);

    // If following bottom (auto-scroll), always show the latest content
    // Otherwise, respect user's scroll position
    let scroll_offset = if app.follow_bottom {
        app.max_scroll
    } else {
        app.chat_scroll.min(app.max_scroll)
    };

    let display_lines: Vec<Line> = all_lines.iter().skip(scroll_offset).take(visible_lines).map(|line| {
        let is_prefix = line == "You:" || line == "AI:" || line == "System:";
        if is_prefix {
            let color = if line == "You:" { Color::Green } else if line == "AI:" { Color::Blue } else { Color::Yellow };
            Line::from(Span::styled(format!(" {} ", line), Style::default().fg(color).add_modifier(Modifier::BOLD)))
        } else if line.starts_with("Thinking") {
            Line::from(Span::styled(line.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC)))
        } else {
            highlight_message_line(line)
        }
    }).collect();

    let paragraph = Paragraph::new(display_lines).block(block);
    frame.render_widget(paragraph, area);
}

fn wrap_line(line: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![line.to_string()];
    }

    let mut result = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut start = 0;

    while start < chars.len() {
        let end = (start + max_width).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        result.push(chunk);
        start = end;
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}

fn highlight_message_line(line: &str) -> Line {
    let trimmed = line.trim_start();

    // Check for headers first
    if trimmed.starts_with("### ") {
        let content = &trimmed[4..];
        return Line::from(Span::styled(
            content.to_string(),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    } else if trimmed.starts_with("## ") {
        let content = &trimmed[3..];
        return Line::from(Span::styled(
            content.to_string(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
    } else if trimmed.starts_with("# ") {
        let content = &trimmed[2..];
        return Line::from(Span::styled(
            content.to_string(),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ));
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let content = &trimmed[2..];
        let content_spans = parse_inline_formatting(content);
        let mut spans = vec![Span::styled("• ", Style::default().fg(Color::Cyan))];
        spans.extend(content_spans);
        return Line::from(spans);
    } else if trimmed.chars().next().map(|c| c.is_numeric()).unwrap_or(false) {
        // Check for numbered list
        if let Some(dot_pos) = trimmed.find(". ") {
            let num_part = &trimmed[..dot_pos + 2];
            let content = &trimmed[dot_pos + 2..];
            let content_spans = parse_inline_formatting(content);
            let mut spans = vec![Span::styled(num_part.to_string(), Style::default().fg(Color::Cyan))];
            spans.extend(content_spans);
            return Line::from(spans);
        }
    }

    // Default: parse inline formatting
    Line::from(parse_inline_formatting(line))
}

fn parse_inline_formatting(line: &str) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut current = String::new();
    let mut in_backtick = false;

    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '`' {
            if in_backtick {
                spans.push(Span::styled(
                    current.clone(),
                    Style::default().fg(Color::Cyan),
                ));
                current.clear();
                in_backtick = false;
            } else {
                if !current.is_empty() {
                    spans.push(Span::raw(current.clone()));
                    current.clear();
                }
                in_backtick = true;
            }
            i += 1;
        } else if c == '*' && !in_backtick {
            if !current.is_empty() {
                spans.push(Span::raw(current.clone()));
                current.clear();
            }

            if i + 1 < chars.len() && chars[i + 1] == '*' {
                let end = find_closing_marker(&chars, i + 2, "**");
                if let Some(end_pos) = end {
                    let bold_text: String = chars[i + 2..end_pos].iter().collect();
                    spans.push(Span::styled(
                        bold_text,
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                    i = end_pos + 2;
                } else {
                    spans.push(Span::raw("**"));
                    i += 2;
                }
            } else {
                let end = find_closing_marker(&chars, i + 1, "*");
                if let Some(end_pos) = end {
                    let italic_text: String = chars[i + 1..end_pos].iter().collect();
                    spans.push(Span::styled(
                        italic_text,
                        Style::default().add_modifier(Modifier::ITALIC),
                    ));
                    i = end_pos + 1;
                } else {
                    spans.push(Span::raw("*"));
                    i += 1;
                }
            }
        } else if c == '@' && !in_backtick {
            if !current.is_empty() {
                spans.push(Span::raw(current.clone()));
                current.clear();
            }
            current.push(c);
            i += 1;
            while i < chars.len() && !chars[i].is_whitespace() {
                current.push(chars[i]);
                i += 1;
            }
            spans.push(Span::styled(
                current.clone(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
            ));
            current.clear();
        } else {
            current.push(c);
            i += 1;
        }
    }

    if !current.is_empty() {
        if in_backtick {
            spans.push(Span::styled(
                current,
                Style::default().fg(Color::Cyan),
            ));
        } else {
            spans.push(Span::raw(current));
        }
    }

    if spans.is_empty() {
        vec![Span::raw(line)]
    } else {
        spans
    }
}

fn find_closing_marker(chars: &[char], start: usize, marker: &str) -> Option<usize> {
    let marker_chars: Vec<char> = marker.chars().collect();
    let marker_len = marker_chars.len();

    let mut i = start;
    while i + marker_len <= chars.len() {
        let mut matches = true;
        for (j, mc) in marker_chars.iter().enumerate() {
            if chars[i + j] != *mc {
                matches = false;
                break;
            }
        }
        if matches {
            return Some(i);
        }
        i += 1;
    }
    None
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
            let path_str = r.path.display().to_string();
            let truncated_path: String = path_str.chars().take(area.width.saturating_sub(8) as usize).collect();
            lines.push(Line::from(Span::styled(
                format!("  [{}] {}", idx + 1, truncated_path),
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

        for line in chunk.content.lines().take(3) {
            let truncated: String = line.chars().take(area.width.saturating_sub(6) as usize).collect();
            lines.push(Line::from(Span::styled(
                format!("  {}", truncated),
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

fn render_logs(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Logs ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let mut lines: Vec<Line> = Vec::new();

    let logs = crate::dev::get_logs();
    let visible_lines = area.height.saturating_sub(2) as usize;
    let start = logs.len().saturating_sub(visible_lines);
    let max_width = area.width.saturating_sub(4) as usize;

    for log in logs.iter().skip(start) {
        let (level, body) = if log.contains(" ERROR ") {
            let parts: Vec<&str> = log.splitn(2, " ERROR ").collect();
            if parts.len() == 2 {
                ("ERROR".to_string(), parts[1].to_string())
            } else {
                ("".to_string(), log.clone())
            }
        } else if log.contains(" WARN ") {
            let parts: Vec<&str> = log.splitn(2, " WARN ").collect();
            if parts.len() == 2 {
                ("WARN".to_string(), parts[1].to_string())
            } else {
                ("".to_string(), log.clone())
            }
        } else if log.contains(" INFO ") {
            let parts: Vec<&str> = log.splitn(2, " INFO ").collect();
            if parts.len() == 2 {
                ("INFO".to_string(), parts[1].to_string())
            } else {
                ("".to_string(), log.clone())
            }
        } else if log.contains(" DEBUG ") {
            let parts: Vec<&str> = log.splitn(2, " DEBUG ").collect();
            if parts.len() == 2 {
                ("DEBUG".to_string(), parts[1].to_string())
            } else {
                ("".to_string(), log.clone())
            }
        } else {
            ("".to_string(), log.clone())
        };

        let (level_color, body_color) = match level.as_str() {
            "ERROR" => (Color::Red, Color::Gray),
            "WARN" => (Color::Yellow, Color::Gray),
            "INFO" => (Color::Cyan, Color::Gray),
            "DEBUG" => (Color::Magenta, Color::Gray),
            _ => (Color::DarkGray, Color::Gray),
        };

        if !level.is_empty() {
            let prefix_len = 4 + level.len() + 3;
            let body_max = max_width.saturating_sub(prefix_len);

            let chars: Vec<char> = body.chars().collect();
            let first_line: String = chars.iter().take(body_max).collect();
            lines.push(Line::from(vec![
                Span::styled("- ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("[{}] ", level), Style::default().fg(level_color)),
                Span::styled(first_line, Style::default().fg(body_color)),
            ]));

            let wrap_max = max_width.saturating_sub(2);
            let remaining: String = chars.iter().skip(body_max).collect();
            for chunk in remaining.as_bytes().chunks(wrap_max) {
                let chunk_str = String::from_utf8_lossy(chunk).to_string();
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(chunk_str, Style::default().fg(body_color)),
                ]));
            }
        } else {
            let wrap_max = max_width.saturating_sub(2);
            for (i, chunk) in body.as_bytes().chunks(wrap_max).enumerate() {
                let chunk_str = String::from_utf8_lossy(chunk).to_string();
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("- ", Style::default().fg(Color::DarkGray)),
                        Span::styled(chunk_str, Style::default().fg(body_color)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default().fg(Color::DarkGray)),
                        Span::styled(chunk_str, Style::default().fg(body_color)),
                    ]));
                }
            }
        }
    }

    if logs.is_empty() {
        lines.push(Line::from(Span::styled(
            "- Ready",
            Style::default().fg(Color::Gray),
        )));
    }

    let paragraph = Paragraph::new(lines).block(block);
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
