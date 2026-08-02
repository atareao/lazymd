use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Focus};

/// Draw the file preview panel.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Preview;

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
            Style::default().fg(ratatui::style::Color::Cyan)
        } else {
            Style::default()
        });

    let content = if app.preview_content.is_empty() {
        "Select a file to preview\n\nPress Enter on a file in the browser panel."
    } else {
        &app.preview_content
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.preview_scroll, 0));

    frame.render_widget(paragraph, area);
}
