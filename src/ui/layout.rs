use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// The three main panel areas.
pub struct PanelAreas {
    pub browser: Rect,
    pub preview: Rect,
    pub log: Rect,
}

/// Divide the terminal into three panels: browser (left), preview (center), log (bottom).
pub fn chunk_areas(area: Rect) -> PanelAreas {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(main[0]);

    PanelAreas {
        browser: top[0],
        preview: top[1],
        log: main[1],
    }
}
