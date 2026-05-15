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
        };
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.width, 1280);
        assert_eq!(restored.height, 720);
        assert_eq!(restored.left, 50);
        assert_eq!(restored.top, 100);
        assert_eq!(restored.font_size, 20);
        assert!(!restored.always_on_top);
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
}
