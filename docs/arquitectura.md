## What this is

A ratatui TUI for bidirectional conversion between Markdown, HTML, PDF, and Typst.
Users browse the filesystem, preview files in any of the four formats, and
convert between them with single-key shortcuts. **All conversions use Rust
crates — never shell out to system tools.**

## Architecture

```
src/
  main.rs              — entrypoint, TUI event loop, panic recovery
  app.rs               — App state, Mode/Focus enums, key dispatch, process mgmt
  preview.rs           — preview rendering for md / html / pdf files
  converter.rs         — crate-based conversion dispatch, tempfile mgmt, error types
  config.rs            — XDG path discovery, Settings struct, TOML persistence
  fs_utils.rs          — file listing, format detection, file-tree recursion
  ui/
    mod.rs             — top-level render(), orchestrates all panels
    layout.rs          — panel layout calculation (browser + preview + log)
    browser_panel.rs   — recursive directory tree browser (ratatui List widget)
    preview_panel.rs   — scrollable preview of the selected file
    log_panel.rs       — conversion status, errors, process output
    help_popup.rs      — keybinding reference overlay
```

## Preview rendering strategy

Each format renders differently in the terminal preview panel:

| Format | Strategy |
|--------|----------|
| **Markdown** | Rendered as plain text (source). Future: use `pulldown-cmark` events to add basic terminal formatting (colored headings, bold). |
| **HTML** | Tags stripped for terminal display via simple tag stripper (`strip_html_tags()`). |
| **PDF** | Text extracted page-by-page via `pdf_oxide::api::Pdf::open()` → `to_text(page)`. Binary/image preview out of scope for v1. |
| **Typst** | Rendered as plain text (source). Same as Markdown preview. |

## Error handling pattern

- `converter.rs` defines a `ConversionError` enum with variants: `UnsupportedFormat`, `ConversionFailed`, `Io`
- All conversion functions return `Result<T, ConversionError>`
- The TUI should display these errors in the log panel, never panic
- `markdown2pdf` errors are wrapped via `.map_err(|e| ConversionError::ConversionFailed(e.to_string()))`
