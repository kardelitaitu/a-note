pub mod config;
pub mod crypto;
pub mod note;
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
/// auto-repair by resetting password_protected to false and saving.
/// Returns the decoded salt bytes or an Err describing the repair.
fn salt_from_config(cfg: &config::Config) -> Result<Vec<u8>, String> {
    if cfg.password_salt.is_empty() {
        // Corrupt state: repair by resetting protection
        let mut repaired = config::load();
        repaired.password_protected = false;
        repaired.password_salt = String::new();
        config::save(&repaired);
        return Err("Password config was corrupted and has been reset. Please set a new password.".to_string());
    }
    hex::decode(&cfg.password_salt).map_err(|e| {
        // Corrupt hex: repair by resetting protection
        let mut repaired = config::load();
        repaired.password_protected = false;
        repaired.password_salt = String::new();
        config::save(&repaired);
        format!("Password config was corrupted (invalid salt: {e}) and has been reset. Please set a new password.")
    })
}

// ── Commands ─────────────────────────────────────────────────────────────

#[tauri::command]
fn load_config() -> config::Config {
    config::load()
}

#[tauri::command]
fn save_config(cfg: config::Config) {
    config::save(&cfg);
}

#[tauri::command]
fn load_note() -> LoadNoteResult {
    let nf = note::load_file();
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
    // We receive Config by value but State only gives a reference.
    // Reload config to get latest password_protected flag.
    let config = config::load();

    if config.password_protected {
        let key_guard = state
            .encryption_key
            .lock()
            .map_err(|e| format!("lock error: {e}"))?;
        if let Some(key) = key_guard.as_ref() {
            let nf = note::NoteFile::from_encrypted(&note, key)?;
            note::save_file(&nf);
            Ok(())
        } else {
            Err("Note is password-protected but not unlocked".to_string())
        }
    } else {
        note::save(&note);
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

    let mut config = config::load();
    let salt = crypto::generate_salt();
    let salt_hex = hex::encode(salt);
    let key = crypto::derive_key(&password, &salt)?;

    // Encrypt current note content
    let note = note::load();
    let nf = note::NoteFile::from_encrypted(&note, &key)?;
    note::save_file(&nf);

    // Update config
    config.password_protected = true;
    config.password_salt = salt_hex;
    config::save(&config);

    // Cache the key
    let mut key_guard = state
        .encryption_key
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *key_guard = Some(key);

    Ok(())
}

/// Unlock the note: derive key from password, decrypt, cache the key.
#[tauri::command]
fn unlock(
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<UnlockResult, String> {
    let config = config::load();
    if !config.password_protected {
        return Err("Note is not password-protected".to_string());
    }

    let salt = salt_from_config(&config)?;
    let key = crypto::derive_key(&password, &salt)?;

    let nf = note::load_file();
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
    Ok(())
}

/// Remove password protection: verify password, decrypt, save unencrypted.
#[tauri::command]
fn remove_password(
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let config = config::load();
    if !config.password_protected {
        return Err("Note is not password-protected".to_string());
    }

    let salt = hex::decode(&config.password_salt)
        .map_err(|e| format!("invalid salt hex: {e}"))?;
    let key = crypto::derive_key(&password, &salt)?;

    let nf = note::load_file();
    let decrypted = nf.decrypt_to_note(&key)?;

    // Save as plaintext
    note::save(&decrypted);

    // Update config
    let mut config = config::load();
    config.password_protected = false;
    config.password_salt = String::new();
    config::save(&config);

    // Clear cached key
    let mut key_guard = state
        .encryption_key
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *key_guard = None;

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

    let config = config::load();
    if !config.password_protected {
        return Err("Note is not password-protected".to_string());
    }

    // Verify old password
    let salt = hex::decode(&config.password_salt)
        .map_err(|e| format!("invalid salt hex: {e}"))?;
    let old_key = crypto::derive_key(&old_pwd, &salt)?;

    let nf = note::load_file();
    let decrypted = nf.decrypt_to_note(&old_key)?;

    // Generate new salt and re-encrypt
    let new_salt = crypto::generate_salt();
    let new_key = crypto::derive_key(&new_pwd, &new_salt)?;
    let new_nf = note::NoteFile::from_encrypted(&decrypted, &new_key)?;
    note::save_file(&new_nf);

    // Update config with new salt
    let mut config = config::load();
    config.password_salt = hex::encode(new_salt);
    config::save(&config);

    // Cache new key
    let mut key_guard = state
        .encryption_key
        .lock()
        .map_err(|e| format!("lock error: {e}"))?;
    *key_guard = Some(new_key);

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
    tauri::Builder::default()
        .manage(AppState {
            encryption_key: Mutex::new(None),
        })
        .setup(|app| {
            let cfg = config::load();
            if let Some(window) = app.get_webview_window("main") {
                if config::exists() {
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
