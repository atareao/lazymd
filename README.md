# lazymd

> A terminal user interface for bidirectional conversion between Markdown, HTML, PDF, and Typst — powered entirely by Rust crates.

[![Rust](https://img.shields.io/badge/rust-2024-edition?logo=rust&style=flat-square)](https://www.rust-lang.org)
[![Ratatui](https://img.shields.io/badge/tui-ratatui-cyan?style=flat-square)](https://ratatui.rs)

**lazymd** is a keyboard-driven TUI that lets you browse your filesystem, preview documents in multiple formats, and convert between them — all without leaving the terminal. Every conversion uses pure Rust crates; no system binaries like `pandoc`, `wkhtmltopdf`, or `firefox` are required.

## Features

- **Browse** your filesystem with a recursive directory tree
- **Preview** Markdown, HTML, PDF, and Typst files inline
- **Convert** between formats with single-key shortcuts
- **Syntax highlighting** in preview: Markdown headings, code, links, and more
- **HTML tag highlighting** in preview for `.html` files
- **File sizes** displayed alongside filenames
- **Mouse support** — click to select and open files
- **Configurable themes** — dark, light, dracula, solarized
- **Persistent state** — remembers your last visited directory
- **Typst support** (optional feature) — compile `.typ` files to PDF/HTML

## Conversion Matrix

| Direction | Crate | Notes |
|-----------|-------|-------|
| Markdown → HTML | `pulldown-cmark` | CommonMark compliant |
| Markdown → PDF | `markdown2pdf` | Single crate, full pipeline |
| HTML → Markdown | `html2md` | Pure Rust |
| HTML → PDF | `printpdf` | With inline CSS support |
| PDF → Markdown | `pdf_oxide` | Best-effort, preserves text |
| PDF → HTML | `pdf_oxide` | Best-effort, preserves structure |
| Typst → PDF | `typst` + `typst-pdf` | Requires `--features typst` |
| Typst → HTML | `typst` + `typst-html` | Requires `--features typst` |

> **No system tools** are used. Every converter is a Rust crate dependency.

## Installation

### Prerequisites

- Rust toolchain ≥ 1.85 (edition 2024)
- A terminal with UTF-8 support

### From source

```bash
git clone https://github.com/your-username/lazymd.git
cd lazymd
cargo build --release
./target/release/lazymd
```

### With Typst support

```bash
cargo build --release --features typst
```

## Usage

Launch the TUI:

```bash
cargo run
```

### Keybindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl+q` | Quit |
| `s` | Open settings (cycle theme) |
| `Tab` | Cycle focus (Browser ↔ Preview ↔ Log) |
| `?` / `Esc` | Toggle help / close preview |

**Browser panel:**

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `Enter` | Open directory or file |
| `h` / `←` | Parent directory (or convert to HTML if preview active) |
| `l` / `→` | Enter directory |
| `r` | Refresh directory |
| `/` | Filter entries by name (type query, Enter to apply, Esc to cancel) |
| `d` | Delete selected file |
| `R` | Rename selected file (appends `-renamed`) |
| `m` / `p` / `t` | Convert to Markdown / PDF / Typst |

**Preview panel:**

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `PageDown` / `PageUp` | Scroll by 10 lines |
| `g` / `Home` | Scroll to top |
| `G` / `End` | Scroll to bottom |
| `m` / `h` / `p` / `t` | Convert to Markdown / HTML / PDF / Typst |

**Log panel:**

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `PageDown` / `PageUp` | Scroll by 10 lines |
| `g` / `Home` | Scroll to top |
| `G` / `End` | Scroll to bottom |

### Mouse

Click on any file or directory in the browser panel to select and open it.

## Configuration

lazymd stores configuration in the XDG config directory (`~/.config/lazymd/config.toml` on Linux).

```toml
# Remember the last visited directory
last_directory = "/home/user/projects"

# Choose a theme: "dark", "light", "dracula", or "solarized"
theme = "dracula"
```

## Themes

Four built-in themes are available:

| Theme | Preview |
|-------|---------|
| `dark` | Default dark theme |
| `light` | Light theme for bright terminals |
| `dracula` | Dracula color scheme |
| `solarized` | Solarized dark scheme |

## Development

```bash
# Quick check
just check

# Run tests
just test

# Lint
just clippy

# Full pre-commit check
just precommit

# Build with Typst support
just build --features typst
```

### Project structure

```
src/
├── main.rs              # Entrypoint, terminal setup, event loop
├── app.rs               # Application state, key dispatch, navigation
├── config.rs            # XDG config, settings, themes
├── converter.rs         # Crate-based conversion dispatch
├── fs_utils.rs          # File listing, format detection
├── preview.rs           # File preview (text extraction)
└── ui/
    ├── mod.rs           # Top-level render orchestration
    ├── layout.rs        # Panel layout calculation
    ├── browser_panel.rs # Filesystem tree browser
    ├── preview_panel.rs # File content preview with highlighting
    ├── log_panel.rs     # Conversion status and messages
    └── help_popup.rs    # Keybinding reference overlay
```

## Tech Stack

- **TUI framework**: [Ratatui](https://ratatui.rs) + [Crossterm](https://github.com/crossterm-rs/crossterm)
- **Markdown**: [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark)
- **PDF generation**: [markdown2pdf](https://crates.io/crates/markdown2pdf), [printpdf](https://github.com/fschutt/printpdf)
- **PDF extraction**: [pdf_oxide](https://crates.io/crates/pdf_oxide)
- **HTML conversion**: [html2md](https://crates.io/crates/html2md)
- **Typst**: [typst](https://github.com/typst/typst) (optional)
- **Serialization**: [serde](https://serde.rs) + [toml](https://crates.io/crates/toml)

## License

MIT