use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct Note {
    text: String,
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

fn note_path() -> PathBuf {
    exe_dir().join(format!("{}.notes", exe_stem()))
}

pub fn load() -> String {
    let path = note_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Note>(&s).ok())
            .map(|n| n.text)
            .unwrap_or_default()
    } else {
        String::new()
    }
}

pub fn save(text: &str) {
    if let Ok(json) = serde_json::to_string_pretty(&Note {
        text: text.to_string(),
    }) {
        let _ = std::fs::write(note_path(), json);
    }
}
