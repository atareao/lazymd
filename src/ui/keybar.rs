use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

use crate::app::{App, Focus, Mode};

/// Draw the keybinding hint bar at the bottom of the screen.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let text = match app.mode {
        Mode::Help => "[Esc/?] Close help",
        Mode::Settings => "[s/Tab] Cycle theme  [Esc] Close  [q] Quit",
        Mode::Browse | Mode::Preview => match app.focus {
            Focus::Browser => {
                "[1/2/3] Panels  [j/k/\u{2191}/\u{2193}] Nav  [Enter] Open  [h/\u{2190}] Parent  [l/\u{2192}] Enter  \
                 [/] Filter  [r] Refresh  [m] MD  [p] PDF  [t] Typst  [d] Del  [R] Ren  [s] Sett  [?] Help  [q] Quit"
            }
            Focus::Preview => {
                "[1/2/3] Panels  [j/k/\u{2191}/\u{2193}] Scroll  [PgUp/PgDn] Page  [g] Top  [G] End  \
                 [m] MD  [h] HTML  [p] PDF  [t] Typst  [Esc] Back  [q] Quit"
            }
            Focus::Log => "[1/2/3] Panels  [j/k/\u{2191}/\u{2193}] Scroll  [Tab] Focus  [q] Quit",
        },
    };

    let paragraph = Paragraph::new(Text::styled(text, Style::default().fg(app.theme.dim_fg)));
    frame.render_widget(paragraph, area);
}
