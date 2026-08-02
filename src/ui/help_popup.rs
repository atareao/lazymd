use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

const HELP_TEXT: &str = concat!(
    "┌─────────────────────────────────────────────┐\n",
    "│                 Help                        │\n",
    "├─────────────────────────────────────────────┤\n",
    "│  Global                                     │\n",
    "│  q / Ctrl+q    Quit                         │\n",
    "│  s             Settings (cycle theme)       │\n",
    "│  Tab           Cycle focus                  │\n",
    "│  1 / 2 / 3     Jump to Browser/Preview/Log  │\n",
    "│  ? / Esc       Toggle help / close preview  │\n",
    "│                                             │\n",
    "│  Browser (file tree)                        │\n",
    "│  j / ↓         Move cursor down             │\n",
    "│  k / ↑         Move cursor up               │\n",
    "│  Enter         Open directory / file        │\n",
    "│  h / ←         Parent directory             │\n",
    "│  l / →         Enter directory              │\n",
    "│  r             Refresh directory             │\n",
    "│  /             Filter entries (type + Enter)  │\n",
    "│  d             Delete selected file            │\n",
    "│  R             Rename selected file            │\n",
    "│  m / p / t     Convert to MD / PDF / Typst  │\n",
    "│                                             │\n",
    "│  Preview (file viewer)                      │\n",
    "│  j / ↓         Scroll down                  │\n",
    "│  k / ↑         Scroll up                    │\n",
    "│  PageDown      Scroll down 10 lines         │\n",
    "│  PageUp        Scroll up 10 lines           │\n",
    "│  g / Home      Scroll to top                │\n",
    "│  G / End       Scroll to bottom             │\n",
    "│  m / h / p / t Convert to MD/HTML/PDF/Typst │\n",
    "│                                             │\n",
    "│  Log panel                                  │\n",
    "│  j / ↓         Scroll down                  │\n",
    "│  k / ↑         Scroll up                    │\n",
    "│  PageDown      Scroll down 10 lines         │\n",
    "│  PageUp        Scroll up 10 lines           │\n",
    "│  g / Home      Scroll to top                │\n",
    "│  G / End       Scroll to bottom             │\n",
    "└─────────────────────────────────────────────┘",
);

/// Draw the help popup overlay.
pub fn draw(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(55, 55, area);

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
