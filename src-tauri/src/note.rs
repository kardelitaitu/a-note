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
        crate::util::write(&note_path(), &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let note = Note {
            text: "hello world".to_string(),
        };
        let json = serde_json::to_string_pretty(&note).unwrap();
        let restored: Note = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, "hello world");
    }

    #[test]
    fn test_empty_text() {
        let note = Note {
            text: String::new(),
        };
        let json = serde_json::to_string_pretty(&note).unwrap();
        let restored: Note = serde_json::from_str(&json).unwrap();
        assert!(restored.text.is_empty());
    }

    #[test]
    fn test_multiline_text() {
        let note = Note {
            text: "line 1\nline 2\nline 3".to_string(),
        };
        let json = serde_json::to_string_pretty(&note).unwrap();
        let restored: Note = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, "line 1\nline 2\nline 3");
    }

    #[test]
    fn test_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!("a-note-test-note-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.notes");

        let json = serde_json::to_string_pretty(&Note {
            text: "persistent note".to_string(),
        })
        .unwrap();
        crate::util::write(&path, &json);

        let read_back = std::fs::read_to_string(&path).unwrap();
        let restored: Note = serde_json::from_str(&read_back).unwrap();
        assert_eq!(restored.text, "persistent note");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_missing_file_returns_empty() {
        let path = std::env::temp_dir().join("a-note-nonexistent.notes");
        let result = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<Note>(&s).ok())
                .map(|n| n.text)
                .unwrap_or_default()
        } else {
            String::new()
        };
        assert!(result.is_empty());
    }
}
