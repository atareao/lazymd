use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use ratatui::style::Color;

/// User-facing settings persisted to TOML.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub struct Settings {
    pub last_directory: Option<PathBuf>,
    pub theme: Option<String>,
}

/// A set of colors used throughout the TUI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub focus_border: Color,
    pub directory: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub default_fg: Color,
    pub dim_fg: Color,
}

impl Theme {
    /// Dark theme (default).
    pub const fn dark() -> Self {
        Self {
            focus_border: Color::Cyan,
            directory: Color::Yellow,
            selection_bg: Color::Cyan,
            selection_fg: Color::Black,
            default_fg: Color::Reset,
            dim_fg: Color::DarkGray,
        }
    }

    /// Light theme — higher contrast.
    pub const fn light() -> Self {
        Self {
            focus_border: Color::Blue,
            directory: Color::Magenta,
            selection_bg: Color::Blue,
            selection_fg: Color::White,
            default_fg: Color::Reset,
            dim_fg: Color::Gray,
        }
    }

    /// Dracula-inspired theme.
    pub const fn dracula() -> Self {
        Self {
            focus_border: Color::Rgb(189, 147, 249), // purple
            directory: Color::Rgb(80, 250, 123),     // green
            selection_bg: Color::Rgb(189, 147, 249), // purple
            selection_fg: Color::Black,
            default_fg: Color::Reset,
            dim_fg: Color::Rgb(98, 114, 164), // comment gray
        }
    }

    /// Solarized light theme.
    pub const fn solarized() -> Self {
        Self {
            focus_border: Color::Rgb(38, 139, 210), // blue
            directory: Color::Rgb(181, 137, 0),     // yellow
            selection_bg: Color::Rgb(38, 139, 210),
            selection_fg: Color::Rgb(253, 246, 227), // base3
            default_fg: Color::Reset,
            dim_fg: Color::Rgb(147, 161, 161), // base1
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Map a theme string to a Theme struct.
pub fn theme_from_string(name: &str) -> Theme {
    match name.to_lowercase().as_str() {
        "light" => Theme::light(),
        "dracula" => Theme::dracula(),
        "solarized" => Theme::solarized(),
        _ => Theme::dark(),
    }
}

/// Discover the config directory via XDG conventions.
#[allow(dead_code)]
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lazymd")
}

/// Load settings from disk, or return defaults.
#[allow(dead_code)]
pub fn load_settings() -> Settings {
    let path = config_dir().join("config.toml");
    if let Ok(content) = std::fs::read_to_string(&path) {
        toml::from_str(&content).unwrap_or_default()
    } else {
        Settings::default()
    }
}

/// Save settings to disk.
#[allow(dead_code)]
pub fn save_settings(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let content = toml::to_string_pretty(settings)?;
    std::fs::write(dir.join("config.toml"), content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Settings defaults
    // -----------------------------------------------------------------------

    #[test]
    fn test_settings_default() {
        let s = Settings::default();
        assert!(s.last_directory.is_none(), "last_directory should be None");
        assert!(s.theme.is_none(), "theme should be None");
    }

    #[test]
    fn test_settings_default_via_derive() {
        let s: Settings = Default::default();
        assert_eq!(s.last_directory, None);
        assert_eq!(s.theme, None);
    }

    #[test]
    fn test_settings_partial_eq_derived() {
        let a = Settings {
            last_directory: Some(PathBuf::from("/tmp")),
            theme: Some("dark".into()),
        };
        let b = Settings {
            last_directory: Some(PathBuf::from("/tmp")),
            theme: Some("dark".into()),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_settings_not_equal() {
        let a = Settings {
            last_directory: None,
            theme: Some("light".into()),
        };
        let b = Settings {
            last_directory: None,
            theme: Some("dark".into()),
        };
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // config_dir()
    // -----------------------------------------------------------------------

    #[test]
    fn test_config_dir_is_absolute() {
        let dir = config_dir();
        assert!(
            dir.is_absolute(),
            "config_dir() should return an absolute path"
        );
    }

    #[test]
    fn test_config_dir_ends_with_lazymd() {
        let dir = config_dir();
        let file_name = dir
            .file_name()
            .expect("config_dir() should have a final component");
        assert_eq!(file_name, "lazymd", "config_dir() should end with 'lazymd'");
    }

    #[test]
    fn test_config_dir_parent_is_config() {
        let dir = config_dir();
        let parent = dir.parent().expect("config_dir() should have a parent");
        // On Linux: $XDG_CONFIG_HOME or ~/.config
        // On macOS: ~/Library/Application Support
        // On Windows: AppData/Roaming
        // We only check it's a valid, non-empty parent directory.
        assert!(!parent.as_os_str().is_empty(), "parent should not be empty");
    }

    // -----------------------------------------------------------------------
    // TOML serialization round-trip
    // -----------------------------------------------------------------------

    /// Helper: serialise Settings to TOML, then deserialise it back.
    fn roundtrip_toml(settings: &Settings) -> Settings {
        let toml_str =
            toml::to_string_pretty(settings).expect("serializing Settings to TOML should not fail");
        toml::from_str(&toml_str).expect("deserializing Settings from TOML should not fail")
    }

    #[test]
    fn test_toml_roundtrip_default() {
        let original = Settings::default();
        let deserialised = roundtrip_toml(&original);
        assert_eq!(original, deserialised);
    }

    #[test]
    fn test_toml_roundtrip_with_last_directory() {
        let original = Settings {
            last_directory: Some(PathBuf::from("/home/user/projects")),
            theme: None,
        };
        let deserialised = roundtrip_toml(&original);
        assert_eq!(original, deserialised);
    }

    #[test]
    fn test_toml_roundtrip_with_theme() {
        let original = Settings {
            last_directory: None,
            theme: Some("dracula".into()),
        };
        let deserialised = roundtrip_toml(&original);
        assert_eq!(original, deserialised);
    }

    #[test]
    fn test_toml_roundtrip_with_both() {
        let original = Settings {
            last_directory: Some(PathBuf::from("/var/log")),
            theme: Some("monokai".into()),
        };
        let deserialised = roundtrip_toml(&original);
        assert_eq!(original, deserialised);
    }

    #[test]
    fn test_toml_roundtrip_with_windows_path() {
        let original = Settings {
            last_directory: Some(PathBuf::from(r"C:\Users\test\Documents")),
            theme: Some("light".into()),
        };
        let deserialised = roundtrip_toml(&original);
        assert_eq!(original, deserialised);
    }

    #[test]
    fn test_toml_roundtrip_with_unicode_theme() {
        let original = Settings {
            last_directory: Some(PathBuf::from("/home/用户/文档")),
            theme: Some("自定义主题".into()),
        };
        let deserialised = roundtrip_toml(&original);
        assert_eq!(original, deserialised);
    }

    // -----------------------------------------------------------------------
    // TOML output format checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_toml_output_has_correct_field_names() {
        let s = Settings {
            last_directory: Some(PathBuf::from("/tmp")),
            theme: None,
        };
        let toml_str = toml::to_string_pretty(&s).expect("TOML serialization");
        assert!(
            toml_str.contains("last_directory"),
            "TOML should contain 'last_directory'"
        );
        assert!(
            !toml_str.contains("theme"),
            "TOML should not contain 'theme' when None"
        );
    }

    #[test]
    fn test_toml_output_theme_present() {
        let s = Settings {
            last_directory: None,
            theme: Some("solarized".into()),
        };
        let toml_str = toml::to_string_pretty(&s).expect("TOML serialization");
        assert!(toml_str.contains("theme"), "TOML should contain 'theme'");
        assert!(
            toml_str.contains("solarized"),
            "TOML should contain the value 'solarized'"
        );
    }

    #[test]
    fn test_toml_output_empty_settings() {
        let s = Settings::default();
        let toml_str = toml::to_string_pretty(&s).expect("TOML serialization");
        // TOML for a struct with no set fields and serde defaults may be empty or near-empty.
        // This is valid TOML; deserializing an empty string should give defaults.
        let deserialised: Settings = toml::from_str(&toml_str).unwrap_or_default();
        assert_eq!(s, deserialised);
    }

    // -----------------------------------------------------------------------
    // TOML deserialisation from hand-written strings
    // -----------------------------------------------------------------------

    #[test]
    fn test_toml_parse_last_directory_only() {
        let toml_str = r#"last_directory = "/home/user/docs""#;
        let s: Settings = toml::from_str(toml_str).expect("valid TOML");
        assert_eq!(s.last_directory, Some(PathBuf::from("/home/user/docs")));
        assert_eq!(s.theme, None);
    }

    #[test]
    fn test_toml_parse_theme_only() {
        let toml_str = r#"theme = "nord""#;
        let s: Settings = toml::from_str(toml_str).expect("valid TOML");
        assert_eq!(s.last_directory, None);
        assert_eq!(s.theme, Some("nord".into()));
    }

    #[test]
    fn test_toml_parse_invalid_is_default() {
        let toml_str = r#"<<< NOT TOML >>>"#;
        let s: Settings = toml::from_str(toml_str).unwrap_or_default();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn test_toml_parse_wrong_type_field() {
        // If somehow a bool gets in, toml::from_str fails -> we want unwrap_or_default
        let toml_str = r#"theme = 42"#;
        let s: Settings = toml::from_str(toml_str).unwrap_or_default();
        assert_eq!(s, Settings::default());
    }

    // -----------------------------------------------------------------------
    // File-based save / load via temporary directory
    //
    // These tests replicate the exact same logic as save_settings() /
    // load_settings() but against a TempDir so we don't pollute the real
    // XDG config directory.  This validates that the serialisation + file I/O
    // pipeline works correctly end-to-end.
    // -----------------------------------------------------------------------

    /// Save settings to a specific TOML file (same logic as `save_settings`).
    fn save_settings_to(settings: &Settings, path: &Path) {
        let dir = path.parent().expect("path must have a parent directory");
        std::fs::create_dir_all(dir).expect("creating config dir should succeed");
        let content =
            toml::to_string_pretty(settings).expect("serializing Settings should not fail");
        std::fs::write(path, content).expect("writing config file should succeed");
    }

    /// Load settings from a specific TOML file (same logic as `load_settings`).
    fn load_settings_from(path: &Path) -> Settings {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }

    #[test]
    fn test_file_save_and_load_default() {
        let tmp = TempDir::new().expect("creating temp dir");
        let config_path = tmp.path().join("lazymd").join("config.toml");

        let original = Settings::default();
        save_settings_to(&original, &config_path);
        assert!(config_path.exists(), "config.toml should have been created");

        let loaded = load_settings_from(&config_path);
        assert_eq!(original, loaded);
    }

    #[test]
    fn test_file_save_and_load_with_last_directory() {
        let tmp = TempDir::new().expect("creating temp dir");
        let config_path = tmp.path().join("lazymd").join("config.toml");

        let original = Settings {
            last_directory: Some(PathBuf::from("/home/user/projects/my-markdown")),
            theme: None,
        };
        save_settings_to(&original, &config_path);
        let loaded = load_settings_from(&config_path);
        assert_eq!(original, loaded);
    }

    #[test]
    fn test_file_save_and_load_with_theme() {
        let tmp = TempDir::new().expect("creating temp dir");
        let config_path = tmp.path().join("lazymd").join("config.toml");

        let original = Settings {
            last_directory: None,
            theme: Some("ayu-dark".into()),
        };
        save_settings_to(&original, &config_path);
        let loaded = load_settings_from(&config_path);
        assert_eq!(original, loaded);
    }

    #[test]
    fn test_file_save_and_load_with_both_fields() {
        let tmp = TempDir::new().expect("creating temp dir");
        let config_path = tmp.path().join("lazymd").join("config.toml");

        let original = Settings {
            last_directory: Some(PathBuf::from("/tmp/markdown-files")),
            theme: Some("catppuccin".into()),
        };
        save_settings_to(&original, &config_path);
        let loaded = load_settings_from(&config_path);
        assert_eq!(original, loaded);
    }

    #[test]
    fn test_file_save_and_load_overwrites() {
        let tmp = TempDir::new().expect("creating temp dir");
        let config_path = tmp.path().join("lazymd").join("config.toml");

        // Write first set of settings
        let first = Settings {
            last_directory: Some(PathBuf::from("/first")),
            theme: Some("theme-a".into()),
        };
        save_settings_to(&first, &config_path);
        assert_eq!(load_settings_from(&config_path), first);

        // Overwrite with different settings
        let second = Settings {
            last_directory: Some(PathBuf::from("/second")),
            theme: Some("theme-b".into()),
        };
        save_settings_to(&second, &config_path);
        assert_eq!(load_settings_from(&config_path), second);
    }

    #[test]
    fn test_file_load_missing_returns_defaults() {
        let tmp = TempDir::new().expect("creating temp dir");
        let config_path = tmp.path().join("lazymd").join("config.toml");

        // File does not exist yet -> defaults
        let loaded = load_settings_from(&config_path);
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn test_file_load_from_nonexistent_directory() {
        let tmp = TempDir::new().expect("creating temp dir");
        let config_path = tmp.path().join("does-not-exist").join("config.toml");

        let loaded = load_settings_from(&config_path);
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn test_file_save_creates_directory() {
        let tmp = TempDir::new().expect("creating temp dir");
        let config_path = tmp
            .path()
            .join("deeply")
            .join("nested")
            .join("lazymd")
            .join("config.toml");

        let original = Settings {
            last_directory: Some(PathBuf::from("/some/dir")),
            theme: None,
        };
        save_settings_to(&original, &config_path);
        assert!(
            config_path.exists(),
            "save should create intermediate directories"
        );
        let loaded = load_settings_from(&config_path);
        assert_eq!(original, loaded);
    }

    // -----------------------------------------------------------------------
    // load_settings() when real config file does not exist
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_settings_returns_defaults_when_no_file() {
        // This exercises the real load_settings() path.  Since there is
        // (presumably) no ~/.config/lazymd/config.toml during CI, it
        // will return defaults.  If someone runs tests locally and DOES
        // have a config file, this test just validates it can be parsed.
        let settings = load_settings();
        // It should always succeed without panicking, and at minimum the
        // fields should be valid (they may be Some or None depending on
        // the local environment, but we can at least verify the type).
        let _ = settings.last_directory;
        let _ = settings.theme;
    }
}
