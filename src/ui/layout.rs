use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// The three main panel areas plus the keybinding bar.
pub struct PanelAreas {
    pub browser: Rect,
    pub preview: Rect,
    pub log: Rect,
    pub keybar: Rect,
}
/// Divide the terminal into three panels: browser (left), preview (center),
/// log (bottom), and a keybinding hint bar at the very bottom.
pub fn chunk_areas(area: Rect) -> PanelAreas {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let log_and_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)])
        .split(main[0]);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(log_and_panels[0]);

    PanelAreas {
        browser: top[0],
        preview: top[1],
        log: log_and_panels[1],
        keybar: main[1],
    }
}
