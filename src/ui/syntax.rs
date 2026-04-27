// src/ui/syntax.rs

use once_cell::sync::Lazy;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SynColor, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    SyntaxSet::load_defaults_newlines()
});

static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| {
    ThemeSet::load_defaults()
});

pub fn highlight_code(code: &str, extension: &str, theme_name: &str) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET
        .find_syntax_by_extension(extension)
        .or_else(|| SYNTAX_SET.find_syntax_by_extension("txt"))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = THEME_SET.themes.get(theme_name)
        .unwrap_or_else(|| &THEME_SET.themes["base16-eighties.dark"]);

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();

    for line in LinesWithEndings::from(code) {
        let spans: Vec<Span> = match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(highlighted) => {
                highlighted
                    .into_iter()
                    .map(|(style, text)| {
                        Span::styled(
                            text.to_string(),
                            Style::default().fg(syntect_to_ratatui_color(style.foreground)),
                        )
                    })
                    .collect()
            }
            Err(_) => {
                vec![Span::raw(line.to_string())]
            }
        };
        lines.push(Line::from(spans));
    }

    lines
}

pub fn highlight_code_lines(code: &str, extension: &str, theme_name: &str) -> Vec<Line<'static>> {
    if code.is_empty() {
        return vec![Line::raw("")];
    }

    highlight_code(code, extension, theme_name)
}

fn syntect_to_ratatui_color(color: SynColor) -> Color {
    if color.a == 0 {
        Color::Reset
    } else {
        Color::Rgb(color.r, color.g, color.b)
    }
}

pub fn detect_language_from_path(path: &str) -> &str {
    let path_lower = path.to_lowercase();

    if path_lower.ends_with(".rs") { return "rust"; }
    if path_lower.ends_with(".py") { return "python"; }
    if path_lower.ends_with(".js") { return "javascript"; }
    if path_lower.ends_with(".ts") { return "typescript"; }
    if path_lower.ends_with(".jsx") { return "jsx"; }
    if path_lower.ends_with(".tsx") { return "tsx"; }
    if path_lower.ends_with(".go") { return "go"; }
    if path_lower.ends_with(".java") { return "java"; }
    if path_lower.ends_with(".kt") || path_lower.ends_with(".kts") { return "kotlin"; }
    if path_lower.ends_with(".scala") { return "scala"; }
    if path_lower.ends_with(".c") { return "c"; }
    if path_lower.ends_with(".cpp") || path_lower.ends_with(".cc") { return "cpp"; }
    if path_lower.ends_with(".h") { return "c"; }
    if path_lower.ends_with(".hpp") { return "cpp"; }
    if path_lower.ends_with(".sh") || path_lower.ends_with(".bash") { return "bash"; }
    if path_lower.ends_with(".zsh") { return "bash"; }
    if path_lower.ends_with(".json") { return "json"; }
    if path_lower.ends_with(".toml") { return "toml"; }
    if path_lower.ends_with(".yaml") || path_lower.ends_with(".yml") { return "yaml"; }
    if path_lower.ends_with(".md") { return "markdown"; }
    if path_lower.ends_with(".sql") { return "sql"; }
    if path_lower.ends_with(".txt") { return "txt"; }

    "txt"
}

pub fn extract_code_blocks(text: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut in_code_block = false;
    let mut current_language = String::new();
    let mut current_content = String::new();

    for line in text.lines() {
        if line.trim().starts_with("```") {
            if in_code_block {
                if let Some(lang) = current_language.strip_prefix("```") {
                    let lang = lang.trim().to_string();
                    blocks.push(CodeBlock {
                        language: if lang.is_empty() { "txt".to_string() } else { lang },
                        content: current_content.trim_end().to_string(),
                    });
                }
                current_content.clear();
                current_language.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                current_language = line.trim().to_string();
            }
        } else if in_code_block {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    blocks
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub language: String,
    pub content: String,
}

pub fn highlight_text_blocks(text: &str, theme_name: &str) -> Vec<Line<'static>> {
    let code_blocks = extract_code_blocks(text);

    if code_blocks.is_empty() {
        return text.lines().map(|l| Line::raw(l.to_string())).collect();
    }

    let mut lines = Vec::new();
    let mut last_end = 0;

    for block in code_blocks {
        if let Some(start) = text[last_end..].find("```") {
            let global_start = last_end + start;

            let text_before = &text[last_end..global_start];
            for line in text_before.lines() {
                lines.push(Line::raw(line.to_string()));
            }
            lines.push(Line::raw("```".to_string()));

            let highlighted = highlight_code_lines(&block.content, &block.language, theme_name);
            lines.extend(highlighted);

            lines.push(Line::raw("```".to_string()));

            let end_marker = text[global_start..].find(&format!("\n```")).unwrap_or(0);
            if let Some(block_content_end) = text[global_start..].find(&format!("```{}", block.language)) {
                let code_content_end = text[global_start + block_content_end..].find("\n```").unwrap_or(text.len());
                last_end = global_start + block_content_end + code_content_end + 4;
            } else {
                last_end = global_start + end_marker + 4;
            }
        }
    }

    if last_end < text.len() {
        for line in text[last_end..].lines() {
            lines.push(Line::raw(line.to_string()));
        }
    }

    lines
}
