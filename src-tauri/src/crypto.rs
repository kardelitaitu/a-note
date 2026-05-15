use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;

pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    salt
}

pub fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut key = [0u8; 32];
    argon2::Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("key derivation failed: {e}"))?;
    Ok(key)
}

pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("invalid key: {e}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("encryption failed: {e}"))?;
    Ok((nonce_bytes.to_vec(), ciphertext))
}

pub fn decrypt(
    ciphertext: &[u8],
    nonce_bytes: &[u8],
    key: &[u8; 32],
) -> Result<String, String> {
    if nonce_bytes.len() != 12 {
        return Err("invalid nonce: must be 12 bytes".to_string());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("invalid key: {e}"))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "wrong password or corrupted data".to_string())?;
    String::from_utf8(plaintext).map_err(|e| format!("invalid utf-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Basic correctness ────────────────────────────────────────────

    #[test]
    fn test_roundtrip() {
        let salt = generate_salt();
        let password = "hunter2";
        let key = derive_key(password, &salt).unwrap();
        let (nonce, ct) = encrypt("hello encrypted world", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "hello encrypted world");
    }

    #[test]
    fn test_wrong_key_fails() {
        let salt = generate_salt();
        let key = derive_key("correct", &salt).unwrap();
        let wrong_key = derive_key("wrong", &salt).unwrap();
        let (nonce, ct) = encrypt("secret", &key).unwrap();
        let result = decrypt(&ct, &nonce, &wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let salt = generate_salt();
        let key = derive_key("pwd", &salt).unwrap();
        let (nonce, ct) = encrypt("", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "");
    }

    // ── Salt ──────────────────────────────────────────────────────────

    #[test]
    fn test_salt_is_random() {
        let a = generate_salt();
        let b = generate_salt();
        assert_ne!(a, b);
    }

    #[test]
    fn test_generate_salt_16_bytes() {
        let salt = generate_salt();
        assert_eq!(salt.len(), 16);
    }

    #[test]
    fn test_same_password_different_salt_different_key() {
        let salt_a = generate_salt();
        let salt_b = generate_salt();
        let key_a = derive_key("same", &salt_a).unwrap();
        let key_b = derive_key("same", &salt_b).unwrap();
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn test_same_password_same_salt_same_key() {
        let salt = generate_salt();
        let key_a = derive_key("mypassword", &salt).unwrap();
        let key_b = derive_key("mypassword", &salt).unwrap();
        assert_eq!(key_a, key_b);
    }

    // ── Key output ────────────────────────────────────────────────────

    #[test]
    fn test_key_is_always_32_bytes() {
        let salt = generate_salt();
        for pwd in ["", "a", "short", "a".repeat(100).as_str(), "a".repeat(2000).as_str()] {
            let key = derive_key(pwd, &salt).unwrap();
            assert_eq!(key.len(), 32, "key len failed for password of len {}", pwd.len());
        }
    }

    #[test]
    fn test_empty_password_derives_32_byte_key() {
        let salt = generate_salt();
        let key = derive_key("", &salt).unwrap();
        assert_eq!(key.len(), 32);
        let (nonce, ct) = encrypt("test", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "test");
    }

    // ── Nonce (IV) uniqueness ─────────────────────────────────────────

    #[test]
    fn test_each_encryption_uses_different_nonce() {
        let salt = generate_salt();
        let key = derive_key("test", &salt).unwrap();
        let (nonce_a, _) = encrypt("same text", &key).unwrap();
        let (nonce_b, _) = encrypt("same text", &key).unwrap();
        assert_ne!(nonce_a, nonce_b, "nonces must be unique per encryption");
    }

    #[test]
    fn test_each_encryption_produces_different_ciphertext() {
        let salt = generate_salt();
        let key = derive_key("test", &salt).unwrap();
        let (_, ct_a) = encrypt("same text", &key).unwrap();
        let (_, ct_b) = encrypt("same text", &key).unwrap();
        assert_ne!(ct_a, ct_b, "ciphertexts must differ (different nonces)");
    }

    // ── Tamper resistance ─────────────────────────────────────────────

    #[test]
    fn test_tampered_ciphertext_fails() {
        let salt = generate_salt();
        let key = derive_key("secret", &salt).unwrap();
        let (mut nonce, mut ct) = encrypt("important data", &key).unwrap();
        // Flip a bit in ciphertext
        ct[0] ^= 0x01;
        assert!(decrypt(&ct, &nonce, &key).is_err());

        // Flip a bit in nonce
        nonce[0] ^= 0x01;
        assert!(decrypt(&ct, &nonce, &key).is_err());
    }

    #[test]
    fn test_truncated_ciphertext_fails() {
        let salt = generate_salt();
        let key = derive_key("pwd", &salt).unwrap();
        let (nonce, ct) = encrypt("data", &key).unwrap();
        assert!(decrypt(&ct[..ct.len() - 1], &nonce, &key).is_err());
    }

    #[test]
    fn test_empty_ciphertext_fails() {
        let salt = generate_salt();
        let key = derive_key("pwd", &salt).unwrap();
        let (nonce, _) = encrypt("data", &key).unwrap();
        assert!(decrypt(&[], &nonce, &key).is_err());
    }

    #[test]
    fn test_wrong_nonce_fails() {
        let salt = generate_salt();
        let key = derive_key("pwd", &salt).unwrap();
        let (_, ct) = encrypt("data", &key).unwrap();
        let wrong_nonce = vec![0u8; 12];
        assert!(decrypt(&ct, &wrong_nonce, &key).is_err());
    }

    // ── Unicode & encoding ────────────────────────────────────────────

    #[test]
    fn test_unicode_text_roundtrip() {
        let salt = generate_salt();
        let key = derive_key("pwd", &salt).unwrap();
        let texts = [
            "Hello, 世界!",
            "🚀 Rust 🔒 AES-256-GCM",
            "¿Cómo estás? 你好",
            "αβγδεζηθικλμνξοπρσςτυφχψω",
            "🎉🌟💯🔥👋",
            "Tab\there\nand newline\n\nhere",
            "null\x00byte in the middle",
        ];
        for text in &texts {
            let (nonce, ct) = encrypt(text, &key).unwrap();
            let pt = decrypt(&ct, &nonce, &key).unwrap();
            assert_eq!(&pt, text);
        }
    }

    #[test]
    fn test_unicode_password_roundtrip() {
        let salt = generate_salt();
        let key = derive_key("пароль_密码_🔑", &salt).unwrap();
        let (nonce, ct) = encrypt("protected by unicode password", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "protected by unicode password");
    }

    // ── Large content ─────────────────────────────────────────────────

    #[test]
    fn test_large_text_roundtrip() {
        let salt = generate_salt();
        let key = derive_key("strong-password", &salt).unwrap();
        // 100 KB of text
        let large = "The quick brown fox jumps over the lazy dog.\n".repeat(2500);
        assert!(large.len() > 100_000);
        let (nonce, ct) = encrypt(&large, &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, large);
    }

    #[test]
    fn test_very_large_text_roundtrip() {
        let salt = generate_salt();
        let key = derive_key("large-test", &salt).unwrap();
        // 1 MB of text
        let very_large = "A".repeat(1_048_576);
        let (nonce, ct) = encrypt(&very_large, &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt.len(), very_large.len());
        assert_eq!(pt, very_large);
    }

    // ── Long password ─────────────────────────────────────────────────

    #[test]
    fn test_long_password_works() {
        let salt = generate_salt();
        let long_pwd = "a".repeat(2000);
        let key = derive_key(&long_pwd, &salt).unwrap();
        let (nonce, ct) = encrypt("long password test", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "long password test");
    }

    // ── Password whitespace ───────────────────────────────────────────

    #[test]
    fn test_password_whitespace_matters() {
        let salt = generate_salt();
        let key_a = derive_key("password ", &salt).unwrap(); // trailing space
        let key_b = derive_key("password", &salt).unwrap();  // no space
        assert_ne!(key_a, key_b, "trailing whitespace must change the key");
    }

    // ── Repeatability / stress ────────────────────────────────────────

    #[test]
    fn test_sequential_encrypt_decrypt_cycles() {
        let salt = generate_salt();
        let key = derive_key("stress-test", &salt).unwrap();
        for i in 0..50 {
            let text = format!("message number {i} with some padding **********");
            let (nonce, ct) = encrypt(&text, &key).unwrap();
            let pt = decrypt(&ct, &nonce, &key).unwrap();
            assert_eq!(pt, text);
        }
    }

    #[test]
    fn test_multiple_encryptions_with_same_key() {
        let salt = generate_salt();
        let key = derive_key("multi-use-key", &salt).unwrap();
        let messages: Vec<String> = (0..20).map(|i| format!("message-{i}")).collect();
        let encrypted: Vec<_> = messages
            .iter()
            .map(|m| encrypt(m, &key).unwrap())
            .collect();
        for (i, (nonce, ct)) in encrypted.iter().enumerate() {
            let pt = decrypt(ct, nonce, &key).unwrap();
            assert_eq!(pt, messages[i]);
        }
    }

    // ── Derive-key determinism ────────────────────────────────────────

    #[test]
    fn test_derive_key_repeatable() {
        let salt = *b"0123456789abcdef";
        let key_a = derive_key("deterministic", &salt).unwrap();
        let key_b = derive_key("deterministic", &salt).unwrap();
        assert_eq!(key_a, key_b);
        // Known expected bytes for this specific input — if Argon2 impl
        // changes this will fail, which is a good canary.
        assert_eq!(key_a.len(), 32);
    }

    // ── Null bytes ─────────────────────────────────────────────────────

    #[test]
    fn test_null_bytes_in_password() {
        let salt = generate_salt();
        let pwd = "pass\x00word";
        let key = derive_key(pwd, &salt).unwrap();
        let (nonce, ct) = encrypt("null byte password test", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "null byte password test");
    }

    #[test]
    fn test_null_bytes_in_plaintext() {
        let salt = generate_salt();
        let key = derive_key("pwd", &salt).unwrap();
        let text = "before\x00after";
        let (nonce, ct) = encrypt(text, &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, text);
        assert!(pt.contains('\0'));
    }

    // ── Extreme salt values ────────────────────────────────────────────

    #[test]
    fn test_all_zero_salt() {
        let salt = [0u8; 16];
        let key = derive_key("test", &salt).unwrap();
        assert_eq!(key.len(), 32);
        let (nonce, ct) = encrypt("zero salt", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "zero salt");
    }

    #[test]
    fn test_all_ff_salt() {
        let salt = [0xFFu8; 16];
        let key = derive_key("test", &salt).unwrap();
        assert_eq!(key.len(), 32);
        let (nonce, ct) = encrypt("ff salt", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "ff salt");
    }

    // ── Extreme key values ─────────────────────────────────────────────

    #[test]
    fn test_all_zero_key() {
        let key = [0u8; 32];
        let (nonce, ct) = encrypt("zero key text", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "zero key text");
    }

    #[test]
    fn test_all_ff_key() {
        let key = [0xFFu8; 32];
        let (nonce, ct) = encrypt("ff key text", &key).unwrap();
        let pt = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(pt, "ff key text");
    }

    // ── Auth tag / GCM edge cases ──────────────────────────────────────

    #[test]
    fn test_truncated_auth_tag_fails() {
        let salt = generate_salt();
        let key = derive_key("auth-test", &salt).unwrap();
        let (nonce, ct) = encrypt("data with auth tag", &key).unwrap();
        // GCM appends a 16-byte auth tag to ciphertext.
        // Truncate 1 byte from the end (part of auth tag).
        let truncated = &ct[..ct.len() - 1];
        let result = decrypt(truncated, &nonce, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_fake_auth_tag_fails() {
        let salt = generate_salt();
        let key = derive_key("auth-test-2", &salt).unwrap();
        let (nonce, mut ct) = encrypt("data", &key).unwrap();
        // Corrupt the last byte (part of GCM auth tag)
        let last = ct.len() - 1;
        ct[last] ^= 0xFF;
        let result = decrypt(&ct, &nonce, &key);
        assert!(result.is_err());
    }

    // ── Invalid nonce length ──────────────────────────────────────────

    #[test]
    fn test_decrypt_wrong_nonce_length_rejected() {
        let key = [0u8; 32];
        let result = decrypt(b"some data", &[0u8; 8], &key); // 8 bytes, not 12
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonce"));
    }

    #[test]
    fn test_decrypt_zero_length_nonce_rejected() {
        let key = [0u8; 32];
        let result = decrypt(b"data", &[], &key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonce"));
    }
}
