pub mod config;
pub mod crypto;
pub mod diagnostics;
pub mod note;
pub mod paths;
pub mod storage;
pub mod tray;
pub mod util;

use serde::Serialize;
use std::sync::Mutex;
use tauri::window::Color;
use tauri::Manager;

/// Cached encryption key — kept in memory while the note is unlocked.
/// Cleared on `lock()` or when the lock timer fires.
struct AppState {
    encryption_key: Mutex<Option<[u8; 32]>>,
}

impl AppState {
    /// Borrow the cached encryption key (read-only).
    fn get_key(&self) -> Result<Option<[u8; 32]>, String> {
        self.encryption_key
            .lock()
            .map(|g| g.clone())
            .map_err(|e| format!("lock error: {e}"))
    }

    /// Store a derived encryption key.
    fn set_key(&self, key: [u8; 32]) -> Result<(), String> {
        let mut guard = self
            .encryption_key
            .lock()
            .map_err(|e| format!("lock error: {e}"))?;
        *guard = Some(key);
        Ok(())
    }

    /// Clear the cached encryption key (lock).
    fn clear_key(&self) -> Result<(), String> {
        let mut guard = self
            .encryption_key
            .lock()
            .map_err(|e| format!("lock error: {e}"))?;
        *guard = None;
        Ok(())
    }
}

#[tauri::command]
fn update_tray_color(color: String, app: tauri::AppHandle) {
    tray::update_color(&app, &color);
}

#[tauri::command]
fn set_start_with_windows(enabled: bool) -> Result<(), String> {
    util::set_startup_registry(enabled);
    let mut data = storage::try_load()?;
    data.config.start_with_windows = enabled;
    data.log = diagnostics::flush_to_log_str();
    storage::save(&data)?;
    diagnostics::event(
        "startup",
        &format!(
            "Start with Windows {}",
            if enabled { "enabled" } else { "disabled" }
        ),
    );
    Ok(())
}

// ── Response types ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LoadNoteResult {
    pub locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_pos: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_top: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UnlockResult {
    pub ok: bool,
    pub text: Option<String>,
    pub cursor_pos: u32,
    pub scroll_top: u32,
}

/// Try to decode the password salt from config.
/// Fail-closed behavior: never mutates on-disk encrypted data automatically.
fn decode_salt_hex(salt_hex: &str) -> Result<Vec<u8>, String> {
    let salt = hex::decode(salt_hex).map_err(|e| {
        format!(
            "Password configuration is invalid (salt hex: {e}). Encrypted note was preserved."
        )
    })?;
    if salt.len() < 8 {
        return Err(format!(
            "Password configuration is invalid (salt length: {} bytes). Encrypted note was preserved.",
            salt.len()
        ));
    }
    Ok(salt)
}

fn try_recover_salt_from_legacy_config() -> Result<Vec<u8>, String> {
    let legacy_cfg = config::load();
    if legacy_cfg.password_salt.is_empty() {
        return Err("legacy password salt not found".to_string());
    }

    let salt = decode_salt_hex(&legacy_cfg.password_salt)?;

    // Persist recovered salt into combined storage so future unlocks don't depend
    // on legacy files.
    let mut data = storage::try_load()?;
    data.config.password_protected = true;
    data.config.password_salt = legacy_cfg.password_salt;
    storage::save(&data)?;

    Ok(salt)
}

fn salt_from_config(cfg: &config::Config) -> Result<Vec<u8>, String> {
    if cfg.password_salt.is_empty() {
        if let Ok(salt) = try_recover_salt_from_legacy_config() {
            return Ok(salt);
        }
        return Err(
            "Password configuration is missing salt. Encrypted note was preserved. Restore a valid config backup before unlocking."
                .to_string(),
        );
    }

    match decode_salt_hex(&cfg.password_salt) {
        Ok(salt) => Ok(salt),
        Err(_) => {
            if let Ok(salt) = try_recover_salt_from_legacy_config() {
                return Ok(salt);
            }
            decode_salt_hex(&cfg.password_salt)
        }
    }
}

// ── Commands ─────────────────────────────────────────────────────────────

#[tauri::command]
fn load_config() -> Result<config::Config, String> {
    Ok(storage::try_load()?.config)
}

#[tauri::command]
fn save_config(cfg: config::Config) -> Result<(), String> {
    let mut data = storage::try_load()?;
    let mut merged = cfg;

    // Password metadata is security-critical and should be authored only by
    // password commands (set/remove/change). Preserve persisted values here
    // to avoid accidental UI overwrites from stale client state.
    merged.password_protected = data.config.password_protected;
    merged.password_salt = data.config.password_salt.clone();

    data.config = merged;
    data.log = diagnostics::flush_to_log_str();
    storage::save(&data)
}

#[tauri::command]
fn load_note() -> Result<LoadNoteResult, String> {
    let nf = storage::try_load()?.note;
    if nf.encrypted {
        // Locked - frontend must prompt for password
        Ok(LoadNoteResult {
            locked: true,
            text: None,
            cursor_pos: Some(nf.cursor_pos),
            scroll_top: Some(nf.scroll_top),
        })
    } else {
        Ok(LoadNoteResult {
            locked: false,
            text: Some(nf.text),
            cursor_pos: Some(nf.cursor_pos),
            scroll_top: Some(nf.scroll_top),
        })
    }
}

#[tauri::command]
fn save_note(
    note: note::Note,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut data = storage::try_load()?;

    if data.config.password_protected {
        let key = state.get_key()?;
        if let Some(key) = key {
            let nf = note::NoteFile::from_encrypted(&note, &key)?;
            data.note = nf;
            data.log = diagnostics::flush_to_log_str();
            storage::save(&data)
        } else {
            Err("Note is password-protected but not unlocked".to_string())
        }
    } else {
        data.note = note::NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: note.text,
            cursor_pos: note.cursor_pos,
            scroll_top: note.scroll_top,
        };
        data.log = diagnostics::flush_to_log_str();
        storage::save(&data)
    }
}

fn ensure_set_password_preconditions(data: &storage::NoteData) -> Result<(), String> {
    if data.config.password_protected || data.note.encrypted {
        return Err(
            "Password is already set or note data is already encrypted. Use Change password after unlocking."
                .to_string(),
        );
    }
    Ok(())
}

/// Set a password on the note: generate salt, derive key, encrypt current
/// content, persist everything.
#[tauri::command]
fn set_password(
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if password.is_empty() {
        return Err("Password cannot be empty".to_string());
    }

    let mut data = storage::try_load()?;
    ensure_set_password_preconditions(&data)?;
    let salt = crypto::generate_salt();
    let salt_hex = hex::encode(salt);
    let key = crypto::derive_key(&password, &salt)?;

    // Encrypt current note content
    let note = note::Note {
        text: data.note.text.clone(),
        cursor_pos: data.note.cursor_pos,
        scroll_top: data.note.scroll_top,
    };
    let nf = note::NoteFile::from_encrypted(&note, &key)?;

    // Update data
    data.note = nf;
    data.config.password_protected = true;
    data.config.password_salt = salt_hex;
    data.log = diagnostics::flush_to_log_str();
    storage::save(&data)?;

    state.set_key(key)?;

    diagnostics::event("password", "Password set");
    Ok(())
}

/// Unlock the note: derive key from password, decrypt, cache the key.
#[tauri::command]
fn unlock(
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<UnlockResult, String> {
    let data = storage::try_load()?;
    let cfg = &data.config;
    if !cfg.password_protected {
        return Err("Note is not password-protected".to_string());
    }

    let salt = salt_from_config(cfg)?;
    let key = crypto::derive_key(&password, &salt)?;

    let nf = data.note;
    if !nf.encrypted {
        return Err("Note file is not encrypted but password is set".to_string());
    }

    let decrypted = nf.decrypt_to_note(&key)?;

    state.set_key(key)?;

    diagnostics::event("unlock", "Note unlocked");
    Ok(UnlockResult {
        ok: true,
        text: Some(decrypted.text),
        cursor_pos: decrypted.cursor_pos,
        scroll_top: decrypted.scroll_top,
    })
}

/// Lock the note: clear the cached encryption key.
#[tauri::command]
fn lock(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.clear_key()?;
    diagnostics::event("lock", "Note locked");
    Ok(())
}

/// Remove password protection: verify password, decrypt, save unencrypted.
#[tauri::command]
fn remove_password(
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut data = storage::try_load()?;
    if !data.config.password_protected {
        return Err("Note is not password-protected".to_string());
    }
    if !data.note.encrypted {
        return Err(
            "Cannot remove password because note data is not encrypted. Repair the storage file first."
                .to_string(),
        );
    }

    let salt = salt_from_config(&data.config)?;
    let key = crypto::derive_key(&password, &salt)?;

    let decrypted = data.note.decrypt_to_note(&key)?;

    // Save as plaintext within combined file
    data.note = note::NoteFile {
        encrypted: false,
        nonce_hex: None,
        ciphertext_hex: None,
        text: decrypted.text,
        cursor_pos: decrypted.cursor_pos,
        scroll_top: decrypted.scroll_top,
    };
    data.config.password_protected = false;
    data.config.password_salt = String::new();
    data.log = diagnostics::flush_to_log_str();
    storage::save(&data)?;

    state.clear_key()?;

    diagnostics::event("password", "Password removed");
    Ok(())
}

/// Change password: verify old, re-encrypt with new.
#[tauri::command]
fn change_password(
    old_pwd: String,
    new_pwd: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if new_pwd.is_empty() {
        return Err("New password cannot be empty".to_string());
    }

    let mut data = storage::try_load()?;
    if !data.config.password_protected {
        return Err("Note is not password-protected".to_string());
    }
    if !data.note.encrypted {
        return Err(
            "Cannot change password because note data is not encrypted. Repair the storage file first."
                .to_string(),
        );
    }

    // Verify old password
    let salt = salt_from_config(&data.config)?;
    let old_key = crypto::derive_key(&old_pwd, &salt)?;

    let decrypted = data.note.decrypt_to_note(&old_key)?;

    // Generate new salt and re-encrypt
    let new_salt = crypto::generate_salt();
    let new_key = crypto::derive_key(&new_pwd, &new_salt)?;
    let new_nf = note::NoteFile::from_encrypted(&decrypted, &new_key)?;

    // Update data
    data.note = new_nf;
    data.config.password_salt = hex::encode(new_salt);
    data.log = diagnostics::flush_to_log_str();
    storage::save(&data)?;

    state.set_key(new_key)?;

    diagnostics::event("password", "Password changed");
    Ok(())
}

#[tauri::command]
fn get_app_name() -> String {
    crate::paths::exe_stem()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    diagnostics::init();

    // Migrate from legacy separate-file format (v0.1.x → v0.2.0)
    if storage::legacy_exists() {
        let data = storage::migrate_from_legacy();
        diagnostics::restore_from_log_str(&data.log);
    } else if storage::exists() {
        let data = storage::load();
        diagnostics::restore_from_log_str(&data.log);
    }

    tauri::Builder::default()
        .manage(AppState {
            encryption_key: Mutex::new(None),
        })
        .manage(tray::TrayState::<tauri::Wry>::new())
        .setup(|app| {
            let data = storage::load();
            let cfg = data.config;

            let exe_name = crate::paths::exe_stem();
            let tray_color = if cfg.titlebar_color.is_empty() {
                "#5dade2"
            } else {
                &cfg.titlebar_color
            };

            let _ = tray::build(app.handle(), &exe_name, tray_color);
            if let Some(window) = app.get_webview_window("main") {
                if storage::exists() || storage::legacy_exists() {
                    let _ = window.set_position(tauri::PhysicalPosition::new(cfg.left, cfg.top));
                    let _ = window.set_size(tauri::PhysicalSize::new(cfg.width, cfg.height));
                } else {
                    let _ = window.set_size(tauri::PhysicalSize::new(cfg.width, cfg.height));
                    let _ = window.center();
                }
                let _ = window.set_always_on_top(cfg.always_on_top);
                #[cfg(windows)]
                let _ = window.set_background_color(Some(Color(30, 30, 30, 255)));
                let _ = window.show();
                let _ = window.set_focus();
                // Set taskbar title to match exe filename
                let _ = window.set_title(&exe_name);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_name,
            load_config,
            save_config,
            load_note,
            save_note,
            set_password,
            unlock,
            lock,
            remove_password,
            change_password,
            update_tray_color,
            set_start_with_windows,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn notes_path() -> PathBuf {
        crate::paths::notes_path()
    }

    fn legacy_config_path() -> PathBuf {
        crate::paths::legacy_config_path()
    }

    fn cleanup_notes_file() {
        let _ = std::fs::remove_file(notes_path());
    }

    fn cleanup_legacy_config_file() {
        let _ = std::fs::remove_file(legacy_config_path());
    }

    // ── salt_from_config ──────────────────────────────────────────

    #[test]
    fn test_salt_from_config_ok() {
        let hex_salt = "deadbeef010203040506070809101112";
        let cfg = config::Config {
            password_salt: hex_salt.to_string(),
            ..config::Config::default()
        };
        let result = salt_from_config(&cfg);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), hex::decode(hex_salt).unwrap());
    }

    #[test]
    fn test_salt_from_config_ok_full_salt() {
        let hex_salt = "aabbccddee00112233445566778899ff";
        let cfg = config::Config {
            password_salt: hex_salt.to_string(),
            ..config::Config::default()
        };
        let result = salt_from_config(&cfg);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), hex::decode(hex_salt).unwrap());
    }

    #[test]
    fn test_salt_from_config_uppercase_hex() {
        let hex_salt = "DEADBEEF01020304";
        let cfg = config::Config {
            password_salt: hex_salt.to_string(),
            ..config::Config::default()
        };
        let result = salt_from_config(&cfg);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), hex::decode(hex_salt).unwrap());
    }

    // ── get_app_name ──────────────────────────────────────────────

    #[test]
    fn test_get_app_name_not_empty() {
        let name = get_app_name();
        assert!(!name.is_empty(), "app name should not be empty");
    }

    // ── LoadNoteResult ────────────────────────────────────────────

    #[test]
    fn test_load_note_result_locked() {
        let res = LoadNoteResult {
            locked: true,
            text: None,
            cursor_pos: Some(5),
            scroll_top: Some(10),
        };
        let json = serde_json::to_string(&res).unwrap();
        assert!(json.contains("\"locked\":true"));
        assert!(!json.contains("\"text\""));
    }

    #[test]
    fn test_load_note_result_unlocked() {
        let res = LoadNoteResult {
            locked: false,
            text: Some("hello".to_string()),
            cursor_pos: Some(3),
            scroll_top: Some(0),
        };
        let json = serde_json::to_string(&res).unwrap();
        assert!(json.contains("\"locked\":false"));
        assert!(json.contains("\"text\":\"hello\""));
    }

    // ── UnlockResult ──────────────────────────────────────────────

    #[test]
    fn test_unlock_result_serialization() {
        let res = UnlockResult {
            ok: true,
            text: Some("secret note".to_string()),
            cursor_pos: 42,
            scroll_top: 7,
        };
        let json = serde_json::to_string(&res).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["text"], "secret note");
        assert_eq!(parsed["cursor_pos"], 42);
        assert_eq!(parsed["scroll_top"], 7);
    }

    #[test]
    fn test_salt_from_config_fail_closed_preserves_encrypted_note() {
        // Missing salt should fail closed with an explicit recovery message.
        let _lock = storage::STORAGE_TEST_FILE_LOCK
            .lock()
            .expect("failed to lock storage test lock");
        cleanup_legacy_config_file();
        let mut cfg = config::Config::default();
        cfg.password_protected = true;
        cfg.password_salt = String::new();

        let result = salt_from_config(&cfg);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("preserved"));
        cleanup_legacy_config_file();
    }

    #[test]
    fn test_salt_from_config_recovers_from_legacy_config_file() {
        let _lock = storage::STORAGE_TEST_FILE_LOCK
            .lock()
            .expect("failed to lock storage test lock");
        cleanup_notes_file();
        cleanup_legacy_config_file();

        let password = "legacy-recovery";
        let salt = crypto::generate_salt();
        let key = crypto::derive_key(password, &salt).unwrap();
        let note = note::Note {
            text: "recover me".to_string(),
            cursor_pos: 2,
            scroll_top: 1,
        };
        let encrypted = note::NoteFile::from_encrypted(&note, &key).unwrap();

        let mut combined_cfg = config::Config::default();
        combined_cfg.password_protected = true;
        combined_cfg.password_salt = String::new();
        let combined = storage::NoteData {
            version: 1,
            config: combined_cfg,
            note: encrypted,
            log: String::new(),
        };
        storage::save(&combined).unwrap();

        let mut legacy_cfg = config::Config::default();
        legacy_cfg.password_protected = true;
        legacy_cfg.password_salt = hex::encode(salt);
        let legacy_json = serde_json::to_string_pretty(&legacy_cfg).unwrap();
        std::fs::write(legacy_config_path(), legacy_json).unwrap();

        let loaded = storage::load();
        let recovered = salt_from_config(&loaded.config).unwrap();
        assert_eq!(recovered, salt.to_vec());

        let healed = storage::load();
        assert_eq!(healed.config.password_salt, hex::encode(salt));
        let verify_key = crypto::derive_key(password, &recovered).unwrap();
        let decrypted = healed.note.decrypt_to_note(&verify_key).unwrap();
        assert_eq!(decrypted.text, "recover me");

        cleanup_notes_file();
        cleanup_legacy_config_file();
    }

    #[test]
    fn test_set_password_precondition_rejects_when_already_protected() {
        let mut data = storage::NoteData {
            version: 1,
            config: config::Config::default(),
            note: note::NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: "plain".to_string(),
                cursor_pos: 0,
                scroll_top: 0,
            },
            log: String::new(),
        };
        data.config.password_protected = true;
        data.config.password_salt = "aabbccddeeff00112233445566778899".to_string();

        let result = ensure_set_password_preconditions(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already set"));
    }

    #[test]
    fn test_set_password_precondition_rejects_when_note_already_encrypted() {
        let data = storage::NoteData {
            version: 1,
            config: config::Config::default(),
            note: note::NoteFile {
                encrypted: true,
                nonce_hex: Some("00112233445566778899aabb".to_string()),
                ciphertext_hex: Some("deadbeef".to_string()),
                text: String::new(),
                cursor_pos: 0,
                scroll_top: 0,
            },
            log: String::new(),
        };

        let result = ensure_set_password_preconditions(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already encrypted"));
    }

    #[test]
    fn test_save_config_write_failure_is_ui_visible() {
        let _lock = storage::STORAGE_TEST_FILE_LOCK
            .lock()
            .expect("failed to lock storage test lock");
        cleanup_notes_file();

        util::set_forced_write_error_for_current_thread(Some(
            "forced write failure (test)".to_string(),
        ));

        let result = save_config(config::Config::default());
        util::set_forced_write_error_for_current_thread(None);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("forced write failure (test)"));

        cleanup_notes_file();
    }

    #[test]
    fn test_save_config_preserves_password_metadata_from_storage() {
        let _lock = storage::STORAGE_TEST_FILE_LOCK
            .lock()
            .expect("failed to lock storage test lock");
        cleanup_notes_file();
        cleanup_legacy_config_file();

        let password = "preserve-salt";
        let salt = crypto::generate_salt();
        let salt_hex = hex::encode(salt);
        let key = crypto::derive_key(password, &hex::decode(&salt_hex).unwrap()).unwrap();
        let note = note::Note {
            text: "secret".to_string(),
            cursor_pos: 1,
            scroll_top: 0,
        };
        let encrypted = note::NoteFile::from_encrypted(&note, &key).unwrap();

        let mut stored_cfg = config::Config::default();
        stored_cfg.password_protected = true;
        stored_cfg.password_salt = salt_hex.clone();
        stored_cfg.width = 300;

        let initial = storage::NoteData {
            version: 1,
            config: stored_cfg.clone(),
            note: encrypted,
            log: String::new(),
        };
        storage::save(&initial).unwrap();

        // Simulate stale UI config object (missing/empty password metadata).
        let mut ui_cfg = stored_cfg;
        ui_cfg.width = 777;
        ui_cfg.password_protected = false;
        ui_cfg.password_salt = String::new();

        save_config(ui_cfg).unwrap();

        let loaded = storage::load();
        assert_eq!(loaded.config.width, 777);
        assert!(loaded.config.password_protected);
        assert_eq!(loaded.config.password_salt, salt_hex);

        let recovered_key = crypto::derive_key(password, &hex::decode(&loaded.config.password_salt).unwrap()).unwrap();
        let decrypted = loaded.note.decrypt_to_note(&recovered_key).unwrap();
        assert_eq!(decrypted.text, "secret");

        cleanup_notes_file();
        cleanup_legacy_config_file();
    }

    // ── decode_salt_hex ──────────────────────────────────────────

    #[test]
    fn test_decode_salt_hex_short() {
        // "aabbccddee" = 5 bytes, which is < 8 byte minimum
        let result = decode_salt_hex("aabbccddee");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("salt length"));
    }

    #[test]
    fn test_decode_salt_hex_invalid() {
        let result = decode_salt_hex("ZZZZZZZZZZZZZZZZ");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("salt hex"));
    }

    #[test]
    fn test_decode_salt_hex_valid() {
        let hex_salt = "deadbeef010203040506070809101112";
        let result = decode_salt_hex(hex_salt);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), hex::decode(hex_salt).unwrap());
    }

    #[test]
    fn test_decode_salt_hex_empty() {
        let result = decode_salt_hex("");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_salt_hex_8_bytes() {
        // Exactly 8 bytes (16 hex chars) — the minimum
        let hex_salt = "aabbccddee001122";
        let result = decode_salt_hex(hex_salt);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), hex::decode(hex_salt).unwrap());
    }

    // ── ensure_set_password_preconditions ────────────────────────

    #[test]
    fn test_ensure_set_password_preconditions_ok() {
        let data = storage::NoteData {
            version: 1,
            config: config::Config::default(),
            note: note::NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: "plain".to_string(),
                cursor_pos: 0,
                scroll_top: 0,
            },
            log: String::new(),
        };
        // Both password_protected (default false) and encrypted (false) — should pass
        let result = ensure_set_password_preconditions(&data);
        assert!(result.is_ok());
    }
}
