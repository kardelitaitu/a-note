//! Integration tests for the sticky-notes encryption system.
//!
//! These tests exercise the public API of the library crate —
//! salt generation, key derivation, encryption, decryption, and
//! the NoteFile on-disk format — as an end-to-end consumer would.
//!
//! Run with: `cargo test --test encryption`

use sticky_notes_lib::crypto;
use sticky_notes_lib::note::{self, Note, NoteFile};

// ── Full workflow: salt → key → encrypt → file → decrypt ─────────────

#[test]
fn test_full_encryption_workflow() {
    // Simulate the exact flow used by the app:
    // 1. User sets password → generate salt + derive key
    let salt = crypto::generate_salt();
    let salt_hex = hex::encode(salt);
    let password = "my-secure-password";

    let derived_key = crypto::derive_key(password, &salt).unwrap();
    assert_eq!(derived_key.len(), 32);

    // 2. User types note → encrypt it
    let plaintext = "This is my secret sticky note content!";
    let (nonce, ciphertext) = crypto::encrypt(plaintext, &derived_key).unwrap();
    assert!(!ciphertext.is_empty());
    assert_eq!(nonce.len(), 12);

    // 3. Persist as NoteFile (simulates save_file)
    let nf = NoteFile {
        encrypted: true,
        nonce_hex: Some(hex::encode(&nonce)),
        ciphertext_hex: Some(hex::encode(&ciphertext)),
        text: String::new(),
        cursor_pos: 10,
        scroll_top: 5,
    };

    // Serialize to JSON (as would be written to disk)
    let json = serde_json::to_string_pretty(&nf).unwrap();
    assert!(json.contains("\"encrypted\": true"));
    assert!(json.contains("\"nonce_hex\""));
    assert!(json.contains("\"ciphertext_hex\""));

    // 4. Simulate app reopening + unlocking
    //    Load config → get salt_hex + password_provided
    let loaded_salt = hex::decode(&salt_hex).unwrap();
    let unlock_key = crypto::derive_key(password, &loaded_salt).unwrap();
    assert_eq!(unlock_key, derived_key);

    // 5. Parse NoteFile from JSON (as would be read from disk)
    let loaded_nf: NoteFile = serde_json::from_str(&json).unwrap();
    assert!(loaded_nf.encrypted);

    // 6. Decrypt
    let decrypted = loaded_nf.decrypt_to_note(&unlock_key).unwrap();
    assert_eq!(decrypted.text, plaintext);
    assert_eq!(decrypted.cursor_pos, 10);
    assert_eq!(decrypted.scroll_top, 5);
}

// ── Wrong password at integration level ───────────────────────────────

#[test]
fn test_full_workflow_wrong_password() {
    let salt = crypto::generate_salt();
    let good_key = crypto::derive_key("correct-password", &salt).unwrap();
    let (nonce, ciphertext) = crypto::encrypt("secret data", &good_key).unwrap();

    let nf = NoteFile {
        encrypted: true,
        nonce_hex: Some(hex::encode(&nonce)),
        ciphertext_hex: Some(hex::encode(&ciphertext)),
        text: String::new(),
        cursor_pos: 0,
        scroll_top: 0,
    };

    // Wrong password → different key → decryption fails
    let wrong_key = crypto::derive_key("wrong-password", &salt).unwrap();
    assert_ne!(good_key, wrong_key);
    let result = nf.decrypt_to_note(&wrong_key);
    assert!(result.is_err());
}

// ── NoteFile JSON roundtrip (mimics disk I/O) ─────────────────────────

#[test]
fn test_notefile_json_roundtrip_encrypted() {
    let salt = crypto::generate_salt();
    let key = crypto::derive_key("json-test", &salt).unwrap();
    let note = Note {
        text: "JSON roundtrip test".to_string(),
        cursor_pos: 7,
        scroll_top: 3,
    };

    let nf = NoteFile::from_encrypted(&note, &key).unwrap();
    let json = serde_json::to_string_pretty(&nf).unwrap();
    let restored: NoteFile = serde_json::from_str(&json).unwrap();

    let decrypted = restored.decrypt_to_note(&key).unwrap();
    assert_eq!(decrypted.text, "JSON roundtrip test");
    assert_eq!(decrypted.cursor_pos, 7);
    assert_eq!(decrypted.scroll_top, 3);
}

#[test]
fn test_notefile_json_roundtrip_plaintext() {
    let nf = NoteFile {
        encrypted: false,
        nonce_hex: None,
        ciphertext_hex: None,
        text: "plain json".to_string(),
        cursor_pos: 5,
        scroll_top: 1,
    };

    let json = serde_json::to_string_pretty(&nf).unwrap();
    let restored: NoteFile = serde_json::from_str(&json).unwrap();
    assert!(!restored.encrypted);
    assert_eq!(restored.text, "plain json");
}

// ── Config-like salt persistence ──────────────────────────────────────

#[test]
fn test_salt_persistence_pattern() {
    // Salt stored as hex string in config (mimics config::Config.password_salt)
    let salt = crypto::generate_salt();
    let salt_hex = hex::encode(salt);

    // Later: load salt from config
    let loaded_salt = hex::decode(&salt_hex).unwrap();
    assert_eq!(loaded_salt.len(), 16);

    // Key derivation works with restored salt
    let key = crypto::derive_key("persistent", &loaded_salt).unwrap();
    let (nonce, ct) = crypto::encrypt("salt persistence test", &key).unwrap();
    let pt = crypto::decrypt(&ct, &nonce, &key).unwrap();
    assert_eq!(pt, "salt persistence test");
}

// ── File I/O simulation (create temp file, write NoteFile, read back) ─

#[test]
fn test_file_io_simulation_encrypted() {
    let dir = std::env::temp_dir()
        .join(format!("encryption-integration-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("test.notes");

    let salt = crypto::generate_salt();
    let key = crypto::derive_key("file-io", &salt).unwrap();
    let note = Note {
        text: "file io test".to_string(),
        cursor_pos: 4,
        scroll_top: 2,
    };
    let nf = NoteFile::from_encrypted(&note, &key).unwrap();

    // Write to file
    let json = serde_json::to_string_pretty(&nf).unwrap();
    std::fs::write(&path, &json).unwrap();

    // Read back
    let read_back = std::fs::read_to_string(&path).unwrap();
    let restored: NoteFile = serde_json::from_str(&read_back).unwrap();
    let decrypted = restored.decrypt_to_note(&key).unwrap();

    assert_eq!(decrypted.text, "file io test");
    assert_eq!(decrypted.cursor_pos, 4);
    assert_eq!(decrypted.scroll_top, 2);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_file_io_simulation_plaintext() {
    let dir = std::env::temp_dir()
        .join(format!("encryption-integration-plain-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("test.notes");

    let nf = NoteFile {
        encrypted: false,
        nonce_hex: None,
        ciphertext_hex: None,
        text: "plain file io".to_string(),
        cursor_pos: 0,
        scroll_top: 0,
    };

    let json = serde_json::to_string_pretty(&nf).unwrap();
    std::fs::write(&path, &json).unwrap();

    let read_back = std::fs::read_to_string(&path).unwrap();
    let restored: NoteFile = serde_json::from_str(&read_back).unwrap();
    assert!(!restored.encrypted);
    assert_eq!(restored.text, "plain file io");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── Legacy format backward compat (no encrypted field) ────────────────

#[test]
fn test_legacy_file_backward_compat() {
    // Simulate a pre-encryption sticky.notes file
    let legacy_json = r#"{"text":"legacy note","cursor_pos":3,"scroll_top":5}"#;
    let nf: NoteFile = serde_json::from_str(legacy_json).unwrap();
    assert!(!nf.encrypted);
    let note = nf.decrypt_to_note(&[0u8; 32]).unwrap();
    assert_eq!(note.text, "legacy note");
    assert_eq!(note.cursor_pos, 3);
    assert_eq!(note.scroll_top, 5);
}

// ── Large unicode content end-to-end ──────────────────────────────────

#[test]
fn test_unicode_end_to_end() {
    let salt = crypto::generate_salt();
    let key = crypto::derive_key("unicode-pwd- integration", &salt).unwrap();

    let texts = [
        "Hello, 世界!",
        "🚀 Rust 🔒 AES-256-GCM — 完全な暗号化",
        "αβγδεζηθικλμνξοπρσςτυφχψω\n下一行",
        "🎉🌟💯🔥👋\n\nTab\there\nnull\x00byte",
    ];

    for text in &texts {
        let (nonce, ct) = crypto::encrypt(text, &key).unwrap();
        let pt = crypto::decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(&pt, text);
    }
}

// ── Re-encryption with new key (password change simulation) ───────────

#[test]
fn test_re_encryption_with_new_key() {
    let salt_a = crypto::generate_salt();
    let salt_b = crypto::generate_salt();
    let key_a = crypto::derive_key("old-password", &salt_a).unwrap();
    let key_b = crypto::derive_key("new-password", &salt_b).unwrap();

    let note = Note {
        text: "password changed".to_string(),
        cursor_pos: 0,
        scroll_top: 0,
    };

    // Encrypt with old key
    let nf_a = NoteFile::from_encrypted(&note, &key_a).unwrap();
    let decrypted = nf_a.decrypt_to_note(&key_a).unwrap();

    // Re-encrypt with new key (simulates change_password)
    let nf_b = NoteFile::from_encrypted(&decrypted, &key_b).unwrap();
    let result = nf_b.decrypt_to_note(&key_b).unwrap();
    assert_eq!(result.text, "password changed");

    // Old key should NOT work on new ciphertext
    let old_result = nf_b.decrypt_to_note(&key_a);
    assert!(old_result.is_err());
}

// ── Tamper detection at file level ────────────────────────────────────

#[test]
fn test_tampered_notefile_fails() {
    let salt = crypto::generate_salt();
    let key = crypto::derive_key("anti-tamper", &salt).unwrap();
    let note = Note {
        text: "do not change me".to_string(),
        cursor_pos: 0,
        scroll_top: 0,
    };
    let nf = NoteFile::from_encrypted(&note, &key).unwrap();

    let json = serde_json::to_string(&nf).unwrap();

    // Flip a bit in the ciphertext_hex value
    let tampered = json.replace("ciphertext_hex\":\"", "ciphertext_hex\":\"0");
    let restored: NoteFile = serde_json::from_str(&tampered).unwrap();
    let result = restored.decrypt_to_note(&key);
    assert!(result.is_err(), "tampered ciphertext must fail decryption");
}
