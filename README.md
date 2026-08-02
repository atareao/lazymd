# lazymd

A terminal UI (TUI) for bidirectional conversion between **Markdown**, **HTML**, and **PDF**.

Built with [ratatui](https://ratatui.rs/) and pure Rust conversion crates — **no external tools** or system dependencies required.

## Features

- **Browse** the filesystem with a recursive directory tree
- **Preview** Markdown, HTML, and PDF files directly in the terminal
- **Convert** between any of the three formats with single-key shortcuts
- **No system binaries** — all conversions use Rust crates (`pulldown-cmark`, `markdown2pdf`, `printpdf`, `html2md`, `pdf_oxide`)

## Conversion Matrix

| Direction | Crate | Status |
|-----------|-------|--------|
| MD → HTML | `pulldown-cmark` | ✅ |
| MD → PDF  | `markdown2pdf` | ✅ |
| HTML → MD | `html2md` | ✅ |
| HTML → PDF| `printpdf` (html feature) | ✅ |
| PDF → MD  | `pdf_oxide` | ✅ (best-effort) |
| PDF → HTML| `pdf_oxide` | ✅ (best-effort) |

## Requirements

- **Rust** ≥ 1.85 (edition 2024)
- **No system tools** — everything is a Rust crate

## Installation

```sh
git clone https://github.com/atareao/lazymd.git
cd lazymd
cargo build --release
./target/release/lazymd
```

## Usage

Launch the TUI:

```sh
cargo run
```

### Keybindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl+q` | Quit |
| `Tab` | Cycle focus (browser ↔ preview ↔ log) |
| `j`/`↓` / `k`/`↑` | Move cursor |
| `Enter` | Open directory / file |
| `h`/`←` / `l`/`→` | Parent dir / enter dir |
| `m` | Convert file to **M**arkdown |
| `h` | Convert file to **H**TML (on previewed file) |
| `p` | Convert file to **P**DF |
| `?` / `Esc` | Toggle / close help |

## Development

```sh
just build        # debug build
just run          # launch the TUI
just check        # fast compilation check
just test         # run tests
just clippy       # lint
just fmt          # format code
just precommit    # fmt + clippy + test
```

## Architecture

```
src/
  main.rs              — entrypoint, TUI event loop, panic recovery
  app.rs               — App state, Mode/Focus enums, key dispatch
  preview.rs           — preview rendering for md / html / pdf files
  converter.rs         — crate-based conversion dispatch, error types
  config.rs            — XDG path discovery, Settings struct, TOML persistence
  fs_utils.rs          — file listing, format detection, file-tree
  ui/
    mod.rs             — top-level render(), orchestrates all panels
    layout.rs          — panel layout calculation
    browser_panel.rs   — directory tree browser (ratatui List widget)
    preview_panel.rs   — scrollable preview panel
    log_panel.rs       — conversion status, errors, process output
    help_popup.rs      — keybinding reference overlay
```

## License

MIT