use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::App;

/// Draw a centered confirmation dialog with a warning style.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    // Dialog dimensions: 50% width, 30% height
    let dialog_width = area.width.saturating_mul(50) / 100;
    let dialog_height = area.height.saturating_mul(30) / 100;
    let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_height)) / 3;
    let dialog_area = Rect::new(
        x,
        y,
        dialog_width.min(area.width),
        dialog_height.min(area.height),
    );

    // Clear the area behind the dialog
    frame.render_widget(Clear, dialog_area);

    // Block with red border for urgency
    let block = Block::default()
        .title(" ⚠ Delete file ")
        .borders(Borders::ALL)
        .border_style(Style::default().red().bold())
        .style(
            Style::default()
                .bg(app.theme.dim_fg)
                .fg(app.theme.default_fg),
        );

    // Inner layout: message on top, keys at bottom
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(dialog_area);

    // File name in yellow, centered
    let file_text = Text::styled(
        format!(" {} ", app.pending_delete_name),
        Style::default().yellow().bold(),
    );
    let file_para = Paragraph::new(file_text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(file_para, inner[0]);

    // Separator line
    let sep = Paragraph::new(Text::styled(
        "─".repeat(dialog_width.saturating_sub(4) as usize),
        Style::default().dark_gray(),
    ))
    .alignment(Alignment::Center);
    frame.render_widget(sep, inner[1]);

    // Key hints
    let keys = Text::styled(
        "  [Y] Yes, delete     [N] No, cancel  ",
        Style::default().white().bold(),
    );
    let keys_para = Paragraph::new(keys).alignment(Alignment::Center);
    frame.render_widget(keys_para, inner[2]);

    // Render the block behind everything
    frame.render_widget(block, dialog_area);
}
