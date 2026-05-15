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

    #[test]
    fn test_salt_is_random() {
        let a = generate_salt();
        let b = generate_salt();
        assert_ne!(a, b);
    }

    #[test]
    fn test_same_password_different_salt_different_key() {
        let salt_a = generate_salt();
        let salt_b = generate_salt();
        let key_a = derive_key("same", &salt_a).unwrap();
        let key_b = derive_key("same", &salt_b).unwrap();
        assert_ne!(key_a, key_b);
    }
}
