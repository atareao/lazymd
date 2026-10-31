## Gotchas

- **Edition 2024** (requires rustc ≥ 1.85). `cargo fmt` uses 2024 style. `gen`
  is a reserved keyword. Run `cargo fmt` after any significant edit.
- **ratatui + crossterm**: pin recent stable (`ratatui ≥ 0.28`,
  `crossterm ≥ 0.28`). Same stack as sibling project `lazymagick`.
- **No system tools**. Every converter is a Rust crate. If a crate is missing
  from `Cargo.toml` the build fails — no runtime surprises from missing
  binaries. The tool should give a clear error (not a panic) if a conversion
  fails at the crate level.
- **`markdown2pdf`** es el crate para MD→PDF. No necesita `printpdf` para esa ruta.
  `printpdf` solo se necesita para HTML→PDF directo (con feature `html`).
- **`printpdf` html feature**: requiere el flag `features = ["html"]` o
  `PdfDocument::from_html()` no compila.
- **Testing TUI apps**: test business logic (converter dispatch, file-type
  detection, config parsing) with standard `#[test]`. TUI integration tests
  are low priority.
- **Tempfiles**: conversions that pipe through intermediate files (e.g.,
  MD→PDF via temp HTML) should use `tempfile::NamedTempFile` and clean up
  in all exit paths.
- **PDF preview**: render as text via `pdf_oxide::api::Pdf::open()` → `to_text()`.
  Binary or image preview is out of scope for v1.
- **File-type detection**: use extension (`.md`, `.html`/`.htm`, `.pdf`) plus
  magic-byte fallback (first bytes of PDF: `%PDF`, HTML: `<` or `<!DOCTYPE`).
- **`just`** command runner is available system-wide with a `justfile`.
- **`printpdf` vs `markdown2pdf`**: `markdown2pdf` handles MD→PDF in a single
  crate. `printpdf` (with `html` feature) is only needed for HTML→PDF direct.
  Do NOT use `printpdf` for MD→PDF.
- **pdf_oxide API**: Always use `pdf_oxide::api::Pdf`. Do not use internal
  `AutoExtractor` or `TextExtractor` directly — they're internal and may break
  across versions.
- **Typst World trait**: The `World` implementation in `converter.rs` is minimal
  and only supports single-file `.typ` sources. External file dependencies
  (images, includes) will fail with `FileError::NotFound`.
