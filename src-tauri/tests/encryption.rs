//! Integration tests for the sticky-notes encryption system.
//!
//! These tests exercise the public API of the library crate —
//! salt generation, key derivation, encryption, decryption, and
//! the NoteFile on-disk format — as an end-to-end consumer would.
//!
//! Run with: `cargo test --test encryption`

use sticky_notes_lib::crypto;
use sticky_notes_lib::note::{Note, NoteFile};

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

// ── Empty content edge cases ─────────────────────────────────────────

#[test]
fn test_empty_plaintext_full_workflow() {
    let salt = crypto::generate_salt();
    let key = crypto::derive_key("empty-test", &salt).unwrap();
    let (nonce, ct) = crypto::encrypt("", &key).unwrap();
    let pt = crypto::decrypt(&ct, &nonce, &key).unwrap();
    assert_eq!(pt, "");
}

#[test]
fn test_empty_password_workflow() {
    let salt = crypto::generate_salt();
    // Empty password derives a key via Argon2 (it accepts empty strings)
    let key = crypto::derive_key("", &salt).unwrap();
    let (nonce, ct) = crypto::encrypt("empty pwd test", &key).unwrap();
    let pt = crypto::decrypt(&ct, &nonce, &key).unwrap();
    assert_eq!(pt, "empty pwd test");
}

// ── Multiple keys with same content ───────────────────────────────────

#[test]
fn test_same_content_different_keys_produce_different_ciphertexts() {
    let salt = crypto::generate_salt();
    let key_a = crypto::derive_key("key-a", &salt).unwrap();
    let key_b = crypto::derive_key("key-b", &salt).unwrap();

    let (nonce_a, ct_a) = crypto::encrypt("same content", &key_a).unwrap();
    let (nonce_b, ct_b) = crypto::encrypt("same content", &key_b).unwrap();

    // Different keys should produce completely different outputs
    assert_ne!(ct_a, ct_b, "different keys should produce different ciphertexts");
    assert_ne!(nonce_a, nonce_b, "different keys still get unique nonces");
}

// ── Lock/unlock cycle repeated 3x ───────────────────────────────────────

#[test]
fn test_lock_unlock_cycle() {
    for i in 0..3 {
        let salt = crypto::generate_salt();
        let key = crypto::derive_key("cycle-password", &salt).unwrap();

        let plaintext = format!("cycle iteration {i}");
        let (nonce, ciphertext) = crypto::encrypt(&plaintext, &key).unwrap();

        let nf = NoteFile {
            encrypted: true,
            nonce_hex: Some(hex::encode(&nonce)),
            ciphertext_hex: Some(hex::encode(&ciphertext)),
            text: String::new(),
            cursor_pos: 10,
            scroll_top: 5,
        };

        let decrypted = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, plaintext);
        assert_eq!(decrypted.cursor_pos, 10);
        assert_eq!(decrypted.scroll_top, 5);
    }
}

// ── Wrong key fails, then correct key succeeds ──────────────────────────

#[test]
fn test_unlock_wrong_then_correct() {
    let salt = crypto::generate_salt();
    let correct_key = crypto::derive_key("correct-password", &salt).unwrap();
    let wrong_key = crypto::derive_key("wrong-password", &salt).unwrap();

    let note = Note {
        text: "protected content".to_string(),
        cursor_pos: 3,
        scroll_top: 1,
    };
    let nf = NoteFile::from_encrypted(&note, &correct_key).unwrap();

    // Wrong key should fail
    let result = nf.decrypt_to_note(&wrong_key);
    assert!(result.is_err(), "wrong key must fail decryption");

    // Correct key should succeed
    let decrypted = nf.decrypt_to_note(&correct_key).unwrap();
    assert_eq!(decrypted.text, "protected content");
    assert_eq!(decrypted.cursor_pos, 3);
    assert_eq!(decrypted.scroll_top, 1);
}

// ── Ciphertext differs from plaintext, then decrypts correctly ──────────

#[test]
fn test_save_encrypted_note_roundtrip() {
    let salt = crypto::generate_salt();
    let key = crypto::derive_key("roundtrip-test", &salt).unwrap();

    let note = Note {
        text: "my secret sticky note".to_string(),
        cursor_pos: 7,
        scroll_top: 2,
    };

    let nf = NoteFile::from_encrypted(&note, &key).unwrap();

    // Ciphertext must differ from plaintext bytes
    let ct_hex = nf.ciphertext_hex.as_ref().unwrap();
    let ciphertext = hex::decode(ct_hex).unwrap();
    assert_ne!(
        ciphertext.as_slice(),
        note.text.as_bytes(),
        "ciphertext must differ from plaintext"
    );

    // Decrypt back and verify
    let decrypted = nf.decrypt_to_note(&key).unwrap();
    assert_eq!(decrypted.text, "my secret sticky note");
    assert_eq!(decrypted.cursor_pos, 7);
    assert_eq!(decrypted.scroll_top, 2);
}

// ── Same password, different salt → different ciphertext ────────────────

#[test]
fn test_change_password_same() {
    let password = "my-password";

    let salt_a = crypto::generate_salt();
    let salt_b = crypto::generate_salt();
    let key_a = crypto::derive_key(password, &salt_a).unwrap();
    let key_b = crypto::derive_key(password, &salt_b).unwrap();

    // Different salts must produce different keys
    assert_ne!(key_a, key_b);

    let note = Note {
        text: "password change test".to_string(),
        cursor_pos: 0,
        scroll_top: 0,
    };

    // Encrypt with key_a, then re-encrypt with key_b
    let nf_a = NoteFile::from_encrypted(&note, &key_a).unwrap();
    let decrypted = nf_a.decrypt_to_note(&key_a).unwrap();
    let nf_b = NoteFile::from_encrypted(&decrypted, &key_b).unwrap();

    // Both decrypt to same plaintext
    let result_a = nf_a.decrypt_to_note(&key_a).unwrap();
    let result_b = nf_b.decrypt_to_note(&key_b).unwrap();
    assert_eq!(result_a.text, "password change test");
    assert_eq!(result_b.text, "password change test");

    // Ciphertexts must differ (different key + different nonce)
    assert_ne!(nf_a.ciphertext_hex, nf_b.ciphertext_hex);
}

// ── Encrypt → unencrypted → re-encrypt roundtrip ────────────────────────

#[test]
fn test_remove_password_then_re_set() {
    let salt = crypto::generate_salt();
    let key = crypto::derive_key("password123", &salt).unwrap();

    // 1. Encrypt to NoteFile
    let note = Note {
        text: "roundtrip content".to_string(),
        cursor_pos: 5,
        scroll_top: 3,
    };
    let nf_encrypted = NoteFile::from_encrypted(&note, &key).unwrap();

    // 2. Decrypt to get Note back
    let decrypted_note = nf_encrypted.decrypt_to_note(&key).unwrap();
    assert_eq!(decrypted_note.text, "roundtrip content");

    // 3. Save as unencrypted NoteFile
    let nf_unencrypted = NoteFile {
        encrypted: false,
        nonce_hex: None,
        ciphertext_hex: None,
        text: decrypted_note.text.clone(),
        cursor_pos: decrypted_note.cursor_pos,
        scroll_top: decrypted_note.scroll_top,
    };

    // 4. Re-encrypt from the unencrypted data (simulating re-setting password)
    let note_from_unencrypted = Note {
        text: nf_unencrypted.text.clone(),
        cursor_pos: nf_unencrypted.cursor_pos,
        scroll_top: nf_unencrypted.scroll_top,
    };
    let nf_reencrypted = NoteFile::from_encrypted(&note_from_unencrypted, &key).unwrap();

    // 5. Final roundtrip
    let final_note = nf_reencrypted.decrypt_to_note(&key).unwrap();
    assert_eq!(final_note.text, "roundtrip content");
    assert_eq!(final_note.cursor_pos, 5);
    assert_eq!(final_note.scroll_top, 3);
}

// ── Non-ASCII content (emoji, CJK, mixed scripts) ───────────────────────

#[test]
fn test_non_ascii_content_encryption() {
    let salt = crypto::generate_salt();
    let key = crypto::derive_key("unicode-password", &salt).unwrap();

    let contents = [
        "Hello, 世界! 🌍",
        "こんにちは、世界！",
        "안녕하세요, 세계! 🚀",
        "emoji party: 🎉🌟💯🔥👋",
        "Mixed: Français, 中文, العربية, Русский 🎈",
        "Line breaks\nand tabs\tand emoji 🎯\n\n下一行",
        "null\x00byte\thandling 🛠️",
    ];

    for content in &contents {
        let note = Note {
            text: content.to_string(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let decrypted = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(&decrypted.text, content, "non-ASCII content roundtrip failed");
    }
}

// ── Empty salt rejected by derive_key ───────────────────────────────────

#[test]
fn test_empty_salt_rejected() {
    let result = crypto::derive_key("any-password", &[]);
    assert!(result.is_err(), "empty salt must be rejected by derive_key");
}

// ── Zero key (all-zero 32 bytes) still works (AES accepts it) ───────────

#[test]
fn test_zero_key_encrypt_decrypt() {
    let zero_key = [0u8; 32];

    let note = Note {
        text: "zero key test".to_string(),
        cursor_pos: 42,
        scroll_top: 7,
    };

    let nf = NoteFile::from_encrypted(&note, &zero_key).unwrap();
    let decrypted = nf.decrypt_to_note(&zero_key).unwrap();
    assert_eq!(decrypted.text, "zero key test");
    assert_eq!(decrypted.cursor_pos, 42);
    assert_eq!(decrypted.scroll_top, 7);
}

// ── cursor_pos and scroll_top survive encrypt/decrypt cycle ─────────────

#[test]
fn test_cursor_scroll_preserved_through_encryption() {
    let salt = crypto::generate_salt();
    let key = crypto::derive_key("cursor-scroll-test", &salt).unwrap();

    let positions = [
        (0u32, 0u32),
        (1u32, 0u32),
        (0u32, 1u32),
        (10u32, 5u32),
        (100u32, 50u32),
        (9999u32, 8888u32),
    ];

    for &(cursor_pos, scroll_top) in &positions {
        let note = Note {
            text: "preserve me".to_string(),
            cursor_pos,
            scroll_top,
        };
        let nf = NoteFile::from_encrypted(&note, &key).unwrap();
        let decrypted = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.cursor_pos, cursor_pos, "cursor_pos not preserved for ({cursor_pos}, {scroll_top})");
        assert_eq!(decrypted.scroll_top, scroll_top, "scroll_top not preserved for ({cursor_pos}, {scroll_top})");
        assert_eq!(decrypted.text, "preserve me");
    }
}

// ── Consecutive decrypts with same key all succeed ──────────────────────

#[test]
fn test_consecutive_decrypts_same_key() {
    let salt = crypto::generate_salt();
    let key = crypto::derive_key("consecutive-test", &salt).unwrap();

    let note = Note {
        text: "decrypt me many times".to_string(),
        cursor_pos: 8,
        scroll_top: 4,
    };
    let nf = NoteFile::from_encrypted(&note, &key).unwrap();

    for i in 0..5 {
        let decrypted = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, "decrypt me many times", "decrypt failed on iteration {i}");
        assert_eq!(decrypted.cursor_pos, 8, "cursor_pos mismatch on iteration {i}");
        assert_eq!(decrypted.scroll_top, 4, "scroll_top mismatch on iteration {i}");
    }
}
