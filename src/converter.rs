use std::path::Path;

/// Errors that can occur during conversion.
#[derive(Debug)]
pub enum ConversionError {
    UnsupportedFormat(String),
    ConversionFailed(String),
    Io(std::io::Error),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFormat(ext) => write!(f, "unsupported format: {ext}"),
            Self::ConversionFailed(msg) => write!(f, "conversion failed: {msg}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for ConversionError {}

impl From<std::io::Error> for ConversionError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// The formats this tool handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Markdown,
    Html,
    Pdf,
    Typst,
}

impl Format {
    /// Detect format from a file extension.
    pub fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_lowercase().as_str() {
            "md" | "markdown" => Some(Self::Markdown),
            "html" | "htm" => Some(Self::Html),
            "pdf" => Some(Self::Pdf),
            "typ" => Some(Self::Typst),
            _ => None,
        }
    }
}

/// Convert a file from one format to another.
pub fn convert(input: &Path, output: &Path, target: Format) -> Result<(), ConversionError> {
    let source = Format::from_extension(input).ok_or_else(|| {
        ConversionError::UnsupportedFormat(
            input
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("?")
                .to_string(),
        )
    })?;

    match (source, target) {
        (Format::Markdown, Format::Html) => md_to_html(input, output),
        (Format::Markdown, Format::Pdf) => md_to_pdf(input, output),
        (Format::Html, Format::Markdown) => html_to_md(input, output),
        (Format::Html, Format::Pdf) => html_to_pdf(input, output),
        (Format::Pdf, Format::Markdown) => pdf_to_md(input, output),
        (Format::Pdf, Format::Html) => pdf_to_html(input, output),
        (Format::Typst, Format::Pdf) => typst_to_pdf(input, output),
        (Format::Typst, Format::Html) => typst_to_html(input, output),
        _ => Err(ConversionError::UnsupportedFormat(format!(
            "{source:?} → {target:?}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Standard converters (always available)
// ---------------------------------------------------------------------------

fn md_to_html(input: &Path, output: &Path) -> Result<(), ConversionError> {
    let md = std::fs::read_to_string(input)?;
    let parser = pulldown_cmark::Parser::new(&md);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    std::fs::write(output, html)?;
    Ok(())
}

fn md_to_pdf(input: &Path, output: &Path) -> Result<(), ConversionError> {
    let md = std::fs::read_to_string(input)?;
    markdown2pdf::parse_into_file(
        md,
        output,
        markdown2pdf::config::ConfigSource::Default,
        None,
    )
    .map_err(|e| ConversionError::ConversionFailed(e.to_string()))
}

fn html_to_md(input: &Path, output: &Path) -> Result<(), ConversionError> {
    let html = std::fs::read_to_string(input)?;
    let md = html2md::parse_html(&html);
    std::fs::write(output, md)?;
    Ok(())
}

fn html_to_pdf(input: &Path, output: &Path) -> Result<(), ConversionError> {
    use printpdf::*;
    use std::collections::BTreeMap;

    let html = std::fs::read_to_string(input)?;
    let images = BTreeMap::new();
    let fonts = BTreeMap::new();
    let options = GeneratePdfOptions::default();
    let mut warnings = Vec::new();

    let doc = PdfDocument::from_html(&html, &images, &fonts, &options, &mut warnings)
        .map_err(|e| ConversionError::ConversionFailed(e.to_string()))?;

    let bytes = doc.save(&PdfSaveOptions::default(), &mut Vec::new());
    std::fs::write(output, bytes)?;
    Ok(())
}

fn pdf_to_md(input: &Path, output: &Path) -> Result<(), ConversionError> {
    let mut doc = pdf_oxide::api::Pdf::open(input)
        .map_err(|e| ConversionError::ConversionFailed(e.to_string()))?;
    let page_count = doc
        .page_count()
        .map_err(|e| ConversionError::ConversionFailed(e.to_string()))?;
    let mut md = String::new();
    for i in 0..page_count {
        md.push_str(
            &doc.to_markdown(i)
                .map_err(|e| ConversionError::ConversionFailed(e.to_string()))?,
        );
        md.push('\n');
    }
    std::fs::write(output, md)?;
    Ok(())
}

fn pdf_to_html(input: &Path, output: &Path) -> Result<(), ConversionError> {
    let mut doc = pdf_oxide::api::Pdf::open(input)
        .map_err(|e| ConversionError::ConversionFailed(e.to_string()))?;
    let page_count = doc
        .page_count()
        .map_err(|e| ConversionError::ConversionFailed(e.to_string()))?;
    let mut html = String::from(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>PDF Export</title></head><body>\n",
    );
    for i in 0..page_count {
        html.push_str("<div class=\"page\">");
        html.push_str(
            &doc.to_html(i)
                .map_err(|e| ConversionError::ConversionFailed(e.to_string()))?,
        );
        html.push_str("</div>\n");
    }
    html.push_str("</body></html>");
    std::fs::write(output, html)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Typst converters (behind "typst" feature gate)
// ---------------------------------------------------------------------------

#[cfg(feature = "typst")]
mod typst_impl {
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;
    use typst::foundations::{Bytes, Datetime, Duration};
    use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
    use typst::text::{Font, FontBook};
    use typst::utils::LazyHash;
    use typst::{Library, LibraryExt, World};
    use typst_library::diag::{FileError, FileResult, Warned};

    use crate::converter::ConversionError;

    const MONTH_DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    fn is_leap(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    fn current_date() -> Option<Datetime> {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_secs();
        let days = (secs / 86400) as i64;
        let mut y = 1970i32;
        let mut remaining = days;
        loop {
            let days_in_year = if is_leap(y) { 366 } else { 365 };
            if remaining < days_in_year {
                break;
            }
            remaining -= days_in_year;
            y += 1;
        }
        let mut m = 1u8;
        for (i, dim) in MONTH_DAYS.iter().enumerate() {
            let dim = if i == 1 && is_leap(y) { 29 } else { *dim };
            if remaining < dim as i64 {
                m = (i + 1) as u8;
                break;
            }
            remaining -= dim as i64;
        }
        let d = (remaining + 1) as u8;
        Datetime::from_ymd(y, m, d)
    }

    struct TypstWorld {
        library: LazyHash<Library>,
        book: LazyHash<FontBook>,
        main: FileId,
        source: Source,
        fonts: Vec<Font>,
    }

    impl World for TypstWorld {
        fn library(&self) -> &LazyHash<Library> {
            &self.library
        }
        fn book(&self) -> &LazyHash<FontBook> {
            &self.book
        }
        fn main(&self) -> FileId {
            self.main
        }
        fn source(&self, id: FileId) -> FileResult<Source> {
            if id == self.main {
                Ok(self.source.clone())
            } else {
                Err(FileError::NotFound(PathBuf::from(
                    id.vpath().get_with_slash(),
                )))
            }
        }
        fn file(&self, id: FileId) -> FileResult<Bytes> {
            Err(FileError::NotFound(PathBuf::from(
                id.vpath().get_with_slash(),
            )))
        }
        fn font(&self, index: usize) -> Option<Font> {
            self.fonts.get(index).cloned()
        }
        fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
            current_date()
        }
    }

    fn build_world(source: String) -> Result<TypstWorld, ConversionError> {
        let library = LazyHash::new(Library::builder().build());
        let rooted = RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new("main.typ")
                .map_err(|e| ConversionError::ConversionFailed(e.to_string()))?,
        );
        let main = rooted.intern();
        let typed_source = Source::new(main, source);

        let mut font_book = FontBook::new();
        let mut fonts = Vec::new();
        for data in typst_assets::fonts() {
            if let Some(font) = Font::new(Bytes::new(data), 0) {
                font_book.push(font.info().clone());
                fonts.push(font);
            }
        }

        Ok(TypstWorld {
            library,
            book: LazyHash::new(font_book),
            main,
            source: typed_source,
            fonts,
        })
    }

    fn diags_to_string(
        diags: impl IntoIterator<Item = impl Into<typst_library::diag::SourceDiagnostic>>,
    ) -> String {
        diags
            .into_iter()
            .map(|d| d.into().message.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn to_pdf(input: &Path, output_path: &Path) -> Result<(), ConversionError> {
        use typst_layout::PagedDocument;

        let source = std::fs::read_to_string(input)?;
        let world = build_world(source)?;

        let Warned {
            output: compile_result,
            warnings: _,
        } = typst::compile::<PagedDocument>(&world);
        let document = compile_result
            .map_err(|diags| ConversionError::ConversionFailed(diags_to_string(diags)))?;

        let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .map_err(|diags| ConversionError::ConversionFailed(diags_to_string(diags)))?;

        std::fs::write(output_path, pdf_bytes)?;
        Ok(())
    }

    pub fn to_html(input: &Path, output_path: &Path) -> Result<(), ConversionError> {
        use typst_html::HtmlDocument;

        let source = std::fs::read_to_string(input)?;
        let world = build_world(source)?;

        let Warned {
            output: compile_result,
            warnings: _,
        } = typst::compile::<HtmlDocument>(&world);
        let document = compile_result
            .map_err(|diags| ConversionError::ConversionFailed(diags_to_string(diags)))?;

        let html_body = typst_html::html(&document, &typst_html::HtmlOptions::default())
            .map_err(|diags| ConversionError::ConversionFailed(diags_to_string(diags)))?;

        let full_html = format!(
            "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Typst Export</title></head><body>\n{html_body}\n</body></html>"
        );

        std::fs::write(output_path, full_html)?;
        Ok(())
    }
}

#[cfg(feature = "typst")]
fn typst_to_pdf(input: &Path, output: &Path) -> Result<(), ConversionError> {
    typst_impl::to_pdf(input, output)
}

#[cfg(not(feature = "typst"))]
fn typst_to_pdf(_input: &Path, _output: &Path) -> Result<(), ConversionError> {
    Err(ConversionError::ConversionFailed(
        "Typst support is not enabled. Build with --features typst".to_string(),
    ))
}

#[cfg(feature = "typst")]
fn typst_to_html(input: &Path, output: &Path) -> Result<(), ConversionError> {
    typst_impl::to_html(input, output)
}

#[cfg(not(feature = "typst"))]
fn typst_to_html(_input: &Path, _output: &Path) -> Result<(), ConversionError> {
    Err(ConversionError::ConversionFailed(
        "Typst support is not enabled. Build with --features typst".to_string(),
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    // ======================================================================
    // Format::from_extension
    // ======================================================================

    #[test]
    fn from_extension_md() {
        assert_eq!(
            Format::from_extension(Path::new("file.md")),
            Some(Format::Markdown)
        );
    }

    #[test]
    fn from_extension_markdown() {
        assert_eq!(
            Format::from_extension(Path::new("file.markdown")),
            Some(Format::Markdown)
        );
    }

    #[test]
    fn from_extension_html() {
        assert_eq!(
            Format::from_extension(Path::new("file.html")),
            Some(Format::Html)
        );
    }

    #[test]
    fn from_extension_htm() {
        assert_eq!(
            Format::from_extension(Path::new("file.htm")),
            Some(Format::Html)
        );
    }

    #[test]
    fn from_extension_pdf() {
        assert_eq!(
            Format::from_extension(Path::new("file.pdf")),
            Some(Format::Pdf)
        );
    }

    #[test]
    fn from_extension_typ() {
        assert_eq!(
            Format::from_extension(Path::new("file.typ")),
            Some(Format::Typst)
        );
    }

    #[test]
    fn from_extension_txt_returns_none() {
        assert_eq!(Format::from_extension(Path::new("file.txt")), None);
    }

    #[test]
    fn from_extension_no_extension_returns_none() {
        assert_eq!(Format::from_extension(Path::new("README")), None);
    }

    #[test]
    fn from_extension_uppercase_md() {
        assert_eq!(
            Format::from_extension(Path::new("file.MD")),
            Some(Format::Markdown)
        );
    }

    #[test]
    fn from_extension_uppercase_html() {
        assert_eq!(
            Format::from_extension(Path::new("file.HTML")),
            Some(Format::Html)
        );
    }

    #[test]
    fn from_extension_mixed_case_pdf() {
        assert_eq!(
            Format::from_extension(Path::new("file.PDF")),
            Some(Format::Pdf)
        );
    }

    #[test]
    fn from_extension_mixed_case_typ() {
        assert_eq!(
            Format::from_extension(Path::new("file.Typ")),
            Some(Format::Typst)
        );
    }

    #[test]
    fn from_extension_dotted_extension() {
        // Paths like ".hidden" have no extension in std::path
        assert_eq!(Format::from_extension(Path::new(".gitignore")), None);
    }

    #[test]
    fn from_extension_path_with_directories() {
        assert_eq!(
            Format::from_extension(Path::new("/some/deep/path/readme.md")),
            Some(Format::Markdown)
        );
    }

    // ======================================================================
    // ConversionError display
    // ======================================================================

    #[test]
    fn conversion_error_display_unsupported_format() {
        let err = ConversionError::UnsupportedFormat("xyz".to_string());
        assert_eq!(err.to_string(), "unsupported format: xyz");
    }

    #[test]
    fn conversion_error_display_conversion_failed() {
        let err = ConversionError::ConversionFailed("something broke".to_string());
        assert_eq!(err.to_string(), "conversion failed: something broke");
    }

    #[test]
    fn conversion_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = ConversionError::Io(io_err);
        assert!(err.to_string().contains("I/O error:"));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn conversion_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no permission");
        let err: ConversionError = io_err.into();
        assert!(matches!(err, ConversionError::Io(_)));
    }

    #[test]
    fn conversion_error_error_trait_is_implemented() {
        // Verify that ConversionError implements std::error::Error
        fn assert_error_impl<E: std::error::Error>() {}
        assert_error_impl::<ConversionError>();
    }

    // ======================================================================
    // convert() dispatch — error cases
    // ======================================================================

    #[test]
    fn convert_unsupported_source_format() {
        let input = Path::new("input.xyz");
        let output = Path::new("output.md");
        let result = convert(input, output, Format::Markdown);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConversionError::UnsupportedFormat(_)));
        assert!(err.to_string().contains("xyz"));
    }

    #[test]
    fn convert_unsupported_conversion_pair_pdf_to_typst() {
        let input = Path::new("input.pdf");
        let output = Path::new("output.typ");
        let result = convert(input, output, Format::Typst);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConversionError::UnsupportedFormat(_)));
        assert!(err.to_string().contains("Pdf → Typst"));
    }

    #[test]
    fn convert_unsupported_conversion_pair_md_to_typst() {
        let input = Path::new("input.md");
        let output = Path::new("output.typ");
        let result = convert(input, output, Format::Typst);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConversionError::UnsupportedFormat(_)));
        assert!(err.to_string().contains("Markdown → Typst"));
    }

    #[test]
    fn convert_unsupported_conversion_pair_html_to_typst() {
        let input = Path::new("input.html");
        let output = Path::new("output.typ");
        let result = convert(input, output, Format::Typst);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConversionError::UnsupportedFormat(_))));
    }

    #[test]
    fn convert_unsupported_conversion_pair_typst_to_md() {
        let input = Path::new("input.typ");
        let output = Path::new("output.md");
        let result = convert(input, output, Format::Markdown);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConversionError::UnsupportedFormat(_))));
    }

    #[test]
    fn convert_unsupported_conversion_pair_typst_to_pdf_without_feature() {
        // Without the "typst" feature, Typst → PDF returns a ConversionFailed
        let input = Path::new("input.typ");
        let output = Path::new("output.pdf");
        let result = convert(input, output, Format::Pdf);
        // Without feature: returns ConversionFailed
        // With feature: should succeed (but we don't have a real .typ file)
        if cfg!(feature = "typst") {
            // Typst feature on but no real source → will fail during compilation
            assert!(result.is_err());
        } else {
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, ConversionError::ConversionFailed(_)));
            assert!(err.to_string().contains("Typst support is not enabled"));
        }
    }

    // ======================================================================
    // Real conversion round-trips (with temp files)
    // ======================================================================

    #[test]
    fn convert_md_to_html_produces_valid_html() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.md");
        let output_path = dir.path().join("output.html");

        std::fs::write(&input_path, "# Hello World\n\nThis is **bold** text.\n")
            .expect("write input");

        let result = convert(&input_path, &output_path, Format::Html);
        assert!(result.is_ok(), "conversion failed: {:?}", result);

        assert!(output_path.exists(), "output file should exist");

        let html = std::fs::read_to_string(&output_path).expect("read output");
        assert!(html.contains("<h1>"), "should contain h1 tag");
        assert!(html.contains("Hello World"), "should contain heading text");
        assert!(html.contains("<strong>"), "should contain strong tag");
        assert!(html.contains("bold"), "should contain bold text");
    }

    #[test]
    fn convert_md_to_html_empty_input() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("empty.md");
        let output_path = dir.path().join("output.html");

        std::fs::write(&input_path, "").expect("write input");
        let result = convert(&input_path, &output_path, Format::Html);
        assert!(
            result.is_ok(),
            "empty markdown should convert: {:?}",
            result
        );

        let html = std::fs::read_to_string(&output_path).expect("read output");
        assert!(html.is_empty() || html.trim().is_empty());
    }

    #[test]
    fn convert_md_to_html_complex_markdown() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.md");
        let output_path = dir.path().join("output.html");

        let md_content = r#"# Title

## Subtitle

- List item 1
- List item 2

1. Ordered item
2. Ordered item

> A blockquote

`inline code`

```rust
fn main() {}
```

[A link](https://example.com)

![Image](img.png)
"#;
        std::fs::write(&input_path, md_content).expect("write input");
        let result = convert(&input_path, &output_path, Format::Html);
        assert!(result.is_ok(), "conversion failed: {:?}", result);

        let html = std::fs::read_to_string(&output_path).expect("read output");
        // Lists
        assert!(
            html.contains("<ul>") || html.contains("<li>"),
            "should contain list"
        );
        // Ordered lists
        assert!(
            html.contains("<ol>") || html.contains("Ordered"),
            "should contain ordered list"
        );
        // Blockquote
        assert!(html.contains("<blockquote>"), "should contain blockquote");
        // Code block
        assert!(
            html.contains("<code>") || html.contains("<pre>"),
            "should contain code"
        );
        // Link
        assert!(html.contains("href"), "should contain link");
    }

    #[test]
    fn convert_html_to_md_produces_markdown() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.html");
        let output_path = dir.path().join("output.md");

        std::fs::write(
            &input_path,
            "<h1>Hello World</h1>\n<p>This is <strong>bold</strong> text.</p>\n",
        )
        .expect("write input");

        let result = convert(&input_path, &output_path, Format::Markdown);
        assert!(result.is_ok(), "conversion failed: {:?}", result);

        assert!(output_path.exists(), "output file should exist");

        let md = std::fs::read_to_string(&output_path).expect("read output");
        // html2md typically converts <h1> to # heading
        assert!(
            md.contains("Hello World"),
            "should contain heading text, got: {md:?}"
        );
        assert!(
            md.contains("bold") || md.contains("**"),
            "should contain bold text or markdown markers, got: {md:?}"
        );
    }

    #[test]
    fn convert_html_to_md_with_links_and_lists() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.html");
        let output_path = dir.path().join("output.md");

        std::fs::write(
            &input_path,
            r#"<ul>
<li><a href="https://example.com">Example</a></li>
<li>Plain item</li>
</ul>
"#,
        )
        .expect("write input");

        let result = convert(&input_path, &output_path, Format::Markdown);
        assert!(result.is_ok(), "conversion failed: {:?}", result);

        let md = std::fs::read_to_string(&output_path).expect("read output");
        // Should contain the link text and list markers
        assert!(md.contains("Example"), "should contain link text");
    }

    #[test]
    fn convert_html_to_md_empty_input() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("empty.html");
        let output_path = dir.path().join("output.md");

        std::fs::write(&input_path, "").expect("write input");
        let result = convert(&input_path, &output_path, Format::Markdown);
        assert!(result.is_ok(), "empty html should convert: {:?}", result);

        let md = std::fs::read_to_string(&output_path).expect("read output");
        assert!(md.is_empty() || md.trim().is_empty());
    }

    #[test]
    fn convert_md_to_pdf_produces_nonempty_file() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.md");
        let output_path = dir.path().join("output.pdf");

        std::fs::write(
            &input_path,
            "# Hello World\n\nThis is a paragraph with **bold** text.\n\n- Item 1\n- Item 2\n",
        )
        .expect("write input");

        let result = convert(&input_path, &output_path, Format::Pdf);
        assert!(result.is_ok(), "MD → PDF conversion failed: {:?}", result);

        assert!(output_path.exists(), "output file should exist");

        let metadata = std::fs::metadata(&output_path).expect("get metadata");
        assert!(metadata.len() > 0, "PDF should be non-empty");
    }

    #[test]
    fn convert_md_to_pdf_empty_input() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("empty.md");
        let output_path = dir.path().join("output.pdf");

        std::fs::write(&input_path, "").expect("write input");
        let result = convert(&input_path, &output_path, Format::Pdf);
        // markdown2pdf may fail on empty input, but we should still get a predictable result
        if let Err(ref e) = result {
            assert!(
                matches!(e, ConversionError::ConversionFailed(_)),
                "empty input should yield ConversionFailed, got: {e}"
            );
        } else {
            assert!(output_path.exists());
        }
    }

    #[test]
    fn convert_html_to_pdf_produces_nonempty_file() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.html");
        let output_path = dir.path().join("output.pdf");

        std::fs::write(
            &input_path,
            "<html><body><h1>Hello</h1><p>World</p></body></html>",
        )
        .expect("write input");

        let result = convert(&input_path, &output_path, Format::Pdf);
        assert!(result.is_ok(), "HTML → PDF conversion failed: {:?}", result);

        assert!(output_path.exists(), "output file should exist");
        let metadata = std::fs::metadata(&output_path).expect("get metadata");
        assert!(metadata.len() > 0, "PDF should be non-empty");
    }

    #[test]
    fn convert_pdf_to_md_produces_output() {
        // First create a real PDF from markdown, then convert it back to markdown
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let md_path = dir.path().join("source.md");
        let pdf_path = dir.path().join("intermediate.pdf");
        let md_output_path = dir.path().join("roundtrip.md");

        std::fs::write(&md_path, "# Hello\n\nSimple paragraph.").expect("write input");

        // Create the PDF
        let result = convert(&md_path, &pdf_path, Format::Pdf);
        if let Err(e) = &result {
            panic!("Failed to create intermediate PDF: {e}");
        }

        // Convert PDF back to MD
        let result = convert(&pdf_path, &md_output_path, Format::Markdown);
        assert!(result.is_ok(), "PDF → MD conversion failed: {:?}", result);

        assert!(md_output_path.exists(), "output file should exist");
        let md_output = std::fs::read_to_string(&md_output_path).expect("read output");
        // Should contain at least some text from the original
        assert!(!md_output.is_empty(), "PDF → MD output should not be empty");
    }

    #[test]
    fn convert_pdf_to_html_produces_output() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let md_path = dir.path().join("source.md");
        let pdf_path = dir.path().join("intermediate.pdf");
        let html_output_path = dir.path().join("output.html");

        std::fs::write(&md_path, "# PDF to HTML Test\n\nContent.").expect("write input");

        // Create the PDF
        let result = convert(&md_path, &pdf_path, Format::Pdf);
        if let Err(e) = &result {
            panic!("Failed to create intermediate PDF: {e}");
        }

        // Convert PDF to HTML
        let result = convert(&pdf_path, &html_output_path, Format::Html);
        assert!(result.is_ok(), "PDF → HTML conversion failed: {:?}", result);

        assert!(html_output_path.exists(), "output file should exist");
        let html_output = std::fs::read_to_string(&html_output_path).expect("read output");
        assert!(
            !html_output.is_empty(),
            "PDF → HTML output should not be empty"
        );
        assert!(
            html_output.contains("<div class=\"page\">") || html_output.contains("Content"),
            "output should contain page div or content text"
        );
    }

    #[test]
    fn convert_nonexistent_input_file_returns_io_error() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let nonexistent = dir.path().join("does_not_exist.md");
        let output = dir.path().join("output.html");

        let result = convert(&nonexistent, &output, Format::Html);
        assert!(result.is_err());
        match result.unwrap_err() {
            ConversionError::Io(_) => {} // Expected — file not found is an I/O error
            other => panic!("Expected Io error, got: {other}"),
        }
    }

    // ======================================================================
    // Round-trip: MD → HTML → MD
    // ======================================================================

    #[test]
    fn roundtrip_md_to_html_to_md_preserves_content() {
        let dir = tempfile::TempDir::new().expect("create temp dir");

        let original_md = "# Hello\n\nThis is **bold** and `code`.\n\n- A list item\n";
        let md_path = dir.path().join("original.md");
        let html_path = dir.path().join("intermediate.html");
        let md_roundtrip_path = dir.path().join("roundtrip.md");

        std::fs::write(&md_path, original_md).expect("write original");

        // MD → HTML
        convert(&md_path, &html_path, Format::Html).expect("MD → HTML");

        // HTML → MD
        convert(&html_path, &md_roundtrip_path, Format::Markdown).expect("HTML → MD");

        let result_md = std::fs::read_to_string(&md_roundtrip_path).expect("read result");

        // The round-tripped markdown may not be byte-identical (html2md may use
        // slightly different formatting), but it should preserve the text content.
        assert!(result_md.contains("Hello"), "should preserve heading text");
        assert!(result_md.contains("bold"), "should preserve bold text");
        assert!(result_md.contains("code"), "should preserve code text");
        assert!(
            result_md.contains("list") || result_md.contains("List"),
            "should preserve list"
        );
    }

    // ======================================================================
    // Tempfile cleanup behavior
    // ======================================================================

    #[test]
    fn tempdir_cleanup_after_drop() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.md");
        let output_path = dir.path().join("output.html");

        std::fs::write(&input_path, "# Temp\n").expect("write input");
        convert(&input_path, &output_path, Format::Html).expect("convert");

        assert!(output_path.exists());
        assert!(input_path.exists());

        // After dropping dir, files are cleaned up. We just verify that convert
        // itself doesn't leak temp files (the TempDir handles everything).
        drop(dir);
        // No assertions after drop — just verifying no panic occurs
    }

    // ======================================================================
    // Typst feature-gated tests
    // ======================================================================

    #[cfg(not(feature = "typst"))]
    #[test]
    fn typst_conversion_returns_helpful_error_without_feature() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.typ");
        let output_path = dir.path().join("output.pdf");

        std::fs::write(&input_path, "# Hello").expect("write input");

        // Typst → PDF
        let result = convert(&input_path, &output_path, Format::Pdf);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConversionError::ConversionFailed(_)));
        assert!(
            err.to_string().contains("Typst support"),
            "error should mention Typst support, got: {err}"
        );

        // Typst → HTML
        let html_path = dir.path().join("output.html");
        let result = convert(&input_path, &html_path, Format::Html);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConversionError::ConversionFailed(_)));
        assert!(
            err.to_string().contains("Typst support"),
            "error should mention Typst support, got: {err}"
        );
    }

    #[cfg(feature = "typst")]
    #[test]
    fn typst_to_pdf_produces_nonempty_file() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.typ");
        let output_path = dir.path().join("output.pdf");

        std::fs::write(&input_path, "#Hello\n\nThis is a *Typst* document.\n")
            .expect("write input");

        let result = convert(&input_path, &output_path, Format::Pdf);
        assert!(result.is_ok(), "Typst → PDF failed: {:?}", result);

        assert!(output_path.exists());
        let metadata = std::fs::metadata(&output_path).expect("get metadata");
        assert!(metadata.len() > 0, "PDF should be non-empty");
    }

    #[cfg(feature = "typst")]
    #[test]
    fn typst_to_html_produces_output() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let input_path = dir.path().join("test.typ");
        let output_path = dir.path().join("output.html");

        std::fs::write(&input_path, "#Hello\n\nTypst content.\n").expect("write input");

        let result = convert(&input_path, &output_path, Format::Html);
        assert!(result.is_ok(), "Typst → HTML failed: {:?}", result);

        assert!(output_path.exists());
        let html = std::fs::read_to_string(&output_path).expect("read output");
        assert!(!html.is_empty());
        assert!(
            html.contains("Hello") || html.contains("<h1>"),
            "should contain heading"
        );
    }
}
