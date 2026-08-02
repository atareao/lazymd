pub mod browser_panel;
pub mod help_popup;
pub mod layout;
pub mod log_panel;
pub mod preview_panel;

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::App;

/// Top-level render function — dispatches to all panels.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let areas = layout::chunk_areas(area);

    browser_panel::draw(frame, areas.browser, app);
    preview_panel::draw(frame, areas.preview, app);
    log_panel::draw(frame, areas.log, app);

    if app.mode == crate::app::Mode::Help {
        help_popup::draw(frame, area);
    }
}
