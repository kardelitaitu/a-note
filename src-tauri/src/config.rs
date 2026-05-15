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

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .expect("failed to get exe path")
        .parent()
        .expect("failed to get exe parent")
        .to_path_buf()
}

fn config_path() -> PathBuf {
    exe_dir().join("sticky.config")
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
        let _ = std::fs::write(config_path(), json);
    }
}
