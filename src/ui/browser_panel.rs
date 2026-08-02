use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, Focus};
use crate::fs_utils::FsEntry;

/// Draw the filesystem browser panel.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Browser;

    let block = Block::default()
        .title(format!(" Browser: {} ", app.current_dir.display()))
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(ratatui::style::Color::Cyan)
        } else {
            Style::default()
        });

    let mut items: Vec<ListItem> = Vec::new();

    // Add parent directory entry if not at filesystem root
    if app.current_dir.parent().is_some() {
        let style = if is_focused && app.selected_index == 0 && !app.entries.is_empty() {
            Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Cyan)
        } else {
            Style::default().fg(ratatui::style::Color::Yellow)
        };
        items.push(ListItem::new(Text::styled("  📁 ../", style)));
    }

    for (i, entry) in app.entries.iter().enumerate() {
        let adjusted_index = if app.current_dir.parent().is_some() {
            i + 1
        } else {
            i
        };
        let is_selected = is_focused && adjusted_index == app.selected_index;

        let (prefix, style) = match entry {
            FsEntry::Directory { .. } => {
                let style = if is_selected {
                    Style::default()
                        .fg(ratatui::style::Color::Black)
                        .bg(ratatui::style::Color::Cyan)
                } else {
                    Style::default().fg(ratatui::style::Color::Yellow)
                };
                ("📁 ", style)
            }
            FsEntry::File { extension, .. } => {
                let icon = match extension.as_str() {
                    "md" | "markdown" => "📝 ",
                    "html" | "htm" => "🌐 ",
                    "pdf" => "📄 ",
                    "typ" => "✍️ ",
                    _ => "📄 ",
                };
                let style = if is_selected {
                    Style::default()
                        .fg(ratatui::style::Color::Black)
                        .bg(ratatui::style::Color::Cyan)
                } else {
                    Style::default()
                };
                (icon, style)
            }
        };

        items.push(ListItem::new(Text::styled(
            format!("  {prefix}{}", entry.name()),
            style,
        )));
    }

    if items.is_empty() {
        items.push(ListItem::new(Text::styled(
            "  (empty directory)",
            Style::default().dim(),
        )));
    }

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
