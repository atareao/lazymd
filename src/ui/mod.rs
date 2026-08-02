pub mod browser_panel;
pub mod confirm_dialog;
pub mod help_popup;
pub mod keybar;
pub mod layout;
pub mod log_panel;
pub mod preview_panel;
pub mod settings_panel;

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::App;

/// Top-level render function — dispatches to all panels.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let areas = layout::chunk_areas(area);

    browser_panel::draw(frame, areas.browser, app);
    preview_panel::draw(frame, areas.preview, app);
    log_panel::draw(frame, areas.log, app);
    keybar::draw(frame, areas.keybar, app);

    if app.mode == crate::app::Mode::Help {
        help_popup::draw(frame, area);
    }

    if app.mode == crate::app::Mode::Settings {
        settings_panel::draw(frame, area, app);
    }

    if app.mode == crate::app::Mode::ConfirmDelete {
        confirm_dialog::draw(frame, area, app);
    }
}
