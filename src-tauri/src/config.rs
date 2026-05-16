use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub font_size: u32,
    pub always_on_top: bool,
    #[serde(default)]
    pub word_wrap: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub titlebar_color: String,
    #[serde(default = "default_fill")]
    pub titlebar_fill: u8,
    #[serde(default)]
    pub password_protected: bool,
    #[serde(default)]
    pub password_salt: String,
    #[serde(default = "default_lock_timeout")]
    pub lock_timeout_minutes: u32,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default)]
    pub start_with_windows: bool,
}

fn default_fill() -> u8 {
    100
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_lock_timeout() -> u32 {
    10
}

fn default_font_family() -> String {
    "Cascadia Code".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 300,
            height: 400,
            left: 100,
            top: 100,
            font_size: 14,
            always_on_top: true,
            word_wrap: false,
            theme: default_theme(),
            titlebar_color: String::new(),
            titlebar_fill: default_fill(),
            password_protected: false,
            password_salt: String::new(),
            lock_timeout_minutes: default_lock_timeout(),
            font_family: default_font_family(),
            start_with_windows: false,
        }
    }
}

fn config_path() -> PathBuf {
    crate::paths::exe_dir().join(format!("{}.config", crate::paths::exe_stem()))
}

pub fn exists() -> bool {
    config_path().exists()
}

pub fn load() -> Config {
    let path = config_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Config::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.width, 300);
        assert_eq!(cfg.height, 400);
        assert_eq!(cfg.font_size, 14);
        assert!(cfg.always_on_top);
    }

    #[test]
    fn test_roundtrip() {
        let cfg = Config {
            width: 1280,
            height: 720,
            left: 50,
            top: 100,
            font_size: 20,
            always_on_top: false,
            word_wrap: true,
            theme: "light".to_string(),
            titlebar_color: "#ff6b6b".to_string(),
            titlebar_fill: 80,
            password_protected: true,
            password_salt: "aabbccdd".to_string(),
            lock_timeout_minutes: 15,
            font_family: "Inter".to_string(),
            start_with_windows: false,
        };
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.width, 1280);
        assert_eq!(restored.height, 720);
        assert_eq!(restored.left, 50);
        assert_eq!(restored.top, 100);
        assert_eq!(restored.font_size, 20);
        assert!(!restored.always_on_top);
        assert!(restored.word_wrap);
        assert_eq!(restored.theme, "light");
        assert_eq!(restored.titlebar_color, "#ff6b6b");
        assert_eq!(restored.titlebar_fill, 80);
        assert!(restored.password_protected);
        assert_eq!(restored.password_salt, "aabbccdd");
        assert_eq!(restored.lock_timeout_minutes, 15);
        assert_eq!(restored.font_family, "Inter");
    }

    #[test]
    fn test_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!("a-note-test-config-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.config");

        let cfg = Config {
            width: 800,
            height: 600,
            left: 200,
            top: 300,
            font_size: 18,
            always_on_top: true,
            word_wrap: true,
            theme: "light".to_string(),
            titlebar_color: "#3498db".to_string(),
            titlebar_fill: 90,
            password_protected: true,
            password_salt: "deadbeef".to_string(),
            lock_timeout_minutes: 30,
            font_family: "Roboto".to_string(),
            start_with_windows: false,
        };
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        crate::util::write(&path, &json).unwrap();

        let read_back = std::fs::read_to_string(&path).unwrap();
        let restored: Config = serde_json::from_str(&read_back).unwrap();
        assert_eq!(restored.width, 800);
        assert_eq!(restored.height, 600);
        assert_eq!(restored.left, 200);
        assert_eq!(restored.top, 300);
        assert_eq!(restored.font_size, 18);
        assert!(restored.always_on_top);
        assert_eq!(restored.theme, "light");
        assert_eq!(restored.titlebar_color, "#3498db");
        assert_eq!(restored.titlebar_fill, 90);
        assert!(restored.password_protected);
        assert_eq!(restored.password_salt, "deadbeef");
        assert_eq!(restored.lock_timeout_minutes, 30);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_corrupt_file_returns_default() {
        let dir = std::env::temp_dir().join(format!("a-note-test-config-corrupt-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.config");
        std::fs::write(&path, "not valid json").unwrap();

        let result: Config = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        assert_eq!(result.width, Config::default().width);
        assert_eq!(result.font_size, Config::default().font_size);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_theme_default_dark_when_missing() {
        let old_json = r#"{"width":300,"height":400,"left":100,"top":100,"font_size":14,"always_on_top":true,"word_wrap":false}"#;
        let restored: Config = serde_json::from_str(old_json).unwrap();
        assert_eq!(restored.theme, "dark");
    }

    #[test]
    fn test_password_defaults_when_missing() {
        let old_json = r#"{"width":300,"height":400,"left":100,"top":100,"font_size":14,"always_on_top":true,"word_wrap":false,"theme":"dark"}"#;
        let restored: Config = serde_json::from_str(old_json).unwrap();
        assert!(!restored.password_protected);
        assert_eq!(restored.password_salt, "");
        assert_eq!(restored.lock_timeout_minutes, 10);
    }

    #[test]
    fn test_titlebar_defaults_when_missing() {
        let old_json = r#"{"width":300,"height":400,"left":100,"top":100,"font_size":14,"always_on_top":true,"word_wrap":false,"theme":"dracula"}"#;
        let restored: Config = serde_json::from_str(old_json).unwrap();
        assert_eq!(restored.titlebar_color, "");
        assert_eq!(restored.titlebar_fill, 100);
    }

    #[test]
    fn test_word_wrap_default_false_when_missing() {
        let old_json = r#"{"width":300,"height":400,"left":100,"top":100,"font_size":14,"always_on_top":true}"#;
        let restored: Config = serde_json::from_str(old_json).unwrap();
        assert!(!restored.word_wrap);
    }

    // ── Password config field persistence ──────────────────────────

    #[test]
    fn test_password_fields_persist_through_save_load() {
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-config-pwd-persist-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.config");

        let cfg = Config {
            password_protected: true,
            password_salt: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6".to_string(),
            lock_timeout_minutes: 15,
            ..Config::default()
        };

        let json = serde_json::to_string_pretty(&cfg).unwrap();
        crate::util::write(&path, &json).unwrap();

        let read_back = std::fs::read_to_string(&path).unwrap();
        let restored: Config = serde_json::from_str(&read_back).unwrap();
        assert!(restored.password_protected);
        assert_eq!(restored.password_salt, "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6");
        assert_eq!(restored.lock_timeout_minutes, 15);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_old_config_without_password_defaults() {
        // Config saved before encryption feature existed — no password fields
        let old_json = r#"{
            "width": 400,
            "height": 500,
            "left": 50,
            "top": 60,
            "font_size": 16,
            "always_on_top": true,
            "word_wrap": false,
            "theme": "dracula"
        }"#;
        let restored: Config = serde_json::from_str(old_json).unwrap();
        assert!(!restored.password_protected);
        assert_eq!(restored.password_salt, "");
        assert_eq!(restored.lock_timeout_minutes, 10);
    }

    #[test]
    fn test_password_fields_roundtrip_with_other_fields() {
        let cfg = Config {
            width: 800,
            height: 600,
            left: 100,
            top: 200,
            font_size: 20,
            always_on_top: false,
            word_wrap: true,
            theme: "nord".to_string(),
            titlebar_color: "#ff6b6b".to_string(),
            titlebar_fill: 75,
            password_protected: true,
            password_salt: "deadbeef010203040506070809101112".to_string(),
            lock_timeout_minutes: 30,
            font_family: "Fira Code".to_string(),
            start_with_windows: false,
        };
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.width, 800);
        assert_eq!(restored.theme, "nord");
        assert!(restored.password_protected);
        assert_eq!(restored.password_salt, "deadbeef010203040506070809101112");
        assert_eq!(restored.lock_timeout_minutes, 30);
        assert_eq!(restored.font_family, "Fira Code");
    }

    #[test]
    fn test_font_family_default_when_missing() {
        let old_json = r#"{"width":300,"height":400,"left":100,"top":100,"font_size":14,"always_on_top":true}"#;
        let restored: Config = serde_json::from_str(old_json).unwrap();
        assert_eq!(restored.font_family, "Cascadia Code");
    }

    #[test]
    fn test_font_family_backward_compat() {
        let old_json = r#"{"width":300,"height":400,"left":100,"top":100,"font_size":14,"always_on_top":true,"word_wrap":false,"theme":"dark"}"#;
        let restored: Config = serde_json::from_str(old_json).unwrap();
        assert_eq!(restored.font_family, "Cascadia Code");
    }

    #[test]
    fn test_font_family_persists_through_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-font-persist-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("font-config.json");

        let cfg = Config {
            font_family: "Fira Code".to_string(),
            ..Config::default()
        };
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        crate::util::write(&path, &json).unwrap();

        let read_back = std::fs::read_to_string(&path).unwrap();
        let restored: Config = serde_json::from_str(&read_back).unwrap();
        assert_eq!(restored.font_family, "Fira Code");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_font_family_default_on_corrupt_json() {
        // Corrupt JSON → serde error → and_then chain returns default Config
        // This simulates the config::load() fallback path
        let corrupt = "not valid json at all";
        let result: Result<Config, _> = serde_json::from_str(corrupt);
        assert!(result.is_err());
        // The fallback is Config::default() which has font_family = "Cascadia Code"
        let fallback = Config::default();
        assert_eq!(fallback.font_family, "Cascadia Code");
    }

    #[test]
    fn test_start_with_windows_default_false() {
        let cfg = Config::default();
        assert!(!cfg.start_with_windows);
    }

    #[test]
    fn test_start_with_windows_roundtrip() {
        let cfg = Config {
            start_with_windows: true,
            ..Config::default()
        };
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert!(restored.start_with_windows);
    }

    #[test]
    fn test_start_with_windows_missing_in_json_defaults_false() {
        let old_json = r#"{"width":300,"height":400,"left":100,"top":100,"font_size":14,"always_on_top":true}"#;
        let restored: Config = serde_json::from_str(old_json).unwrap();
        assert!(!restored.start_with_windows);
    }
}
