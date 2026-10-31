## Typst feature gate

Typst support is **optional** and behind the `typst` feature flag. Build with:

```sh
cargo build --features typst
# or via just:
just build --features typst
```

Without the feature, `.typ` files are still recognized and previewable (as plain
text), but conversion to PDF/HTML will return an error message instructing the
user to enable the feature.

### Typst compilation pipeline

Typst → PDF/HTML uses the full `typst` compiler crate (v0.15). The pipeline is:

1. Implement the `World` trait (provides library, fonts, source files)
2. Call `typst::compile::<PagedDocument>(&world)` for PDF, or
   `typst::compile::<HtmlDocument>(&world)` for HTML
3. Export via `typst_pdf::pdf()` or `typst_html::html()`

Fonts are provided by `typst-assets` (embedded Libertine, New Computer Modern,
etc.). No system font configuration is needed.

The `World` implementation in `converter.rs` (`typst_impl` module) is minimal:
- Single source file (`main.typ`)
- No external file dependencies
- Current date via `SystemTime`
- All embedded fonts from `typst_assets::fonts()`
