use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Focus};

/// Draw the log / status panel at the bottom of the screen.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Log;

    let block = Block::default()
        .title(" Log ")
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(ratatui::style::Color::Cyan)
        } else {
            Style::default()
        });

    let content = if app.log_messages.is_empty() {
        "Ready".to_string()
    } else {
        app.log_messages.join("\n")
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.log_scroll, 0));

    frame.render_widget(paragraph, area);
}
