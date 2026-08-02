use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const HELP_TEXT: &str = concat!(
    "┌─────────────────────────────────────────────┐\n",
    "│                 Help                        │\n",
    "├─────────────────────────────────────────────┤\n",
    "│  q / Ctrl+q    Quit                         │\n",
    "│  Tab           Cycle focus                  │\n",
    "│  j / ↓         Move cursor down             │\n",
    "│  k / ↑         Move cursor up               │\n",
    "│  Enter         Open directory / file        │\n",
    "│  h / ←         Parent directory             │\n",
    "│  l / →         Enter directory              │\n",
    "│  m             Convert to Markdown          │\n",
    "│  h             Convert to HTML              │\n",
    "│  p             Convert to PDF               │\n",
    "│  t             Convert Typst to PDF/HTML    │\n",
    "│  ? / Esc       Toggle help                  │\n",
    "└─────────────────────────────────────────────┘",
);

/// Draw the help popup overlay.
pub fn draw(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(50, 16, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().white());

    let text = Text::from(HELP_TEXT);
    let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_x = area.width.saturating_mul(percent_x) / 100;
    let popup_y = area.height.saturating_mul(percent_y) / 100;
    let x = area.x + (area.width.saturating_sub(popup_x)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_y)) / 2;
    Rect::new(x, y, popup_x.min(area.width), popup_y.min(area.height))
}
