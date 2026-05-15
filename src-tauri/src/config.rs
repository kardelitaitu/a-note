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
}

fn default_fill() -> u8 {
    100
}

fn default_theme() -> String {
    "dark".to_string()
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
        }
    }
}

fn exe_stem() -> String {
    std::env::current_exe()
        .expect("failed to get exe path")
        .file_stem()
        .expect("failed to get exe stem")
        .to_string_lossy()
        .to_string()
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .expect("failed to get exe path")
        .parent()
        .expect("failed to get exe parent")
        .to_path_buf()
}

fn config_path() -> PathBuf {
    exe_dir().join(format!("{}.config", exe_stem()))
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

pub fn save(config: &Config) {
    if let Ok(json) = serde_json::to_string_pretty(config) {
        crate::util::write(&config_path(), &json);
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
        };
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        crate::util::write(&path, &json);

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
}
