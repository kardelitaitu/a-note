//! Property-based tests using proptest.
//!
//! These generate random inputs and verify invariants hold
//! across a range of values. Case count is kept low (10 per test)
//! because each iteration runs Argon2 key derivation (~10ms/call).
//!
//! Run with: `cargo test --test property`

use proptest::prelude::*;

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
}
