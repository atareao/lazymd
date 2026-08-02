use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::config::{Settings, Theme, load_settings, save_settings, theme_from_string};
use crate::converter::{Format, convert};
use crate::fs_utils::{FsEntry, list_entries};
use crate::preview::preview_file;
use crate::ui::layout;

/// Application mode — what the user is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Mode {
    /// Browsing the filesystem tree.
    Browse,
    /// Previewing a file.
    Preview,
    /// Viewing the help overlay.
    Help,
    /// Viewing the settings panel.
    Settings,
    /// Confirmation dialog for destructive actions.
    ConfirmDelete,
}

/// Which panel has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Browser,
    Preview,
    Log,
}

/// Top-level application state.
pub struct App {
    pub mode: Mode,
    pub focus: Focus,
    pub running: bool,
    pub current_dir: PathBuf,
    pub entries: Vec<FsEntry>,
    pub all_entries: Vec<FsEntry>,
    pub filter_query: String,
    pub filtering: bool,
    pub selected_index: usize,
    pub preview_content: String,
    pub preview_path: Option<PathBuf>,
    pub log_messages: Vec<String>,
    pub preview_scroll: u16,
    pub log_scroll: u16,
    pub theme: Theme,
    pub theme_name: String,
    pub pending_delete_path: Option<PathBuf>,
    pub pending_delete_name: String,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let settings = load_settings();
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let entries = list_entries(&current_dir).unwrap_or_default();
        let theme_name = settings.theme.clone().unwrap_or_else(|| "dark".to_string());
        let theme = theme_from_string(&theme_name);
        let all_entries = entries.clone();
        Self {
            mode: Mode::Browse,
            focus: Focus::Browser,
            running: true,
            current_dir,
            entries,
            all_entries,
            filter_query: String::new(),
            filtering: false,
            selected_index: 0,
            preview_content: String::new(),
            preview_path: None,
            log_messages: vec!["Ready".to_string()],
            preview_scroll: 0,
            log_scroll: 0,
            theme,
            theme_name,
            pending_delete_path: None,
            pending_delete_name: String::new(),
        }
    }

    /// Navigate to a directory and reload entries.
    pub fn navigate_to(&mut self, path: &Path) {
        if let Ok(entries) = list_entries(path) {
            self.current_dir = path.to_path_buf();
            self.all_entries = entries;
            self.entries = self.all_entries.clone();
            self.filter_query.clear();
            self.selected_index = 0;
            self.preview_content.clear();
            self.preview_path = None;
            self.preview_scroll = 0;
            self.add_log(format!("📂 {}", self.current_dir.display()));
            // Persist the last visited directory
            let _ = save_settings(&Settings {
                last_directory: Some(self.current_dir.clone()),
                theme: None,
            });
        } else {
            self.add_log(format!("Cannot open directory: {}", path.display()));
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            let max_index = if self.current_dir.parent().is_some() {
                self.entries.len() // visual index includes ".." at 0
            } else {
                self.entries.len().saturating_sub(1)
            };
            self.selected_index = (self.selected_index + 1).min(max_index);
        }
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Return the real entry index (adjusted for ".." virtual entry).
    fn real_selected_index(&self) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }
        let has_parent = self.current_dir.parent().is_some();
        let idx = if has_parent {
            self.selected_index.saturating_sub(1)
        } else {
            self.selected_index
        };
        if idx < self.entries.len() {
            Some(idx)
        } else {
            None
        }
    }

    /// Enter the selected directory, or preview the selected file.
    pub fn activate_selection(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        // Adjust index: the browser panel shows a virtual ".." entry at index 0
        // when the current directory has a parent. The actual entries start at
        // visual index 1 in that case.
        let has_parent = self.current_dir.parent().is_some();
        if has_parent && self.selected_index == 0 {
            // ".." entry — go to parent directory
            self.parent_directory();
            return;
        }
        let entry_index = if has_parent {
            self.selected_index.saturating_sub(1)
        } else {
            self.selected_index
        };

        if entry_index >= self.entries.len() {
            return;
        }

        let path = self.entries[entry_index].path().to_path_buf();
        let is_dir = matches!(&self.entries[entry_index], FsEntry::Directory { .. });

        if is_dir {
            self.navigate_to(&path);
        } else {
            self.mode = Mode::Preview;
            self.preview_path = Some(path.clone());
            match preview_file(&path) {
                Ok(content) => {
                    self.preview_content = content;
                    self.preview_scroll = 0;
                    self.focus = Focus::Preview;
                }
                Err(e) => {
                    self.add_log(format!("Preview error: {e}"));
                    self.preview_content = format!("Error previewing file:\n{e}");
                    self.preview_path = None;
                }
            }
        }
    }

    /// Go to parent directory.
    pub fn parent_directory(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            let parent = parent.to_path_buf();
            if parent != self.current_dir {
                self.navigate_to(&parent);
            }
        }
    }

    /// Enter the selected child directory (or parent dir if index 0 with parent visible).
    pub fn enter_child_directory(&mut self) {
        let has_parent = self.current_dir.parent().is_some();
        if has_parent && self.selected_index == 0 {
            // The ".." entry is at index 0
            self.parent_directory();
        } else if let Some(entry_idx) = self.real_selected_index()
            && entry_idx < self.entries.len()
            && let FsEntry::Directory { .. } = &self.entries[entry_idx]
        {
            self.activate_selection();
        }
    }

    /// Add a message to the log panel.
    pub fn add_log(&mut self, msg: String) {
        self.log_messages.push(msg);
        self.log_scroll = self.log_messages.len().saturating_sub(1) as u16;
    }

    /// Cycle to the next theme and persist the choice.
    pub fn cycle_theme(&mut self) {
        const THEMES: [&str; 4] = ["dark", "light", "dracula", "solarized"];
        let idx = THEMES
            .iter()
            .position(|t| *t == self.theme_name)
            .unwrap_or(0);
        self.theme_name = THEMES[(idx + 1) % THEMES.len()].to_string();
        self.theme = theme_from_string(&self.theme_name);
        self.add_log(format!("🎨 Theme: {}", self.theme_name));
        let _ = save_settings(&Settings {
            last_directory: Some(self.current_dir.clone()),
            theme: Some(self.theme_name.clone()),
        });
    }

    /// Filter entries by name (case-insensitive substring match).
    pub fn set_filter(&mut self, query: &str) {
        self.filter_query = query.to_string();
        if query.is_empty() {
            self.entries = self.all_entries.clone();
        } else {
            self.entries = self
                .all_entries
                .iter()
                .filter(|e| e.name().to_lowercase().contains(&query.to_lowercase()))
                .cloned()
                .collect();
        }
        self.selected_index = 0;
        self.preview_content.clear();
        self.preview_path = None;
        self.preview_scroll = 0;
    }

    /// Delete the selected file.
    /// Execute the confirmed delete operation.
    pub fn confirm_delete(&mut self) {
        let path = match &self.pending_delete_path {
            Some(p) => p.clone(),
            None => return,
        };
        let name = self.pending_delete_name.clone();
        match std::fs::remove_file(&path) {
            Ok(()) => {
                self.add_log(format!("🗑️ Deleted: {name}"));
                self.navigate_to(&self.current_dir.clone());
            }
            Err(e) => {
                self.add_log(format!("✗ Delete failed: {e}"));
            }
        }
        self.pending_delete_path = None;
        self.pending_delete_name.clear();
    }

    /// Rename the selected file (or directory) to a new name.
    pub fn rename_selected(&mut self, new_name: &str) {
        let entry_idx = match self.real_selected_index() {
            Some(idx) => idx,
            None => return,
        };
        let entry = &self.entries[entry_idx];
        let old_name = entry.name().to_string();
        if new_name.is_empty() || new_name == old_name {
            self.add_log("Rename skipped: name unchanged or empty".to_string());
            return;
        }
        let new_path = self.current_dir.join(new_name);
        match std::fs::rename(entry.path(), &new_path) {
            Ok(()) => {
                self.add_log(format!("✏️ Renamed: {old_name} → {new_name}"));
                self.navigate_to(&self.current_dir.clone());
            }
            Err(e) => {
                self.add_log(format!("✗ Rename failed: {e}"));
            }
        }
    }

    /// Convert the currently previewed (or selected) file to the given format.
    pub fn convert_current(&mut self, target: Format) {
        let input = match &self.preview_path {
            Some(p) => p.clone(),
            None => {
                // Fall back to the selected entry in the browser
                let entry_idx = match self.real_selected_index() {
                    Some(idx) => idx,
                    None => {
                        self.add_log("No file selected for conversion".to_string());
                        return;
                    }
                };
                match &self.entries[entry_idx] {
                    FsEntry::File { path, .. } => path.clone(),
                    _ => {
                        self.add_log("Selected item is not a file".to_string());
                        return;
                    }
                }
            }
        };

        let source_fmt = Format::from_extension(&input);
        if source_fmt == Some(target) {
            self.add_log(format!("File is already in {} format", format_name(target)));
            return;
        }

        // Build output path: same stem, new extension
        let output_ext = match target {
            Format::Markdown => "md",
            Format::Html => "html",
            Format::Pdf => "pdf",
            Format::Typst => "typ",
        };
        let mut output = input.clone();
        output.set_extension(output_ext);

        // Avoid overwriting the input
        if output == input {
            let stem = input
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("output");
            output = input
                .parent()
                .unwrap_or(Path::new("."))
                .join(format!("{stem}-converted.{output_ext}"));
        }

        // Avoid overwriting existing files: append a number suffix
        let mut counter = 1;
        let original_output = output.clone();
        while output.exists() {
            let stem = original_output
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("output");
            let new_name = format!("{stem}-{counter}.{output_ext}");
            output = original_output
                .parent()
                .unwrap_or(Path::new("."))
                .join(&new_name);
            counter += 1;
        }

        self.add_log(format!(
            "Converting: {} → {}",
            input
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("?"),
            output
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("?")
        ));

        match convert(&input, &output, target) {
            Ok(()) => {
                self.add_log(format!("✓ Conversion complete: {}", output.display()));
                // Refresh the browser to show the new file
                self.navigate_to(&self.current_dir.clone());
                // Preview the result
                self.preview_path = Some(output);
                if let Ok(content) = preview_file(self.preview_path.as_ref().unwrap()) {
                    self.preview_content = content;
                    self.preview_scroll = 0;
                }
            }
            Err(e) => {
                self.add_log(format!("✗ Conversion failed: {e}"));
            }
        }
    }

    /// Handle a key event. Returns true if the event was consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Help mode: only Esc or ? exits
        if self.mode == Mode::Help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                self.mode = Mode::Browse;
                return true;
            }
            return true;
        }

        // Settings mode: Esc exits, s/Tab cycles theme, q quits
        if self.mode == Mode::Settings {
            return match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Browse;
                    true
                }
                KeyCode::Char('s') | KeyCode::Tab => {
                    self.cycle_theme();
                    true
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.running = false;
                    true
                }
                _ => true,
            };
        }

        // Confirm delete mode: y/Enter confirms, n/Esc cancels
        if self.mode == Mode::ConfirmDelete {
            return match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.confirm_delete();
                    self.mode = Mode::Browse;
                    true
                }
                KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.pending_delete_path = None;
                    self.pending_delete_name.clear();
                    self.mode = Mode::Browse;
                    true
                }
                _ => true,
            };
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.running = false;
                true
            }
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
                true
            }
            KeyCode::Char('s') => {
                self.mode = Mode::Settings;
                true
            }
            KeyCode::Esc => {
                self.mode = Mode::Browse;
                true
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Browser => Focus::Preview,
                    Focus::Preview => Focus::Log,
                    Focus::Log => Focus::Browser,
                };
                true
            }
            KeyCode::Char('1') => {
                self.focus = Focus::Browser;
                true
            }
            KeyCode::Char('2') => {
                self.focus = Focus::Preview;
                true
            }
            KeyCode::Char('3') => {
                self.focus = Focus::Log;
                true
            }
            _ if self.filtering && self.focus == Focus::Browser => self.handle_filter_key(key),
            _ => match self.focus {
                Focus::Browser => self.handle_browser_key(key),
                Focus::Preview => self.handle_preview_key(key),
                Focus::Log => self.handle_log_key(key),
            },
        }
    }

    fn handle_browser_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next();
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_prev();
                true
            }
            KeyCode::Enter => {
                self.activate_selection();
                true
            }
            KeyCode::Char('h') if self.preview_path.is_some() => {
                self.convert_current(Format::Html);
                true
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.parent_directory();
                true
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.enter_child_directory();
                true
            }
            KeyCode::Char('m') => {
                self.convert_current(Format::Markdown);
                true
            }
            KeyCode::Char('p') => {
                self.convert_current(Format::Pdf);
                true
            }
            KeyCode::Char('t') => {
                self.convert_current(Format::Typst);
                true
            }
            KeyCode::Char('r') => {
                self.add_log("Refreshing directory...".to_string());
                self.navigate_to(&self.current_dir.clone());
                true
            }
            KeyCode::Char('d') => {
                // Open confirmation dialog instead of deleting directly
                let entry_idx = match self.real_selected_index() {
                    Some(idx) => idx,
                    None => return true,
                };
                if let FsEntry::File { path, name, .. } = &self.entries[entry_idx] {
                    self.pending_delete_path = Some(path.clone());
                    self.pending_delete_name = name.clone();
                    self.mode = Mode::ConfirmDelete;
                } else {
                    self.add_log("Cannot delete directories from the TUI".to_string());
                }
                true
            }
            KeyCode::Char('R') => {
                let entry_idx = match self.real_selected_index() {
                    Some(idx) => idx,
                    None => {
                        self.add_log("No entry selected".to_string());
                        return true;
                    }
                };
                let old_name = self.entries[entry_idx].name().to_string();
                // For v1, append "-renamed" before the extension
                let (stem, ext) = match old_name.rsplit_once('.') {
                    Some((s, e)) => (s.to_string(), format!(".{e}")),
                    None => (old_name.clone(), String::new()),
                };
                let new_name = format!("{stem}-renamed{ext}");
                self.rename_selected(&new_name);
                true
            }
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter_query.clear();
                self.add_log("Filter: type query, Enter to apply, Esc to cancel".to_string());
                true
            }
            _ => false,
        }
    }

    /// Handle keys during filter input mode.
    fn handle_filter_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.filtering = false;
                self.set_filter("");
                self.add_log("Filter cancelled".to_string());
                true
            }
            KeyCode::Enter => {
                self.filtering = false;
                if self.filter_query.is_empty() {
                    self.add_log("Filter cleared".to_string());
                } else {
                    self.add_log(format!(
                        "Filter: {} ({} matches)",
                        self.filter_query,
                        self.entries.len()
                    ));
                }
                true
            }
            KeyCode::Char(c) => {
                self.filter_query.push(c);
                let query = self.filter_query.clone();
                self.set_filter(&query);
                true
            }
            KeyCode::Backspace => {
                self.filter_query.pop();
                let query = self.filter_query.clone();
                self.set_filter(&query);
                true
            }
            _ => true,
        }
    }

    fn handle_preview_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
                true
            }
            KeyCode::PageDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(10);
                true
            }
            KeyCode::PageUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(10);
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.preview_scroll = 0;
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                // Set to a very high value; the Paragraph widget will clamp
                self.preview_scroll = u16::MAX;
                true
            }
            KeyCode::Char('m') => {
                self.convert_current(Format::Markdown);
                true
            }
            KeyCode::Char('h') => {
                self.convert_current(Format::Html);
                true
            }
            KeyCode::Char('p') => {
                self.convert_current(Format::Pdf);
                true
            }
            KeyCode::Char('t') => {
                self.convert_current(Format::Typst);
                true
            }
            _ => false,
        }
    }

    fn handle_log_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.log_scroll =
                    (self.log_scroll + 1).min(self.log_messages.len().saturating_sub(1) as u16);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
                true
            }
            KeyCode::PageDown => {
                self.log_scroll =
                    (self.log_scroll + 10).min(self.log_messages.len().saturating_sub(1) as u16);
                true
            }
            KeyCode::PageUp => {
                self.log_scroll = self.log_scroll.saturating_sub(10);
                true
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.log_scroll = 0;
                true
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.log_scroll = self.log_messages.len().saturating_sub(1) as u16;
                true
            }
            _ => false,
        }
    }

    /// Handle a mouse event. Returns true if the event was consumed.
    pub fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> bool {
        if event.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }

        let areas = layout::chunk_areas(area);
        let browser = areas.browser;

        // Check if click is within the browser panel
        if event.row >= browser.y
            && event.row < browser.y + browser.height
            && event.column >= browser.x
            && event.column < browser.x + browser.width
        {
            // Content starts after the top border (1 line)
            let content_row = event.row.saturating_sub(browser.y).saturating_sub(1) as usize;
            let has_parent = self.current_dir.parent().is_some();

            let click_index = if has_parent {
                if content_row == 0 {
                    // Clicked on ".." — select index 0
                    0
                } else {
                    // content_row - 1 maps to entries index, but +1 for the ".." entry
                    content_row
                }
            } else {
                content_row
            };

            // Clamp to valid range
            let max_index = if has_parent {
                self.entries.len() // +1 for ".."
            } else {
                self.entries.len().saturating_sub(1)
            };

            if click_index <= max_index && !self.entries.is_empty() {
                self.selected_index = click_index;
                self.focus = Focus::Browser;
                self.activate_selection();
            }

            return true;
        }

        false
    }

    /// Advance one tick of the application loop.
    #[allow(dead_code)]
    pub fn tick(&mut self) {}

    /// Render the current state into the terminal frame.
    #[allow(dead_code)]
    pub fn render(&self, _frame: &mut Frame, _area: Rect) {}
}

fn format_name(f: Format) -> &'static str {
    match f {
        Format::Markdown => "Markdown",
        Format::Html => "HTML",
        Format::Pdf => "PDF",
        Format::Typst => "Typst",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Create an `App` with a fixed set of entries (no filesystem dependency).
    fn app_with_entries(entries: Vec<FsEntry>) -> App {
        let all_entries = entries.clone();
        App {
            mode: Mode::Browse,
            focus: Focus::Browser,
            running: true,
            current_dir: PathBuf::from("/test"),
            entries,
            all_entries,
            filter_query: String::new(),
            filtering: false,
            selected_index: 0,
            preview_content: String::new(),
            preview_path: None,
            log_messages: vec!["Ready".to_string()],
            preview_scroll: 0,
            log_scroll: 0,
            theme: Theme::dark(),
            theme_name: "dark".to_string(),
            pending_delete_path: None,
            pending_delete_name: String::new(),
        }
    }

    /// Create a temporary directory with a known structure for filesystem tests.
    fn create_test_dir() -> TempDir {
        let dir = TempDir::new().expect("failed to create temp dir");
        let root = dir.path();

        // Supported files
        fs::write(root.join("readme.md"), "# Hello").unwrap();
        fs::write(root.join("index.html"), "<h1>Hi</h1>").unwrap();
        fs::write(root.join("doc.typ"), "# Doc").unwrap();

        // Subdirectories
        fs::create_dir(root.join("subdir")).unwrap();
        fs::create_dir(root.join("another")).unwrap();
        fs::create_dir(root.join("subdir/nested")).unwrap();

        dir
    }

    /// Create an `App` initialised to point at a `TempDir`.
    fn app_in_temp_dir(dir: &TempDir) -> App {
        let path = dir.path().to_path_buf();
        let all_entries = list_entries(&path).unwrap_or_default();
        let entries = all_entries.clone();
        App {
            mode: Mode::Browse,
            focus: Focus::Browser,
            running: true,
            current_dir: path,
            entries,
            all_entries,
            filter_query: String::new(),
            filtering: false,
            selected_index: 0,
            preview_content: String::new(),
            preview_path: None,
            log_messages: vec!["Ready".to_string()],
            preview_scroll: 0,
            log_scroll: 0,
            theme: Theme::dark(),
            theme_name: "dark".to_string(),
            pending_delete_path: None,
            pending_delete_name: String::new(),
        }
    }

    // ======================================================================
    // App::new()
    // ======================================================================

    #[test]
    fn new_app_is_running() {
        let app = App::new();
        assert!(app.running, "new app should be running");
    }

    #[test]
    fn new_app_mode_is_browse() {
        let app = App::new();
        assert_eq!(app.mode, Mode::Browse);
    }

    #[test]
    fn new_app_focus_is_browser() {
        let app = App::new();
        assert_eq!(app.focus, Focus::Browser);
    }

    #[test]
    fn new_app_current_dir_not_empty() {
        let app = App::new();
        assert!(
            !app.current_dir.as_os_str().is_empty(),
            "current_dir should not be empty"
        );
    }

    #[test]
    fn new_app_log_starts_with_ready() {
        let app = App::new();
        assert!(!app.log_messages.is_empty(), "log should not be empty");
        assert_eq!(app.log_messages[0], "Ready");
    }

    #[test]
    fn new_app_selection_at_zero() {
        let app = App::new();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn new_app_preview_is_empty() {
        let app = App::new();
        assert!(app.preview_content.is_empty());
        assert!(app.preview_path.is_none());
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn default_equals_new() {
        let a = App::new();
        let b = App::default();
        assert_eq!(a.running, b.running);
        assert_eq!(a.mode, b.mode);
        assert_eq!(a.focus, b.focus);
        assert_eq!(a.selected_index, b.selected_index);
        assert_eq!(a.log_messages, b.log_messages);
        // current_dir depends on config which may be modified by parallel tests
        // so we only check it's non-empty
        assert!(!a.current_dir.as_os_str().is_empty());
        assert!(!b.current_dir.as_os_str().is_empty());
        assert_eq!(a.filtering, b.filtering);
        assert_eq!(a.filter_query, b.filter_query);
    }

    // ======================================================================
    // select_next / select_prev
    // ======================================================================

    #[test]
    fn select_next_increments() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        app.select_next();
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn select_next_stays_at_bound() {
        let mut app = app_with_entries(vec![FsEntry::File {
            path: PathBuf::from("a.md"),
            name: "a.md".into(),
            extension: "md".into(),
            size: 0,
        }]);
        app.select_next(); // has_parent so max=1
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn select_next_empty_entries_no_panic() {
        let mut app = app_with_entries(vec![]);
        app.select_next();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn select_next_multiple_times() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("c.md"),
                name: "c.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        app.select_next();
        app.select_next();
        assert_eq!(app.selected_index, 2);
        app.select_next(); // at bound
        assert_eq!(app.selected_index, 3);
    }

    #[test]
    fn select_prev_decrements() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        app.selected_index = 1;
        app.select_prev();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn select_prev_stays_at_zero() {
        let mut app = app_with_entries(vec![FsEntry::File {
            path: PathBuf::from("a.md"),
            name: "a.md".into(),
            extension: "md".into(),
            size: 0,
        }]);
        app.selected_index = 0;
        app.select_prev();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn select_prev_empty_entries_no_panic() {
        let mut app = app_with_entries(vec![]);
        app.select_prev();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn select_next_then_prev_roundtrip() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("c.md"),
                name: "c.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        app.select_next();
        app.select_next();
        assert_eq!(app.selected_index, 2);
        app.select_prev();
        assert_eq!(app.selected_index, 1);
        app.select_prev();
        assert_eq!(app.selected_index, 0);
    }

    // ======================================================================
    // add_log
    // ======================================================================

    #[test]
    fn add_log_appends_message() {
        let mut app = App::new();
        let initial_len = app.log_messages.len();
        app.add_log("test message".to_string());
        assert_eq!(app.log_messages.len(), initial_len + 1);
        assert_eq!(app.log_messages.last().unwrap(), "test message");
    }

    #[test]
    fn add_log_updates_scroll() {
        let mut app = App::new();
        app.add_log("msg1".to_string());
        app.add_log("msg2".to_string());
        assert_eq!(app.log_scroll, (app.log_messages.len() - 1) as u16);
    }

    #[test]
    fn add_log_multiple_messages() {
        let mut app = App::new();
        for i in 0..10 {
            app.add_log(format!("msg {i}"));
        }
        assert_eq!(app.log_messages.len(), 11); // "Ready" + 10
        assert_eq!(app.log_messages.last().unwrap(), "msg 9");
        assert_eq!(app.log_scroll, 10);
    }

    // ======================================================================
    // navigate_to
    // ======================================================================

    #[test]
    fn navigate_to_valid_directory_updates_state() {
        let dir = create_test_dir();
        let mut app = app_with_entries(vec![]);
        app.navigate_to(dir.path());
        assert_eq!(app.current_dir, dir.path());
        assert!(!app.entries.is_empty(), "should have entries");
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn navigate_to_invalid_directory_logs_error() {
        let mut app = app_with_entries(vec![]);
        let bad = PathBuf::from("/nonexistent/path/that/does/not/exist");
        app.navigate_to(&bad);
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("Cannot open directory")),
            "should log error"
        );
    }

    #[test]
    fn navigate_to_invalid_directory_does_not_change_current_dir() {
        let mut app = app_with_entries(vec![]);
        let original = app.current_dir.clone();
        let bad = PathBuf::from("/nonexistent/path/that/does/not/exist");
        app.navigate_to(&bad);
        assert_eq!(app.current_dir, original);
    }

    #[test]
    fn navigate_to_resets_preview_state() {
        let dir = create_test_dir();
        let mut app = app_with_entries(vec![]);
        app.preview_content = "old".to_string();
        app.preview_path = Some(PathBuf::from("old.md"));
        app.preview_scroll = 42;
        app.navigate_to(dir.path());
        assert!(app.preview_content.is_empty());
        assert!(app.preview_path.is_none());
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn navigate_to_reloads_entries() {
        let dir = create_test_dir();
        let mut app = app_with_entries(vec![]);
        app.navigate_to(dir.path());
        let names: Vec<&str> = app.entries.iter().map(|e| e.name()).collect();
        // Directories first, alphabetically: "another", "subdir"
        // Then files, alphabetically: "doc.typ", "index.html", "readme.md"
        assert_eq!(
            names,
            vec!["another", "subdir", "doc.typ", "index.html", "readme.md"]
        );
    }

    // ======================================================================
    // set_filter / filter
    // ======================================================================

    #[test]
    fn set_filter_filters_entries() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("readme.md"),
                name: "readme.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("index.html"),
                name: "index.html".into(),
                extension: "html".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("doc.typ"),
                name: "doc.typ".into(),
                extension: "typ".into(),
                size: 0,
            },
        ]);
        app.set_filter("read");
        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].name(), "readme.md");
    }

    #[test]
    fn set_filter_empty_restores_all() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        app.set_filter("a");
        assert_eq!(app.entries.len(), 1);
        app.set_filter("");
        assert_eq!(app.entries.len(), 2);
    }

    #[test]
    fn set_filter_is_case_insensitive() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("ReadMe.md"),
                name: "ReadMe.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("index.html"),
                name: "index.html".into(),
                extension: "html".into(),
                size: 0,
            },
        ]);
        app.set_filter("readme");
        assert_eq!(app.entries.len(), 1);
    }

    #[test]
    fn set_filter_resets_selection_and_preview() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        app.selected_index = 1;
        app.preview_content = "old".to_string();
        app.preview_path = Some(PathBuf::from("b.md"));
        app.set_filter("a");
        assert_eq!(app.selected_index, 0);
        assert!(app.preview_content.is_empty());
        assert!(app.preview_path.is_none());
    }

    #[test]
    fn browser_key_slash_enters_filter_mode() {
        let mut app = app_with_entries(vec![]);
        let key = KeyEvent::new(KeyCode::Char('/'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert!(app.filtering);
        assert!(app.filter_query.is_empty());
    }

    #[test]
    fn filter_key_char_appends_to_query() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("readme.md"),
                name: "readme.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("index.html"),
                name: "index.html".into(),
                extension: "html".into(),
                size: 0,
            },
        ]);
        app.filtering = true;
        let key = KeyEvent::new(KeyCode::Char('r'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_filter_key(key));
        assert_eq!(app.filter_query, "r");
        let key = KeyEvent::new(KeyCode::Char('e'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_filter_key(key));
        assert_eq!(app.filter_query, "re");
        // Should filter as we type
        assert_eq!(app.entries.len(), 1);
    }

    #[test]
    fn filter_key_backspace_removes_char() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("report.md"),
                name: "report.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("readme.md"),
                name: "readme.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("index.html"),
                name: "index.html".into(),
                extension: "html".into(),
                size: 0,
            },
        ]);
        app.filtering = true;
        app.filter_query = "re".to_string();
        app.set_filter("re");
        // "re" matches "report.md" and "readme.md"
        assert_eq!(app.entries.len(), 2);
        let key = KeyEvent::new(KeyCode::Backspace, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_filter_key(key));
        assert_eq!(app.filter_query, "r");
        // "r" matches "report.md" and "readme.md" (still 2)
        assert_eq!(app.entries.len(), 2);
        // Backspace again to empty
        let key = KeyEvent::new(KeyCode::Backspace, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_filter_key(key));
        assert_eq!(app.filter_query, "");
        assert_eq!(app.entries.len(), 3);
    }

    #[test]
    fn filter_key_escape_cancels() {
        let mut app = app_with_entries(vec![]);
        app.filtering = true;
        app.filter_query = "test".to_string();
        let key = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_filter_key(key));
        assert!(!app.filtering);
        assert!(app.filter_query.is_empty());
    }

    #[test]
    fn filter_key_enter_applies() {
        let mut app = app_with_entries(vec![]);
        app.filtering = true;
        app.filter_query = "test".to_string();
        let key = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_filter_key(key));
        assert!(!app.filtering);
    }

    #[test]
    fn navigate_to_clears_filter() {
        let dir = create_test_dir();
        let mut app = app_with_entries(vec![]);
        app.filter_query = "test".to_string();
        app.navigate_to(dir.path());
        assert!(app.filter_query.is_empty());
        assert!(!app.filtering);
    }

    // ======================================================================
    // Confirmation dialog / delete
    // ======================================================================

    #[test]
    fn browser_key_d_opens_confirm_dialog() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.md");
        fs::write(&file_path, "content").unwrap();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "test.md")
            .unwrap()
            + 1; // visual index
        app.selected_index = idx;
        let key = KeyEvent::new(KeyCode::Char('d'), crossterm::event::KeyModifiers::NONE);
        app.handle_browser_key(key);
        assert_eq!(
            app.mode,
            Mode::ConfirmDelete,
            "pressing d should open confirm dialog"
        );
        assert_eq!(app.pending_delete_name, "test.md");
        assert!(app.pending_delete_path.is_some());
    }

    #[test]
    fn confirm_delete_removes_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.md");
        fs::write(&file_path, "content").unwrap();
        let mut app = app_in_temp_dir(&dir);
        app.pending_delete_path = Some(file_path.clone());
        app.pending_delete_name = "test.md".to_string();
        app.confirm_delete();
        assert!(!file_path.exists(), "file should be deleted");
        assert!(app.log_messages.iter().any(|m| m.contains("Deleted")));
    }

    #[test]
    fn confirm_delete_empty_path_does_nothing() {
        let mut app = app_with_entries(vec![]);
        app.confirm_delete(); // should not panic
        assert!(app.pending_delete_path.is_none());
    }

    #[test]
    fn browser_key_d_on_directory_is_rejected() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "subdir")
            .unwrap()
            + 1; // visual index
        app.selected_index = idx;
        let key = KeyEvent::new(KeyCode::Char('d'), crossterm::event::KeyModifiers::NONE);
        app.handle_browser_key(key);
        assert!(
            app.log_messages.iter().any(|m| m.contains("Cannot delete")),
            "should log rejection"
        );
        assert_ne!(
            app.mode,
            Mode::ConfirmDelete,
            "should not open confirm dialog for directories"
        );
    }

    #[test]
    fn confirm_delete_nonexistent_file_logs_error() {
        let dir = TempDir::new().unwrap();
        let mut app = app_in_temp_dir(&dir);
        app.pending_delete_path = Some(dir.path().join("ghost.md"));
        app.pending_delete_name = "ghost.md".to_string();
        app.confirm_delete();
        assert!(app.log_messages.iter().any(|m| m.contains("failed")));
    }

    #[test]
    fn handle_key_y_confirms_delete() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.md");
        fs::write(&file_path, "content").unwrap();
        let mut app = app_in_temp_dir(&dir);
        app.mode = Mode::ConfirmDelete;
        app.pending_delete_path = Some(file_path.clone());
        app.pending_delete_name = "test.md".to_string();
        let key = KeyEvent::new(KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert!(!file_path.exists(), "file should be deleted after confirm");
        assert_eq!(app.mode, Mode::Browse);
    }

    #[test]
    fn handle_key_n_cancels_delete() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.md");
        fs::write(&file_path, "content").unwrap();
        let mut app = app_in_temp_dir(&dir);
        app.mode = Mode::ConfirmDelete;
        app.pending_delete_path = Some(file_path.clone());
        app.pending_delete_name = "test.md".to_string();
        let key = KeyEvent::new(KeyCode::Char('n'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert!(
            file_path.exists(),
            "file should NOT be deleted after cancel"
        );
        assert_eq!(app.mode, Mode::Browse);
        assert!(app.pending_delete_path.is_none());
    }

    #[test]
    fn handle_key_esc_cancels_delete() {
        let mut app = App::new();
        app.mode = Mode::ConfirmDelete;
        app.pending_delete_path = Some(PathBuf::from("test.md"));
        app.pending_delete_name = "test.md".to_string();
        let key = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.mode, Mode::Browse);
        assert!(app.pending_delete_path.is_none());
    }

    #[test]
    fn rename_selected_renames_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.md");
        fs::write(&file_path, "content").unwrap();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "test.md")
            .unwrap();
        app.selected_index = idx;
        app.rename_selected("renamed.md");
        assert!(!file_path.exists(), "old name should be gone");
        assert!(
            dir.path().join("renamed.md").exists(),
            "new name should exist"
        );
        assert!(app.log_messages.iter().any(|m| m.contains("Renamed")));
    }

    #[test]
    fn rename_selected_empty_name_skips() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.md");
        fs::write(&file_path, "content").unwrap();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "test.md")
            .unwrap();
        app.selected_index = idx;
        app.rename_selected("");
        assert!(file_path.exists(), "file should remain");
        assert!(app.log_messages.iter().any(|m| m.contains("skipped")));
    }

    #[test]
    fn rename_selected_same_name_skips() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.md");
        fs::write(&file_path, "content").unwrap();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "test.md")
            .unwrap();
        app.selected_index = idx;
        app.rename_selected("test.md");
        assert!(file_path.exists());
        assert!(app.log_messages.iter().any(|m| m.contains("skipped")));
    }

    #[test]
    fn rename_selected_empty_entries_no_panic() {
        let mut app = app_with_entries(vec![]);
        app.rename_selected("new.md"); // should not panic
    }

    #[test]
    fn browser_key_uppercase_r_renames_selected() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.md");
        fs::write(&file_path, "content").unwrap();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "test.md")
            .unwrap();
        app.selected_index = idx;
        let key = KeyEvent::new(KeyCode::Char('R'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert!(dir.path().join("test-renamed.md").exists());
    }

    // ======================================================================
    // parent_directory
    // ======================================================================

    #[test]
    fn parent_directory_navigates_up() {
        let dir = create_test_dir();
        let subdir = dir.path().join("subdir");
        let mut app = app_in_temp_dir(&dir);
        app.navigate_to(&subdir);
        assert_eq!(app.current_dir, subdir);
        app.parent_directory();
        assert_eq!(app.current_dir, dir.path());
    }

    #[test]
    fn parent_directory_at_deep_path() {
        let dir = create_test_dir();
        let nested = dir.path().join("subdir").join("nested");
        let mut app = app_in_temp_dir(&dir);
        app.navigate_to(&nested);
        app.parent_directory();
        assert_eq!(app.current_dir, dir.path().join("subdir"));
        app.parent_directory();
        assert_eq!(app.current_dir, dir.path());
    }

    #[test]
    fn parent_directory_at_root_does_not_crash() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        app.parent_directory(); // go up to /tmp/...
        app.parent_directory(); // go up to /tmp
        app.parent_directory(); // go up to /
        app.parent_directory(); // at /, parent() returns None — no crash
        // Should still be at /
        assert_eq!(app.current_dir, PathBuf::from("/"));
    }

    // ======================================================================
    // activate_selection
    // ======================================================================

    #[test]
    fn activate_selection_enters_directory() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| matches!(e, FsEntry::Directory { name, .. } if name == "subdir"))
            .expect("subdir entry should exist");
        let has_parent = app.current_dir.parent().is_some();
        app.selected_index = if has_parent { idx + 1 } else { idx };
        app.activate_selection();
        assert_eq!(app.current_dir, dir.path().join("subdir"));
        assert_eq!(app.mode, Mode::Browse);
    }

    #[test]
    fn activate_selection_previews_file() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| matches!(e, FsEntry::File { name, .. } if name == "readme.md"))
            .expect("readme.md should exist");
        let has_parent = app.current_dir.parent().is_some();
        app.selected_index = if has_parent { idx + 1 } else { idx };
        app.activate_selection();
        assert_eq!(app.mode, Mode::Preview);
        assert_eq!(app.focus, Focus::Preview);
        assert!(app.preview_path.is_some());
        assert!(!app.preview_content.is_empty());
    }

    #[test]
    fn activate_selection_preview_content_matches() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| matches!(e, FsEntry::File { name, .. } if name == "readme.md"))
            .expect("readme.md should exist");
        let has_parent = app.current_dir.parent().is_some();
        app.selected_index = if has_parent { idx + 1 } else { idx };
        app.activate_selection();
        assert_eq!(app.preview_content, "# Hello");
    }

    #[test]
    fn activate_selection_empty_entries_no_panic() {
        let mut app = app_with_entries(vec![]);
        app.activate_selection();
        assert_eq!(app.mode, Mode::Browse);
        assert!(app.preview_path.is_none());
    }

    // ======================================================================
    // enter_child_directory
    // ======================================================================

    #[test]
    fn enter_child_directory_goes_to_parent_at_index_zero() {
        let dir = create_test_dir();
        let subdir = dir.path().join("subdir");
        let mut app = app_in_temp_dir(&dir);
        app.navigate_to(&subdir);
        // index 0 with parent present triggers parent_directory
        app.selected_index = 0;
        app.enter_child_directory();
        assert_eq!(app.current_dir, dir.path());
    }

    #[test]
    fn enter_child_directory_selects_subdirectory() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        // With has_parent=true, visual index 0 = "..", 1 = entries[0], 2 = entries[1]
        let second_entry_name = app.entries[1].name().to_string();
        app.selected_index = 2; // visual index for entries[1]
        app.enter_child_directory();
        assert_eq!(
            app.current_dir,
            dir.path().join(&second_entry_name),
            "should enter entries[1] via visual index 2"
        );
    }

    #[test]
    fn enter_child_directory_empty_entries_no_panic() {
        let mut app = app_with_entries(vec![]);
        app.enter_child_directory();
        assert_eq!(app.mode, Mode::Browse);
    }

    // ======================================================================
    // handle_key — top-level dispatch
    // ======================================================================

    #[test]
    fn handle_key_q_quits() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::Char('q'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert!(!app.running);
    }

    #[test]
    fn handle_key_capital_q_quits() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::Char('Q'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert!(!app.running);
    }

    #[test]
    fn handle_key_question_enters_help() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::Char('?'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.mode, Mode::Help);
    }

    #[test]
    fn handle_key_esc_exits_help() {
        let mut app = App::new();
        app.mode = Mode::Help;
        let key = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.mode, Mode::Browse);
    }

    #[test]
    fn handle_key_question_exits_help() {
        let mut app = App::new();
        app.mode = Mode::Help;
        let key = KeyEvent::new(KeyCode::Char('?'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.mode, Mode::Browse);
    }

    #[test]
    fn handle_key_in_help_consumes_all_keys() {
        let mut app = App::new();
        app.mode = Mode::Help;
        // 'q' in help mode should NOT quit
        let key = KeyEvent::new(KeyCode::Char('q'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert!(app.running);
    }

    #[test]
    fn handle_key_1_jumps_to_browser() {
        let mut app = App::new();
        app.focus = Focus::Log;
        let key = KeyEvent::new(KeyCode::Char('1'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.focus, Focus::Browser);
    }

    #[test]
    fn handle_key_2_jumps_to_preview() {
        let mut app = App::new();
        app.focus = Focus::Log;
        let key = KeyEvent::new(KeyCode::Char('2'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.focus, Focus::Preview);
    }

    #[test]
    fn handle_key_3_jumps_to_log() {
        let mut app = App::new();
        app.focus = Focus::Browser;
        let key = KeyEvent::new(KeyCode::Char('3'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.focus, Focus::Log);
    }

    #[test]
    fn handle_key_tab_cycles_focus() {
        let mut app = App::new();
        let tab = KeyEvent::new(KeyCode::Tab, crossterm::event::KeyModifiers::NONE);
        assert_eq!(app.focus, Focus::Browser);
        assert!(app.handle_key(tab));
        assert_eq!(app.focus, Focus::Preview);
        assert!(app.handle_key(tab));
        assert_eq!(app.focus, Focus::Log);
        assert!(app.handle_key(tab));
        assert_eq!(app.focus, Focus::Browser);
    }

    #[test]
    fn handle_key_esc_from_preview_returns_browse() {
        let mut app = App::new();
        app.mode = Mode::Preview;
        let key = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.mode, Mode::Browse);
    }

    #[test]
    fn handle_key_dispatches_to_browser_focus() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        app.focus = Focus::Browser;
        let key = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn handle_key_dispatches_to_preview_focus() {
        let mut app = App::new();
        app.focus = Focus::Preview;
        let key = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.preview_scroll, 1);
    }

    #[test]
    fn handle_key_dispatches_to_log_focus() {
        let mut app = App::new();
        app.add_log("msg1".to_string()); // give the log enough entries to scroll
        app.focus = Focus::Log;
        let key = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.log_scroll, 1);
    }

    #[test]
    fn handle_key_unhandled_returns_false() {
        let mut app = App::new();
        app.focus = Focus::Browser;
        let key = KeyEvent::new(KeyCode::Char('x'), crossterm::event::KeyModifiers::NONE);
        assert!(!app.handle_key(key));
    }

    // ======================================================================
    // handle_browser_key
    // ======================================================================

    #[test]
    fn browser_key_j_moves_down() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        let key = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn browser_key_down_moves_down() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        let key = KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn browser_key_k_moves_up() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        app.selected_index = 1;
        let key = KeyEvent::new(KeyCode::Char('k'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn browser_key_up_moves_up() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
                size: 0,
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
                size: 0,
            },
        ]);
        app.selected_index = 1;
        let key = KeyEvent::new(KeyCode::Up, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn browser_key_enter_activates_file() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "readme.md")
            .expect("readme.md should exist");
        app.selected_index = idx;
        let key = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(app.mode, Mode::Preview);
    }

    #[test]
    fn browser_key_enter_enters_directory() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "subdir")
            .expect("subdir should exist");
        let has_parent = app.current_dir.parent().is_some();
        app.selected_index = if has_parent { idx + 1 } else { idx };
        let key = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(app.current_dir, dir.path().join("subdir"));
    }

    #[test]
    fn browser_key_h_goes_to_parent() {
        let dir = create_test_dir();
        let subdir = dir.path().join("subdir");
        let mut app = app_in_temp_dir(&dir);
        app.navigate_to(&subdir);
        // No preview_path set → h means parent directory
        let key = KeyEvent::new(KeyCode::Char('h'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(app.current_dir, dir.path());
    }

    #[test]
    fn browser_key_h_converts_html_when_preview_active() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        app.preview_path = Some(dir.path().join("readme.md"));
        // preview_path is set → h means convert to HTML
        let key = KeyEvent::new(KeyCode::Char('h'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("Converting") || m.contains("already")),
            "h with preview should trigger HTML conversion"
        );
    }

    #[test]
    fn browser_key_left_goes_to_parent() {
        let dir = create_test_dir();
        let subdir = dir.path().join("subdir");
        let mut app = app_in_temp_dir(&dir);
        app.navigate_to(&subdir);
        let key = KeyEvent::new(KeyCode::Left, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(app.current_dir, dir.path());
    }

    #[test]
    fn browser_key_l_enters_child() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        // With has_parent=true, visual index 0 = "..", 1 = entries[0], 2 = entries[1]
        let entry_name = app.entries[1].name().to_string();
        app.selected_index = 2; // visual index for entries[1]
        let key = KeyEvent::new(KeyCode::Char('l'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(
            app.current_dir,
            dir.path().join(&entry_name),
            "'l' should enter the entry at visual index 2 (entries[1])"
        );
    }

    #[test]
    fn browser_key_right_enters_child() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        let entry_name = app.entries[1].name().to_string();
        app.selected_index = 2; // visual index for entries[1]
        let key = KeyEvent::new(KeyCode::Right, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(
            app.current_dir,
            dir.path().join(&entry_name),
            "'right' should enter the entry at visual index 2 (entries[1])"
        );
    }

    #[test]
    fn convert_current_uses_selected_file_when_no_preview() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        // Select readme.md in the browser WITHOUT activating it (no preview_path)
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "readme.md")
            .expect("readme.md should exist");
        app.selected_index = idx + 1; // visual index (".." at 0)
        app.preview_path = None;
        assert!(app.preview_path.is_none());
        // Convert MD → PDF without opening the file
        app.convert_current(Format::Pdf);
        let log = app.log_messages.join(" ");
        assert!(
            log.contains("Converting") || log.contains("already"),
            "should convert the selected file, got: {log}"
        );
    }

    #[test]
    fn convert_current_selected_directory_logs_not_a_file() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        // Select a directory (subdir) without previewing
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "subdir")
            .expect("subdir should exist");
        app.selected_index = idx + 1; // visual index
        app.preview_path = None;
        app.convert_current(Format::Pdf);
        assert!(
            app.log_messages.iter().any(|m| m.contains("not a file")),
            "selecting a directory should log 'not a file'"
        );
    }

    #[test]
    fn browser_key_m_triggers_convert() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        // Select a file first so convert_current has something to work with
        let idx = app
            .entries
            .iter()
            .position(|e| e.name() == "readme.md")
            .expect("readme.md should exist");
        app.selected_index = idx;
        // Activate it first
        app.activate_selection();
        assert_eq!(app.mode, Mode::Preview);
        // Now M from browser should still work (but convert_current logs)
        let key = KeyEvent::new(KeyCode::Char('m'), crossterm::event::KeyModifiers::NONE);
        app.handle_browser_key(key);
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("Converting") || m.contains("already"))
        );
    }

    #[test]
    fn browser_key_p_triggers_convert() {
        let mut app = App::new();
        app.preview_path = Some(PathBuf::from("test.md"));
        let key = KeyEvent::new(KeyCode::Char('p'), crossterm::event::KeyModifiers::NONE);
        app.handle_browser_key(key);
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("Converting") || m.contains("No file"))
        );
    }

    #[test]
    fn browser_key_t_triggers_convert() {
        let mut app = App::new();
        app.preview_path = Some(PathBuf::from("test.html"));
        let key = KeyEvent::new(KeyCode::Char('t'), crossterm::event::KeyModifiers::NONE);
        app.handle_browser_key(key);
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("Converting") || m.contains("already"))
        );
    }

    #[test]
    fn browser_key_unknown_returns_false() {
        let mut app = app_with_entries(vec![]);
        let key = KeyEvent::new(KeyCode::Char('x'), crossterm::event::KeyModifiers::NONE);
        assert!(!app.handle_browser_key(key));
    }

    // ======================================================================
    // handle_preview_key
    // ======================================================================

    #[test]
    fn preview_key_j_scrolls_down() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_preview_key(key));
        assert_eq!(app.preview_scroll, 1);
    }

    #[test]
    fn preview_key_down_scrolls_down() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_preview_key(key));
        assert_eq!(app.preview_scroll, 1);
    }

    #[test]
    fn preview_key_k_scrolls_up() {
        let mut app = App::new();
        app.preview_scroll = 5;
        let key = KeyEvent::new(KeyCode::Char('k'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_preview_key(key));
        assert_eq!(app.preview_scroll, 4);
    }

    #[test]
    fn preview_key_up_scrolls_up() {
        let mut app = App::new();
        app.preview_scroll = 5;
        let key = KeyEvent::new(KeyCode::Up, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_preview_key(key));
        assert_eq!(app.preview_scroll, 4);
    }

    #[test]
    fn preview_key_k_does_not_go_below_zero() {
        let mut app = App::new();
        app.preview_scroll = 0;
        let key = KeyEvent::new(KeyCode::Char('k'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_preview_key(key));
        assert_eq!(app.preview_scroll, 0);
    }

    #[test]
    fn preview_key_m_converts() {
        let mut app = App::new();
        app.entries = vec![];
        let key = KeyEvent::new(KeyCode::Char('m'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_preview_key(key));
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("No file selected")),
            "should log 'No file selected'"
        );
    }

    #[test]
    fn preview_key_h_converts() {
        let mut app = App::new();
        app.entries = vec![];
        let key = KeyEvent::new(KeyCode::Char('h'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_preview_key(key));
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("No file selected")),
            "should log 'No file selected'"
        );
    }

    #[test]
    fn preview_key_p_converts() {
        let mut app = App::new();
        app.entries = vec![];
        let key = KeyEvent::new(KeyCode::Char('p'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_preview_key(key));
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("No file selected")),
            "should log 'No file selected'"
        );
    }

    #[test]
    fn preview_key_t_converts() {
        let mut app = App::new();
        app.entries = vec![];
        let key = KeyEvent::new(KeyCode::Char('t'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_preview_key(key));
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("No file selected")),
            "should log 'No file selected'"
        );
    }

    #[test]
    fn preview_key_unknown_returns_false() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::Char('x'), crossterm::event::KeyModifiers::NONE);
        assert!(!app.handle_preview_key(key));
    }

    // ======================================================================
    // handle_log_key
    // ======================================================================

    #[test]
    fn log_key_j_scrolls_down() {
        let mut app = App::new();
        app.add_log("msg1".to_string());
        app.add_log("msg2".to_string());
        app.log_scroll = 0;
        let key = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_log_key(key));
        assert_eq!(app.log_scroll, 1);
    }

    #[test]
    fn log_key_down_scrolls_down() {
        let mut app = App::new();
        app.add_log("msg1".to_string());
        app.log_scroll = 0;
        let key = KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_log_key(key));
        assert_eq!(app.log_scroll, 1);
    }

    #[test]
    fn log_key_j_does_not_exceed_bound() {
        let mut app = App::new();
        app.add_log("msg1".to_string());
        app.log_scroll = 1; // last index
        let key = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_log_key(key));
        assert_eq!(app.log_scroll, 1);
    }

    #[test]
    fn log_key_k_scrolls_up() {
        let mut app = App::new();
        app.add_log("msg1".to_string());
        app.add_log("msg2".to_string());
        app.log_scroll = 2;
        let key = KeyEvent::new(KeyCode::Char('k'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_log_key(key));
        assert_eq!(app.log_scroll, 1);
    }

    #[test]
    fn log_key_up_scrolls_up() {
        let mut app = App::new();
        app.add_log("msg1".to_string());
        app.add_log("msg2".to_string());
        app.log_scroll = 2;
        let key = KeyEvent::new(KeyCode::Up, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_log_key(key));
        assert_eq!(app.log_scroll, 1);
    }

    #[test]
    fn log_key_k_does_not_go_below_zero() {
        let mut app = App::new();
        app.log_scroll = 0;
        let key = KeyEvent::new(KeyCode::Char('k'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_log_key(key));
        assert_eq!(app.log_scroll, 0);
    }

    #[test]
    fn log_key_unknown_returns_false() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::Char('x'), crossterm::event::KeyModifiers::NONE);
        assert!(!app.handle_log_key(key));
    }

    // ======================================================================
    // convert_current
    // ======================================================================

    #[test]
    fn convert_current_no_preview_path_logs() {
        let mut app = App::new();
        app.preview_path = None;
        app.entries = vec![]; // ensure no entries to fall back on
        app.convert_current(Format::Html);
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("No file selected")),
            "should warn about no file"
        );
    }

    #[test]
    fn convert_current_same_format_md() {
        let dir = create_test_dir();
        let mut app = App::new();
        app.preview_path = Some(dir.path().join("readme.md"));
        app.convert_current(Format::Markdown);
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("already in Markdown")),
            "should note same format"
        );
    }

    #[test]
    fn convert_current_same_format_html() {
        let dir = create_test_dir();
        let mut app = App::new();
        app.preview_path = Some(dir.path().join("index.html"));
        app.convert_current(Format::Html);
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("already in HTML")),
            "should note same format"
        );
    }

    #[test]
    fn convert_current_same_format_pdf() {
        let dir = create_test_dir();
        let mut app = App::new();
        let pdf_path = dir.path().join("test.pdf");
        fs::write(&pdf_path, "dummy").unwrap();
        app.preview_path = Some(pdf_path);
        app.convert_current(Format::Pdf);
        assert!(
            app.log_messages
                .iter()
                .any(|m| m.contains("already in PDF")),
            "should note same format"
        );
    }

    #[test]
    fn convert_current_different_format_logs_conversion() {
        let dir = create_test_dir();
        let mut app = App::new();
        app.preview_path = Some(dir.path().join("readme.md"));
        // Converting MD → HTML
        app.convert_current(Format::Html);
        // Should either succeed or log a conversion attempt
        let log = app.log_messages.join(" ");
        assert!(
            log.contains("Converting") || log.contains("already") || log.contains("Conversion"),
            "should log conversion activity, got: {log}"
        );
    }

    #[test]
    fn convert_current_refreshes_browser_entries() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        // Preview a markdown file and convert it to HTML
        let md_path = dir.path().join("readme.md");
        let html_path = dir.path().join("readme.html");
        std::fs::remove_file(&html_path).ok(); // ensure it doesn't already exist
        app.preview_path = Some(md_path);
        app.convert_current(Format::Html);
        // The browser should now include the generated HTML file
        assert!(
            app.entries.iter().any(|e| e.path() == html_path),
            "browser should show the generated HTML file after conversion"
        );
    }

    // ======================================================================
    // Enums
    // ======================================================================

    #[test]
    fn mode_values_are_distinct() {
        assert_ne!(Mode::Browse, Mode::Preview);
        assert_ne!(Mode::Browse, Mode::Help);
        assert_ne!(Mode::Preview, Mode::Help);
    }

    #[test]
    fn focus_values_are_distinct() {
        assert_ne!(Focus::Browser, Focus::Preview);
        assert_ne!(Focus::Browser, Focus::Log);
        assert_ne!(Focus::Preview, Focus::Log);
    }

    #[test]
    fn mode_debug_format() {
        let _ = format!("{:?}", Mode::Browse);
        let _ = format!("{:?}", Mode::Preview);
        let _ = format!("{:?}", Mode::Help);
    }

    #[test]
    fn mode_clone_copy() {
        let m = Mode::Browse;
        let _ = m; // Copy
        assert_eq!(m, Mode::Browse);
    }

    // ======================================================================
    // handle_mouse / tick (stubs)
    // ======================================================================

    #[test]
    fn handle_mouse_returns_false_for_non_left_click() {
        let mut app = App::new();
        let event = MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let area = Rect::new(0, 0, 100, 50);
        assert!(!app.handle_mouse(event, area));
    }

    #[test]
    fn handle_mouse_returns_false_for_mouse_up() {
        let mut app = App::new();
        let event = MouseEvent {
            kind: crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let area = Rect::new(0, 0, 100, 50);
        assert!(!app.handle_mouse(event, area));
    }

    #[test]
    fn tick_does_nothing() {
        let mut app = App::new();
        app.tick();
        // Should not panic, no observable state change
        assert!(app.running);
    }

    // ======================================================================
    // Settings / cycle_theme
    // ======================================================================

    #[test]
    fn cycle_theme_cycles_through_all_themes() {
        let mut app = App::new();
        // Reset to known state regardless of on-disk config
        app.theme_name = "dark".to_string();
        app.theme = Theme::dark();
        let themes = ["dark", "light", "dracula", "solarized"];
        for expected in themes {
            assert_eq!(app.theme_name, expected, "expected theme {expected}");
            app.cycle_theme();
        }
        // After cycling through all 4, we should be back to dark
        assert_eq!(app.theme_name, "dark");
    }

    #[test]
    fn cycle_theme_wraps_around() {
        let mut app = App::new();
        // Reset to known state regardless of on-disk config
        app.theme_name = "dark".to_string();
        app.theme = Theme::dark();
        // Cycle 5 times — should be back to "light" (index 1)
        for _ in 0..5 {
            app.cycle_theme();
        }
        assert_eq!(app.theme_name, "light");
    }

    #[test]
    fn cycle_theme_logs_new_theme() {
        let mut app = App::new();
        // Reset to known state regardless of on-disk config
        app.theme_name = "dark".to_string();
        app.theme = Theme::dark();
        app.cycle_theme();
        assert!(
            app.log_messages.iter().any(|m| m.contains("Theme: light")),
            "cycle_theme should log the new theme"
        );
    }

    #[test]
    fn handle_key_s_enters_settings_mode() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::Char('s'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.mode, Mode::Settings);
    }

    #[test]
    fn settings_esc_returns_to_browse() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        let key = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(app.mode, Mode::Browse);
    }

    #[test]
    fn settings_s_cycles_theme() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        let start = app.theme_name.clone();
        let key = KeyEvent::new(KeyCode::Char('s'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_ne!(
            app.theme_name, start,
            "cycling theme should change theme_name"
        );
    }

    #[test]
    fn settings_tab_cycles_theme() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        let start = app.theme_name.clone();
        let key = KeyEvent::new(KeyCode::Tab, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_ne!(app.theme_name, start, "Tab in settings should cycle theme");
    }

    #[test]
    fn settings_q_quits() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        let key = KeyEvent::new(KeyCode::Char('q'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert!(!app.running, "q in settings should quit the app");
    }

    #[test]
    fn settings_unknown_key_does_not_exit() {
        let mut app = App::new();
        app.mode = Mode::Settings;
        let key = KeyEvent::new(KeyCode::Char('x'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_key(key));
        assert_eq!(
            app.mode,
            Mode::Settings,
            "unknown key should stay in settings"
        );
    }
}
