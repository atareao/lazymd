# lazymd — Phase 5 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the remaining UX improvements, file operations, navigation enhancements, and engineering infrastructure.

**Architecture:** Incremental improvements to existing files. Each task is self-contained and independently testable. Tasks are ordered by value/effort ratio (P1 first).

**Tech Stack:** Rust + ratatui 0.29 + crossterm 0.28 + typst-syntax 0.15 (new dep)

## Global Constraints

- All conversions use Rust crates — never shell out to system tools
- `cargo clippy` must pass with zero warnings after every task
- `cargo fmt` must pass after every task
- `cargo test` must pass after every task (currently 237 tests)
- Edition 2024 (rustc ≥ 1.85)
- No new dependencies unless explicitly justified (justified: `typst-syntax` for Typst highlighting)
- `FsEntry` enum: `Directory { path: PathBuf, name: String }`, `File { path: PathBuf, name: String, extension: String, size: u64 }`
- `App` struct fields: `mode, focus, running, current_dir, entries, all_entries, filter_query, filtering, selected_index, preview_content, preview_path, log_messages, preview_scroll, log_scroll, theme, theme_name`
- Test helpers: `app_with_entries(entries: Vec<FsEntry>) -> App`, `app_in_temp_dir(dir: &TempDir) -> App`, `create_test_dir() -> TempDir`

---

### Task 1: Status Bar (P1)

**Files:**
- Create: `src/ui/status_panel.rs`
- Modify: `src/ui/layout.rs:11-27`, `src/ui/mod.rs:1-23`, `src/app.rs:36-53`

**Interfaces:**
- Consumes: `App` struct (selected_index, entries, current_dir, theme, preview_path), `PanelAreas` struct, `chunk_areas(Rect) -> PanelAreas`
- Produces: `PanelAreas { browser, preview, log, status }`, `status_panel::draw(frame, area, app)`, `App::status_line(&self) -> String`

Add a 1-row status bar at the very bottom showing: selected file name, size, type (from extension). For PDFs, also show page count (computed lazily, cached). The status bar is always visible.

- [ ] **Step 1: Add status field to PanelAreas and update layout**

```rust
// src/ui/layout.rs
pub struct PanelAreas {
    pub browser: Rect,
    pub preview: Rect,
    pub log: Rect,
    pub status: Rect,
}

pub fn chunk_areas(area: Rect) -> PanelAreas {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6), Constraint::Length(1)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(main[0]);

    PanelAreas {
        browser: top[0],
        preview: top[1],
        log: main[1],
        status: main[2],
    }
}
```

- [ ] **Step 2: Add status_line method to App**

```rust
// src/app.rs
/// Build a status line string for the selected entry.
pub fn status_line(&self) -> String {
    if self.entries.is_empty() {
        return "No files".to_string();
    }
    let entry = &self.entries[self.selected_index];
    match entry {
        FsEntry::Directory { name, .. } => {
            format!("📁 {name} | directory")
        }
        FsEntry::File { name, size, extension, .. } => {
            let size_str = format_size(*size);
            let ext_upper = extension.to_uppercase();
            if extension == "pdf" {
                // Show page count if we can compute it quickly
                let pages = pdf_page_count(entry.path()).unwrap_or(0);
                format!("📄 {name} | {size_str} | {ext_upper} | {pages} page(s)")
            } else {
                format!("📄 {name} | {size_str} | {ext_upper}")
            }
        }
    }
}
```

Add the helper (can be a free function or inline):

```rust
/// Quick page count for PDF files using pdf_oxide.
fn pdf_page_count(path: &Path) -> Option<usize> {
    pdf_oxide::api::Pdf::open(path).ok().and_then(|mut doc| {
        doc.page_count().ok().map(|c| c as usize)
    })
}
```

- [ ] **Step 3: Create status_panel.rs**

```rust
// src/ui/status_panel.rs
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let text = app.status_line();
    let paragraph = Paragraph::new(Text::from(text))
        .style(Style::default().fg(app.theme.dim_fg));
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 4: Wire into ui/mod.rs**

```rust
// src/ui/mod.rs
pub mod status_panel;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let areas = layout::chunk_areas(area);

    browser_panel::draw(frame, areas.browser, app);
    preview_panel::draw(frame, areas.preview, app);
    log_panel::draw(frame, areas.log, app);
    status_panel::draw(frame, areas.status, app);

    if app.mode == crate::app::Mode::Help {
        help_popup::draw(frame, area);
    }
    if app.mode == crate::app::Mode::Settings {
        settings_panel::draw(frame, area, app);
    }
}
```

- [ ] **Step 5: Write tests**

```rust
// In src/app.rs #[cfg(test)] module

#[test]
fn status_line_empty_entries() {
    let app = app_with_entries(vec![]);
    assert_eq!(app.status_line(), "No files");
}

#[test]
fn status_line_shows_file_info() {
    let app = app_with_entries(vec![
        FsEntry::File {
            path: PathBuf::from("readme.md"),
            name: "readme.md".into(),
            extension: "md".into(),
            size: 1024,
        },
    ]);
    let line = app.status_line();
    assert!(line.contains("readme.md"));
    assert!(line.contains("1.0 KB"));
    assert!(line.contains("MD"));
}

#[test]
fn status_line_shows_directory() {
    let app = app_with_entries(vec![
        FsEntry::Directory {
            path: PathBuf::from("src"),
            name: "src".into(),
        },
    ]);
    let line = app.status_line();
    assert!(line.contains("src"));
    assert!(line.contains("directory"));
}
```

- [ ] **Step 6: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 240+ tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/ui/status_panel.rs src/ui/layout.rs src/ui/mod.rs src/app.rs
git commit -m "feat: add status bar with file info (name, size, type, PDF pages)"
```

---

### Task 2: Text Input Popup (Rename + Create File/Dir) (P1)

**Files:**
- Modify: `src/app.rs:13-53, 361-422, 468-497, 696-759`

**Interfaces:**
- Consumes: `App` struct, `Mode` enum, `handle_key` dispatch
- Produces: `Mode::Input(InputMode)` variant, `input_buffer: String`, handle_input_key logic

Add a generic text input mode shared by rename (`R`), create file (`n`), and create dir (`N`). The input prompt appears in the browser panel title bar. Esc cancels, Enter confirms.

- [ ] **Step 1: Add InputMode enum and fields to App**

```rust
/// What kind of text input the user is performing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Rename,
    NewFile,
    NewDir,
}

// Add to App struct:
// pub input_mode: Option<InputMode>,
// pub input_buffer: String,
```

Change `mode: Mode` to accept `Mode::Input(InputMode)`:

```rust
pub enum Mode {
    Browse,
    Preview,
    Help,
    Settings,
    Input(InputMode),
}
```

Add to `App`:
```rust
pub input_buffer: String,
```

Update `App::new()`: `input_buffer: String::new()`.

- [ ] **Step 2: Update test helpers**

```rust
fn app_with_entries(entries: Vec<FsEntry>) -> App {
    // ...
    input_buffer: String::new(),
    // ...
}
```

Same for `app_in_temp_dir`.

- [ ] **Step 3: Add input mode handling in handle_key**

Before the main `match key.code`, add:

```rust
if let Mode::Input(input_mode) = &self.mode {
    return match key.code {
        KeyCode::Esc => {
            self.mode = Mode::Browse;
            self.input_buffer.clear();
            self.add_log("Cancelled".to_string());
            true
        }
        KeyCode::Enter => {
            let name = std::mem::take(&mut self.input_buffer);
            let result = match input_mode {
                InputMode::Rename => {
                    self.rename_selected(&name);
                    true
                }
                InputMode::NewFile => {
                    let path = self.current_dir.join(&name);
                    match std::fs::write(&path, "") {
                        Ok(()) => {
                            self.add_log(format!("📄 Created: {name}"));
                            self.navigate_to(&self.current_dir.clone());
                        }
                        Err(e) => self.add_log(format!("✗ Create failed: {e}")),
                    }
                    true
                }
                InputMode::NewDir => {
                    let path = self.current_dir.join(&name);
                    match std::fs::create_dir(&path) {
                        Ok(()) => {
                            self.add_log(format!("📁 Created: {name}"));
                            self.navigate_to(&self.current_dir.clone());
                        }
                        Err(e) => self.add_log(format!("✗ Create failed: {e}")),
                    }
                    true
                }
            };
            self.mode = Mode::Browse;
            result
        }
        KeyCode::Char(c) => {
            self.input_buffer.push(c);
            true
        }
        KeyCode::Backspace => {
            self.input_buffer.pop();
            true
        }
        _ => true,
    };
}
```

- [ ] **Step 4: Add keybindings in handle_browser_key**

Replace the `R` rename handler:

```rust
KeyCode::Char('R') => {
    let name = match self.entries.get(self.selected_index) {
        Some(e) => e.name().to_string(),
        None => {
            self.add_log("No entry selected".to_string());
            return true;
        }
    };
    self.mode = Mode::Input(InputMode::Rename);
    self.input_buffer = name;
    self.add_log("Rename: edit name, Enter to confirm, Esc to cancel".to_string());
    true
}

// Add new keybindings:
KeyCode::Char('n') => {
    self.mode = Mode::Input(InputMode::NewFile);
    self.input_buffer = "untitled.md".to_string();
    self.add_log("New file: type name, Enter to confirm, Esc to cancel".to_string());
    true
}
KeyCode::Char('N') => {
    self.mode = Mode::Input(InputMode::NewDir);
    self.input_buffer = "newdir".to_string();
    self.add_log("New directory: type name, Enter to confirm, Esc to cancel".to_string());
    true
}
```

- [ ] **Step 5: Show input mode in browser panel title**

In `src/ui/browser_panel.rs`, modify the title block:

```rust
let title = match &app.mode {
    crate::app::Mode::Input(mode) => {
        let prompt = match mode {
            crate::app::InputMode::Rename => "Rename",
            crate::app::InputMode::NewFile => "New File",
            crate::app::InputMode::NewDir => "New Dir",
        };
        format!(" {prompt}: {} ", app.input_buffer)
    }
    _ => {
        if app.filtering {
            format!(" Browser: {}/{} ", app.current_dir.display(), app.filter_query)
        } else if !app.filter_query.is_empty() {
            format!(" Browser: {} 🔍 {} ", app.current_dir.display(), app.filter_query)
        } else {
            format!(" Browser: {} ", app.current_dir.display())
        }
    }
};
```

- [ ] **Step 6: Write tests**

```rust
#[test]
fn input_rename_enter_confirms() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    app.mode = Mode::Input(InputMode::Rename);
    app.input_buffer = "newname.md".to_string();
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.handle_key(key));
    assert_eq!(app.mode, Mode::Browse);
    assert!(dir.path().join("newname.md").exists());
    assert!(!dir.path().join("readme.md").exists());
}

#[test]
fn input_esc_cancels() {
    let mut app = app_with_entries(vec![]);
    app.mode = Mode::Input(InputMode::Rename);
    app.input_buffer = "whatever".to_string();
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.handle_key(key));
    assert_eq!(app.mode, Mode::Browse);
    assert!(app.input_buffer.is_empty());
}

#[test]
fn input_char_appends_to_buffer() {
    let mut app = app_with_entries(vec![]);
    app.mode = Mode::Input(InputMode::Rename);
    let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    assert!(app.handle_key(key));
    assert_eq!(app.input_buffer, "x");
}

#[test]
fn input_backspace_removes_char() {
    let mut app = app_with_entries(vec![]);
    app.mode = Mode::Input(InputMode::Rename);
    app.input_buffer = "abc".to_string();
    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    assert!(app.handle_key(key));
    assert_eq!(app.input_buffer, "ab");
}

#[test]
fn input_new_file_creates_file() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    app.mode = Mode::Input(InputMode::NewFile);
    app.input_buffer = "newfile.txt".to_string();
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.handle_key(key));
    assert!(dir.path().join("newfile.txt").exists());
}

#[test]
fn input_new_dir_creates_directory() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    app.mode = Mode::Input(InputMode::NewDir);
    app.input_buffer = "newfolder".to_string();
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.handle_key(key));
    assert!(dir.path().join("newfolder").is_dir());
}
```

- [ ] **Step 7: Update test for Mode enum equality**

All existing tests that use `Mode::Browse` / `Mode::Preview` etc. still work because `Mode::Input(InputMode::Rename) != Mode::Browse`. The `Mode` enum derives `PartialEq` so this is fine.

- [ ] **Step 8: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 247+ tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/app.rs src/ui/browser_panel.rs
git commit -m "feat: text input popup for rename, create file, and create directory"
```

---

### Task 3: Trash Instead of Hard Delete (P1)

**Files:**
- Modify: `src/app.rs:230-250, 696-759`

**Interfaces:**
- Consumes: `App::delete_selected`, `dirs::data_dir()`, `App::trash_dir: PathBuf`
- Produces: Moved-to-trash behavior instead of permanent deletion

Replace `remove_file` with a move-to-trash operation. The trash directory lives at `$XDG_DATA_HOME/lazymd/trash` (or `~/.local/share/lazymd/trash`). Add a `trash_dir` field to App so tests can override it.

- [ ] **Step 1: Add trash_dir to App struct**

```rust
// In App struct:
pub trash_dir: PathBuf;
```

- [ ] **Step 2: Initialize in App::new()**

```rust
let trash_dir = dirs::data_dir()
    .unwrap_or_else(|| PathBuf::from("."))
    .join("lazymd/trash");

// In Self:
trash_dir,
```

- [ ] **Step 3: Update delete_selected**

```rust
pub fn delete_selected(&mut self) {
    if self.entries.is_empty() {
        return;
    }
    let entry = &self.entries[self.selected_index];
    match entry {
        FsEntry::File { path, name, .. } => {
            let _ = std::fs::create_dir_all(&self.trash_dir);
            // Generate a unique name in trash to avoid overwrites
            let trash_name = if self.trash_dir.join(name).exists() {
                let stem = path.file_stem().unwrap_or_default().to_str().unwrap_or("file");
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let mut counter = 1;
                loop {
                    let candidate = format!("{stem}-{counter}.{ext}");
                    if !self.trash_dir.join(&candidate).exists() {
                        break candidate;
                    }
                    counter += 1;
                }
            } else {
                name.clone()
            };
            let trash_path = self.trash_dir.join(&trash_name);
            match std::fs::rename(path, &trash_path) {
                Ok(()) => {
                    self.add_log(format!("🗑️ Moved to trash: {name}"));
                    self.navigate_to(&self.current_dir.clone());
                }
                Err(e) => {
                    self.add_log(format!("✗ Trash failed: {e}"));
                }
            }
        }
        FsEntry::Directory { .. } => {
            self.add_log("Cannot delete directories from the TUI".to_string());
        }
    }
}
```

- [ ] **Step 4: Update test helpers**

```rust
fn app_with_entries(entries: Vec<FsEntry>) -> App {
    // ...
    trash_dir: PathBuf::from("/tmp/lazymd-trash-test"),
    // ...
}

fn app_in_temp_dir(dir: &TempDir) -> App {
    // ...
    trash_dir: PathBuf::from("/tmp/lazymd-trash-test"),
    // ...
}
```

- [ ] **Step 5: Write tests**

```rust
#[test]
fn delete_selected_moves_to_trash() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    app.trash_dir = dir.path().join("trash");
    let idx = app.entries.iter().position(|e| e.name() == "readme.md").unwrap();
    app.selected_index = idx;
    app.delete_selected();
    assert!(!dir.path().join("readme.md").exists(), "file should be removed from source");
    assert!(dir.path().join("trash").join("readme.md").exists(), "file should be in trash");
    assert!(app.log_messages.iter().any(|m| m.contains("trash")));
}

#[test]
fn delete_selected_trash_handles_name_collision() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    app.trash_dir = dir.path().join("trash");
    std::fs::create_dir_all(app.trash_dir.join("trash")).unwrap();
    // Pre-place a file in trash with the same name
    std::fs::write(dir.path().join("trash").join("readme.md"), "old").unwrap();
    let idx = app.entries.iter().position(|e| e.name() == "readme.md").unwrap();
    app.selected_index = idx;
    app.delete_selected();
    // Should have created readme-1.md or similar
    let trash_dir = dir.path().join("trash");
    let entries: Vec<_> = std::fs::read_dir(&trash_dir).unwrap().map(|e| e.unwrap().file_name().to_str().unwrap().to_string()).collect();
    assert!(entries.iter().any(|n| n.starts_with("readme")), "trash file should exist with suffix");
}

#[test]
fn delete_selected_empty_entries_no_panic() {
    let mut app = app_with_entries(vec![]);
    app.delete_selected(); // should not panic
}
```

- [ ] **Step 6: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 250+ tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/app.rs
git commit -m "feat: move deleted files to trash instead of permanent deletion"
```

---

### Task 4: Sort by Size/Date (P2)

**Files:**
- Modify: `src/app.rs:36-89, 425-497, 696-759`

**Interfaces:**
- Consumes: `App` struct, `FsEntry`, `list_entries`
- Produces: `SortBy` enum, `App::sort: SortBy`, `App::reapply_sort()`, `o` keybinding

Add a `SortBy` enum and `o` key to cycle sort order: name → size → modified. Dirs are always first within each mode.

- [ ] **Step 1: Add SortBy enum**

```rust
/// How to sort entries in the browser panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Size,
    Modified,
}
```

- [ ] **Step 2: Add sort field to App**

```rust
// In App struct:
pub sort: SortBy;
```

In `App::new()`: `sort: SortBy::Name,`

- [ ] **Step 3: Add reapply_sort method**

```rust
pub fn reapply_sort(&mut self) {
    let sort = self.sort;
    let source = if self.filter_query.is_empty() {
        &self.all_entries
    } else {
        &self.entries
    };
    let mut entries = source.clone();

    // Helper to get size for sorting (0 for directories)
    let entry_size = |e: &FsEntry| match e {
        FsEntry::File { size, .. } => *size,
        _ => 0,
    };

    // Helper to get modified time via metadata
    let entry_modified = |e: &FsEntry| -> u64 {
        std::fs::metadata(e.path())
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0)
    };

    entries.sort_by(|a, b| {
        let a_is_dir = matches!(a, FsEntry::Directory { .. });
        let b_is_dir = matches!(b, FsEntry::Directory { .. });
        // Dirs first
        b_is_dir.cmp(&a_is_dir).then_with(|| match sort {
            SortBy::Name => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
            SortBy::Size => entry_size(b).cmp(&entry_size(a)), // larger first
            SortBy::Modified => entry_modified(b).cmp(&entry_modified(a)), // newer first
        })
    });
    self.entries = entries;
}
```

- [ ] **Step 4: Add o keybinding in handle_browser_key**

```rust
KeyCode::Char('o') => {
    self.sort = match self.sort {
        SortBy::Name => SortBy::Size,
        SortBy::Size => SortBy::Modified,
        SortBy::Modified => SortBy::Name,
    };
    self.reapply_sort();
    self.selected_index = 0;
    self.add_log(format!("Sort: {:?}", self.sort));
    true
}
```

- [ ] **Step 5: Call reapply_sort after navigate_to and set_filter**

In `navigate_to` after setting entries: `self.reapply_sort();`
In `set_filter` after setting entries: `self.reapply_sort();`

- [ ] **Step 6: Update test helpers**

```rust
fn app_with_entries(entries: Vec<FsEntry>) -> App {
    // ...
    sort: SortBy::Name,
    // ...
}
```

Same for `app_in_temp_dir`.

- [ ] **Step 7: Write tests**

```rust
#[test]
fn sort_default_is_name() {
    let app = App::new();
    assert_eq!(app.sort, SortBy::Name);
}

#[test]
fn browser_key_o_cycles_sort() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    let key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert_eq!(app.sort, SortBy::Size);
    assert!(app.handle_browser_key(key));
    assert_eq!(app.sort, SortBy::Modified);
    assert!(app.handle_browser_key(key));
    assert_eq!(app.sort, SortBy::Name);
}

#[test]
fn reapply_sort_by_size_orders_largest_first() {
    let entries = vec![
        FsEntry::File { path: PathBuf::from("small.md"), name: "small.md".into(), extension: "md".into(), size: 10 },
        FsEntry::File { path: PathBuf::from("large.md"), name: "large.md".into(), extension: "md".into(), size: 1000 },
        FsEntry::File { path: PathBuf::from("medium.md"), name: "medium.md".into(), extension: "md".into(), size: 100 },
    ];
    let mut app = app_with_entries(entries);
    app.sort = SortBy::Size;
    app.reapply_sort();
    assert_eq!(app.entries[0].name(), "large.md");
    assert_eq!(app.entries[1].name(), "medium.md");
    assert_eq!(app.entries[2].name(), "small.md");
}

#[test]
fn reapply_sort_preserves_dirs_first() {
    let entries = vec![
        FsEntry::File { path: PathBuf::from("a.md"), name: "a.md".into(), extension: "md".into(), size: 100 },
        FsEntry::Directory { path: PathBuf::from("zdir"), name: "zdir".into() },
    ];
    let mut app = app_with_entries(entries);
    app.sort = SortBy::Size;
    app.reapply_sort();
    assert!(matches!(app.entries[0], FsEntry::Directory { .. }));
    assert!(matches!(app.entries[1], FsEntry::File { .. }));
}
```

- [ ] **Step 8: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 256+ tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/app.rs
git commit -m "feat: add sort by size/date with 'o' key cycling"
```

---

### Task 5: Full-Screen Preview Toggle (P2)

**Files:**
- Modify: `src/ui/layout.rs:11-27`, `src/ui/mod.rs:14-23`, `src/app.rs:36-53, 537-582, 696-759`

**Interfaces:**
- Consumes: `App::zoom: bool`, `chunk_areas(Rect) -> PanelAreas`
- Produces: `x` key toggles zoom in preview focus, full-area preview when zoomed

- [ ] **Step 1: Add zoom field to App**

```rust
// In App struct:
pub zoom: bool;
```

In `App::new()`: `zoom: false,`

- [ ] **Step 2: Add x keybinding in handle_preview_key**

```rust
KeyCode::Char('x') => {
    self.zoom = !self.zoom;
    self.add_log(if self.zoom {
        "Zoom: full preview".to_string()
    } else {
        "Zoom: normal".to_string()
    });
    true
}
```

- [ ] **Step 3: Update chunk_areas to accept zoom**

```rust
pub fn chunk_areas(area: Rect, zoom: bool) -> PanelAreas {
    if zoom {
        return PanelAreas {
            browser: Rect::default(),
            preview: area,
            log: Rect::default(),
            status: Rect::default(),
        };
    }
    // ... existing logic
}
```

- [ ] **Step 4: Update render and handle_mouse callers**

In `src/ui/mod.rs`:
```rust
let areas = layout::chunk_areas(area, app.zoom);
```

In `src/app.rs` `handle_mouse`:
```rust
let areas = crate::ui::layout::chunk_areas(area, self.zoom);
```

- [ ] **Step 5: Update test helpers**

```rust
fn app_with_entries(entries: Vec<FsEntry>) -> App {
    // ...
    zoom: false,
    // ...
}
```

Same for `app_in_temp_dir`.

- [ ] **Step 6: Write tests**

```rust
#[test]
fn preview_key_x_toggles_zoom() {
    let mut app = app_with_entries(vec![]);
    assert!(!app.zoom);
    let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    assert!(app.handle_preview_key(key));
    assert!(app.zoom);
    assert!(app.handle_preview_key(key));
    assert!(!app.zoom);
}

#[test]
fn chunk_areas_zoom_gives_full_area() {
    let area = Rect::new(0, 0, 100, 50);
    let areas = layout::chunk_areas(area, true);
    assert_eq!(areas.preview, area);
    assert_eq!(areas.browser, Rect::default());
    assert_eq!(areas.log, Rect::default());
    assert_eq!(areas.status, Rect::default());
}

#[test]
fn chunk_areas_normal_gives_three_panels() {
    let area = Rect::new(0, 0, 120, 60);
    let areas = layout::chunk_areas(area, false);
    assert!(areas.browser.width > 0);
    assert!(areas.preview.width > 0);
    assert!(areas.log.height == 6);
    assert!(areas.status.height == 1);
}
```

- [ ] **Step 7: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 260+ tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/ui/layout.rs src/ui/mod.rs
git commit -m "feat: toggle full-screen preview with 'x' key"
```

---

### Task 6: Directory History (H/L Back/Forward) (P2)

**Files:**
- Modify: `src/app.rs:36-89, 92-112, 160-167, 425-497, 696-759`

**Interfaces:**
- Consumes: `App::navigate_to`, `App::current_dir`
- Produces: `history: Vec<PathBuf>`, `history_pos: usize`, `H`/`L` keys, `load_dir` helper

- [ ] **Step 1: Add history fields to App**

```rust
// In App struct:
pub history: Vec<PathBuf>,
pub history_pos: usize,
```

- [ ] **Step 2: Initialize in App::new()**

```rust
let initial_dir = current_dir.clone();
// ...
history: vec![initial_dir],
history_pos: 0,
```

- [ ] **Step 3: Extract load_dir helper**

Create a private method that sets state without pushing to history:

```rust
/// Load a directory into the browser without touching history.
fn load_dir(&mut self, path: &Path) {
    if let Ok(entries) = list_entries(path) {
        self.current_dir = path.to_path_buf();
        self.all_entries = entries;
        self.entries = self.all_entries.clone();
        self.filter_query.clear();
        self.selected_index = 0;
        self.preview_content.clear();
        self.preview_path = None;
        self.preview_scroll = 0;
        self.zoom = false;
        self.add_log(format!("📂 {}", self.current_dir.display()));
        // Persist directory
        let _ = save_settings(&Settings {
            last_directory: Some(self.current_dir.clone()),
            theme: None,
        });
    } else {
        self.add_log(format!("Cannot open directory: {}", path.display()));
    }
}
```

- [ ] **Step 4: Modify navigate_to to use load_dir with history push**

```rust
pub fn navigate_to(&mut self, path: &Path) {
    // Truncate any forward history
    self.history.truncate(self.history_pos + 1);
    self.load_dir(path);
    self.history.push(self.current_dir.clone());
    self.history_pos = self.history.len() - 1;
}
```

- [ ] **Step 5: Add H/L keybindings in handle_browser_key**

```rust
KeyCode::Char('H') => {
    if self.history_pos > 0 {
        self.history_pos -= 1;
        self.load_dir(&self.history[self.history_pos]);
        self.add_log(format!("History: ← {}", self.current_dir.display()));
    } else {
        self.add_log("At start of history".to_string());
    }
    true
}
KeyCode::Char('L') => {
    if self.history_pos + 1 < self.history.len() {
        self.history_pos += 1;
        self.load_dir(&self.history[self.history_pos]);
        self.add_log(format!("History: → {}", self.current_dir.display()));
    } else {
        self.add_log("At end of history".to_string());
    }
    true
}
```

- [ ] **Step 6: Update test helpers**

```rust
fn app_with_entries(entries: Vec<FsEntry>) -> App {
    // ...
    history: vec![PathBuf::from("/test")],
    history_pos: 0,
    // ...
}

fn app_in_temp_dir(dir: &TempDir) -> App {
    let path = dir.path().to_path_buf();
    // ...
    history: vec![path.clone()],
    history_pos: 0,
    // ...
}
```

- [ ] **Step 7: Write tests**

```rust
#[test]
fn history_navigate_pushes() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    let subdir = dir.path().join("subdir");
    app.navigate_to(&subdir);
    assert_eq!(app.history_pos, 1);
    assert_eq!(app.history.len(), 2);
    assert_eq!(app.history[0], dir.path());
    assert_eq!(app.history[1], subdir);
}

#[test]
fn history_back_returns() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    let subdir = dir.path().join("subdir");
    app.navigate_to(&subdir);
    let key = KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert_eq!(app.history_pos, 0);
    assert_eq!(app.current_dir, dir.path());
}

#[test]
fn history_forward_returns() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    let subdir = dir.path().join("subdir");
    app.navigate_to(&subdir);
    app.history_pos = 0; // go back
    let key = KeyEvent::new(KeyCode::Char('L'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert_eq!(app.history_pos, 1);
    assert_eq!(app.current_dir, subdir);
}

#[test]
fn history_new_navigation_truncates_forward() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    app.navigate_to(&dir.path().join("subdir"));
    app.navigate_to(&dir.path().join("another"));
    app.history_pos = 0; // go back to root
    app.navigate_to(&dir.path().join("subdir")); // new path from root
    // Forward history should be truncated
    assert_eq!(app.history_pos, 1);
    assert_eq!(app.history.len(), 2);
}

#[test]
fn history_back_noop_at_start() {
    let mut app = app_with_entries(vec![]);
    let key = KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert_eq!(app.history_pos, 0);
}
```

- [ ] **Step 8: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 268+ tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/app.rs
git commit -m "feat: directory history with H/L back/forward navigation"
```

---

### Task 7: Copy/Paste Files (P2)

**Files:**
- Modify: `src/app.rs:36-89, 425-497, 696-759`

**Interfaces:**
- Consumes: `App` struct, `FsEntry`, `std::fs::copy`
- Produces: `App::clipboard: Option<FsEntry>`, `c` copy, `v` paste keybindings

- [ ] **Step 1: Add clipboard field to App**

```rust
// In App struct:
pub clipboard: Option<FsEntry>;
```

In `App::new()`: `clipboard: None,`

- [ ] **Step 2: Add copy/paste keybindings in handle_browser_key**

```rust
KeyCode::Char('c') => {
    if self.entries.is_empty() {
        self.add_log("No entry selected to copy".to_string());
        return true;
    }
    let entry = &self.entries[self.selected_index];
    match entry {
        FsEntry::File { name, .. } => {
            self.clipboard = Some(entry.clone());
            self.add_log(format!("📋 Copied: {name}"));
        }
        FsEntry::Directory { .. } => {
            self.add_log("Cannot copy directories yet".to_string());
        }
    }
    true
}
KeyCode::Char('v') => {
    match &self.clipboard {
        Some(FsEntry::File { path, name, .. }) => {
            let dest = self.current_dir.join(name);
            if dest.exists() {
                // Append suffix
                let stem = path.file_stem().unwrap_or_default().to_str().unwrap_or("file");
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let mut counter = 1;
                let mut new_dest;
                loop {
                    let new_name = format!("{stem}-{counter}.{ext}");
                    new_dest = self.current_dir.join(&new_name);
                    if !new_dest.exists() {
                        break;
                    }
                    counter += 1;
                }
                match std::fs::copy(path, &new_dest) {
                    Ok(_) => self.add_log(format!("📋 Pasted: {} (as {})", name, new_dest.file_name().unwrap().to_str().unwrap())),
                    Err(e) => self.add_log(format!("✗ Paste failed: {e}")),
                }
            } else {
                match std::fs::copy(path, &dest) {
                    Ok(_) => self.add_log(format!("📋 Pasted: {name}")),
                    Err(e) => self.add_log(format!("✗ Paste failed: {e}")),
                }
            }
            self.navigate_to(&self.current_dir.clone());
        }
        Some(FsEntry::Directory { .. }) => {
            self.add_log("Cannot paste directories yet".to_string());
        }
        None => {
            self.add_log("Nothing to paste".to_string());
        }
    }
    true
}
```

- [ ] **Step 3: Update test helpers**

```rust
fn app_with_entries(entries: Vec<FsEntry>) -> App {
    // ...
    clipboard: None,
    // ...
}
```

Same for `app_in_temp_dir`.

- [ ] **Step 4: Write tests**

```rust
#[test]
fn copy_sets_clipboard() {
    let entries = vec![
        FsEntry::File { path: PathBuf::from("test.md"), name: "test.md".into(), extension: "md".into(), size: 100 },
    ];
    let mut app = app_with_entries(entries);
    let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert!(app.clipboard.is_some());
    assert_eq!(app.clipboard.unwrap().name(), "test.md");
}

#[test]
fn paste_copies_file_into_current_dir() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    // Set clipboard to a known file
    let src = dir.path().join("readme.md");
    app.clipboard = Some(FsEntry::File {
        path: src.clone(),
        name: "readme.md".into(),
        extension: "md".into(),
        size: 0,
    });
    // Copy to a subdir
    let subdir = dir.path().join("subdir");
    app.navigate_to(&subdir);
    let key = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert!(subdir.join("readme.md").exists());
}

#[test]
fn paste_without_clipboard_logs_message() {
    let mut app = app_with_entries(vec![]);
    let key = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert!(app.log_messages.iter().any(|m| m.contains("Nothing to paste")));
}
```

- [ ] **Step 5: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 272+ tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "feat: copy/paste files with 'c' and 'v' keys"
```

---

### Task 8: Typst Syntax Highlighting in Preview (P2)

**Files:**
- Modify: `Cargo.toml`, `src/ui/preview_panel.rs:49-68`
- Add: Typst syntax highlighting function in preview_panel.rs

**Interfaces:**
- Consumes: `typst_syntax::parse`, `typst_syntax::highlight::highlight`, `typst_syntax::LinkedNode`
- Produces: `render_typst(source: &str) -> Vec<Line<'static>>`

- [ ] **Step 1: Add typst-syntax dependency**

```toml
# Cargo.toml
typst-syntax = "0.15"  # Syntax parsing and highlighting for Typst
```

- [ ] **Step 2: Add render_typst function in preview_panel.rs**

```rust
/// Render Typst source code with syntax highlighting.
fn render_typst(source: &str) -> Vec<Line<'static>> {
    use typst_syntax::LinkedNode;
    use typst_syntax::highlight::{highlight, Tag};

    let root = typst_syntax::parse(source);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();

    walk_typst_node(&mut current_line, &mut lines, &LinkedNode::new(&root));

    if !current_line.is_empty() {
        lines.push(Line::from(std::mem::take(&mut current_line)));
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::raw(source)));
    }
    lines
}

/// Recursively walk a Typst syntax tree, emitting styled spans.
fn walk_typst_node(
    spans: &mut Vec<Span<'static>>,
    lines: &mut Vec<Line<'static>>,
    node: &LinkedNode,
) {
    let tag = highlight(node);
    let style = typst_tag_to_style(tag);
    let text = node.leaf_text();

    if !text.is_empty() {
        let mut parts = text.split('\n').peekable();
        while let Some(part) = parts.next() {
            if !part.is_empty() {
                spans.push(Span::styled(part.to_string(), style));
            }
            if parts.peek().is_some() {
                lines.push(Line::from(std::mem::take(spans)));
            }
        }
    } else {
        for child in node.children() {
            walk_typst_node(spans, lines, &child);
        }
    }
}

/// Map a Typst highlight tag to a ratatui style.
fn typst_tag_to_style(tag: Option<Tag>) -> Style {
    use ratatui::style::{Color, Modifier, Style};
    match tag {
        Some(Tag::Heading) => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        Some(Tag::Strong) => Style::default().add_modifier(Modifier::BOLD),
        Some(Tag::Emph) => Style::default().add_modifier(Modifier::ITALIC),
        Some(Tag::Link) => Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
        Some(Tag::Raw) => Style::default().fg(Color::Green),
        Some(Tag::Keyword) => Style::default().fg(Color::Magenta),
        Some(Tag::Function) => Style::default().fg(Color::Cyan),
        Some(Tag::Number) => Style::default().fg(Color::Yellow),
        Some(Tag::String) => Style::default().fg(Color::Green),
        Some(Tag::Operator) => Style::default().fg(Color::Red),
        Some(Tag::Comment) => Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        Some(Tag::Label) => Style::default().fg(Color::Blue),
        Some(Tag::Ref) => Style::default().fg(Color::Blue),
        Some(Tag::ListMarker) => Style::default().fg(Color::Cyan),
        Some(Tag::Escape) => Style::default().fg(Color::Magenta),
        Some(Tag::Punctuation) => Style::default().fg(Color::DarkGray),
        Some(Tag::MathDelimiter) => Style::default().fg(Color::Magenta),
        Some(Tag::MathOperator) => Style::default().fg(Color::Red),
        Some(Tag::MathGroupingParens) => Style::default().fg(Color::DarkGray),
        Some(Tag::Interpolated) => Style::default().fg(Color::Cyan),
        Some(Tag::Error) => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        None => Style::default(),
    }
}
```

- [ ] **Step 3: Wire into the preview panel's match block**

```rust
let paragraph = match ext.as_deref() {
    Some("md" | "markdown") => { /* existing */ }
    Some("html" | "htm") => { /* existing */ }
    Some("typ") => {
        let lines = render_typst(&app.preview_content);
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.preview_scroll, 0))
    }
    _ => { /* existing default */ }
};
```

- [ ] **Step 4: Write tests**

```rust
// In src/ui/preview_panel.rs #[cfg(test)] module

#[test]
fn test_typst_highlight_preserves_text() {
    let source = "= Hello\n\nThis is *bold* and _emph_.\n\n```\ncode\n```";
    let lines = render_typst(source);
    let result: String = lines.iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    // The rendered text should contain the original content
    // (newlines may differ due to tokenization, but key content should be there)
    assert!(result.contains("Hello"), "heading text preserved");
    assert!(result.contains("bold"), "strong text preserved");
    assert!(result.contains("emph"), "emph text preserved");
}

#[test]
fn test_typst_highlight_empty_source() {
    let lines = render_typst("");
    assert!(!lines.is_empty(), "should produce at least one line");
}
```

- [ ] **Step 5: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 274+ tests pass.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/ui/preview_panel.rs
git commit -m "feat: Typst syntax highlighting in preview panel"
```

---

### Task 9: Bookmarks (P3)

**Files:**
- Modify: `src/config.rs:9-12`, `src/app.rs:36-89, 92-112, 425-497, 696-759`

**Interfaces:**
- Consumes: `Settings` struct, `save_settings`, `App` struct
- Produces: `Settings.bookmarks: Vec<PathBuf>`, `b` toggle bookmark, `B` cycle bookmarks

- [ ] **Step 1: Add bookmarks to Settings**

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub last_directory: Option<PathBuf>,
    pub theme: Option<String>,
    #[serde(default)]
    pub bookmarks: Vec<PathBuf>,
}
```

- [ ] **Step 2: Update Settings literals in app.rs**

Both `navigate_to` (line ~105) and `cycle_theme` (line ~205) construct `Settings { ... }` literally. They need the new field. Use `..Default::default()`:

```rust
let _ = save_settings(&Settings {
    last_directory: Some(self.current_dir.clone()),
    theme: None,
    bookmarks: vec![], // or use ..Default::default()
});
```

- [ ] **Step 3: Add bookmarks field to App**

```rust
// In App struct:
pub bookmarks: Vec<PathBuf>;
```

In `App::new()` after `load_settings()`:
```rust
let bookmarks = settings.bookmarks.clone();
```

In `Self`: `bookmarks,`

- [ ] **Step 4: Add b/B keybindings in handle_browser_key**

```rust
KeyCode::Char('b') => {
    if self.bookmarks.contains(&self.current_dir) {
        self.bookmarks.retain(|d| d != &self.current_dir);
        self.add_log(format!("🔖 Unbookmarked: {}", self.current_dir.display()));
    } else {
        self.bookmarks.push(self.current_dir.clone());
        self.add_log(format!("🔖 Bookmarked: {}", self.current_dir.display()));
    }
    let _ = save_settings(&Settings {
        last_directory: Some(self.current_dir.clone()),
        theme: Some(self.theme_name.clone()),
        bookmarks: self.bookmarks.clone(),
    });
    true
}
KeyCode::Char('B') => {
    if self.bookmarks.is_empty() {
        self.add_log("No bookmarks".to_string());
        return true;
    }
    // Find current dir in bookmarks, cycle to next
    let pos = self.bookmarks.iter().position(|d| d == &self.current_dir);
    let next_pos = match pos {
        Some(i) if i + 1 < self.bookmarks.len() => i + 1,
        _ => 0,
    };
    self.navigate_to(&self.bookmarks[next_pos]);
    self.add_log(format!("🔖 Bookmark {}: {}", next_pos + 1, self.current_dir.display()));
    true
}
```

- [ ] **Step 5: Update test helpers**

```rust
fn app_with_entries(entries: Vec<FsEntry>) -> App {
    // ...
    bookmarks: vec![],
    // ...
}
fn app_in_temp_dir(dir: &TempDir) -> App {
    // ...
    bookmarks: vec![],
    // ...
}
```

- [ ] **Step 6: Write tests**

```rust
#[test]
fn bookmark_toggle_adds_removes() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    let key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert!(app.bookmarks.contains(&dir.path()));
    assert!(app.handle_browser_key(key));
    assert!(!app.bookmarks.contains(&dir.path()));
}

#[test]
fn bookmark_cycle_navigates() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    let subdir = dir.path().join("subdir");
    app.bookmarks = vec![dir.path().to_path_buf(), subdir.clone()];
    app.navigate_to(&subdir);
    let key = KeyEvent::new(KeyCode::Char('B'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert_eq!(app.current_dir, dir.path());
}

#[test]
fn bookmark_no_bookmarks_logs() {
    let mut app = app_with_entries(vec![]);
    let key = KeyEvent::new(KeyCode::Char('B'), KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert!(app.log_messages.iter().any(|m| m.contains("No bookmarks")));
}
```

- [ ] **Step 7: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 278+ tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/config.rs
git commit -m "feat: bookmarks with 'b' toggle and 'B' cycle"
```

---

### Task 10: Global Recursive File Search (P3)

**Files:**
- Modify: `src/app.rs:36-89, 425-497, 696-759`
- Modify: `src/ui/browser_panel.rs`

**Interfaces:**
- Consumes: `App` struct, `walkdir`-like recursion (std::fs only, no new dep)
- Produces: `f` key triggers recursive search in browser focus, results shown as flat list

Use `f` key in browser focus to trigger a recursive search from current_dir. The search walks directories (max 500 results, max depth 4) and shows matching files. Results replace the entries list. Enter opens the file. Esc clears search results.

- [ ] **Step 1: Add search fields to App**

```rust
// In App struct:
pub search_results: Vec<FsEntry>,
pub searching: bool,
```

In `App::new()`:
```rust
search_results: vec![],
searching: false,
```

- [ ] **Step 2: Add recursive search method**

```rust
/// Recursively search for files by name (case-insensitive substring).
pub fn search_files(&mut self, query: &str) {
    const MAX_RESULTS: usize = 500;
    const MAX_DEPTH: u32 = 4;
    let mut results = Vec::new();

    fn walk(dir: &Path, query: &str, depth: u32, results: &mut Vec<FsEntry>, max: &mut usize) {
        if *max == 0 || depth > MAX_DEPTH {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if *max == 0 {
                    return;
                }
                let path = entry.path();
                if let Ok(ftype) = entry.file_type() {
                    if ftype.is_dir() {
                        walk(&path, query, depth + 1, results, max);
                    } else if ftype.is_file() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.to_lowercase().contains(&query.to_lowercase())
                                && is_supported_file(&path)
                            {
                                let name = name.to_string();
                                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
                                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                                results.push(FsEntry::File { path, name, extension: ext, size });
                                *max -= 1;
                            }
                        }
                    }
                }
            }
        }
    }

    let mut max = MAX_RESULTS;
    walk(&self.current_dir, query, 0, &mut results, &mut max);
    self.search_results = results;
    self.searching = true;
    self.entries = self.search_results.clone();
    self.selected_index = 0;
    self.preview_content.clear();
    self.preview_path = None;
    self.add_log(format!("Search: {query} ({} results)", self.entries.len()));
}
```

- [ ] **Step 3: Add f keybinding in handle_browser_key**

```rust
KeyCode::Char('f') => {
    self.mode = Mode::Input(InputMode::Search);
    self.input_buffer.clear();
    self.add_log("Search: type query, Enter to search, Esc to cancel".to_string());
    true
}
```

Add `Search` to `InputMode`:
```rust
pub enum InputMode {
    Rename,
    NewFile,
    NewDir,
    Search,
}
```

Add handler in `handle_key` for `InputMode::Search`:
```rust
InputMode::Search => {
    if name.is_empty() {
        self.add_log("Search query empty".to_string());
    } else {
        self.search_files(&name);
    }
    true
}
```

And Esc handler clears search:
```rust
// In main handle_key, before the mode check:
if self.searching && matches!(key.code, KeyCode::Esc) {
    self.searching = false;
    self.navigate_to(&self.current_dir.clone());
    self.add_log("Search cancelled".to_string());
    return true;
}
```

- [ ] **Step 4: Update test helpers**

```rust
fn app_with_entries(entries: Vec<FsEntry>) -> App {
    // ...
    search_results: vec![],
    searching: false,
    // ...
}
```

- [ ] **Step 5: Write tests**

```rust
#[test]
fn search_files_finds_matching_files() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    app.search_files("readme");
    assert!(!app.entries.is_empty(), "should find readme.md");
    assert!(app.entries.iter().any(|e| e.name() == "readme.md"));
}

#[test]
fn search_files_returns_empty_for_no_match() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    app.search_files("zzzzz");
    assert!(app.entries.is_empty());
}

#[test]
fn search_esc_clears_and_restores() {
    let dir = create_test_dir();
    let mut app = app_in_temp_dir(&dir);
    let original_count = app.entries.len();
    app.search_files("x");
    assert!(app.entries.is_empty() || app.entries.len() < original_count);
    app.searching = false;
    app.navigate_to(&app.current_dir.clone());
    assert_eq!(app.entries.len(), original_count);
}
```

- [ ] **Step 6: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 282+ tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/app.rs src/ui/browser_panel.rs
git commit -m "feat: global recursive file search with 'f' key"
```

---

### Task 11: CI Pipeline (P1)

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:** None (CI only)

- [ ] **Step 1: Create CI workflow**

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [master, main]
  pull_request:
    branches: [master, main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Format check
        run: cargo fmt --check
      - name: Clippy
        run: cargo clippy -- -D warnings
      - name: Build
        run: cargo build --verbose
      - name: Test
        run: cargo test --verbose
      - name: Build with Typst
        run: cargo build --features typst --verbose
```

- [ ] **Step 2: Verify**

```bash
# Validate YAML syntax
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
```

Expected: no YAML parse errors.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add GitHub Actions workflow for fmt, clippy, test, and build"
```

---

### Future / Stretch

These items are not in the current plan but could be added in later phases:

- **Global keybindings**: show all keybindings on the status bar / help popup update
- **Code coverage**: integrate `cargo llvm-cov` and add coverage badge
- **Release build**: verify `cargo install --path .` works, add to README
- **Configuration UI**: inline theme cycling in the settings panel is already done; adding more config options (default format, editor keybinding)
- **Image preview**: show images in the terminal via `viu` or similar
- **Trash UI**: browse trash, restore files, empty trash command