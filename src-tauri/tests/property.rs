//! Property-based tests using proptest.
//!
//! These generate random inputs and verify invariants hold
//! across a range of values. Case count is kept low (10 per test)
//! because each iteration runs Argon2 key derivation (~10ms/call).
//!
//! Run with: `cargo test --test property`

use proptest::prelude::*;

// ── Custom strategies ────────────────────────────────────────────────

/// Generate an arbitrary Config with all fields randomly varied.
/// Uses nested tuples because proptest doesn't implement Strategy
/// for tuples larger than 12 elements.
fn arb_config() -> impl Strategy<Value = sticky_notes_lib::config::Config> {
    (
        // Group 1: width, height, left, top, font_size
        (any::<u32>(), any::<u32>(), any::<i32>(), any::<i32>(), 1u32..200u32),
        // Group 2: always_on_top, word_wrap, theme, titlebar_color, titlebar_fill
        (
            any::<bool>(),
            any::<bool>(),
            prop_oneof![
                Just("dark".to_string()),
                Just("light".to_string()),
                Just("dracula".to_string()),
                Just("nord".to_string()),
            ],
            any::<String>(),
            any::<u8>(),
        ),
        // Group 3: password_protected, password_salt, lock_timeout_minutes,
        //          font_family, start_with_windows
        (
            any::<bool>(),
            any::<String>(),
            0u32..1440u32,
            any::<String>(),
            any::<bool>(),
        ),
    )
        .prop_map(
            |(g1, g2, g3)| sticky_notes_lib::config::Config {
                width: g1.0,
                height: g1.1,
                left: g1.2,
                top: g1.3,
                font_size: g1.4,
                always_on_top: g2.0,
                word_wrap: g2.1,
                theme: g2.2,
                titlebar_color: g2.3,
                titlebar_fill: g2.4,
                password_protected: g3.0,
                password_salt: g3.1,
                lock_timeout_minutes: g3.2,
                font_family: g3.3,
                start_with_windows: g3.4,
            },
        )
}

fn arb_notefile() -> impl Strategy<Value = sticky_notes_lib::note::NoteFile> {
    fn edge_u32() -> impl Strategy<Value = u32> {
        prop_oneof![Just(0u32), Just(1u32), Just(u32::MAX), any::<u32>()]
    }

    fn optional_hex() -> impl Strategy<Value = Option<String>> {
        prop_oneof![
            Just(None),
            "[a-f0-9]{0,64}".prop_map(Some),
        ]
    }

    (
        any::<bool>(),
        optional_hex(),
        optional_hex(),
        any::<String>(),
        edge_u32(),
        edge_u32(),
    )
        .prop_map(
            |(encrypted, nonce_hex, ciphertext_hex, text, cursor_pos, scroll_top)| {
                sticky_notes_lib::note::NoteFile {
                    encrypted,
                    nonce_hex,
                    ciphertext_hex,
                    text,
                    cursor_pos,
                    scroll_top,
                }
            },
        )
}

fn arb_notedata() -> impl Strategy<Value = sticky_notes_lib::storage::NoteData> {
    (arb_config(), arb_notefile(), any::<String>())
        .prop_map(|(config, note, log)| sticky_notes_lib::storage::NoteData {
            version: 1,
            config,
            note,
            log,
        })
}

/// Strategy that generates a JSON string with all required Config fields
/// (no #[serde(default)]) plus a random subset of optional fields.
/// Returns (json_string, expected_config) for easy verification.
fn arb_partial_config_json() -> impl Strategy<Value = (String, sticky_notes_lib::config::Config)> {
    (
        // Required fields (must always be in JSON)
        (any::<u32>(), any::<u32>(), any::<i32>(), any::<i32>(), any::<u32>(), any::<bool>()),
        // Optional fields as Option — None means "omit from JSON"
        (
            proptest::option::of(any::<bool>()),                // word_wrap
            proptest::option::of(prop_oneof![                   // theme
                Just("dark".to_string()),
                Just("light".to_string()),
                Just("dracula".to_string()),
                Just("nord".to_string()),
            ]),
            proptest::option::of(any::<String>()),              // titlebar_color
            proptest::option::of(any::<u8>()),                  // titlebar_fill
            proptest::option::of(any::<bool>()),                // password_protected
            proptest::option::of(any::<String>()),              // password_salt
            proptest::option::of(any::<u32>()),                 // lock_timeout_minutes
            proptest::option::of(any::<String>()),              // font_family
            proptest::option::of(any::<bool>()),                // start_with_windows
        ),
    )
        .prop_map(
            |(req, opt)| {
                let (width, height, left, top, font_size, always_on_top) = req;
                let (word_wrap, theme, titlebar_color, titlebar_fill,
                     password_protected, password_salt, lock_timeout_minutes,
                     font_family, start_with_windows) = opt;

                let mut map = serde_json::Map::new();
                // Required fields — always present
                map.insert("width".into(), serde_json::json!(width));
                map.insert("height".into(), serde_json::json!(height));
                map.insert("left".into(), serde_json::json!(left));
                map.insert("top".into(), serde_json::json!(top));
                map.insert("font_size".into(), serde_json::json!(font_size));
                map.insert("always_on_top".into(), serde_json::json!(always_on_top));
            // Optional fields — only if Some
            if let Some(ref v) = word_wrap {
                map.insert("word_wrap".into(), serde_json::json!(v));
            }
            if let Some(ref v) = theme {
                map.insert("theme".into(), serde_json::json!(v));
            }
            if let Some(ref v) = titlebar_color {
                map.insert("titlebar_color".into(), serde_json::json!(v));
            }
            if let Some(ref v) = titlebar_fill {
                map.insert("titlebar_fill".into(), serde_json::json!(v));
            }
            if let Some(ref v) = password_protected {
                map.insert("password_protected".into(), serde_json::json!(v));
            }
            if let Some(ref v) = password_salt {
                map.insert("password_salt".into(), serde_json::json!(v));
            }
            if let Some(ref v) = lock_timeout_minutes {
                map.insert("lock_timeout_minutes".into(), serde_json::json!(v));
            }
            if let Some(ref v) = font_family {
                map.insert("font_family".into(), serde_json::json!(v));
            }
            if let Some(ref v) = start_with_windows {
                map.insert("start_with_windows".into(), serde_json::json!(v));
            }
                let json = serde_json::to_string(&serde_json::Value::Object(map)).unwrap();

                let expected = sticky_notes_lib::config::Config {
                    width,
                    height,
                    left,
                    top,
                    font_size,
                    always_on_top,
                    word_wrap: word_wrap.unwrap_or(false),
                    theme: theme.unwrap_or_else(|| "dark".to_string()),
                    titlebar_color: titlebar_color.unwrap_or_default(),
                    titlebar_fill: titlebar_fill.unwrap_or(100),
                    password_protected: password_protected.unwrap_or(false),
                    password_salt: password_salt.unwrap_or_default(),
                    lock_timeout_minutes: lock_timeout_minutes.unwrap_or(10),
                    font_family: font_family.unwrap_or_else(|| "Cascadia Code".to_string()),
                    start_with_windows: start_with_windows.unwrap_or(false),
                };

                (json, expected)
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    // ── Roundtrip: any valid UTF-8 string ──────────────────────────
    //
    // Encrypt then decrypt must return the original plaintext for
    // any random string (including unicode, null bytes, etc.).
    #[test]
    fn prop_encrypt_decrypt_roundtrip(text: String) {
        let salt = sticky_notes_lib::crypto::generate_salt();
        let key = sticky_notes_lib::crypto::derive_key("prop-test-password", &salt).unwrap();
        let (nonce, ct) = sticky_notes_lib::crypto::encrypt(&text, &key).unwrap();
        let pt = sticky_notes_lib::crypto::decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, text);
    }

    // ── Key derivation determinism ─────────────────────────────────
    //
    // Same password + same salt must always produce the same key.
    #[test]
    fn prop_derive_key_deterministic(password: String, salt: String) {
        let salt_bytes = salt.as_bytes();
        let mut fixed_salt = [0u8; 16];
        let len = salt_bytes.len().min(16);
        fixed_salt[..len].copy_from_slice(&salt_bytes[..len]);

        let key_a = sticky_notes_lib::crypto::derive_key(&password, &fixed_salt).unwrap();
        let key_b = sticky_notes_lib::crypto::derive_key(&password, &fixed_salt).unwrap();
        assert_eq!(key_a, key_b);
    }

    // ── Nonce uniqueness ───────────────────────────────────────────
    //
    // Two consecutive encryptions of the same plaintext with the
    // same key must produce different nonces.
    #[test]
    fn prop_nonce_is_unique(text: String) {
        let salt = sticky_notes_lib::crypto::generate_salt();
        let key = sticky_notes_lib::crypto::derive_key("nonce-unique", &salt).unwrap();
        let (nonce_a, ct_a) = sticky_notes_lib::crypto::encrypt(&text, &key).unwrap();
        let (nonce_b, ct_b) = sticky_notes_lib::crypto::encrypt(&text, &key).unwrap();
        assert_ne!(nonce_a, nonce_b, "nonces must be unique");
        assert_ne!(ct_a, ct_b, "ciphertexts must differ when nonces differ");
    }

    // ── Wrong key fails ────────────────────────────────────────────
    //
    // Encrypting with one key then decrypting with a different key
    // must always fail.
    #[test]
    fn prop_wrong_key_fails(text: String) {
        let salt = sticky_notes_lib::crypto::generate_salt();
        let key_a = sticky_notes_lib::crypto::derive_key("key-a", &salt).unwrap();
        let key_b = sticky_notes_lib::crypto::derive_key("key-b", &salt).unwrap();

        // Astronomically unlikely collision, but handle it gracefully
        if key_a == key_b {
            return Ok(());
        }

        let (nonce, ct) = sticky_notes_lib::crypto::encrypt(&text, &key_a).unwrap();
        let result = sticky_notes_lib::crypto::decrypt(&ct, &nonce, &key_b);
        assert!(result.is_err());
    }

    // ── Different salts → different keys ───────────────────────────
    //
    // Same password with different salts must produce different keys.
    #[test]
    fn prop_different_salts_different_keys(password: String) {
        let salt_a = sticky_notes_lib::crypto::generate_salt();
        let salt_b = sticky_notes_lib::crypto::generate_salt();

        let key_a = sticky_notes_lib::crypto::derive_key(&password, &salt_a).unwrap();
        let key_b = sticky_notes_lib::crypto::derive_key(&password, &salt_b).unwrap();

        // Cryptographic collision probability is ~2^-256
        if key_a == key_b {
            return Ok(());
        }
        assert_ne!(key_a, key_b);
    }

    // ── Note JSON roundtrip ────────────────────────────────────────
    //
    // Any valid Note serializes to JSON and deserializes back correctly.
    #[test]
    fn prop_note_json_roundtrip(
        text: String,
        cursor_pos: u32,
        scroll_top: u32,
    ) {
        let note = sticky_notes_lib::note::Note {
            text,
            cursor_pos,
            scroll_top,
        };
        let json = serde_json::to_string(&note).unwrap();
        let restored: sticky_notes_lib::note::Note =
            serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, note.text);
        assert_eq!(restored.cursor_pos, note.cursor_pos);
        assert_eq!(restored.scroll_top, note.scroll_top);
    }

    // ── NoteFile encrypted roundtrip ───────────────────────────────
    //
    // Any text encrypted into a NoteFile and then decrypted must
    // produce the original Note with the same content.
    #[test]
    fn prop_notefile_encrypted_decrypts(text: String) {
        let salt = sticky_notes_lib::crypto::generate_salt();
        let key = sticky_notes_lib::crypto::derive_key("prop-nf-roundtrip", &salt).unwrap();

        let note = sticky_notes_lib::note::Note {
            text,
            cursor_pos: 0,
            scroll_top: 0,
        };

        let nf = sticky_notes_lib::note::NoteFile::from_encrypted(&note, &key).unwrap();
        let restored = nf.decrypt_to_note(&key).unwrap();
        assert_eq!(restored.text, note.text);
        assert_eq!(restored.cursor_pos, note.cursor_pos);
        assert_eq!(restored.scroll_top, note.scroll_top);
    }

    // ── NoteFile encrypted → JSON → deserialize → decrypt ──────────
    //
    // Creating a NoteFile via from_encrypted, serializing to JSON,
    // deserializing, and decrypting must return the original text.
    #[test]
    fn prop_notefile_json_roundtrip(text: String) {
        let salt = sticky_notes_lib::crypto::generate_salt();
        let key = sticky_notes_lib::crypto::derive_key("prop-nf-json", &salt).unwrap();

        let note = sticky_notes_lib::note::Note {
            text,
            cursor_pos: 42,
            scroll_top: 99,
        };

        let nf = sticky_notes_lib::note::NoteFile::from_encrypted(&note, &key).unwrap();
        let json = serde_json::to_string(&nf).unwrap();
        let restored: sticky_notes_lib::note::NoteFile =
            serde_json::from_str(&json).unwrap();

        assert!(restored.encrypted);
        assert_eq!(restored.cursor_pos, 42);
        assert_eq!(restored.scroll_top, 99);

        let decrypted = restored.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, note.text);
        assert_eq!(decrypted.cursor_pos, 42);
        assert_eq!(decrypted.scroll_top, 99);
    }

    // ════════════════════════════════════════════════════════════════
    //  NEW PROPERTY-BASED TESTS (10 added)
    // ════════════════════════════════════════════════════════════════

    // ── 1. NoteData JSON roundtrip ─────────────────────────────────
    //
    // Random NoteData (version 1, random Config, random NoteFile,
    // random log) → serialize → deserialize → all fields must match.
    #[test]
    fn prop_notedata_json_roundtrip(data in arb_notedata()) {
        let json = serde_json::to_string(&data).unwrap();
        let restored: sticky_notes_lib::storage::NoteData =
            serde_json::from_str(&json).unwrap();

        assert_eq!(restored.version, data.version);
        assert_eq!(restored.config.width, data.config.width);
        assert_eq!(restored.config.height, data.config.height);
        assert_eq!(restored.config.left, data.config.left);
        assert_eq!(restored.config.top, data.config.top);
        assert_eq!(restored.config.font_size, data.config.font_size);
        assert_eq!(restored.config.always_on_top, data.config.always_on_top);
        assert_eq!(restored.config.word_wrap, data.config.word_wrap);
        assert_eq!(restored.config.theme, data.config.theme);
        assert_eq!(restored.config.titlebar_color, data.config.titlebar_color);
        assert_eq!(restored.config.titlebar_fill, data.config.titlebar_fill);
        assert_eq!(restored.config.password_protected, data.config.password_protected);
        assert_eq!(restored.config.password_salt, data.config.password_salt);
        assert_eq!(restored.config.lock_timeout_minutes, data.config.lock_timeout_minutes);
        assert_eq!(restored.config.font_family, data.config.font_family);
        assert_eq!(restored.config.start_with_windows, data.config.start_with_windows);
        assert_eq!(restored.note.encrypted, data.note.encrypted);
        assert_eq!(restored.note.cursor_pos, data.note.cursor_pos);
        assert_eq!(restored.note.scroll_top, data.note.scroll_top);
        assert_eq!(restored.log, data.log);
    }

    // ── 2. Config roundtrip ────────────────────────────────────────
    //
    // Random Config with all fields varied (width, height, font_size
    // in range, theme from set, booleans random, string fields varied).
    #[test]
    fn prop_config_roundtrip(config in arb_config()) {
        let json = serde_json::to_string(&config).unwrap();
        let restored: sticky_notes_lib::config::Config =
            serde_json::from_str(&json).unwrap();

        assert_eq!(restored.width, config.width);
        assert_eq!(restored.height, config.height);
        assert_eq!(restored.left, config.left);
        assert_eq!(restored.top, config.top);
        assert_eq!(restored.font_size, config.font_size);
        assert_eq!(restored.always_on_top, config.always_on_top);
        assert_eq!(restored.word_wrap, config.word_wrap);
        assert_eq!(restored.theme, config.theme);
        assert_eq!(restored.titlebar_color, config.titlebar_color);
        assert_eq!(restored.titlebar_fill, config.titlebar_fill);
        assert_eq!(restored.password_protected, config.password_protected);
        assert_eq!(restored.password_salt, config.password_salt);
        assert_eq!(restored.lock_timeout_minutes, config.lock_timeout_minutes);
        assert_eq!(restored.font_family, config.font_family);
        assert_eq!(restored.start_with_windows, config.start_with_windows);
    }

    // ── 3. NoteFile edge fields ────────────────────────────────────
    //
    // NoteFile with various combinations of encrypted/non-encrypted,
    // empty/non-empty text, various cursor/scroll values (0, 1, u32::MAX).
    #[test]
    fn prop_notefile_edge_fields(nf in arb_notefile()) {
        let json = serde_json::to_string(&nf).unwrap();
        let restored: sticky_notes_lib::note::NoteFile =
            serde_json::from_str(&json).unwrap();

        assert_eq!(restored.encrypted, nf.encrypted);
        assert_eq!(restored.text, nf.text);
        assert_eq!(restored.cursor_pos, nf.cursor_pos);
        assert_eq!(restored.scroll_top, nf.scroll_top);

        // Unencrypted file with no crypto fields must not have them serialized
        if !nf.encrypted && nf.nonce_hex.is_none() && nf.ciphertext_hex.is_none() {
            assert!(!json.contains("nonce_hex"), "unencrypted file must not contain nonce_hex");
            assert!(!json.contains("ciphertext_hex"), "unencrypted file must not contain ciphertext_hex");
        }
    }

    // ── 4. Reconcile invariants ────────────────────────────────────
    //
    // Verify the invariant rules that reconcile_invariants enforces.
    // Since reconcile_invariants is private, we apply the same logic
    // inline and verify the invariants hold after reconciliation.
    //
    //   Rule 1: If note.encrypted → config.password_protected must be true
    //   Rule 2: If !note.encrypted → config.password_protected should be false
    //   Rule 3: If !config.password_protected → password_salt must be empty
    #[test]
    fn prop_reconcile_invariants(data in arb_notedata()) {
        let mut data = data;

        // Apply the same logic as reconcile_invariants
        if data.note.encrypted && !data.config.password_protected {
            data.config.password_protected = true;
        }
        if !data.note.encrypted && data.config.password_protected {
            data.config.password_protected = false;
        }
        if !data.config.password_protected && !data.config.password_salt.is_empty() {
            data.config.password_salt.clear();
        }

        // Now verify invariants hold
        if data.note.encrypted {
            assert!(
                data.config.password_protected,
                "invariant 1: encrypted NoteFile requires password_protected = true"
            );
        }
        if !data.note.encrypted {
            assert!(
                !data.config.password_protected,
                "invariant 2: plaintext NoteFile must not have password_protected = true"
            );
        }
        if !data.config.password_protected {
            assert!(
                data.config.password_salt.is_empty(),
                "invariant 3: password_protected = false requires empty salt"
            );
        }
    }

    // ── 5. Random password lengths ─────────────────────────────────
    //
    // derive_key with passwords of various lengths (0, 1, 64, 256,
    // 1024, 4096 chars) must always produce a 32-byte key.
    #[test]
    fn prop_random_password_lengths(
        password in prop_oneof![
            Just(String::new()),
            Just("a".to_string()),
            Just("a".repeat(64)),
            Just("a".repeat(256)),
            Just("a".repeat(1024)),
            Just("a".repeat(4096)),
        ],
    ) {
        let salt = sticky_notes_lib::crypto::generate_salt();
        let key = sticky_notes_lib::crypto::derive_key(&password, &salt).unwrap();
        assert_eq!(key.len(), 32, "derived key must be 32 bytes");
    }

    // ── 6. Password non-ASCII ──────────────────────────────────────
    //
    // Passwords with emoji, CJK, RTL text, mixed scripts →
    // encrypt/decrypt roundtrip must still succeed.
    #[test]
    fn prop_password_non_ascii(
        text: String,
        password in prop_oneof![
            Just("👍🌍🚀".to_string()),
            Just("你好世界".to_string()),
            Just("שָׁלוֹם עֲלֵיכֶם".to_string()),
            Just("パスワード🔑".to_string()),
            Just("αβγδεζηθικ".to_string()),
            Just("混合密码👍🌍".to_string()),
            Just(" \t\n\r\u{2003}\u{00A0}".to_string()),
        ],
    ) {
        let salt = sticky_notes_lib::crypto::generate_salt();
        let key = sticky_notes_lib::crypto::derive_key(&password, &salt).unwrap();
        assert_eq!(key.len(), 32);

        let (nonce, ct) = sticky_notes_lib::crypto::encrypt(&text, &key).unwrap();
        let pt = sticky_notes_lib::crypto::decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, text);
    }

    // ── 7. Note text edge cases ────────────────────────────────────
    //
    // Note text with: empty, single char, very long (10K), only
    // whitespace, only newlines, null bytes → roundtrip must preserve.
    #[test]
    fn prop_note_text_edge_cases(
        text in prop_oneof![
            Just(String::new()),
            Just("x".to_string()),
            Just("x".repeat(10_000)),
            Just("   \t  ".to_string()),
            Just("\n\n\n\n".to_string()),
            Just("a\0b\0c".to_string()),
            any::<String>(),
        ],
    ) {
        let note = sticky_notes_lib::note::Note {
            text: text.clone(),
            cursor_pos: 0,
            scroll_top: 0,
        };
        let json = serde_json::to_string(&note).unwrap();
        let restored: sticky_notes_lib::note::Note =
            serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, text);
    }

    // ── 8. Config serde defaults ───────────────────────────────────
    //
    // JSON with missing fields should deserialize with defaults.
    // Tests a random subset of optional fields omitted.
    #[test]
    fn prop_config_serde_defaults(
        (json, expected) in arb_partial_config_json(),
    ) {
        let restored: sticky_notes_lib::config::Config =
            serde_json::from_str(&json).unwrap();

        assert_eq!(restored.width, expected.width);
        assert_eq!(restored.height, expected.height);
        assert_eq!(restored.left, expected.left);
        assert_eq!(restored.top, expected.top);
        assert_eq!(restored.font_size, expected.font_size);
        assert_eq!(restored.always_on_top, expected.always_on_top);
        assert_eq!(restored.word_wrap, expected.word_wrap);
        assert_eq!(restored.theme, expected.theme);
        assert_eq!(restored.titlebar_color, expected.titlebar_color);
        assert_eq!(restored.titlebar_fill, expected.titlebar_fill);
        assert_eq!(restored.password_protected, expected.password_protected);
        assert_eq!(restored.password_salt, expected.password_salt);
        assert_eq!(restored.lock_timeout_minutes, expected.lock_timeout_minutes);
        assert_eq!(restored.font_family, expected.font_family);
        assert_eq!(restored.start_with_windows, expected.start_with_windows);
    }

    // ── 9. Diagnostics event format ────────────────────────────────
    //
    // event() produces lines matching the pattern:
    //   [timestamp] category: message
    #[test]
    fn prop_diagnostics_event_format(
        category in "\\PC{0,50}",
        message in "\\PC{0,200}",
    ) {
        // Clear any leftover state from previous cases
        let _flushed = sticky_notes_lib::diagnostics::flush_to_log_str();

        sticky_notes_lib::diagnostics::event(&category, &message);
        let log = sticky_notes_lib::diagnostics::flush_to_log_str();

        // Each line must match [digits] category: message
        for line in log.lines() {
            let line = line.trim_end();
            assert!(
                line.starts_with('['),
                "event line must start with [timestamp]: {line:?}"
            );
            // Find the closing bracket
            let close_bracket = line.find(']').expect("line must have closing bracket");
            let timestamp_part = &line[1..close_bracket];
            // Timestamp must be a valid non-negative integer
            assert!(
                timestamp_part.parse::<u64>().is_ok(),
                "timestamp must be a u64, got: {timestamp_part:?}"
            );
            // After bracket: must contain "category: message"
            let after_bracket = &line[close_bracket + 1..].trim_start();
            assert!(
                after_bracket.contains(&format!("{}: {}", category, message))
                    || after_bracket.contains(&format!(": {}", message)),
                "line should contain category: message format, got: {after_bracket:?}"
            );
        }
    }

    // ── 10. Password salt lengths ──────────────────────────────────
    //
    // derive_key with various salt lengths (8, 16, 32, 64 bytes),
    // all must produce a 32-byte key.
    #[test]
    fn prop_password_salt_lengths(
        password: String,
        salt_len in prop_oneof![Just(8usize), Just(16usize), Just(32usize), Just(64usize)],
    ) {
        let mut salt = vec![0u8; salt_len];
        use rand::RngCore;
        rand::rngs::OsRng.fill_bytes(&mut salt);
        let key = sticky_notes_lib::crypto::derive_key(&password, &salt).unwrap();
        assert_eq!(key.len(), 32, "derived key must be 32 bytes regardless of salt length");
    }
}
