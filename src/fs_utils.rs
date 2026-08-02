use std::path::{Path, PathBuf};

/// A filesystem entry shown in the browser panel.
#[derive(Debug, Clone)]
pub enum FsEntry {
    Directory {
        path: PathBuf,
        name: String,
    },
    File {
        path: PathBuf,
        name: String,
        extension: String,
        size: u64,
    },
}

/// List entries in a directory, sorted: directories first, then files, both alphabetically.
pub fn list_entries(path: &Path) -> std::io::Result<Vec<FsEntry>> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_str().unwrap_or("?").to_string();

        if entry.file_type()?.is_dir() {
            dirs.push(FsEntry::Directory { path, name });
        } else if is_supported_file(&path) {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();
            let size = entry.metadata()?.len();
            files.push(FsEntry::File {
                path,
                name,
                extension: ext,
                size,
            });
        }
    }

    dirs.sort_by(|a, b| a.name().cmp(b.name()));
    files.sort_by(|a, b| a.name().cmp(b.name()));

    let mut result: Vec<FsEntry> = dirs;
    result.extend(files);
    Ok(result)
}

/// Returns true if the file has a supported extension (.md, .html, .pdf).
pub fn is_supported_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "markdown" | "html" | "htm" | "pdf" | "typ")
    )
}

/// Format a file size in human-readable form (B, KB, MB).
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

impl FsEntry {
    pub fn name(&self) -> &str {
        match self {
            Self::Directory { name, .. } | Self::File { name, .. } => name,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::Directory { path, .. } | Self::File { path, .. } => path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // is_supported_file tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_supported_md() {
        assert!(is_supported_file(Path::new("readme.md")));
    }

    #[test]
    fn test_supported_markdown() {
        assert!(is_supported_file(Path::new("article.markdown")));
    }

    #[test]
    fn test_supported_html() {
        assert!(is_supported_file(Path::new("index.html")));
    }

    #[test]
    fn test_supported_htm() {
        assert!(is_supported_file(Path::new("page.htm")));
    }

    #[test]
    fn test_supported_pdf() {
        assert!(is_supported_file(Path::new("report.pdf")));
    }

    #[test]
    fn test_supported_typ() {
        assert!(is_supported_file(Path::new("paper.typ")));
    }

    #[test]
    fn test_unsupported_txt() {
        assert!(!is_supported_file(Path::new("notes.txt")));
    }

    #[test]
    fn test_unsupported_rs() {
        assert!(!is_supported_file(Path::new("main.rs")));
    }

    #[test]
    fn test_unsupported_no_extension() {
        assert!(!is_supported_file(Path::new("Makefile")));
    }

    #[test]
    fn test_unsupported_different_case() {
        // Extension matching is case-sensitive — uppercase should not match.
        assert!(!is_supported_file(Path::new("readme.MD")));
        assert!(!is_supported_file(Path::new("index.HTML")));
    }

    #[test]
    fn test_supported_with_path_components() {
        assert!(is_supported_file(Path::new("/home/user/docs/guide.md")));
        assert!(is_supported_file(Path::new("./subdir/page.html")));
    }

    // -----------------------------------------------------------------------
    // FsEntry tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_fs_entry_directory_name() {
        let entry = FsEntry::Directory {
            path: PathBuf::from("/some/path/src"),
            name: "src".to_string(),
        };
        assert_eq!(entry.name(), "src");
    }

    #[test]
    fn test_fs_entry_directory_path() {
        let p = PathBuf::from("/some/path/src");
        let entry = FsEntry::Directory {
            path: p.clone(),
            name: "src".to_string(),
        };
        assert_eq!(entry.path(), p);
    }

    #[test]
    fn test_fs_entry_file_name() {
        let entry = FsEntry::File {
            path: PathBuf::from("/some/path/doc.md"),
            name: "doc.md".to_string(),
            extension: "md".to_string(),
            size: 0,
        };
        assert_eq!(entry.name(), "doc.md");
    }

    #[test]
    fn test_fs_entry_file_path() {
        let p = PathBuf::from("/some/path/doc.md");
        let entry = FsEntry::File {
            path: p.clone(),
            name: "doc.md".to_string(),
            extension: "md".to_string(),
            size: 0,
        };
        assert_eq!(entry.path(), p);
    }

    // -----------------------------------------------------------------------
    // list_entries tests
    // -----------------------------------------------------------------------

    fn create_test_dir() -> TempDir {
        let dir = TempDir::new().expect("failed to create temp dir");
        let root = dir.path();

        // Create supported files
        fs::write(root.join("readme.md"), "# Hello").unwrap();
        fs::write(root.join("index.html"), "<h1>Hi</h1>").unwrap();
        fs::write(root.join("paper.pdf"), "PDF").unwrap();
        fs::write(root.join("doc.typ"), "# Hello").unwrap();

        // Create subdirectories
        fs::create_dir(root.join("images")).unwrap();
        fs::create_dir(root.join("src")).unwrap();

        // Create unsupported files (should be filtered out)
        fs::write(root.join("notes.txt"), "text").unwrap();
        fs::write(root.join("main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("Makefile"), "all:").unwrap();

        dir
    }

    #[test]
    fn test_list_entries_ok() {
        let dir = create_test_dir();
        let result = list_entries(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_entries_dirs_before_files() {
        let dir = create_test_dir();
        let entries = list_entries(dir.path()).unwrap();
        let mut seen_file = false;
        for entry in &entries {
            match entry {
                FsEntry::Directory { .. } => {
                    assert!(!seen_file, "directory after file: {:?}", entry.name());
                }
                FsEntry::File { .. } => {
                    seen_file = true;
                }
            }
        }
    }

    #[test]
    fn test_list_entries_alphabetical_dirs() {
        let dir = create_test_dir();
        let entries = list_entries(dir.path()).unwrap();
        let dir_names: Vec<&str> = entries
            .iter()
            .filter_map(|e| match e {
                FsEntry::Directory { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        // images, src → alphabetical
        assert_eq!(dir_names, vec!["images", "src"]);
    }

    #[test]
    fn test_list_entries_alphabetical_files() {
        let dir = create_test_dir();
        let entries = list_entries(dir.path()).unwrap();
        let file_names: Vec<&str> = entries
            .iter()
            .filter_map(|e| match e {
                FsEntry::File { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        // doc.typ, index.html, paper.pdf, readme.md → alphabetical
        assert_eq!(
            file_names,
            vec!["doc.typ", "index.html", "paper.pdf", "readme.md"]
        );
    }

    #[test]
    fn test_list_entries_filters_unsupported() {
        let dir = create_test_dir();
        let entries = list_entries(dir.path()).unwrap();
        let file_extensions: Vec<&str> = entries
            .iter()
            .filter_map(|e| match e {
                FsEntry::File { extension, .. } => Some(extension.as_str()),
                _ => None,
            })
            .collect();
        // Should not contain "txt", "rs", or no extension
        assert!(!file_extensions.contains(&"txt"));
        assert!(!file_extensions.contains(&"rs"));
        // Should contain only supported extensions
        for ext in &file_extensions {
            assert!(
                ["md", "html", "pdf", "typ"].contains(ext),
                "unexpected extension: {ext}"
            );
        }
    }

    #[test]
    fn test_list_entries_total_count() {
        let dir = create_test_dir();
        let entries = list_entries(dir.path()).unwrap();
        // 2 directories + 4 supported files
        assert_eq!(entries.len(), 6);
    }

    #[test]
    fn test_list_entries_non_existent_path() {
        let result = list_entries(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
    }

    #[test]
    fn test_list_entries_empty_dir() {
        let dir = TempDir::new().expect("failed to create temp dir");
        let entries = list_entries(dir.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_list_entries_only_unsupported() {
        let dir = TempDir::new().expect("failed to create temp dir");
        fs::write(dir.path().join("notes.txt"), "text").unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        let entries = list_entries(dir.path()).unwrap();
        // Directories still appear (none here), but unsupported files are filtered
        assert!(entries.is_empty());
    }

    #[test]
    fn test_list_entries_only_dirs() {
        let dir = TempDir::new().expect("failed to create temp dir");
        fs::create_dir(dir.path().join("aaa")).unwrap();
        fs::create_dir(dir.path().join("bbb")).unwrap();
        let entries = list_entries(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0], FsEntry::Directory { .. }));
        assert!(matches!(entries[1], FsEntry::Directory { .. }));
    }

    #[test]
    fn test_list_entries_only_supported_files() {
        let dir = TempDir::new().expect("failed to create temp dir");
        fs::write(dir.path().join("a.md"), "#").unwrap();
        fs::write(dir.path().join("b.html"), "<br>").unwrap();
        let entries = list_entries(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0], FsEntry::File { .. }));
        assert!(matches!(entries[1], FsEntry::File { .. }));
    }
}
