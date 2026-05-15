use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Note {
    pub text: String,
    #[serde(default)]
    pub cursor_pos: u32,
    #[serde(default)]
    pub scroll_top: u32,
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

pub fn load() -> Note {
    let path = note_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Note>(&s).ok())
            .unwrap_or(Note {
                text: String::new(),
                cursor_pos: 0,
                scroll_top: 0,
            })
    } else {
        Note {
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        }
    }
}

pub fn save(note: &Note) {
    if let Ok(json) = serde_json::to_string_pretty(note) {
        crate::util::write(&note_path(), &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_note() -> Note {
        Note {
            text: "hello world".to_string(),
            cursor_pos: 4,
            scroll_top: 0,
        }
    }

    #[test]
    fn test_roundtrip() {
        let note = test_note();
        let json = serde_json::to_string_pretty(&note).unwrap();
        let restored: Note = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, "hello world");
        assert_eq!(restored.cursor_pos, 4);
    }

    #[test]
    fn test_empty_text() {
        let note = Note {
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let json = serde_json::to_string_pretty(&note).unwrap();
        let restored: Note = serde_json::from_str(&json).unwrap();
        assert!(restored.text.is_empty());
        assert_eq!(restored.cursor_pos, 0);
    }

    #[test]
    fn test_multiline_text() {
        let note = Note {
            text: "line 1\nline 2\nline 3".to_string(),
            cursor_pos: 10,
            scroll_top: 20,
        };
        let json = serde_json::to_string_pretty(&note).unwrap();
        let restored: Note = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, "line 1\nline 2\nline 3");
        assert_eq!(restored.cursor_pos, 10);
        assert_eq!(restored.scroll_top, 20);
    }

    #[test]
    fn test_file_roundtrip_with_cursor_scroll() {
        let dir = std::env::temp_dir().join(format!("a-note-test-cursor-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.notes");

        let json = serde_json::to_string_pretty(&Note {
            text: "remember my spot".to_string(),
            cursor_pos: 6,
            scroll_top: 42,
        })
        .unwrap();
        crate::util::write(&path, &json);

        let read_back = std::fs::read_to_string(&path).unwrap();
        let restored: Note = serde_json::from_str(&read_back).unwrap();
        assert_eq!(restored.text, "remember my spot");
        assert_eq!(restored.cursor_pos, 6);
        assert_eq!(restored.scroll_top, 42);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_backward_compat_no_cursor_scroll() {
        let old_json = r#"{"text":"old note"}"#;
        let restored: Note = serde_json::from_str(old_json).unwrap();
        assert_eq!(restored.text, "old note");
        assert_eq!(restored.cursor_pos, 0);
        assert_eq!(restored.scroll_top, 0);
    }

    #[test]
    fn test_missing_file_returns_default_note() {
        let path = std::env::temp_dir().join("a-note-nonexistent.notes");
        let result = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<Note>(&s).ok())
                .unwrap_or(Note {
                    text: String::new(),
                    cursor_pos: 0,
                    scroll_top: 0,
                })
        } else {
            Note {
                text: String::new(),
                cursor_pos: 0,
                scroll_top: 0,
            }
        };
        assert!(result.text.is_empty());
        assert_eq!(result.cursor_pos, 0);
        assert_eq!(result.scroll_top, 0);
    }
}
