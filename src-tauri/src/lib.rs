pub mod config;
pub mod crypto;
pub mod diagnostics;
pub mod note;
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

#[tauri::command]
fn update_tray_color(color: String, app: tauri::AppHandle) {
    tray::update_color(&app, &color);
}

#[tauri::command]
fn set_start_with_windows(enabled: bool) {
    util::set_startup_registry(enabled);
    let mut data = storage::load();
    data.config.start_with_windows = enabled;
    data.log = diagnostics::flush_to_log_str();
    storage::save(&data);
    diagnostics::event(
        "startup",
        &format!(
            "Start with Windows {}",
            if enabled { "enabled" } else { "disabled" }
        ),
    );
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
/// If the config is corrupt (password_protected true but salt missing/invalid),
/// auto-repair by resetting password_protected to false, converting any
/// encrypted note back to plaintext (empty content), and saving.
/// Returns the decoded salt bytes or an Err describing the repair.
fn salt_from_config(cfg: &config::Config) -> Result<Vec<u8>, String> {
    if cfg.password_salt.is_empty() {
        // Corrupt state: repair by resetting protection and clearing encrypted note
        let mut data = storage::load();
        data.config.password_protected = false;
        data.config.password_salt = String::new();
        if data.note.encrypted {
            data.note = note::NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: String::new(),
                cursor_pos: data.note.cursor_pos,
                scroll_top: data.note.scroll_top,
            };
        }
        storage::save(&data);
        return Err("Password config was corrupted and has been reset. Please set a new password.".to_string());
    }
    hex::decode(&cfg.password_salt).map_err(|e| {
        // Corrupt hex: repair by resetting protection
        let mut data = storage::load();
        data.config.password_protected = false;
        data.config.password_salt = String::new();
        if data.note.encrypted {
            data.note = note::NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: String::new(),
                cursor_pos: data.note.cursor_pos,
                scroll_top: data.note.scroll_top,
            };
        }
        storage::save(&data);
        format!("Password config was corrupted (invalid salt: {e}) and has been reset. Please set a new password.")
    })
}

// ── Commands ─────────────────────────────────────────────────────────────

#[tauri::command]
fn load_config() -> config::Config {
    storage::load().config
}

#[tauri::command]
fn save_config(cfg: config::Config) {
    let mut data = storage::load();
    data.config = cfg;
    data.log = diagnostics::flush_to_log_str();
    storage::save(&data);
}

#[tauri::command]
fn load_note() -> LoadNoteResult {
    let nf = storage::load().note;
    if nf.encrypted {
        // Locked — frontend must prompt for password
        LoadNoteResult {
            locked: true,
            text: None,
            cursor_pos: Some(nf.cursor_pos),
            scroll_top: Some(nf.scroll_top),
        }
    } else {
        LoadNoteResult {
            locked: false,
            text: Some(nf.text),
            cursor_pos: Some(nf.cursor_pos),
            scroll_top: Some(nf.scroll_top),
        }
    }
}

#[tauri::command]
fn save_note(
    note: note::Note,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut data = storage::load();

    if data.config.password_protected {
        let key_guard = state
            .encryption_key
            .lock()
            .map_err(|e| format!("lock error: {e}"))?;
        if let Some(key) = key_guard.as_ref() {
            let nf = note::NoteFile::from_encrypted(&note, key)?;
            data.note = nf;
            data.log = diagnostics::flush_to_log_str();
            storage::save(&data);
            Ok(())
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
        storage::save(&data);
        Ok(())
    }
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

    let mut data = storage::load();
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
    storage::save(&data);

    // Cache the key
    let mut key_guard = state
        .encryption_key
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *key_guard = Some(key);

    diagnostics::event("password", "Password set");
    Ok(())
}

/// Unlock the note: derive key from password, decrypt, cache the key.
#[tauri::command]
fn unlock(
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<UnlockResult, String> {
    let data = storage::load();
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

    // Cache the key
    let mut key_guard = state
        .encryption_key
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *key_guard = Some(key);

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
    let mut key_guard = state
        .encryption_key
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *key_guard = None;
    diagnostics::event("lock", "Note locked");
    Ok(())
}

/// Remove password protection: verify password, decrypt, save unencrypted.
#[tauri::command]
fn remove_password(
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut data = storage::load();
    if !data.config.password_protected {
        return Err("Note is not password-protected".to_string());
    }

    let salt = hex::decode(&data.config.password_salt)
        .map_err(|e| format!("invalid salt hex: {e}"))?;
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
    storage::save(&data);

    // Clear cached key
    let mut key_guard = state
        .encryption_key
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *key_guard = None;

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

    let mut data = storage::load();
    if !data.config.password_protected {
        return Err("Note is not password-protected".to_string());
    }

    // Verify old password
    let salt = hex::decode(&data.config.password_salt)
        .map_err(|e| format!("invalid salt hex: {e}"))?;
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
    storage::save(&data);

    // Cache new key
    let mut key_guard = state
        .encryption_key
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *key_guard = Some(new_key);

    diagnostics::event("password", "Password changed");
    Ok(())
}

#[tauri::command]
fn get_app_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "Notes".to_string())
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

            let exe_name = std::env::current_exe()
                .ok()
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "Notes".to_string());
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
    fn test_salt_from_config_repair_clears_encrypted_note() {
        // Simulate the exact corrupt state: password_protected=true,
        // password_salt="", but note IS encrypted with valid crypto fields.
        // salt_from_config should repair by resetting both password and note.
        let mut cfg = config::Config::default();
        cfg.password_protected = true;
        cfg.password_salt = String::new();

        let key = [0xABu8; 32];
        let note = note::Note {
            text: "lost content".to_string(),
            cursor_pos: 10,
            scroll_top: 5,
        };
        let nf = note::NoteFile::from_encrypted(&note, &key).unwrap();
        let data = storage::NoteData {
            version: 1,
            config: cfg,
            note: nf,
            log: String::new(),
        };
        storage::save(&data);

        let loaded = storage::load();
        let result = salt_from_config(&loaded.config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("corrupted"));

        let repaired = storage::load();
        assert!(!repaired.config.password_protected);
        assert!(!repaired.note.encrypted);
        assert!(repaired.note.text.is_empty());
        assert_eq!(repaired.note.cursor_pos, 10);
        assert_eq!(repaired.note.scroll_top, 5);

        // Clean up
        let _ = std::fs::remove_file(
            std::env::current_exe().unwrap().parent().unwrap().join(
                std::env::current_exe().unwrap().file_stem().unwrap().to_string_lossy().as_ref().to_string() + ".notes"
            )
        );
    }
}
