use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

/// Draw the settings panel overlay.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.focus_border));

    let text = format!(
        "Theme: {theme}\n\n\
         Available themes: dark, light, dracula, solarized\n\n\
         Config: ~/.config/lazymd/config.toml\n\n\
         Keys:\n\
         \x20 s / Tab   Cycle theme\n\
         \x20 Esc       Close settings\n\
         \x20 q         Quit",
        theme = app.theme_name
    );

    let paragraph = Paragraph::new(Text::from(text))
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
