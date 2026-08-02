use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;

use crate::config::{Settings, load_settings, save_settings};
use crate::converter::{Format, convert};
use crate::fs_utils::{FsEntry, list_entries};
use crate::preview::preview_file;

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
    pub selected_index: usize,
    pub preview_content: String,
    pub preview_path: Option<PathBuf>,
    pub log_messages: Vec<String>,
    pub preview_scroll: u16,
    pub log_scroll: u16,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let settings = load_settings();
        let current_dir = settings
            .last_directory
            .filter(|p| p.exists())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let entries = list_entries(&current_dir).unwrap_or_default();
        Self {
            mode: Mode::Browse,
            focus: Focus::Browser,
            running: true,
            current_dir,
            entries,
            selected_index: 0,
            preview_content: String::new(),
            preview_path: None,
            log_messages: vec!["Ready".to_string()],
            preview_scroll: 0,
            log_scroll: 0,
        }
    }

    /// Navigate to a directory and reload entries.
    pub fn navigate_to(&mut self, path: &Path) {
        if let Ok(entries) = list_entries(path) {
            self.current_dir = path.to_path_buf();
            self.entries = entries;
            self.selected_index = 0;
            self.preview_content.clear();
            self.preview_path = None;
            self.preview_scroll = 0;
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
            self.selected_index = (self.selected_index + 1).min(self.entries.len() - 1);
        }
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Enter the selected directory, or preview the selected file.
    pub fn activate_selection(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let path = self.entries[self.selected_index].path().to_path_buf();
        let is_dir = matches!(
            &self.entries[self.selected_index],
            FsEntry::Directory { .. }
        );

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
        } else if !self.entries.is_empty() {
            let entry_index = if has_parent {
                self.selected_index.saturating_sub(1)
            } else {
                self.selected_index
            };
            if entry_index < self.entries.len()
                && let FsEntry::Directory { .. } = &self.entries[entry_index]
            {
                self.activate_selection();
            }
        }
    }

    /// Add a message to the log panel.
    pub fn add_log(&mut self, msg: String) {
        self.log_messages.push(msg);
        self.log_scroll = self.log_messages.len().saturating_sub(1) as u16;
    }

    /// Convert the currently previewed file to the given format.
    pub fn convert_current(&mut self, target: Format) {
        let input = match &self.preview_path {
            Some(p) => p.clone(),
            None => {
                self.add_log("No file selected for conversion".to_string());
                return;
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

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.running = false;
                true
            }
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
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
            _ => false,
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
            _ => false,
        }
    }

    /// Handle a mouse event. Returns true if the event was consumed.
    #[allow(dead_code)]
    pub fn handle_mouse(&mut self, _event: MouseEvent) -> bool {
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
        App {
            mode: Mode::Browse,
            focus: Focus::Browser,
            running: true,
            current_dir: PathBuf::from("/test"),
            entries,
            selected_index: 0,
            preview_content: String::new(),
            preview_path: None,
            log_messages: vec!["Ready".to_string()],
            preview_scroll: 0,
            log_scroll: 0,
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
        let entries = list_entries(&path).unwrap_or_default();
        App {
            mode: Mode::Browse,
            focus: Focus::Browser,
            running: true,
            current_dir: path,
            entries,
            selected_index: 0,
            preview_content: String::new(),
            preview_path: None,
            log_messages: vec!["Ready".to_string()],
            preview_scroll: 0,
            log_scroll: 0,
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
        assert_eq!(a.current_dir, b.current_dir);
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
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
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
        }]);
        app.select_next(); // len-1 == 0
        assert_eq!(app.selected_index, 0);
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
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
            },
            FsEntry::File {
                path: PathBuf::from("c.md"),
                name: "c.md".into(),
                extension: "md".into(),
            },
        ]);
        app.select_next();
        app.select_next();
        assert_eq!(app.selected_index, 2);
        app.select_next(); // at bound
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn select_prev_decrements() {
        let mut app = app_with_entries(vec![
            FsEntry::File {
                path: PathBuf::from("a.md"),
                name: "a.md".into(),
                extension: "md".into(),
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
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
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
            },
            FsEntry::File {
                path: PathBuf::from("c.md"),
                name: "c.md".into(),
                extension: "md".into(),
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
        app.selected_index = idx;
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
        app.selected_index = idx;
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
        app.selected_index = idx;
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
        // enter_child_directory treats selected_index 0 as ".." (parent)
        // when has_parent is true. selected_index=1 maps to entries[1]
        // (activate_selection uses the raw index).
        let second_entry_name = app.entries[1].name().to_string();
        app.selected_index = 1;
        app.enter_child_directory();
        assert_eq!(
            app.current_dir,
            dir.path().join(&second_entry_name),
            "should enter entries[1] via selected_index=1"
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
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
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
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
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
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
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
            },
            FsEntry::File {
                path: PathBuf::from("b.md"),
                name: "b.md".into(),
                extension: "md".into(),
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
        app.selected_index = idx;
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
        let key = KeyEvent::new(KeyCode::Char('h'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(app.current_dir, dir.path());
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
        // enter_child_directory treats index 0 as ".." when has_parent=true.
        // selected_index=1 → entries[1] via activate_selection.
        let entry_name = app.entries[1].name().to_string();
        app.selected_index = 1;
        let key = KeyEvent::new(KeyCode::Char('l'), crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(
            app.current_dir,
            dir.path().join(&entry_name),
            "'l' should enter the entry at selected_index=1"
        );
    }

    #[test]
    fn browser_key_right_enters_child() {
        let dir = create_test_dir();
        let mut app = app_in_temp_dir(&dir);
        let entry_name = app.entries[1].name().to_string();
        app.selected_index = 1;
        let key = KeyEvent::new(KeyCode::Right, crossterm::event::KeyModifiers::NONE);
        assert!(app.handle_browser_key(key));
        assert_eq!(
            app.current_dir,
            dir.path().join(&entry_name),
            "'right' should enter the entry at selected_index=1"
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
    fn handle_mouse_returns_false() {
        let mut app = App::new();
        let event = MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert!(!app.handle_mouse(event));
    }

    #[test]
    fn tick_does_nothing() {
        let mut app = App::new();
        app.tick();
        // Should not panic, no observable state change
        assert!(app.running);
    }
}
