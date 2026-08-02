# Phase 2: TUI Integration Implementation Plan

> **For agentic workers:** Tasks are implemented sequentially. Each task produces independently testable, compilable code. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect the existing conversion engine and preview logic to the TUI panels — implement real file browsing, file preview, conversion triggers, and log output.

**Architecture:** The TUI has three panels (browser, preview, log) plus a help overlay. `App` holds all state. Key events dispatch to `App::handle_key()`. Panels read from `App` state to render. Conversions write results to the log panel and optionally open the output in preview.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28, pulldown-cmark, markdown2pdf, printpdf, html2md, pdf_oxide

## Global Constraints

- Edition 2024 (rustc ≥ 1.85). `gen` is a reserved keyword.
- All conversions use Rust crates — no system binaries.
- `cargo check` must pass after every task.
- `cargo clippy` must pass with no warnings after every task.
- `cargo fmt` must be run after every task.
- No new dependencies beyond what's in Cargo.toml.
- Follow existing patterns in the codebase (error handling, module structure).

---

### Task 1: App State — File Tree, Selection, Logs, Preview

**Files:**
- Modify: `src/app.rs` (entire file)

**Interfaces:**
- Consumes: `crate::fs_utils::{list_entries, FsEntry}`, `crate::preview::preview_file`, `crate::converter::{convert, Format, ConversionError}`
- Produces: `App` struct with fields: `current_dir`, `entries`, `selected_index`, `preview_content`, `preview_path`, `log_messages`, `preview_scroll`, `log_scroll`

- [ ] **Step 1: Replace app.rs with full state implementation**

Write the complete new `src/app.rs`:

```rust
use std::path::{Path, PathBuf};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::converter::{convert, ConversionError, Format};
use crate::fs_utils::{list_entries, FsEntry};
use crate::preview::preview_file;

/// Application mode — what the user is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Browse,
    Preview,
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

impl App {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
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
        let entry = &self.entries[self.selected_index];
        match entry {
            FsEntry::Directory { path, .. } => {
                self.navigate_to(path);
            }
            FsEntry::File { path, .. } => {
                self.preview_path = Some(path.clone());
                match preview_file(path) {
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
    }

    /// Go to parent directory.
    pub fn parent_directory(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            let parent = parent.to_path_buf();
            // Only navigate if parent exists and is different from current
            if parent != self.current_dir {
                self.navigate_to(&parent);
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
            self.add_log(format!(
                "File is already in {} format",
                format_name(target)
            ));
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
            let stem = input.file_stem().unwrap_or_default().to_str().unwrap_or("output");
            output = input.parent().unwrap_or(Path::new(".")).join(format!(
                "{}-converted.{}",
                stem,
                output_ext
            ));
        }

        self.add_log(format!(
            "Converting: {} → {}",
            input.file_name().unwrap_or_default().to_str().unwrap_or("?"),
            output.file_name().unwrap_or_default().to_str().unwrap_or("?")
        ));

        match convert(&input, &output, target) {
            Ok(()) => {
                self.add_log(format!("✓ Conversion complete: {}", output.display()));
                // Preview the result
                self.preview_path = Some(output);
                match preview_file(&self.preview_path.as_ref().unwrap()) {
                    Ok(content) => {
                        self.preview_content = content;
                        self.preview_scroll = 0;
                    }
                    Err(e) => {
                        self.add_log(format!("Preview of result failed: {e}"));
                    }
                }
            }
            Err(e) => {
                self.add_log(format!("✗ Conversion failed: {e}"));
            }
        }
    }

    /// Handle a key event. Returns true if the event was consumed.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Global keys (work in any mode except Help)
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
            _ => {
                // Focus-specific keys
                match self.focus {
                    Focus::Browser => self.handle_browser_key(key),
                    Focus::Preview => self.handle_preview_key(key),
                    Focus::Log => self.handle_log_key(key),
                }
            }
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
                // Enter directory if selected item is a directory
                if !self.entries.is_empty() {
                    if let FsEntry::Directory { .. } = &self.entries[self.selected_index] {
                        self.activate_selection();
                    }
                }
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
            // 'h' for HTML conversion is handled above as parent dir when in browser
            // HTML conversion from browser: use Ctrl+h or just don't bind here
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
                self.log_scroll = (self.log_scroll + 1).min(self.log_messages.len().saturating_sub(1) as u16);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
                true
            }
            _ => false,
        }
    }

    pub fn handle_mouse(&mut self, _event: MouseEvent) -> bool {
        false
    }

    pub fn tick(&mut self) {}
}
```

Helper function (add at bottom of app.rs):

```rust
fn format_name(f: Format) -> &'static str {
    match f {
        Format::Markdown => "Markdown",
        Format::Html => "HTML",
        Format::Pdf => "PDF",
        Format::Typst => "Typst",
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully (may have warnings about unused imports in other files — that's expected until later tasks)

- [ ] **Step 3: Run cargo fmt**

Run: `cargo fmt`
Expected: No changes (or formatting applied cleanly)

---

### Task 2: Browser Panel — Real File Listing

**Files:**
- Modify: `src/ui/browser_panel.rs` (entire file)
- Modify: `src/ui/mod.rs` (pass &App to browser_panel::draw)

**Interfaces:**
- Consumes: `App` (current_dir, entries, selected_index, focus)
- Produces: Rendered browser panel with directory tree

- [ ] **Step 1: Rewrite browser_panel.rs**

```rust
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

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
        let parent_style = if is_focused && app.selected_index == 0 && !app.entries.is_empty() {
            Style::default().fg(ratatui::style::Color::Black).bg(ratatui::style::Color::Cyan)
        } else {
            Style::default().fg(ratatui::style::Color::Yellow)
        };
        items.push(ListItem::new(Text::styled("  📁 ../", parent_style)));
    }

    for (i, entry) in app.entries.iter().enumerate() {
        let adjusted_index = if app.current_dir.parent().is_some() { i + 1 } else { i };
        let is_selected = is_focused && adjusted_index == app.selected_index;

        let (prefix, style) = match entry {
            FsEntry::Directory { name, .. } => {
                let style = if is_selected {
                    Style::default().fg(ratatui::style::Color::Black).bg(ratatui::style::Color::Cyan)
                } else {
                    Style::default().fg(ratatui::style::Color::Yellow)
                };
                ("📁 ", style)
            }
            FsEntry::File { name, extension, .. } => {
                let icon = match extension.as_str() {
                    "md" | "markdown" => "📝 ",
                    "html" | "htm" => "🌐 ",
                    "pdf" => "📄 ",
                    "typ" => "✍️ ",
                    _ => "📄 ",
                };
                let style = if is_selected {
                    Style::default().fg(ratatui::style::Color::Black).bg(ratatui::style::Color::Cyan)
                } else {
                    Style::default()
                };
                (icon, style)
            }
        };

        items.push(ListItem::new(Text::styled(format!("  {prefix}{}", entry.name()), style)));
    }

    if items.is_empty() {
        items.push(ListItem::new(Text::styled("  (empty directory)", Style::default().dim())));
    }

    let list = List::new(items).block(block).highlight_style(
        Style::default().fg(ratatui::style::Color::Black).bg(ratatui::style::Color::Cyan),
    );
    frame.render_widget(list, area);
}
```

- [ ] **Step 2: Update ui/mod.rs to pass &App to browser_panel**

Edit `src/ui/mod.rs` line 16:
```rust
    browser_panel::draw(frame, areas.browser, app);
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Run cargo fmt**

Run: `cargo fmt`
Expected: Formatting applied cleanly

---

### Task 3: Preview Panel — Real File Preview

**Files:**
- Modify: `src/ui/preview_panel.rs` (entire file)
- Modify: `src/ui/mod.rs` (pass &App to preview_panel::draw)

**Interfaces:**
- Consumes: `App` (preview_content, preview_path, preview_scroll, focus)
- Produces: Rendered preview panel with file content

- [ ] **Step 1: Rewrite preview_panel.rs**

```rust
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Focus};

/// Draw the file preview panel.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Preview;

    let title = match &app.preview_path {
        Some(path) => format!(
            " Preview: {} ",
            path.file_name().unwrap_or_default().to_str().unwrap_or("?")
        ),
        None => " Preview ".to_string(),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(ratatui::style::Color::Cyan)
        } else {
            Style::default()
        });

    let content = if app.preview_content.is_empty() {
        "Select a file to preview\n\nPress Enter on a file in the browser panel."
    } else {
        &app.preview_content
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.preview_scroll, 0));

    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 2: Update ui/mod.rs to pass &App to preview_panel**

Edit `src/ui/mod.rs` line 17:
```rust
    preview_panel::draw(frame, areas.preview, app);
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Run cargo fmt**

Run: `cargo fmt`
Expected: Formatting applied cleanly

---

### Task 4: Log Panel — Real Log Display

**Files:**
- Modify: `src/ui/log_panel.rs` (entire file)
- Modify: `src/ui/mod.rs` (pass &App to log_panel::draw)

**Interfaces:**
- Consumes: `App` (log_messages, log_scroll, focus)
- Produces: Rendered log panel with scrollable messages

- [ ] **Step 1: Rewrite log_panel.rs**

```rust
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Focus};

/// Draw the log / status panel at the bottom of the screen.
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus == Focus::Log;

    let block = Block::default()
        .title(" Log ")
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(ratatui::style::Color::Cyan)
        } else {
            Style::default()
        });

    let content = if app.log_messages.is_empty() {
        "Ready".to_string()
    } else {
        app.log_messages.join("\n")
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.log_scroll, 0));

    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 2: Update ui/mod.rs to pass &App to log_panel**

Edit `src/ui/mod.rs` line 18:
```rust
    log_panel::draw(frame, areas.log, app);
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Run cargo fmt**

Run: `cargo fmt`
Expected: Formatting applied cleanly

---

### Task 5: Key Handling — Wire Up Navigation and Conversion in main.rs

**Files:**
- Modify: `src/main.rs` (simplify event loop — remove duplicate q handling)

**Interfaces:**
- Consumes: `App::handle_key()` (already implemented in Task 1)
- Produces: Clean event loop that delegates all key handling to App

- [ ] **Step 1: Simplify main.rs event loop**

Replace the event loop in `src/main.rs` (lines 39-80) with a cleaner version:

```rust
fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    while app.running {
        terminal.draw(|frame| {
            let area = frame.area();
            ui::render(app, frame, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                app.handle_key(key);
            }
        }
    }
    Ok(())
}
```

This removes the special-case handling of q, ?, Esc, Tab from main.rs since those are now handled in App::handle_key().

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 3: Run cargo fmt**

Run: `cargo fmt`
Expected: Formatting applied cleanly

---

### Task 6: Clean Up Warnings and Final Polish

**Files:**
- Modify: `src/app.rs` (remove unused imports if any)
- Modify: `src/main.rs` (remove unused imports if any)

- [ ] **Step 1: Run clippy and fix warnings**

Run: `cargo clippy -- -D warnings`
Expected: Passes with no warnings

- [ ] **Step 2: Run full check**

Run: `cargo check`
Expected: Compiles with 0 warnings

- [ ] **Step 3: Run cargo fmt**

Run: `cargo fmt`
Expected: No changes

---

### Task 7: CodeGraph Re-index

- [ ] **Step 1: Re-index CodeGraph**

Run: `just codegraph`
Expected: CodeGraph index updated with all source files

- [ ] **Step 2: Verify CodeGraph status**

Run: `just codegraph-status`
Expected: Shows all source files indexed