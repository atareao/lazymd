use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Focus};

/// Draw the file preview panel.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Preview;
    let theme = &app.theme;

    let title = match &app.preview_path {
        Some(path) => format!(
            " Preview: {} ",
            path.file_name().unwrap_or_default().to_str().unwrap_or("?")
        ),
        None => " Preview ".to_string(),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(theme.focus_border)
        } else {
            Style::default()
        });

    if app.preview_content.is_empty() {
        let paragraph = Paragraph::new(
            "Select a file to preview\n\nPress Enter on a file in the browser panel.",
        )
        .block(block)
        .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
        return;
    }

    // Check the file extension to choose renderer
    let ext = app
        .preview_path
        .as_ref()
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    let paragraph = match ext.as_deref() {
        Some("md" | "markdown") => {
            let lines = render_markdown(&app.preview_content);
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((app.preview_scroll, 0))
        }
        Some("html" | "htm") => {
            let lines = render_html(&app.preview_content);
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((app.preview_scroll, 0))
        }
        _ => Paragraph::new(app.preview_content.as_str())
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0)),
    };

    frame.render_widget(paragraph, area);
}

/// Render markdown content as styled ratatui lines with syntax highlighting.
fn render_markdown(content: &str) -> Vec<Line<'static>> {
    use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    // Style stack — each new tag pushes a style, which is popped on End.
    let mut style_stack: Vec<Style> = vec![Style::default()];

    // Helper to flush current_line into lines
    let flush_line = |spans: &mut Vec<Span<'static>>, target: &mut Vec<Line<'static>>| {
        if !spans.is_empty() {
            target.push(Line::from(std::mem::take(spans)));
        }
    };

    let parser = Parser::new(content);
    for event in parser {
        match event {
            Event::Start(tag) => {
                let base = *style_stack.last().unwrap_or(&Style::default());
                let style = match &tag {
                    Tag::Heading { level, .. } => match level {
                        HeadingLevel::H1 => base.fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        HeadingLevel::H2 => base.fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        HeadingLevel::H3 => base.fg(Color::Green).add_modifier(Modifier::BOLD),
                        _ => base.fg(Color::White).add_modifier(Modifier::BOLD),
                    },
                    Tag::CodeBlock(_) => base.fg(Color::Green),
                    Tag::Strong => base.add_modifier(Modifier::BOLD),
                    Tag::Emphasis => base.add_modifier(Modifier::ITALIC),
                    Tag::Link { .. } => base.fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
                    Tag::List(_) | Tag::Item => base,
                    Tag::Paragraph => base,
                    Tag::BlockQuote(_) => base.fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                    Tag::Strikethrough => base.add_modifier(Modifier::CROSSED_OUT),
                    _ => base,
                };
                style_stack.push(style);
            }
            Event::End(tag_end) => {
                match &tag_end {
                    TagEnd::Heading(_) | TagEnd::Paragraph | TagEnd::Item | TagEnd::CodeBlock => {
                        flush_line(&mut current_line, &mut lines);
                    }
                    _ => {}
                }
                style_stack.pop();
            }
            Event::Text(text) | Event::Code(text) => {
                let style = *style_stack.last().unwrap_or(&Style::default());
                current_line.push(Span::styled(text.to_string(), style));
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_line(&mut current_line, &mut lines);
            }
            Event::Html(text) => {
                // Render HTML inline as dimmed text
                current_line.push(Span::styled(
                    text.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Event::FootnoteReference(text) => {
                current_line.push(Span::styled(
                    format!("[^{text}]"),
                    Style::default().fg(Color::Blue),
                ));
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "[x]" } else { "[ ]" };
                current_line.push(Span::styled(
                    marker,
                    Style::default().fg(if checked {
                        Color::Green
                    } else {
                        Color::DarkGray
                    }),
                ));
            }
            // Catch remaining events: inline/display math, inline HTML, thematic break
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                let style = *style_stack.last().unwrap_or(&Style::default());
                current_line.push(Span::styled(text.to_string(), style.fg(Color::Magenta)));
            }
            Event::InlineHtml(text) => {
                current_line.push(Span::styled(
                    text.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Event::Rule => {
                flush_line(&mut current_line, &mut lines);
                lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    flush_line(&mut current_line, &mut lines);

    lines
}

/// Render HTML content as styled ratatui lines with tag highlighting.
fn render_html(content: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut in_tag = false;

    for ch in content.chars() {
        if ch == '<' {
            // Flush accumulated text before tag
            if !buf.is_empty() {
                let text = std::mem::take(&mut buf);
                current_line.push(Span::raw(text));
            }
            buf.push('<');
            in_tag = true;
        } else if ch == '>' && in_tag {
            buf.push('>');
            current_line.push(Span::styled(
                std::mem::take(&mut buf),
                Style::default().fg(Color::Blue),
            ));
            in_tag = false;
        } else if ch == '\n' {
            if !buf.is_empty() {
                let text = std::mem::take(&mut buf);
                if in_tag {
                    current_line.push(Span::styled(text, Style::default().fg(Color::Blue)));
                    in_tag = false;
                } else {
                    current_line.push(Span::raw(text));
                }
            }
            if !current_line.is_empty() {
                lines.push(Line::from(std::mem::take(&mut current_line)));
            }
        } else {
            buf.push(ch);
        }
    }

    // Flush remaining text
    if !buf.is_empty() {
        let text = std::mem::take(&mut buf);
        if in_tag {
            current_line.push(Span::styled(text, Style::default().fg(Color::Blue)));
        } else {
            current_line.push(Span::raw(text));
        }
    }
    if !current_line.is_empty() {
        lines.push(Line::from(std::mem::take(&mut current_line)));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::raw(content.to_string())));
    }

    lines
}
