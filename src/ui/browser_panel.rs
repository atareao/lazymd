use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::app::{App, Focus};
use crate::fs_utils::{FsEntry, format_size};

/// Draw the filesystem browser panel.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Browser;
    let theme = &app.theme;

    let block = Block::default()
        .title(if app.filtering {
            format!(
                " Browser: {} /{}/ ",
                app.current_dir.display(),
                app.filter_query
            )
        } else if !app.filter_query.is_empty() {
            format!(
                " Browser: {} 🔍 {} ",
                app.current_dir.display(),
                app.filter_query
            )
        } else {
            format!(" Browser: {} ", app.current_dir.display())
        })
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(theme.focus_border)
        } else {
            Style::default()
        });

    let mut items: Vec<ListItem> = Vec::new();

    // Add parent directory entry if not at filesystem root
    if app.current_dir.parent().is_some() {
        let style = if is_focused && app.selected_index == 0 && !app.entries.is_empty() {
            Style::default()
                .fg(theme.selection_fg)
                .bg(theme.selection_bg)
        } else {
            Style::default().fg(theme.directory)
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

        let (display_text, style) = match entry {
            FsEntry::Directory { .. } => {
                let style = if is_selected {
                    Style::default()
                        .fg(theme.selection_fg)
                        .bg(theme.selection_bg)
                } else {
                    Style::default().fg(theme.directory)
                };
                (format!("  📁 {}/", entry.name()), style)
            }
            FsEntry::File {
                extension, size, ..
            } => {
                let icon = match extension.as_str() {
                    "md" | "markdown" => "📝 ",
                    "html" | "htm" => "🌐 ",
                    "pdf" => "📄 ",
                    "typ" => "✍️  ",
                    _ => "📄 ",
                };
                let style = if is_selected {
                    Style::default()
                        .fg(theme.selection_fg)
                        .bg(theme.selection_bg)
                } else {
                    Style::default()
                };
                let padding = " ".repeat(24usize.saturating_sub(entry.name().len()));
                (
                    format!(
                        "  {icon}{}{} {} ",
                        entry.name(),
                        padding,
                        format_size(*size)
                    ),
                    style,
                )
            }
        };

        items.push(ListItem::new(Text::styled(display_text, style)));
    }

    if items.is_empty() {
        items.push(ListItem::new(Text::styled(
            "  (empty directory)",
            Style::default().fg(theme.dim_fg),
        )));
    }

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
