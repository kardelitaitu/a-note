use crate::crypto;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Plaintext Note (Tauri command interface) ──────────────────────────────

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

// ── On-disk NoteFile (supports both plaintext + encrypted) ──────────────

/// The on-disk representation of a note file.
///
/// **Unencrypted** (backward compatible with existing `Note` format):
/// ```json
/// { "encrypted": false, "text": "...", "cursor_pos": 0, "scroll_top": 0 }
/// ```
///
/// **Encrypted:**
/// ```json
/// { "encrypted": true, "nonce_hex": "...", "ciphertext_hex": "...",
///   "cursor_pos": 0, "scroll_top": 0 }
/// ```
///
/// Existing legacy files (without `encrypted` field) deserialize as
/// `encrypted: false` via serde default, preserving full backward compat.
#[derive(Debug, Serialize, Deserialize)]
pub struct NoteFile {
    /// Whether the content is AES-256-GCM encrypted.
    #[serde(default)]
    pub encrypted: bool,

    /// Hex-encoded 12-byte nonce (present when encrypted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce_hex: Option<String>,

    /// Hex-encoded ciphertext (present when encrypted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ciphertext_hex: Option<String>,

    /// Plaintext content (present when NOT encrypted).
    /// Skipped when empty so encrypted files stay clean.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,

    /// Cursor position — preserved in plaintext even when encrypted,
    /// so it can be restored after unlock.
    #[serde(default)]
    pub cursor_pos: u32,

    /// Scroll offset — same reasoning as cursor_pos.
    #[serde(default)]
    pub scroll_top: u32,
}

impl NoteFile {
    /// Create an encrypted `NoteFile` by encrypting the note's text
    /// with the given derived key.
    ///
    /// `cursor_pos` and `scroll_top` are stored in plaintext so they
    /// survive the lock/unlock cycle.
    pub fn from_encrypted(note: &Note, key: &[u8; 32]) -> Result<Self, String> {
        let (nonce, ciphertext) = crypto::encrypt(&note.text, key)?;
        Ok(NoteFile {
            encrypted: true,
            nonce_hex: Some(hex::encode(&nonce)),
            ciphertext_hex: Some(hex::encode(&ciphertext)),
            text: String::new(),
            cursor_pos: note.cursor_pos,
            scroll_top: note.scroll_top,
        })
    }

    /// Decrypt this `NoteFile` back into a plaintext `Note`.
    ///
    /// If `encrypted` is `false`, returns directly from the stored fields.
    pub fn decrypt_to_note(&self, key: &[u8; 32]) -> Result<Note, String> {
        if !self.encrypted {
            return Ok(Note {
                text: self.text.clone(),
                cursor_pos: self.cursor_pos,
                scroll_top: self.scroll_top,
            });
        }
        let nonce_hex = self
            .nonce_hex
            .as_deref()
            .ok_or_else(|| "missing nonce_hex in encrypted note".to_string())?;
        let ct_hex = self
            .ciphertext_hex
            .as_deref()
            .ok_or_else(|| "missing ciphertext_hex in encrypted note".to_string())?;
        let nonce =
            hex::decode(nonce_hex).map_err(|e| format!("invalid nonce hex: {e}"))?;
        let ciphertext =
            hex::decode(ct_hex).map_err(|e| format!("invalid ciphertext hex: {e}"))?;
        let plaintext = crypto::decrypt(&ciphertext, &nonce, key)?;
        Ok(Note {
            text: plaintext,
            cursor_pos: self.cursor_pos,
            scroll_top: self.scroll_top,
        })
    }
}

/// Load a `NoteFile` from disk. Returns a default unencrypted file if
/// the path doesn't exist or is corrupt.
pub fn load_file() -> NoteFile {
    let path = note_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<NoteFile>(&s).ok())
            .unwrap_or_else(|| NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: String::new(),
                cursor_pos: 0,
                scroll_top: 0,
            })
    } else {
        NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        }
    }
}

/// Save a `NoteFile` to disk.
pub fn save_file(nf: &NoteFile) {
    if let Ok(json) = serde_json::to_string_pretty(nf) {
        crate::util::write(&note_path(), &json);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{derive_key, generate_salt};

    // ── Note tests (existing) ──────────────────────────────────────────

    fn test_note() -> Note {
        Note {
            text: "hello world".to_string(),
            cursor_pos: 4,
            scroll_top: 0,
        }
    }

    #[test]
    fn test_note_roundtrip() {
        let note = test_note();
        let json = serde_json::to_string_pretty(&note).unwrap();
        let restored: Note = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, "hello world");
        assert_eq!(restored.cursor_pos, 4);
    }

    #[test]
    fn test_note_empty_text() {
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
    fn test_note_multiline_text() {
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
    fn test_note_backward_compat_no_cursor_scroll() {
        let old_json = r#"{"text":"old note"}"#;
        let restored: Note = serde_json::from_str(old_json).unwrap();
        assert_eq!(restored.text, "old note");
        assert_eq!(restored.cursor_pos, 0);
        assert_eq!(restored.scroll_top, 0);
    }

    // ── NoteFile backward compat ──────────────────────────────────────

    #[test]
    fn test_notefile_legacy_format_deserializes() {
        // Old format: no "encrypted" field → defaults to false
        let legacy = r#"{"text":"legacy note","cursor_pos":3,"scroll_top":5}"#;
        let nf: NoteFile = serde_json::from_str(legacy).unwrap();
        assert!(!nf.encrypted);
        assert_eq!(nf.text, "legacy note");
        assert_eq!(nf.cursor_pos, 3);
        assert_eq!(nf.scroll_top, 5);
        assert!(nf.nonce_hex.is_none());
        assert!(nf.ciphertext_hex.is_none());
    }

    #[test]
    fn test_notefile_minimal_legacy_format() {
        // Even more minimal: just text
        let legacy = r#"{"text":"minimal"}"#;
        let nf: NoteFile = serde_json::from_str(legacy).unwrap();
        assert!(!nf.encrypted);
        assert_eq!(nf.text, "minimal");
        assert_eq!(nf.cursor_pos, 0);
        assert_eq!(nf.scroll_top, 0);
    }

    #[test]
    fn test_notefile_unencrypted_roundtrip() {
        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: "hello".to_string(),
            cursor_pos: 2,
            scroll_top: 0,
        };
        let json = serde_json::to_string_pretty(&nf).unwrap();
        let restored: NoteFile = serde_json::from_str(&json).unwrap();
        assert!(!restored.encrypted);
        assert_eq!(restored.text, "hello");
        assert_eq!(restored.cursor_pos, 2);
        assert!(restored.nonce_hex.is_none());
    }

    #[test]
    fn test_notefile_unencrypted_serializes_without_crypto_fields() {
        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: "plain".to_string(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let json = serde_json::to_string_pretty(&nf).unwrap();
        // Must NOT contain crypto fields
        assert!(!json.contains("nonce_hex"));
        assert!(!json.contains("ciphertext_hex"));
        // Must contain standard fields
        assert!(json.contains("\"text\""));
        assert!(json.contains("\"cursor_pos\""));
    }

    // ── NoteFile encrypt / decrypt ────────────────────────────────────

    #[test]
    fn test_notefile_encrypt_decrypt_roundtrip() {
        let note = Note {
            text: "secret note content".to_string(),
            cursor_pos: 7,
            scroll_top: 3,
        };
        let salt = generate_salt();
        let key = derive_key("password123", &salt).unwrap();
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();

        assert!(nf.encrypted);
        assert!(nf.nonce_hex.is_some());
        assert!(nf.ciphertext_hex.is_some());
        assert!(nf.text.is_empty()); // encrypted → no plaintext
        assert_eq!(nf.cursor_pos, 7);
        assert_eq!(nf.scroll_top, 3);

        let decrypted = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, "secret note content");
        assert_eq!(decrypted.cursor_pos, 7);
        assert_eq!(decrypted.scroll_top, 3);
    }

    #[test]
    fn test_notefile_decrypt_wrong_key_fails() {
        let note = Note {
            text: "very secret".to_string(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let salt = generate_salt();
        let good_key = derive_key("correct", &salt).unwrap();
        let wrong_key = derive_key("wrong", &salt).unwrap();
        let nf = NoteFile::from_encrypted(&note, &good_key).unwrap();

        let result = nf.decrypt_to_note(&wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_notefile_decrypt_unencrypted_returns_directly() {
        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: "not encrypted at all".to_string(),
            cursor_pos: 10,
            scroll_top: 5,
        };
        // Key doesn't matter — it won't be used
        let key = [0u8; 32];
        let note = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(note.text, "not encrypted at all");
        assert_eq!(note.cursor_pos, 10);
        assert_eq!(note.scroll_top, 5);
    }

    #[test]
    fn test_notefile_encrypt_empty_text() {
        let note = Note {
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let salt = generate_salt();
        let key = derive_key("pwd", &salt).unwrap();
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let decrypted = nf.decrypt_to_note(&key).unwrap();
        assert!(decrypted.text.is_empty());
    }

    #[test]
    fn test_notefile_encrypted_serialization_format() {
        let note = Note {
            text: "hi".to_string(),
            cursor_pos: 1,
            scroll_top: 0,
        };
        let salt = generate_salt();
        let key = derive_key("pwd", &salt).unwrap();
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let json = serde_json::to_string_pretty(&nf).unwrap();

        // Encrypted file must have crypto fields
        assert!(json.contains("\"encrypted\": true"));
        assert!(json.contains("\"nonce_hex\""));
        assert!(json.contains("\"ciphertext_hex\""));
        // Must NOT contain plaintext "text" field
        assert!(!json.contains("\"text\""));
        // Must preserve cursor/scroll
        assert!(json.contains("\"cursor_pos\": 1"));
        assert!(json.contains("\"scroll_top\": 0"));
    }

    // ── load_file / save_file (file I/O) ───────────────────────────────

    #[test]
    fn test_load_file_nonexistent_returns_default() {
        // Redirect note_path to a temp dir
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-load-nonexistent-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("nonexistent.notes");
        // We can't easily mock note_path(), so we test the deserialization logic
        // directly: missing file → default
        let nf = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<NoteFile>(&s).ok())
                .unwrap_or_else(|| NoteFile {
                    encrypted: false,
                    nonce_hex: None,
                    ciphertext_hex: None,
                    text: String::new(),
                    cursor_pos: 0,
                    scroll_top: 0,
                })
        } else {
            NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: String::new(),
                cursor_pos: 0,
                scroll_top: 0,
            }
        };
        assert!(!nf.encrypted);
        assert!(nf.text.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_file_roundtrip_unencrypted() {
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-save-unenc-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.notes");

        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: "roundtrip content".to_string(),
            cursor_pos: 5,
            scroll_top: 2,
        };
        let json = serde_json::to_string_pretty(&nf).unwrap();
        crate::util::write(&path, &json);

        let read_back = std::fs::read_to_string(&path).unwrap();
        let restored: NoteFile = serde_json::from_str(&read_back).unwrap();
        assert!(!restored.encrypted);
        assert_eq!(restored.text, "roundtrip content");
        assert_eq!(restored.cursor_pos, 5);
        assert_eq!(restored.scroll_top, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_file_roundtrip_encrypted() {
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-save-enc-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.notes");

        let note = Note {
            text: "encrypted on disk".to_string(),
            cursor_pos: 3,
            scroll_top: 1,
        };
        let salt = generate_salt();
        let key = derive_key("sekret", &salt).unwrap();
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let json = serde_json::to_string_pretty(&nf).unwrap();
        crate::util::write(&path, &json);

        let read_back = std::fs::read_to_string(&path).unwrap();
        let restored: NoteFile = serde_json::from_str(&read_back).unwrap();
        assert!(restored.encrypted);
        assert!(restored.nonce_hex.is_some());
        assert!(restored.ciphertext_hex.is_some());
        assert!(restored.text.is_empty());

        let decrypted = restored.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, "encrypted on disk");
        assert_eq!(decrypted.cursor_pos, 3);
        assert_eq!(decrypted.scroll_top, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_legacy_file_is_valid_notefile() {
        // Simulate an existing legacy file on disk
        let legacy_json = r#"{"text":"legacy","cursor_pos":2,"scroll_top":0}"#;
        let nf: NoteFile = serde_json::from_str(legacy_json).unwrap();
        // Must pass decrypt_to_note without needing a key
        let key = [0u8; 32];
        let note = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(note.text, "legacy");
        assert_eq!(note.cursor_pos, 2);
    }

    #[test]
    fn test_notefile_missing_crypto_fields_on_encrypted_fails() {
        let bad = r#"{"encrypted":true,"cursor_pos":0,"scroll_top":0}"#;
        let nf: NoteFile = serde_json::from_str(bad).unwrap();
        let key = [0u8; 32];
        let result = nf.decrypt_to_note(&key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing nonce_hex"));
    }

    // ── Unicode content through NoteFile ──────────────────────────────

    #[test]
    fn test_notefile_unicode_roundtrip() {
        let texts = [
            "Hello, 世界!",
            "🚀 Rust 🔒 AES-256-GCM",
            "¿Cómo estás? 你好",
            "αβγδεζηθικλμνξοπρσςτυφχψω",
            "🎉🌟💯🔥👋\nLine 2",
        ];
        let salt = generate_salt();
        let key = derive_key("unicode-pwd", &salt).unwrap();
        for text in &texts {
            let note = Note {
                text: text.to_string(),
                cursor_pos: text.len() as u32 / 2,
                scroll_top: 1,
            };
            let nf = NoteFile::from_encrypted(&note, &key).unwrap();
            let decrypted = nf.decrypt_to_note(&key).unwrap();
            assert_eq!(decrypted.text, *text);
            assert_eq!(decrypted.cursor_pos, note.cursor_pos);
        }
    }

    #[test]
    fn test_notefile_unicode_in_plaintext() {
        let text = "💾 Saved! · Guardado · 保存しました";
        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: text.to_string(),
            cursor_pos: 10,
            scroll_top: 0,
        };
        let json = serde_json::to_string_pretty(&nf).unwrap();
        let restored: NoteFile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, text);
    }

    // ── Large content through NoteFile ────────────────────────────────

    #[test]
    fn test_notefile_large_encrypted_roundtrip() {
        let large = "The quick brown fox jumps over the lazy dog.\n".repeat(2500);
        let note = Note {
            text: large.clone(),
            cursor_pos: 42,
            scroll_top: 100,
        };
        let salt = generate_salt();
        let key = derive_key("large-key", &salt).unwrap();
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let decrypted = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, large);
        assert_eq!(decrypted.cursor_pos, 42);
        assert_eq!(decrypted.scroll_top, 100);
    }

    #[test]
    fn test_notefile_very_large_plaintext_roundtrip() {
        let very_large = "Line\n".repeat(50_000);
        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: very_large.clone(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let json = serde_json::to_string_pretty(&nf).unwrap();
        let restored: NoteFile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text.len(), very_large.len());
        assert_eq!(restored.text, very_large);
    }

    // ── Migration: encrypted → unencrypted ────────────────────────────

    #[test]
    fn test_notefile_encrypted_to_plaintext_migration() {
        let salt = generate_salt();
        let key = derive_key("migrate-pwd", &salt).unwrap();
        let note = Note {
            text: "will be plaintext soon".to_string(),
            cursor_pos: 5,
            scroll_top: 2,
        };

        // Step 1: Encrypt
        let nf_enc = NoteFile::from_encrypted(&note, &key).unwrap();
        assert!(nf_enc.encrypted);

        // Step 2: Decrypt to Note
        let decrypted = nf_enc.decrypt_to_note(&key).unwrap();

        // Step 3: Create plaintext NoteFile
        let nf_plain = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: decrypted.text.clone(),
            cursor_pos: decrypted.cursor_pos,
            scroll_top: decrypted.scroll_top,
        };
        assert!(!nf_plain.encrypted);
        assert!(nf_plain.nonce_hex.is_none());
        assert_eq!(nf_plain.text, "will be plaintext soon");
        assert_eq!(nf_plain.cursor_pos, 5);

        // Step 4: Verify plaintext NoteFile can be read directly
        let direct = nf_plain.decrypt_to_note(&[0u8; 32]).unwrap();
        assert_eq!(direct.text, "will be plaintext soon");
    }

    #[test]
    fn test_notefile_empty_password_migration() {
        // Empty password → encrypt → decrypt with same empty password
        let salt = generate_salt();
        let key = derive_key("", &salt).unwrap();
        let note = Note {
            text: "empty pwd test".to_string(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        // Re-derive key from same empty password
        let key2 = derive_key("", &salt).unwrap();
        let decrypted = nf.decrypt_to_note(&key2).unwrap();
        assert_eq!(decrypted.text, "empty pwd test");
    }

    // ── Multiple sequential encrypt/decrypt on same NoteFile key ──────

    #[test]
    fn test_notefile_sequential_encrypt_decrypt() {
        let salt = generate_salt();
        let key = derive_key("sequential", &salt).unwrap();
        for i in 0..30 {
            let text = format!("note content iteration {i}");
            let note = Note {
                text,
                cursor_pos: i,
                scroll_top: i * 2,
            };
            let nf = NoteFile::from_encrypted(&note, &key).unwrap();
            let decrypted = nf.decrypt_to_note(&key).unwrap();
            assert_eq!(decrypted.cursor_pos, i);
            assert_eq!(decrypted.scroll_top, i * 2);
        }
    }

    // ── Corrupted / edge case NoteFile JSON ───────────────────────────

    #[test]
    fn test_notefile_corrupt_json_falls_back_to_default() {
        // This tests the deserialization path used by note::load_file
        let corrupt = r#"{"encrypted":true,"nonce_hex":"not-hex","ciphertext_hex":"00"}"#;
        let result: Result<NoteFile, _> = serde_json::from_str(corrupt);
        // nonce_hex is a String, so "not-hex" is valid JSON — it deserializes fine
        // The validation happens at decrypt time
        assert!(result.is_ok());
        let nf = result.unwrap();
        assert!(nf.encrypted);
        // Decryption should fail because nonce is invalid hex
        let key = [0u8; 32];
        let decrypted = nf.decrypt_to_note(&key);
        assert!(decrypted.is_err());
    }

    #[test]
    fn test_notefile_encrypted_but_empty_fields_fails() {
        let nf = NoteFile {
            encrypted: true,
            nonce_hex: Some(String::new()),
            ciphertext_hex: Some("00".to_string()),
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let key = [0u8; 32];
        let result = nf.decrypt_to_note(&key);
        // hex::decode("") returns Ok([]), then crypto::decrypt rejects
        // non-12-byte nonce with a proper Err (not a panic).
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonce"));
    }

    #[test]
    fn test_notefile_encrypted_with_invalid_hex_fails() {
        let nf = NoteFile {
            encrypted: true,
            nonce_hex: Some("zzzz".to_string()), // not valid hex
            ciphertext_hex: Some("00".to_string()),
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let key = [0u8; 32];
        let result = nf.decrypt_to_note(&key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonce hex"));
    }

    #[test]
    fn test_notefile_encrypted_with_only_one_field_fails() {
        // nonce present but ciphertext missing
        let nf = NoteFile {
            encrypted: true,
            nonce_hex: Some("aabbccddeeff001122334455".to_string()),
            ciphertext_hex: None,
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let key = [0u8; 32];
        let result = nf.decrypt_to_note(&key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ciphertext_hex"));
    }

    // ── Null bytes through NoteFile ──────────────────────────────

    #[test]
    fn test_notefile_null_bytes_encrypted() {
        let salt = generate_salt();
        let key = derive_key("null-test", &salt).unwrap();
        let text = "before\x00middle\x00after";
        let note = Note {
            text: text.to_string(),
            cursor_pos: 7,
            scroll_top: 0,
        };
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let decrypted = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, text);
        assert!(decrypted.text.contains('\0'));
    }

    // ── All-zero key through NoteFile ───────────────────────────

    #[test]
    fn test_notefile_zero_key_roundtrip() {
        let key = [0u8; 32];
        let note = Note {
            text: "zero key file test".to_string(),
            cursor_pos: 5,
            scroll_top: 2,
        };
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let decrypted = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, "zero key file test");
        assert_eq!(decrypted.cursor_pos, 5);
    }

    // ── Wrong-length nonce through NoteFile ─────────────────────

    #[test]
    fn test_notefile_wrong_length_nonce_fails_gracefully() {
        // Nonce that decodes to wrong length (not 12 bytes)
        let nf = NoteFile {
            encrypted: true,
            nonce_hex: Some("aabbccdd".to_string()), // 4 bytes after decode
            ciphertext_hex: Some("00".to_string()),
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let key = [0u8; 32];
        let result = nf.decrypt_to_note(&key);
        assert!(result.is_err());
    }

    // ── load_file / save_file with temp files ───────────────────

    #[test]
    fn test_save_file_then_load_file_encrypted_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-save-load-enc-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("sticky.notes");

        // Temporarily redirect — we can't mock note_path(), so we write
        // directly to the path that save_file would use.
        let salt = generate_salt();
        let key = derive_key("save-load", &salt).unwrap();
        let note = Note {
            text: "save then load".to_string(),
            cursor_pos: 4,
            scroll_top: 1,
        };
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();

        // Write using save_file logic
        let json = serde_json::to_string_pretty(&nf).unwrap();
        std::fs::write(&path, &json).unwrap();

        // Read back using load_file logic
        let read_back = std::fs::read_to_string(&path).unwrap();
        let restored: NoteFile = serde_json::from_str(&read_back).unwrap();
        let decrypted = restored.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, "save then load");
        assert_eq!(decrypted.cursor_pos, 4);
        assert_eq!(decrypted.scroll_top, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_file_then_load_file_unencrypted_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-save-load-plain-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("sticky.notes");

        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: "plain save load".to_string(),
            cursor_pos: 6,
            scroll_top: 3,
        };
        let json = serde_json::to_string_pretty(&nf).unwrap();
        std::fs::write(&path, &json).unwrap();

        let read_back = std::fs::read_to_string(&path).unwrap();
        let restored: NoteFile = serde_json::from_str(&read_back).unwrap();
        assert!(!restored.encrypted);
        assert_eq!(restored.text, "plain save load");
        assert_eq!(restored.cursor_pos, 6);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_file_nonexistent_returns_default_notefile() {
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-load-nonexistent2-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("does-not-exist.notes");

        // Simulate load_file logic for missing file
        let nf = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<NoteFile>(&s).ok())
                .unwrap_or_else(|| NoteFile {
                    encrypted: false,
                    nonce_hex: None,
                    ciphertext_hex: None,
                    text: String::new(),
                    cursor_pos: 0,
                    scroll_top: 0,
                })
        } else {
            NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: String::new(),
                cursor_pos: 0,
                scroll_top: 0,
            }
        };
        assert!(!nf.encrypted);
        assert!(nf.text.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Corrupt file on disk ────────────────────────────────────

    #[test]
    fn test_load_file_corrupt_returns_default() {
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-load-corrupt-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("corrupt.notes");

        std::fs::write(&path, "not valid json at all").unwrap();

        let nf = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<NoteFile>(&s).ok())
                .unwrap_or_else(|| NoteFile {
                    encrypted: false,
                    nonce_hex: None,
                    ciphertext_hex: None,
                    text: String::new(),
                    cursor_pos: 0,
                    scroll_top: 0,
                })
        } else {
            NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: String::new(),
                cursor_pos: 0,
                scroll_top: 0,
            }
        };
        assert!(!nf.encrypted);
        assert!(nf.text.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Encrypted NoteFile with empty text serialization ────────

    #[test]
    fn test_encrypted_notefile_omits_text_field() {
        let salt = generate_salt();
        let key = derive_key("no-text-field", &salt).unwrap();
        let note = Note {
            text: "this should not appear as plaintext".to_string(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let json = serde_json::to_string(&nf).unwrap();
        // The plaintext "text" field must NOT be present in serialized output
        assert!(!json.contains("\"text\":"));
    }

    // ── Encrypted → plaintext migration (remove_password flow) ─────

    #[test]
    fn test_remove_password_migration_format() {
        // Simulate what remove_password does:
        // 1. Encrypted NoteFile → decrypt to Note
        // 2. Write as plaintext NoteFile (encrypted: false)
        let salt = generate_salt();
        let key = derive_key("migrate-test", &salt).unwrap();

        // Start with encrypted note
        let note = Note {
            text: "secret content".to_string(),
            cursor_pos: 5,
            scroll_top: 2,
        };
        let enc = NoteFile::from_encrypted(&note, &key).unwrap();

        // Decrypt (step 1 of remove_password)
        let decrypted = enc.decrypt_to_note(&key).unwrap();

        // Write as plaintext NoteFile (step 2 of remove_password)
        let plain = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: decrypted.text,
            cursor_pos: decrypted.cursor_pos,
            scroll_top: decrypted.scroll_top,
        };

        // Verify the format: must have encrypted:false and text field
        let json = serde_json::to_string(&plain).unwrap();
        assert!(json.contains("\"encrypted\":false"));
        assert!(json.contains("\"text\":\"secret content\""));
        assert!(json.contains("\"cursor_pos\":5"));
        assert!(json.contains("\"scroll_top\":2"));
        // Must NOT have crypto fields
        assert!(!json.contains("nonce_hex"));
        assert!(!json.contains("ciphertext_hex"));

        // Verify it loads back correctly via load_note path
        let loaded: NoteFile = serde_json::from_str(&json).unwrap();
        assert!(!loaded.encrypted);
        let result = loaded.decrypt_to_note(&[0u8; 32]).unwrap();
        assert_eq!(result.text, "secret content");
        assert_eq!(result.cursor_pos, 5);
        assert_eq!(result.scroll_top, 2);
    }

    #[test]
    fn test_remove_password_migration_file_io() {
        // Full file I/O simulation: encrypted → file → remove password → file → load
        let dir = std::env::temp_dir().join(format!(
            "a-note-test-remove-pwd-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("sticky.notes");

        let salt = generate_salt();
        let key = derive_key("file-migrate", &salt).unwrap();

        // 1. Create and save encrypted NoteFile
        let note = Note {
            text: "will lose encryption".to_string(),
            cursor_pos: 10,
            scroll_top: 3,
        };
        let enc = NoteFile::from_encrypted(&note, &key).unwrap();
        let json_enc = serde_json::to_string_pretty(&enc).unwrap();
        std::fs::write(&path, &json_enc).unwrap();

        // 2. Read back and decrypt (simulate remove_password)
        let read_back = std::fs::read_to_string(&path).unwrap();
        let loaded_enc: NoteFile = serde_json::from_str(&read_back).unwrap();
        assert!(loaded_enc.encrypted);
        let decrypted = loaded_enc.decrypt_to_note(&key).unwrap();

        // 3. Write as plaintext NoteFile
        let plain = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: decrypted.text,
            cursor_pos: decrypted.cursor_pos,
            scroll_top: decrypted.scroll_top,
        };
        let json_plain = serde_json::to_string_pretty(&plain).unwrap();
        std::fs::write(&path, &json_plain).unwrap();

        // 4. Read back — must be plaintext
        let final_read = std::fs::read_to_string(&path).unwrap();
        let loaded_plain: NoteFile = serde_json::from_str(&final_read).unwrap();
        assert!(!loaded_plain.encrypted);
        assert_eq!(loaded_plain.text, "will lose encryption");
        assert_eq!(loaded_plain.cursor_pos, 10);
        assert_eq!(loaded_plain.scroll_top, 3);
        assert!(loaded_plain.nonce_hex.is_none());
        assert!(loaded_plain.ciphertext_hex.is_none());

        // 5. Verify it can be read by old Note::load format too (backward compat)
        let as_note: Note = serde_json::from_str(&final_read).unwrap();
        assert_eq!(as_note.text, "will lose encryption");
        assert_eq!(as_note.cursor_pos, 10);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_remove_password_migration_empty_note() {
        // Edge case: empty note after removing password
        let salt = generate_salt();
        let key = derive_key("empty-migrate", &salt).unwrap();

        let note = Note {
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let enc = NoteFile::from_encrypted(&note, &key).unwrap();
        let decrypted = enc.decrypt_to_note(&key).unwrap();

        let plain = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: decrypted.text,
            cursor_pos: decrypted.cursor_pos,
            scroll_top: decrypted.scroll_top,
        };

        let json = serde_json::to_string(&plain).unwrap();
        // Empty "text" field is skipped by skip_serializing_if
        // only encrypted + cursor_pos + scroll_top are present
        assert!(!json.contains("\"text\":"));
        let loaded: NoteFile = serde_json::from_str(&json).unwrap();
        assert!(!loaded.encrypted);
        assert!(loaded.text.is_empty());
        assert_eq!(loaded.cursor_pos, 0);
        assert_eq!(loaded.scroll_top, 0);
        assert!(loaded.nonce_hex.is_none());
        assert!(loaded.ciphertext_hex.is_none());
    }

    #[test]
    fn test_remove_password_migration_unicode() {
        let salt = generate_salt();
        let key = derive_key("unicode-migrate", &salt).unwrap();

        let text = "Removed 🔒 password — 密码已移除";
        let note = Note {
            text: text.to_string(),
            cursor_pos: 12,
            scroll_top: 1,
        };
        let enc = NoteFile::from_encrypted(&note, &key).unwrap();
        let decrypted = enc.decrypt_to_note(&key).unwrap();

        let plain = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: decrypted.text,
            cursor_pos: decrypted.cursor_pos,
            scroll_top: decrypted.scroll_top,
        };

        let json = serde_json::to_string(&plain).unwrap();
        let loaded: NoteFile = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.text, text);
        assert_eq!(loaded.cursor_pos, 12);
    }

    // ── NoteFile edge cases ────────────────────────────────────────

    #[test]
    fn test_notefile_plaintext_with_encrypted_flag_false() {
        // What happens if someone manually sets encrypted: false but also
        // includes nonce/ciphertext fields? Should ignore crypto fields.
        let json = r#"{"encrypted":false,"nonce_hex":"aa","ciphertext_hex":"bb","text":"normal","cursor_pos":3,"scroll_top":1}"#;
        let nf: NoteFile = serde_json::from_str(json).unwrap();
        assert!(!nf.encrypted);
        assert_eq!(nf.text, "normal");
        // decrypt_to_note should return text directly, ignoring crypto fields
        let note = nf.decrypt_to_note(&[0u8; 32]).unwrap();
        assert_eq!(note.text, "normal");
        assert_eq!(note.cursor_pos, 3);
    }

    #[test]
    fn test_notefile_all_default_fields() {
        // Minimal JSON with no fields at all
        let json = r#"{}"#;
        let nf: NoteFile = serde_json::from_str(json).unwrap();
        assert!(!nf.encrypted);
        assert!(nf.text.is_empty());
        assert_eq!(nf.cursor_pos, 0);
        assert_eq!(nf.scroll_top, 0);
        assert!(nf.nonce_hex.is_none());
        assert!(nf.ciphertext_hex.is_none());
    }

    #[test]
    fn test_notefile_only_text_field() {
        // Even more minimal: just text
        let json = r#"{"text":"just text"}"#;
        let nf: NoteFile = serde_json::from_str(json).unwrap();
        assert!(!nf.encrypted);
        assert_eq!(nf.text, "just text");
        let note = nf.decrypt_to_note(&[0u8; 32]).unwrap();
        assert_eq!(note.text, "just text");
    }

    #[test]
    fn test_note_to_notefile_and_back() {
        // Roundtrip: Note → NoteFile (plain) → Note
        let original = Note {
            text: "roundtrip test".to_string(),
            cursor_pos: 7,
            scroll_top: 4,
        };
        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: original.text.clone(),
            cursor_pos: original.cursor_pos,
            scroll_top: original.scroll_top,
        };
        let json = serde_json::to_string(&nf).unwrap();
        let restored_nf: NoteFile = serde_json::from_str(&json).unwrap();
        let restored = restored_nf.decrypt_to_note(&[0u8; 32]).unwrap();
        assert_eq!(restored.text, "roundtrip test");
        assert_eq!(restored.cursor_pos, 7);
        assert_eq!(restored.scroll_top, 4);
    }

    // ── Note edge cases ─────────────────────────────────────────────

    #[test]
    fn test_note_very_long_text() {
        let text = "A".repeat(100_000);
        let note = Note {
            text,
            cursor_pos: 50_000,
            scroll_top: 1,
        };
        let json = serde_json::to_string(&note).unwrap();
        let restored: Note = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text.len(), 100_000);
        assert_eq!(restored.cursor_pos, 50_000);
    }

    #[test]
    fn test_note_extra_unknown_fields() {
        // Forward compat: extra fields in JSON should be ignored
        let json = r#"{"text":"hello","cursor_pos":3,"scroll_top":1,"unknown_field":"ignored","extra_nested":{"a":1}}"#;
        let restored: Note = serde_json::from_str(json).unwrap();
        assert_eq!(restored.text, "hello");
        assert_eq!(restored.cursor_pos, 3);
        assert_eq!(restored.scroll_top, 1);
    }

    #[test]
    fn test_note_large_values() {
        let note = Note {
            text: "test".to_string(),
            cursor_pos: u32::MAX,
            scroll_top: u32::MAX,
        };
        let json = serde_json::to_string(&note).unwrap();
        let restored: Note = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.cursor_pos, u32::MAX);
        assert_eq!(restored.scroll_top, u32::MAX);
    }

    #[test]
    fn test_notefile_explicit_encrypted_false() {
        // NoteFile with explicit encrypted:false should serialize with that field
        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: "explicit".to_string(),
            cursor_pos: 1,
            scroll_top: 0,
        };
        let json = serde_json::to_string(&nf).unwrap();
        assert!(json.contains("\"encrypted\":false"));
        // Should NOT have crypto fields
        assert!(!json.contains("nonce_hex"));
        assert!(!json.contains("ciphertext_hex"));
    }

    #[test]
    fn test_unencrypted_decrypt_preserves_all_fields() {
        let nf = NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: "preserve me".to_string(),
            cursor_pos: 1234,
            scroll_top: 5678,
        };
        let note = nf.decrypt_to_note(&[0u8; 32]).unwrap();
        assert_eq!(note.text, "preserve me");
        assert_eq!(note.cursor_pos, 1234);
        assert_eq!(note.scroll_top, 5678);
    }

    #[test]
    fn test_encrypted_roundtrip_preserves_cursor_scroll() {
        let note = Note {
            text: "cursor preserved".to_string(),
            cursor_pos: 999,
            scroll_top: 888,
        };
        let key = [0xABu8; 32];
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        assert_eq!(nf.cursor_pos, 999);
        assert_eq!(nf.scroll_top, 888);
        assert!(nf.encrypted);
        // Decrypt and check
        let restored = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(restored.text, "cursor preserved");
        assert_eq!(restored.cursor_pos, 999);
        assert_eq!(restored.scroll_top, 888);
    }

    #[test]
    fn test_notefile_serialization_deserialization_full() {
        let note = Note {
            text: "serialize me".to_string(),
            cursor_pos: 7,
            scroll_top: 3,
        };
        let key = [0x42u8; 32];
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let json = serde_json::to_string(&nf).unwrap();
        let restored: NoteFile = serde_json::from_str(&json).unwrap();
        assert!(restored.encrypted);
        assert!(restored.nonce_hex.is_some());
        assert!(restored.ciphertext_hex.is_some());
        assert_eq!(restored.cursor_pos, 7);
        assert_eq!(restored.scroll_top, 3);
        let decrypted = restored.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, "serialize me");
    }
}
