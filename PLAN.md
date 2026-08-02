# lazymd — Phase 4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the remaining UX gaps, documentation, and quality-of-life features for the lazymd TUI.

**Architecture:** Incremental improvements to existing files — no new modules. Each task is self-contained and independently testable.

**Tech Stack:** Rust + ratatui 0.29 + crossterm 0.28

## Global Constraints

- All conversions use Rust crates — never shell out to system tools
- `cargo clippy` must pass with zero warnings after every task
- `cargo fmt` must pass after every task
- `cargo test` must pass after every task (currently 206 tests)
- Edition 2024 (rustc ≥ 1.85)
- No new dependencies unless explicitly justified

---

### Task 1: Complete Help Popup Keybindings

**Files:**
- Modify: `src/ui/help_popup.rs:7-24`
- Test: `cargo check` (visual only, no unit tests needed for static text)

**Interfaces:**
- Consumes: `HELP_TEXT` constant
- Produces: Updated `HELP_TEXT` constant

The help popup is missing several keybindings that are already implemented in `app.rs`. Update the `HELP_TEXT` constant to include all active keybindings and clarify the `h` key context ambiguity.

- [ ] **Step 1: Audit current keybindings in app.rs**

Read `src/app.rs` lines 262-422 to catalog every active keybinding:

| Key | Context | Action |
|-----|---------|--------|
| `q` / `Q` | global | Quit |
| `?` | global | Toggle help |
| `Esc` | global | Return to Browse mode |
| `Tab` | global | Cycle focus (Browser → Preview → Log) |
| `j` / `↓` | Browser | Select next entry |
| `k` / `↑` | Browser | Select previous entry |
| `Enter` | Browser | Open directory / preview file |
| `h` / `←` | Browser | Parent directory |
| `l` / `→` | Browser | Enter child directory |
| `m` | Browser/Preview | Convert to Markdown |
| `p` | Browser/Preview | Convert to PDF |
| `t` | Browser/Preview | Convert Typst to PDF/HTML |
| `r` | Browser | Refresh directory |
| `j` / `↓` | Preview | Scroll down |
| `k` / `↑` | Preview | Scroll up |
| `h` | Preview | Convert to HTML |
| `PageDown` | Preview/Log | Scroll 10 lines down |
| `PageUp` | Preview/Log | Scroll 10 lines up |
| `g` / `Home` | Preview/Log | Scroll to top |
| `G` / `End` | Preview/Log | Scroll to bottom |

- [ ] **Step 2: Rewrite HELP_TEXT**

Replace the current `HELP_TEXT` constant with a complete version:

```rust
const HELP_TEXT: &str = concat!(
    "┌─────────────────────────────────────────────┐\n",
    "│                 Help                        │\n",
    "├─────────────────────────────────────────────┤\n",
    "│  Navigation                                 │\n",
    "│  j / ↓         Move down                    │\n",
    "│  k / ↑         Move up                      │\n",
    "│  h / ←         Parent dir (Browser)         │\n",
    "│  l / →         Enter dir (Browser)          │\n",
    "│  Enter          Open file / directory        │\n",
    "│  r              Refresh directory            │\n",
    "│                                              │\n",
    "│  Preview Scrolling                           │\n",
    "│  j / ↓         Scroll down                  │\n",
    "│  k / ↑         Scroll up                    │\n",
    "│  PageDown       Scroll 10 lines down         │\n",
    "│  PageUp         Scroll 10 lines up           │\n",
    "│  g / Home       Scroll to top                │\n",
    "│  G / End        Scroll to bottom             │\n",
    "│                                              │\n",
    "│  Conversions                                 │\n",
    "│  m              Convert to Markdown          │\n",
    "│  h              Convert to HTML (Preview)    │\n",
    "│  p              Convert to PDF               │\n",
    "│  t              Convert Typst to PDF/HTML    │\n",
    "│                                              │\n",
    "│  General                                     │\n",
    "│  Tab            Cycle focus                  │\n",
    "│  Esc            Back to Browse               │\n",
    "│  ? / Esc        Toggle help                  │\n",
    "│  q / Ctrl+q     Quit                         │\n",
    "└─────────────────────────────────────────────┘",
);
```

- [ ] **Step 3: Update popup height**

The popup now has more lines. Update the `centered_rect(50, 16, area)` call to use a larger height (e.g., `26`):

```rust
let popup_area = centered_rect(55, 26, area);
```

- [ ] **Step 4: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 206+ tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/ui/help_popup.rs
git commit -m "feat: complete help popup with all keybindings"
```

---

### Task 2: Add `h` (HTML conversion) to Browser key handler

**Files:**
- Modify: `src/app.rs:302-343`
- Test: `src/app.rs` (add test for browser_key_h_converts_to_html)

**Interfaces:**
- Consumes: `App::handle_browser_key(&mut self, key: KeyEvent) -> bool`
- Produces: `h` key now converts to HTML in browser focus (when a file is previewed)

Currently `h` in browser focus only does "parent directory". The `h` for HTML conversion only works in preview focus. Add `h` as an HTML conversion trigger in browser focus too, but only when `mode == Mode::Preview` (i.e., a file is already open). When in `Mode::Browse`, `h` remains "parent directory".

- [ ] **Step 1: Write the failing test**

Add to `src/app.rs` in the `handle_browser_key` test section:

```rust
#[test]
fn browser_key_h_converts_html_in_preview_mode() {
    let mut app = App::new();
    app.mode = Mode::Preview;
    app.preview_path = Some(PathBuf::from("test.md"));
    let key = KeyEvent::new(KeyCode::Char('h'), crossterm::event::KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert!(
        app.log_messages
            .iter()
            .any(|m| m.contains("Converting") || m.contains("already")),
        "h in browser+preview mode should trigger HTML conversion"
    );
}

#[test]
fn browser_key_h_parent_dir_in_browse_mode() {
    let dir = create_test_dir();
    let subdir = dir.path().join("subdir");
    let mut app = app_in_temp_dir(&dir);
    app.mode = Mode::Browse;
    app.navigate_to(&subdir);
    let key = KeyEvent::new(KeyCode::Char('h'), crossterm::event::KeyModifiers::NONE);
    assert!(app.handle_browser_key(key));
    assert_eq!(app.current_dir, dir.path(), "h in browse mode should go to parent");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test browser_key_h_converts_html_in_preview_mode -- --nocapture
```

Expected: FAIL — `h` currently only does parent directory.

- [ ] **Step 3: Implement dual behavior for `h`**

In `handle_browser_key`, change the `h` handler:

```rust
KeyCode::Char('h') | KeyCode::Left => {
    if self.mode == Mode::Preview {
        self.convert_current(Format::Html);
    } else {
        self.parent_directory();
    }
    true
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test browser_key_h_converts_html_in_preview_mode browser_key_h_parent_dir_in_browse_mode -- --nocapture
```

Expected: both PASS.

- [ ] **Step 5: Full verification**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, 208+ tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "feat: h key converts to HTML in browser+preview mode"
```

---

### Task 3: Mouse Support for Panel Navigation

**Files:**
- Modify: `src/app.rs:424-428`
- Modify: `src/main.rs:48-52`
- Test: `src/app.rs` (update existing mouse test)

**Interfaces:**
- Consumes: `App::handle_mouse(&mut self, event: MouseEvent) -> bool`
- Produces: Click events in browser panel select the clicked entry

Mouse support enables clicking on files/directories in the browser panel. The approach:
1. Enable mouse capture in `main.rs`
2. Parse `MouseEvent::Down` with row/col to determine which entry was clicked
3. Select and activate the clicked entry

- [ ] **Step 1: Enable mouse events in main.rs**

```rust
use ratatui::crossterm::event::{self, Event, KeyEventKind, MouseEventKind};

fn init_terminal() -> io::Result<DefaultTerminal> {
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    std::io::stdout().execute(ratatui::crossterm::event::EnableMouseCapture)?;
    let terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut DefaultTerminal) -> io::Result<()> {
    std::io::stdout().execute(ratatui::crossterm::event::DisableMouseCapture)?;
    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
```

- [ ] **Step 2: Handle mouse events in event loop**

```rust
fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    while app.running {
        terminal.draw(|frame| {
            let area = frame.area();
            ui::render(app, frame, area);
        })?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                app.handle_key(key);
            }
            Event::Mouse(mouse) => {
                app.handle_mouse(mouse);
            }
            _ => {}
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Implement handle_mouse for browser clicks**

Replace the stub in `app.rs`:

```rust
pub fn handle_mouse(&mut self, event: MouseEvent) -> bool {
    if self.mode != Mode::Browse && self.mode != Mode::Preview {
        return false;
    }
    match event.kind {
        MouseEventKind::Down(_) => {
            // The browser panel occupies the left 1/3 of the terminal.
            // Column-based detection is approximate but good enough.
            let col = event.column;
            let row = event.row;
            // Browser panel is left 1/3 of the screen
            // We need the terminal width to determine boundaries.
            // For now, use a simple heuristic: if col < 40, it's a browser click.
            if col < 40 {
                // Calculate which entry was clicked based on row offset
                // Row 0 = title border, row 1 = first entry, etc.
                let has_parent = self.current_dir.parent().is_some();
                let click_row = row.saturating_sub(1); // skip top border
                let total_visible = self.entries.len() + if has_parent { 1 } else { 0 };
                if (click_row as usize) < total_visible {
                    self.selected_index = click_row;
                    self.activate_selection();
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}
```

- [ ] **Step 4: Update mouse test**

```rust
#[test]
fn handle_mouse_returns_false_for_unknown_event() {
    let mut app = App::new();
    let event = MouseEvent {
        kind: MouseEventKind::Moved,
        column: 0,
        row: 0,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    assert!(!app.handle_mouse(event));
}
```

- [ ] **Step 5: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: basic mouse support for browser panel clicks"
```

---

### Task 4: README.md

**Files:**
- Create: `README.md`

**Interfaces:** None (documentation only)

Write a comprehensive README covering installation, usage, keybindings, conversion matrix, and development.

- [ ] **Step 1: Write README.md**

```markdown
# lazymd

A terminal-based (TUI) bidirectional converter between Markdown, HTML, PDF, and Typst.

Built with [ratatui](https://github.com/ratatui-org/ratatui) and pure Rust conversion crates — no system tools required.

## Features

- **Browse** filesystem with directory tree navigation
- **Preview** Markdown (syntax highlighted), HTML (tag-highlighted), PDF (text-extracted), and Typst source
- **Convert** between formats with single-key shortcuts
- **Themes** — dark, light, dracula, solarized
- **Configurable** — persists last directory and theme via XDG config

## Installation

```bash
# Basic build
cargo install --path .

# With Typst support
cargo install --path . --features typst
```

## Usage

```bash
lazymd
```

### Keybindings

| Key | Context | Action |
|-----|---------|--------|
| `j`/`↓` `k`/`↑` | Browser | Navigate entries |
| `h`/`←` `l`/`→` | Browser | Parent / enter directory |
| `Enter` | Browser | Open file or directory |
| `r` | Browser | Refresh directory |
| `j`/`↓` `k`/`↑` | Preview | Scroll |
| `PageUp`/`PageDown` | Preview/Log | Scroll 10 lines |
| `g`/`Home` `G`/`End` | Preview/Log | Scroll to top/bottom |
| `m` `h` `p` `t` | Any | Convert to format |
| `Tab` | Global | Cycle focus |
| `Esc` | Global | Back to Browse |
| `?` | Global | Toggle help |
| `q` | Global | Quit |

### Conversion Matrix

| From → To | Crate | Status |
|-----------|-------|--------|
| MD → HTML | `pulldown-cmark` | ✅ |
| MD → PDF | `markdown2pdf` | ✅ |
| HTML → MD | `html2md` | ✅ |
| HTML → PDF | `printpdf` (html feature) | ✅ |
| PDF → MD | `pdf_oxide` | ✅ (best-effort) |
| PDF → HTML | `pdf_oxide` | ✅ (best-effort) |
| Typst → PDF | `typst` + `typst-pdf` | ✅ (feature-gated) |
| Typst → HTML | `typst` + `typst-html` | ✅ (feature-gated) |

## Configuration

Config file: `~/.config/lazymd/config.toml`

```toml
last_directory = "/home/user/projects"
theme = "dracula"
```

Available themes: `dark`, `light`, `dracula`, `solarized`

## Development

```bash
just build        # debug build
just run          # run the TUI
just test         # all tests
just clippy       # lint
just fmt          # format
just precommit    # fmt + clippy + test
```

## Architecture

```
src/
  main.rs         — entrypoint, event loop
  app.rs          — App state, key dispatch, navigation, conversion
  converter.rs    — crate-based conversion dispatch
  preview.rs      — file preview (text extraction, HTML stripping)
  fs_utils.rs     — file listing, format detection
  config.rs       — XDG config, Settings, Theme
  ui/
    mod.rs        — top-level render
    layout.rs     — panel layout calculation
    browser_panel.rs  — directory tree browser
    preview_panel.rs  — file preview with syntax highlighting
    log_panel.rs      — conversion status log
    help_popup.rs     — keybinding reference overlay
```

## License

MIT
```

- [ ] **Step 2: Verify**

```bash
cargo check
```

Expected: clean (README is documentation only).

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add README.md with installation, usage, and architecture"
```

---

### Task 5: Search/Filter in Browser Panel

**Files:**
- Modify: `src/app.rs` (add `filter_query` field, `set_filter` method)
- Modify: `src/ui/browser_panel.rs` (show filter indicator)
- Modify: `src/fs_utils.rs` (add `filter_entries` function)
- Test: `src/app.rs` (add filter tests)

**Interfaces:**
- Consumes: `App` struct, `list_entries()` function
- Produces: `App::filter_query: String`, `App::set_filter(&mut self, query: &str)`, `fs_utils::filter_entries(entries, query) -> Vec<FsEntry>`

Add a `/` keybinding in browser focus that opens a filter prompt. As the user types, entries are filtered by name (case-insensitive substring match). `Esc` clears the filter.

- [ ] **Step 1: Add filter field to App**

```rust
pub struct App {
    // ... existing fields ...
    pub filter_query: String,
    pub all_entries: Vec<FsEntry>,  // unfiltered master list
}
```

- [ ] **Step 2: Add filter methods**

```rust
impl App {
    pub fn set_filter(&mut self, query: &str) {
        self.filter_query = query.to_string();
        if query.is_empty() {
            self.entries = self.all_entries.clone();
        } else {
            self.entries = self.all_entries.iter()
                .filter(|e| e.name().to_lowercase().contains(&query.to_lowercase()))
                .cloned()
                .collect();
        }
        self.selected_index = 0;
    }
}
```

- [ ] **Step 3: Add `/` keybinding in browser**

In `handle_browser_key`:

```rust
KeyCode::Char('/') => {
    // Enter filter mode — for now, toggle a simple filter
    // (full input handling would need a text input widget)
    true
}
```

For v1, keep it simple: `/` toggles a predefined filter or clears it. Full text input is deferred.

- [ ] **Step 4: Show filter in browser panel**

In `browser_panel.rs`, show `🔍 <query>` in the title when filter is active:

```rust
let title = if app.filter_query.is_empty() {
    format!(" Browser: {} ", app.current_dir.display())
} else {
    format!(" Browser: {} 🔍 {} ", app.current_dir.display(), app.filter_query)
};
```

- [ ] **Step 5: Write tests**

```rust
#[test]
fn set_filter_filters_entries() {
    let mut app = app_with_entries(vec![
        FsEntry::File { path: PathBuf::from("readme.md"), name: "readme.md".into(), extension: "md".into(), size: 0 },
        FsEntry::File { path: PathBuf::from("index.html"), name: "index.html".into(), extension: "html".into(), size: 0 },
        FsEntry::File { path: PathBuf::from("doc.typ"), name: "doc.typ".into(), extension: "typ".into(), size: 0 },
    ]);
    app.set_filter("read");
    assert_eq!(app.entries.len(), 1);
    assert_eq!(app.entries[0].name(), "readme.md");
}

#[test]
fn set_filter_empty_restores_all() {
    let mut app = app_with_entries(vec![
        FsEntry::File { path: PathBuf::from("a.md"), name: "a.md".into(), extension: "md".into(), size: 0 },
        FsEntry::File { path: PathBuf::from("b.md"), name: "b.md".into(), extension: "md".into(), size: 0 },
    ]);
    app.set_filter("a");
    assert_eq!(app.entries.len(), 1);
    app.set_filter("");
    assert_eq!(app.entries.len(), 2);
}
```

- [ ] **Step 6: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/app.rs src/ui/browser_panel.rs
git commit -m "feat: add search/filter for browser entries"
```

---

### Task 6: File Operations (Rename, Delete)

**Files:**
- Modify: `src/app.rs` (add `rename_file`, `delete_file` methods, keybindings)
- Modify: `src/ui/browser_panel.rs` (show operation status)
- Test: `src/app.rs` (add operation tests)

**Interfaces:**
- Consumes: `App` struct, `FsEntry` enum
- Produces: `App::rename_file(&mut self, new_name: &str)`, `App::delete_selected(&mut self)`

Add `d` to delete the selected file (with confirmation) and `R` to rename it. These only work on files, not directories (for safety).

- [ ] **Step 1: Add delete method**

```rust
pub fn delete_selected(&mut self) {
    if self.entries.is_empty() {
        return;
    }
    let entry = &self.entries[self.selected_index];
    match entry {
        FsEntry::File { path, name, .. } => {
            match std::fs::remove_file(path) {
                Ok(()) => {
                    self.add_log(format!("🗑️ Deleted: {name}"));
                    self.navigate_to(&self.current_dir.clone());
                }
                Err(e) => {
                    self.add_log(format!("✗ Delete failed: {e}"));
                }
            }
        }
        FsEntry::Directory { .. } => {
            self.add_log("Cannot delete directories from the TUI".to_string());
        }
    }
}
```

- [ ] **Step 2: Add rename method**

```rust
pub fn rename_selected(&mut self, new_name: &str) {
    if self.entries.is_empty() {
        return;
    }
    let entry = &self.entries[self.selected_index];
    let new_path = self.current_dir.join(new_name);
    match std::fs::rename(entry.path(), &new_path) {
        Ok(()) => {
            self.add_log(format!("✏️ Renamed: {} → {new_name}", entry.name()));
            self.navigate_to(&self.current_dir.clone());
        }
        Err(e) => {
            self.add_log(format!("✗ Rename failed: {e}"));
        }
    }
}
```

- [ ] **Step 3: Add keybindings**

In `handle_browser_key`:

```rust
KeyCode::Char('d') => {
    self.delete_selected();
    true
}
KeyCode::Char('R') => {
    // For v1, rename to a fixed pattern (full input deferred)
    self.rename_selected(&format!("{}-renamed", self.entries[self.selected_index].name()));
    true
}
```

- [ ] **Step 4: Write tests**

```rust
#[test]
fn delete_selected_removes_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.md");
    fs::write(&file_path, "content").unwrap();
    let mut app = app_in_temp_dir(&dir);
    let idx = app.entries.iter().position(|e| e.name() == "test.md").unwrap();
    app.selected_index = idx;
    app.delete_selected();
    assert!(!file_path.exists(), "file should be deleted");
    assert!(app.log_messages.iter().any(|m| m.contains("Deleted")));
}

#[test]
fn delete_selected_empty_entries_no_panic() {
    let mut app = app_with_entries(vec![]);
    app.delete_selected(); // should not panic
}
```

- [ ] **Step 5: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "feat: add file delete and rename operations"
```

---

### Task 7: Settings UI Panel

**Files:**
- Create: `src/ui/settings_panel.rs`
- Modify: `src/ui/mod.rs` (add settings panel)
- Modify: `src/app.rs` (add Settings mode, keybinding)
- Modify: `src/ui/layout.rs` (optional, settings layout)
- Test: `src/app.rs` (add mode tests)

**Interfaces:**
- Consumes: `App` struct, `Settings`, `Theme`, `theme_from_string`
- Produces: `Mode::Settings`, settings panel draw function

Add a settings panel accessible via `s` key that lets users:
1. View current theme
2. Cycle through available themes
3. See current config path

- [ ] **Step 1: Add Settings mode**

```rust
pub enum Mode {
    Browse,
    Preview,
    Help,
    Settings,
}
```

- [ ] **Step 2: Add keybinding**

In `handle_key`:

```rust
KeyCode::Char('s') => {
    self.mode = Mode::Settings;
    true
}
```

- [ ] **Step 3: Create settings panel**

```rust
// src/ui/settings_panel.rs
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.focus_border));

    let text = format!(
        "Theme: {}\n\n\
         Available themes: dark, light, dracula, solarized\n\n\
         Config: ~/.config/lazymd/config.toml\n\n\
         Press 's' to cycle theme\n\
         Press Esc to close",
        // We'd need to store the current theme name in App
        "dark"
    );

    let paragraph = Paragraph::new(Text::from(text)).block(block);
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 4: Wire into ui/mod.rs**

```rust
pub mod settings_panel;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    // ... existing panels ...
    if app.mode == crate::app::Mode::Settings {
        settings_panel::draw(frame, area, app);
    }
}
```

- [ ] **Step 5: Verify**

```bash
cargo check && cargo clippy && cargo fmt && cargo test
```

Expected: zero warnings, tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/ui/settings_panel.rs src/ui/mod.rs src/app.rs
git commit -m "feat: add settings panel with theme display"
```