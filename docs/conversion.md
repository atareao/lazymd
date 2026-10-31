## Conversion matrix (all Rust crates, no system tools)

| Direction | Rust crate | Notes |
|-----------|-----------|-------|
| MD → HTML | `pulldown-cmark` | CommonMark parser. `Parser::new()` + `html::push_html()`. |
| MD → PDF  | `markdown2pdf` | Lexer + layout engine + printpdf backend. Single crate, no intermediate HTML. `parse_into_file()` or `parse_into_bytes()`. |
| MD → Typst | `pulldown-cmark` (conversor propio) | Parsea Markdown con `pulldown-cmark` y emite sintaxis Typst. `=` para headings, `#link()` para enlaces, `#figure(image())` para imágenes. Siempre disponible (no necesita feature flag). |
| HTML → MD | `html2md` | Pure Rust, simple API. |
| HTML → PDF| `printpdf` (html feature) | `PdfDocument::from_html()` with inline CSS. |
| PDF → MD  | `pdf_oxide` | Via `pdf_oxide::api::Pdf::open()` + `to_markdown(page)`. Best-effort, loses some structure. |
| PDF → HTML| `pdf_oxide` | Via `pdf_oxide::api::Pdf::open()` + `to_html(page)`. Best-effort. |
| Typst → PDF | `typst` + `typst-pdf` + `typst-assets` | Full compilation pipeline. Requires `--features typst`. |
| Typst → HTML | `typst` + `typst-html` + `typst-assets` | Full compilation pipeline. Requires `--features typst`. |

**No system binaries** (`md2html`, `pdftotext`, `firefox`, etc.) are used.
Every converter is a Rust crate dependency:

```toml
[dependencies]
pulldown-cmark = "0.13"                      # MD → HTML
markdown2pdf = "1.6"                         # MD → PDF (single crate, full pipeline)
printpdf = { version = "0.9", features = ["html"] }  # HTML → PDF
html2md = "0.2"                              # HTML → MD
pdf_oxide = "0.3"                            # PDF → MD / PDF → HTML
ratatui = "0.29"                             # TUI framework
crossterm = "0.28"                           # Terminal backend
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "6"
tempfile = "3"                               # Tempfiles for intermediate conversions

# Typst support (optional, behind "typst" feature)
typst = { version = "0.15", optional = true }
typst-pdf = { version = "0.15", optional = true }
typst-html = { version = "0.15", optional = true }
typst-assets = { version = "0.15", optional = true }
typst-layout = { version = "0.15", optional = true }
typst-library = { version = "0.15", optional = true }
```

## pdf_oxide API (verified)

For PDF reading, use the high-level `pdf_oxide::api::Pdf` struct:

```rust
use pdf_oxide::api::Pdf;

let mut doc = Pdf::open(path)?;
let page_count = doc.page_count()?;

// Extract from each page:
let text = doc.to_text(page_index)?;      // plain text
let md = doc.to_markdown(page_index)?;    // markdown (best-effort)
let html = doc.to_html(page_index)?;      // HTML (best-effort)
```

Do NOT use internal types like `AutoExtractor` or `TextExtractor` directly.
The `Pdf` API is the stable, high-level interface.

## markdown2pdf feature flags

`markdown2pdf` has optional features that are **not needed for local file conversion**:
- `fetch` — enables fetching remote resources (URLs in markdown)
- `svg` — enables SVG rasterization

For lazymd (local files only), the default feature set is sufficient.
