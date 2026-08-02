use std::path::Path;

/// Render a preview of a file as a string suitable for terminal display.
pub fn preview_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "md" | "markdown" | "typ" => preview_markdown(path),
        "html" | "htm" => preview_html(path),
        "pdf" => preview_pdf(path),
        _ => Err(format!("unsupported file type: .{ext}").into()),
    }
}

fn preview_markdown(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let md = std::fs::read_to_string(path)?;
    // For now, render markdown as plain text.
    // Future: use pulldown-cmark events to add terminal formatting.
    Ok(md)
}

fn preview_html(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let html = std::fs::read_to_string(path)?;
    // Strip HTML tags for terminal display.
    let text = strip_html_tags(&html);
    Ok(text)
}

fn preview_pdf(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut doc = pdf_oxide::api::Pdf::open(path)?;
    let page_count = doc.page_count()?;
    let mut text = String::new();
    for i in 0..page_count {
        text.push_str(&doc.to_text(i)?);
        text.push('\n');
    }
    Ok(text)
}

/// Very basic HTML tag stripper.
fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // strip_html_tags tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_strip_simple_tags() {
        assert_eq!(strip_html_tags("<b>bold</b>"), "bold");
    }

    #[test]
    fn test_strip_nested_tags() {
        assert_eq!(strip_html_tags("<div><p>text</p></div>"), "text");
    }

    #[test]
    fn test_strip_self_closing() {
        assert_eq!(strip_html_tags("<br/>"), "");
        assert_eq!(strip_html_tags("<br />"), "");
        assert_eq!(strip_html_tags("<img src=\"x.png\"/>"), "");
    }

    #[test]
    fn test_strip_with_attributes() {
        assert_eq!(strip_html_tags("<a href=\"link\">text</a>"), "text");
    }

    #[test]
    fn test_strip_no_html() {
        assert_eq!(strip_html_tags("plain text"), "plain text");
    }

    #[test]
    fn test_strip_empty_string() {
        assert_eq!(strip_html_tags(""), "");
    }

    #[test]
    fn test_strip_mixed_content() {
        assert_eq!(strip_html_tags("Hello <b>world</b>!"), "Hello world!");
    }

    #[test]
    fn test_strip_multiline() {
        let html = "<ul>\n  <li>one</li>\n  <li>two</li>\n</ul>";
        assert_eq!(strip_html_tags(html), "\n  one\n  two\n");
    }

    #[test]
    fn test_strip_comment_tags() {
        assert_eq!(
            strip_html_tags("before<!-- comment -->after"),
            "beforeafter"
        );
    }

    #[test]
    fn test_strip_script_style_tags() {
        // The basic stripper doesn't understand <script> or <style> semantics,
        // but we test that it correctly strips all angle-bracket content.
        let html = "<script>var x = 1;</script><p>hello</p>";
        assert_eq!(strip_html_tags(html), "var x = 1;hello");
    }

    #[test]
    fn test_strip_only_tags() {
        assert_eq!(strip_html_tags("<tag></tag>"), "");
    }

    // -----------------------------------------------------------------------
    // preview_file tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_preview_markdown_file() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().join("test.md");
        fs::write(&path, "# Hello\n\nThis is **markdown**.").expect("failed to write");

        let result = preview_file(&path).expect("preview should succeed");
        assert_eq!(result, "# Hello\n\nThis is **markdown**.");
    }

    #[test]
    fn test_preview_markdown_extension() {
        let dir = TempDir::new().expect("failed to create temp dir");

        // .markdown extension
        let path_md = dir.path().join("doc.markdown");
        fs::write(&path_md, "# Doc").expect("failed to write");
        assert_eq!(
            preview_file(&path_md).expect("preview should succeed"),
            "# Doc"
        );

        // .typ extension
        let path_typ = dir.path().join("paper.typ");
        fs::write(&path_typ, "= Paper").expect("failed to write");
        assert_eq!(
            preview_file(&path_typ).expect("preview should succeed"),
            "= Paper"
        );
    }

    #[test]
    fn test_preview_html_file() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().join("test.html");
        fs::write(&path, "<h1>Hello</h1>\n<p>This is <b>HTML</b>.</p>").expect("failed to write");

        let result = preview_file(&path).expect("preview should succeed");
        assert_eq!(result, "Hello\nThis is HTML.");
    }

    #[test]
    fn test_preview_htm_extension() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().join("page.htm");
        fs::write(&path, "<i>text</i>").expect("failed to write");

        let result = preview_file(&path).expect("preview should succeed");
        assert_eq!(result, "text");
    }

    #[test]
    fn test_preview_unsupported_txt() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().join("notes.txt");
        fs::write(&path, "some text").expect("failed to write");

        let err = preview_file(&path).expect_err("should return error for .txt");
        let msg = format!("{err}");
        assert!(
            msg.contains("unsupported"),
            "error message should mention unsupported, got: {msg}"
        );
        assert!(
            msg.contains("txt"),
            "error message should mention extension, got: {msg}"
        );
    }

    #[test]
    fn test_preview_unsupported_rs() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().join("main.rs");
        fs::write(&path, "fn main() {}").expect("failed to write");

        let err = preview_file(&path).expect_err("should return error for .rs");
        let msg = format!("{err}");
        assert!(msg.contains("unsupported"), "got: {msg}");
    }

    #[test]
    fn test_preview_no_extension() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().join("Makefile");
        fs::write(&path, "all:").expect("failed to write");

        let err = preview_file(&path).expect_err("should return error for no extension");
        assert!(format!("{err}").contains("unsupported"));
    }

    #[test]
    fn test_preview_nonexistent_file() {
        let result = preview_file(Path::new("/nonexistent/path/that/does/not/exist.md"));

        let err = result.expect_err("should return error for nonexistent file");
        // Should be an I/O error from std::fs::read_to_string
        assert!(
            err.to_string().contains("No such file") || err.to_string().contains("not found"),
            "error should mention file not found, got: {err}"
        );
    }

    #[test]
    fn test_preview_nonexistent_html() {
        let result = preview_file(Path::new("/nonexistent/file.html"));
        let err = result.expect_err("should return error for nonexistent html file");
        assert!(
            err.to_string().contains("No such file") || err.to_string().contains("not found"),
            "got: {err}"
        );
    }

    #[test]
    fn test_preview_pdf_invalid_content() {
        // A file with .pdf extension but invalid PDF content should return an error
        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().join("fake.pdf");
        fs::write(&path, "this is not a pdf").expect("failed to write");

        let result = preview_file(&path);
        assert!(
            result.is_err(),
            "invalid PDF content should return an error"
        );
    }

    #[test]
    fn test_preview_file_case_sensitivity() {
        // Extension matching is case-insensitive (to_lowercase is used)
        let dir = TempDir::new().expect("failed to create temp dir");

        // Uppercase .MD still works
        let path_md = dir.path().join("README.MD");
        fs::write(&path_md, "# Readme").expect("failed to write");
        assert_eq!(
            preview_file(&path_md).expect("uppercase .MD should work"),
            "# Readme"
        );

        // Uppercase .HTML still works
        let path_html = dir.path().join("INDEX.HTML");
        fs::write(&path_html, "<h1>Index</h1>").expect("failed to write");
        assert_eq!(
            preview_file(&path_html).expect("uppercase .HTML should work"),
            "Index"
        );
    }

    // -----------------------------------------------------------------------
    // preview_pdf direct tests (with a real PDF created via printpdf)
    // -----------------------------------------------------------------------

    #[test]
    fn test_preview_pdf_with_valid_pdf() {
        use printpdf::*;

        let dir = TempDir::new().expect("failed to create temp dir");
        let path = dir.path().join("test.pdf");

        // Create a minimal valid PDF with "Hello World" text using printpdf 0.9 API
        let mut doc = PdfDocument::new("Test");
        let ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point::new(Mm(20.0), Mm(270.0)),
            },
            Op::SetFont {
                font: PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                size: Pt(12.0),
            },
            Op::ShowText {
                items: vec![TextItem::Text("Hello World".to_string())],
            },
            Op::EndTextSection,
        ];
        let page = PdfPage::new(Mm(210.0), Mm(297.0), ops);
        let bytes: Vec<u8> = doc
            .with_pages(vec![page])
            .save(&PdfSaveOptions::default(), &mut Vec::new());
        std::fs::write(&path, bytes).expect("failed to write PDF");

        let result = preview_file(&path).expect("preview of valid PDF should succeed");
        assert!(
            result.contains("Hello World"),
            "PDF text should contain 'Hello World', got: {result:?}"
        );
    }
}
