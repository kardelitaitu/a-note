//! Combined storage: single `.notes` file holds config, note content, and log.
//!
//! On first launch after upgrade from v0.1.x, migrates the old separate
//! `{exe}.config`, `{exe}.notes`, and `{exe}.log` files into one combined file.
//!
//! Format: JSON with version field for future evolution.
//!
//! ```json
//! { "version": 1, "config": {...}, "note": {...}, "log": "..." }
//! ```

use std::path::PathBuf;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NoteData {
    pub version: u32,
    pub config: crate::config::Config,
    pub note: crate::note::NoteFile,
    pub log: String,
}

const CURRENT_VERSION: u32 = 1;

fn exe_stem() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "notes".to_string())
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn notes_path() -> PathBuf {
    exe_dir().join(format!("{}.notes", exe_stem()))
}

fn legacy_config_path() -> PathBuf {
    exe_dir().join(format!("{}.config", exe_stem()))
}

fn legacy_log_path() -> PathBuf {
    exe_dir().join(format!("{}.log", exe_stem()))
}

/// Check if the combined notes file already exists.
pub fn exists() -> bool {
    notes_path().exists()
}

/// Check if old separate config file exists (pre-v0.2.0 format).
pub fn legacy_exists() -> bool {
    legacy_config_path().exists()
}

/// Load the combined NoteData from `{exe}.notes`.
/// If the file doesn't exist or is corrupt, returns a fresh default.
/// Also auto-repairs common corrupt states on load.
pub fn load() -> NoteData {
    try_load().unwrap_or_else(|_| fresh())
}

/// Load with surfaced errors so command handlers can bubble failures to UI.
pub fn try_load() -> Result<NoteData, String> {
    let path = notes_path();
    if !path.exists() {
        return Ok(fresh());
    }

    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let mut data = serde_json::from_str::<NoteData>(&raw)
        .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;

    if reconcile_invariants(&mut data) {
        save(&data)?;
    }

    Ok(data)
}

fn reconcile_invariants(data: &mut NoteData) -> bool {
    let mut changed = false;

    // Fail-closed: encrypted content must stay in protected mode.
    if data.note.encrypted && !data.config.password_protected {
        data.config.password_protected = true;
        changed = true;
    }

    // Safe auto-fix: plaintext note must not stay in protected mode.
    if !data.note.encrypted && data.config.password_protected {
        data.config.password_protected = false;
        changed = true;
    }

    // Remove stale salt whenever protection is disabled.
    if !data.config.password_protected && !data.config.password_salt.is_empty() {
        data.config.password_salt.clear();
        changed = true;
    }

    changed
}

/// Save the combined NoteData to `{exe}.notes`.
pub fn save(data: &NoteData) -> Result<(), String> {
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| format!("failed to serialize notes data: {e}"))?;
    crate::util::write(&notes_path(), &json)
}
/// Load just the config from the combined file.
pub fn load_config() -> crate::config::Config {
    load().config
}

/// Save config through the combined file, flushing diagnostics log.
pub fn save_config(cfg: &crate::config::Config) -> Result<(), String> {
    let mut data = try_load()?;
    data.config = cfg.clone();
    data.log = crate::diagnostics::flush_to_log_str();
    save(&data)
}

/// Load just the NoteFile from the combined file.
pub fn load_note_file() -> crate::note::NoteFile {
    load().note
}

/// Save NoteFile through the combined file, flushing diagnostics log.
pub fn save_note_file(nf: &crate::note::NoteFile) -> Result<(), String> {
    let mut data = try_load()?;
    data.note = nf.clone();
    data.log = crate::diagnostics::flush_to_log_str();
    save(&data)
}

/// Atomically save both config and NoteFile (used by password operations).
pub fn save_config_and_note(
    cfg: &crate::config::Config,
    nf: &crate::note::NoteFile,
) -> Result<(), String> {
    let mut data = try_load()?;
    data.config = cfg.clone();
    data.note = nf.clone();
    data.log = crate::diagnostics::flush_to_log_str();
    save(&data)
}

/// Create a fresh NoteData with defaults.
fn fresh() -> NoteData {
    NoteData {
        version: CURRENT_VERSION,
        config: crate::config::Config::default(),
        note: crate::note::NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        },
        log: String::new(),
    }
}

/// Migrate from the old separate-file format (v0.1.x) to the combined format.
/// Reads `{exe}.config`, `{exe}.notes`, and `{exe}.log`, merges them,
/// writes the combined file, and deletes the old files.
pub fn migrate_from_legacy() -> NoteData {
    let config = if legacy_config_path().exists() {
        crate::config::load()
    } else {
        crate::config::Config::default()
    };

    let note = crate::note::load_file();

    let log = std::fs::read_to_string(&legacy_log_path()).unwrap_or_default();

    // Defensive: if config says password-protected but salt is missing or
    // invalid, repair it before writing the combined file. This prevents
    // "Password config was corrupted" errors on the first unlock attempt.
    let config = if config.password_protected && config.password_salt.is_empty() {
        let mut repaired = config;
        repaired.password_protected = false;
        repaired
    } else {
        config
    };

    let data = NoteData {
        version: CURRENT_VERSION,
        config,
        note,
        log,
    };

    let mut data = data;
    let _ = reconcile_invariants(&mut data);

    // Write the combined file
    let _ = save(&data);

    // Remove old files silently
    let _ = std::fs::remove_file(&legacy_config_path());
    let _ = std::fs::remove_file(&legacy_log_path());

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Format / serialization tests (no file I/O) ────────────────

    #[test]
    fn test_fresh_defaults() {
        let data = fresh();
        assert_eq!(data.version, 1);
        assert!(!data.config.password_protected);
        assert!(!data.note.encrypted);
        assert!(data.note.text.is_empty());
        assert!(data.log.is_empty());
    }

    #[test]
    fn test_note_data_json_roundtrip() {
        let mut cfg = crate::config::Config::default();
        cfg.font_size = 24;
        cfg.always_on_top = false;
        cfg.theme = "dracula".to_string();

        let nf = crate::note::NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: "hello from storage".to_string(),
            cursor_pos: 5,
            scroll_top: 2,
        };

        let data = NoteData {
            version: 1,
            config: cfg,
            note: nf,
            log: "[100] startup: started\n".to_string(),
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        let restored: NoteData = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.version, 1);
        assert_eq!(restored.config.font_size, 24);
        assert!(!restored.config.always_on_top);
        assert_eq!(restored.config.theme, "dracula");
        assert!(!restored.note.encrypted);
        assert_eq!(restored.note.text, "hello from storage");
        assert_eq!(restored.note.cursor_pos, 5);
        assert_eq!(restored.note.scroll_top, 2);
        assert!(restored.log.contains("startup"));
    }

    #[test]
    fn test_note_data_with_encrypted_note_roundtrip() {
        let key = [0xABu8; 32];
        let note = crate::note::Note {
            text: "encrypted in storage".to_string(),
            cursor_pos: 10,
            scroll_top: 3,
        };
        let nf = crate::note::NoteFile::from_encrypted(&note, &key).unwrap();

        let data = NoteData {
            version: 1,
            config: crate::config::Config::default(),
            note: nf,
            log: String::new(),
        };

        let json = serde_json::to_string_pretty(&data).unwrap();
        let restored: NoteData = serde_json::from_str(&json).unwrap();

        assert!(restored.note.encrypted);
        assert!(restored.note.nonce_hex.is_some());
        assert!(restored.note.ciphertext_hex.is_some());
        assert_eq!(restored.note.cursor_pos, 10);
        assert_eq!(restored.note.scroll_top, 3);

        // Decrypt and verify
        let decrypted = restored.note.decrypt_to_note(&key).unwrap();
        assert_eq!(decrypted.text, "encrypted in storage");
    }

    #[test]
    fn test_note_data_missing_fields_defaults() {
        // Simulate an old file that was written before config was part of NoteData
        let json = r#"{"version":1,"config":{"width":300,"height":400,"left":100,"top":100,"font_size":14,"always_on_top":true,"titlebar_fill":100},"note":{"text":"minimal","cursor_pos":0,"scroll_top":0},"log":""}"#;
        let data: NoteData = serde_json::from_str(json).unwrap();
        // Config should be populated with defaults for missing fields
        assert_eq!(data.config.word_wrap, false);
        assert_eq!(data.config.theme, "dark");
        assert!(!data.config.password_protected);
        // Note should have the text but defaults for missing fields
        assert_eq!(data.note.text, "minimal");
        assert!(!data.note.encrypted);
    }

    #[test]
    fn test_note_data_version_field() {
        // Version mismatch — still deserializes fine (we don't reject yet)
        let json = r#"{"version":2,"config":{"width":300,"height":400,"left":100,"top":100,"font_size":14,"always_on_top":true,"titlebar_fill":100},"note":{"text":"v2","cursor_pos":0,"scroll_top":0},"log":""}"#;
        let data: NoteData = serde_json::from_str(json).unwrap();
        assert_eq!(data.version, 2);
    }

    // ── File I/O tests ────────────────────────────────────────────
    // Note: these write/read from the actual exe directory because
    // storage paths are determined by exe location. We clean up after
    // ourselves. Serialized with a Mutex to avoid parallel contention.

    use std::sync::Mutex;
    static FILE_LOCK: Mutex<()> = Mutex::new(());

    fn test_stem() -> String {
        exe_stem()
    }

    fn test_dir() -> PathBuf {
        exe_dir()
    }

    fn cleanup_test_files() {
        let _ = std::fs::remove_file(test_dir().join(format!("{}.notes", test_stem())));
        let _ = std::fs::remove_file(test_dir().join(format!("{}.config", test_stem())));
        let _ = std::fs::remove_file(test_dir().join(format!("{}.log", test_stem())));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        let mut cfg = crate::config::Config::default();
        cfg.width = 500;
        cfg.height = 600;
        cfg.font_family = "Fira Code".to_string();

        let data = NoteData {
            version: 1,
            config: cfg,
            note: crate::note::NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: "storage test".to_string(),
                cursor_pos: 7,
                scroll_top: 1,
            },
            log: "[100] test: log entry\n".to_string(),
        };

        save(&data).unwrap();
        assert!(exists(), "combined file should exist after save");

        let loaded = load();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.config.width, 500);
        assert_eq!(loaded.config.height, 600);
        assert_eq!(loaded.note.text, "storage test");
        assert_eq!(loaded.note.cursor_pos, 7);
        assert!(loaded.log.contains("test"));

        cleanup_test_files();
    }

    #[test]
    fn test_load_nonexistent_returns_fresh() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();
        // Make sure no combined file exists
        let _ = std::fs::remove_file(notes_path());
        let data = load();
        assert_eq!(data.version, 1);
        assert!(!data.config.password_protected);
        assert!(data.note.text.is_empty());
    }

    #[test]
    fn test_load_corrupt_returns_fresh() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();
        // Write corrupt content
        std::fs::write(&notes_path(), "not valid json at all").unwrap();
        let data = load();
        assert_eq!(data.version, 1);
        assert!(data.note.text.is_empty());
        cleanup_test_files();
    }

    #[test]
    fn test_load_repairs_password_no_salt_plaintext_note() {
        // Simulate the user's corrupt file: password_protected=true,
        // password_salt="", but note is NOT encrypted and has no text.
        // load() should auto-repair by clearing password_protected.
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        let mut cfg = crate::config::Config::default();
        cfg.password_protected = true;
        cfg.password_salt = String::new();
        cfg.font_size = 20;

        let note = crate::note::NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: String::new(),
            cursor_pos: 0,
            scroll_top: 0,
        };

        let data = NoteData {
            version: 1,
            config: cfg,
            note,
            log: String::new(),
        };
        save(&data).unwrap();

        // Now load should auto-repair
        let loaded = load();
        assert!(!loaded.config.password_protected, "should repair deadlock");
        assert_eq!(loaded.config.font_size, 20, "config values preserved");
        assert!(!loaded.note.encrypted);

        cleanup_test_files();
    }

    #[test]
    fn test_load_repairs_encrypted_note_with_unprotected_flag() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        let salt = crate::crypto::generate_salt();
        let key = crate::crypto::derive_key("repair-encrypted-flag", &salt).unwrap();
        let note = crate::note::Note {
            text: "encrypted payload".to_string(),
            cursor_pos: 3,
            scroll_top: 1,
        };
        let encrypted = crate::note::NoteFile::from_encrypted(&note, &key).unwrap();

        let mut cfg = crate::config::Config::default();
        cfg.password_protected = false;
        cfg.password_salt = hex::encode(salt);

        let data = NoteData {
            version: 1,
            config: cfg,
            note: encrypted,
            log: String::new(),
        };
        save(&data).unwrap();

        let loaded = load();
        assert!(loaded.note.encrypted);
        assert!(
            loaded.config.password_protected,
            "encrypted note must force protected mode"
        );

        let repaired_salt = hex::decode(&loaded.config.password_salt).unwrap();
        let repaired_key = crate::crypto::derive_key("repair-encrypted-flag", &repaired_salt).unwrap();
        let decrypted = loaded.note.decrypt_to_note(&repaired_key).unwrap();
        assert_eq!(decrypted.text, "encrypted payload");

        cleanup_test_files();
    }

    #[test]
    fn test_load_clears_stale_salt_when_unprotected() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        let mut cfg = crate::config::Config::default();
        cfg.password_protected = false;
        cfg.password_salt = "aabbccddeeff00112233445566778899".to_string();

        let data = NoteData {
            version: 1,
            config: cfg,
            note: crate::note::NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: "plain".to_string(),
                cursor_pos: 0,
                scroll_top: 0,
            },
            log: String::new(),
        };
        save(&data).unwrap();

        let loaded = load();
        assert!(!loaded.config.password_protected);
        assert!(loaded.config.password_salt.is_empty(), "stale salt should be removed");
        assert!(!loaded.note.encrypted);

        cleanup_test_files();
    }

    #[test]
    fn test_helper_functions_roundtrip() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Save config via helper
        let mut cfg = crate::config::Config::default();
        cfg.font_size = 30;
        save_config(&cfg).unwrap();

        // Save note via helper
        let nf = crate::note::NoteFile {
            encrypted: false,
            nonce_hex: None,
            ciphertext_hex: None,
            text: "helper test".to_string(),
            cursor_pos: 3,
            scroll_top: 9,
        };
        save_note_file(&nf).unwrap();

        // Load config
        let loaded_cfg = load_config();
        assert_eq!(loaded_cfg.font_size, 30);

        // Load note
        let loaded_nf = load_note_file();
        assert_eq!(loaded_nf.text, "helper test");
        assert_eq!(loaded_nf.scroll_top, 9);

        cleanup_test_files();
    }

    #[test]
    fn test_save_config_and_note_atomic() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        let cfg = crate::config::Config {
            always_on_top: false,
            font_size: 18,
            ..crate::config::Config::default()
        };
        let nf = crate::note::NoteFile {
            encrypted: true,
            nonce_hex: Some("aabbccdd".to_string()),
            ciphertext_hex: Some("11223344".to_string()),
            text: String::new(),
            cursor_pos: 42,
            scroll_top: 0,
        };

        save_config_and_note(&cfg, &nf).unwrap();

        let loaded = load();
        assert!(!loaded.config.always_on_top);
        assert_eq!(loaded.config.font_size, 18);
        assert!(loaded.note.encrypted);
        assert_eq!(loaded.note.cursor_pos, 42);

        cleanup_test_files();
    }

    // ── Migration tests ───────────────────────────────────────────
    //
    // These simulate the old v0.1.x files, run migration, verify the
    // combined file, and clean up.

    #[test]
    fn test_migrate_from_legacy_basic() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Write old config file
        let old_config = r#"{
            "width": 400,
            "height": 500,
            "left": 50,
            "top": 60,
            "font_size": 18,
            "always_on_top": true,
            "theme": "nord"
        }"#;
        let cfg_path = test_dir().join(format!("{}.config", test_stem()));
        std::fs::write(&cfg_path, old_config).unwrap();

        // Write old notes file
        let old_notes = r#"{"text":"legacy note content","cursor_pos":12,"scroll_top":5}"#;
        let notes_path_old = test_dir().join(format!("{}.notes", test_stem()));
        std::fs::write(&notes_path_old, old_notes).unwrap();

        // Write old log file
        let old_log = "[100] startup: started\n[200] password: set\n";
        let log_path = test_dir().join(format!("{}.log", test_stem()));
        std::fs::write(&log_path, old_log).unwrap();

        // Verify legacy exists (old .config file is the migration trigger)
        assert!(legacy_exists());

        // Run migration
        let data = migrate_from_legacy();

        // Check migrated data
        assert_eq!(data.version, 1);
        assert_eq!(data.config.width, 400);
        assert_eq!(data.config.theme, "nord");
        assert_eq!(data.note.text, "legacy note content");
        assert_eq!(data.note.cursor_pos, 12);
        assert_eq!(data.note.scroll_top, 5);
        assert!(data.log.contains("startup"));
        assert!(data.log.contains("password"));

        // Old files should be deleted
        assert!(!cfg_path.exists(), "old .config should be deleted");
        assert!(!log_path.exists(), "old .log should be deleted");
        // Combined file exists
        assert!(exists());

        cleanup_test_files();
    }

    #[test]
    fn test_migrate_from_legacy_encrypted() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Create an encrypted note scenario
        let salt = crate::crypto::generate_salt();
        let key = crate::crypto::derive_key("migration-test", &salt).unwrap();
        let note = crate::note::Note {
            text: "migration secret".to_string(),
            cursor_pos: 7,
            scroll_top: 3,
        };
        let nf = crate::note::NoteFile::from_encrypted(&note, &key).unwrap();
        let nf_json = serde_json::to_string(&nf).unwrap();

        // Write old config with password (serialized properly)
        let mut old_cfg = crate::config::Config::default();
        old_cfg.password_protected = true;
        old_cfg.password_salt = hex::encode(salt);
        let cfg_json = serde_json::to_string_pretty(&old_cfg).unwrap();
        std::fs::write(
            test_dir().join(format!("{}.config", test_stem())),
            &cfg_json,
        )
        .unwrap();

        std::fs::write(
            test_dir().join(format!("{}.notes", test_stem())),
            &nf_json,
        )
        .unwrap();

        std::fs::write(
            test_dir().join(format!("{}.log", test_stem())),
            "",
        )
        .unwrap();

        // Migrate
        let data = migrate_from_legacy();

        assert!(data.config.password_protected);
        assert!(!data.config.password_salt.is_empty());
        assert!(data.note.encrypted);

        // Decrypt the migrated note
        let migrated_key = crate::crypto::derive_key(
            "migration-test",
            &hex::decode(&data.config.password_salt).unwrap(),
        )
        .unwrap();
        let decrypted = data.note.decrypt_to_note(&migrated_key).unwrap();
        assert_eq!(decrypted.text, "migration secret");
        assert_eq!(decrypted.cursor_pos, 7);
        assert_eq!(decrypted.scroll_top, 3);

        cleanup_test_files();
    }

    #[test]
    fn test_migrate_user_scenario_password_unlock_works() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // 1. Setup old-style password-protected config
        let password = "hunter2";
        let salt = crate::crypto::generate_salt();
        let key = crate::crypto::derive_key(password, &salt).unwrap();

        let mut old_cfg = crate::config::Config::default();
        old_cfg.password_protected = true;
        old_cfg.password_salt = hex::encode(salt);
        let cfg_json = serde_json::to_string_pretty(&old_cfg).unwrap();
        std::fs::write(
            test_dir().join(format!("{}.config", test_stem())),
            &cfg_json,
        )
        .unwrap();

        let note = crate::note::Note {
            text: "my secret note".to_string(),
            cursor_pos: 5,
            scroll_top: 2,
        };
        let nf = crate::note::NoteFile::from_encrypted(&note, &key).unwrap();
        let nf_json = serde_json::to_string(&nf).unwrap();
        std::fs::write(
            test_dir().join(format!("{}.notes", test_stem())),
            &nf_json,
        )
        .unwrap();
        std::fs::write(
            test_dir().join(format!("{}.log", test_stem())),
            "[100] startup: started\n",
        )
        .unwrap();

        // 2. Simulate app startup: migrate
        assert!(legacy_exists());
        let _ = migrate_from_legacy();
        assert!(exists(), "combined file should exist after migration");

        // 3. Simulate unlock: load config+note, derive key, decrypt
        let loaded = load();
        assert!(loaded.config.password_protected);
        assert!(!loaded.config.password_salt.is_empty());
        assert!(loaded.note.encrypted);

        let decoded_salt = hex::decode(&loaded.config.password_salt).unwrap();
        let derived_key = crate::crypto::derive_key(password, &decoded_salt).unwrap();
        let decrypted = loaded.note.decrypt_to_note(&derived_key).unwrap();
        assert_eq!(decrypted.text, "my secret note");
        assert_eq!(decrypted.cursor_pos, 5);
        assert_eq!(decrypted.scroll_top, 2);

        // 4. Simulate save after unlock: modify and re-save
        let mut save_data = load();
        save_data.note = crate::note::NoteFile::from_encrypted(
            &crate::note::Note {
                text: "updated note after unlock".to_string(),
                cursor_pos: 0,
                scroll_top: 0,
            },
            &derived_key,
        )
        .unwrap();
        save(&save_data).unwrap();

        // 5. Verify re-save didn't corrupt the config
        let final_data = load();
        assert!(final_data.config.password_protected);
        assert!(!final_data.config.password_salt.is_empty());
        let final_decrypted = final_data.note.decrypt_to_note(&derived_key).unwrap();
        assert_eq!(final_decrypted.text, "updated note after unlock");

        cleanup_test_files();
    }

    #[test]
    fn test_migrate_from_legacy_no_old_files_uses_defaults() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();
        // Don't create any old files — migrate should still work
        // with defaults for config, empty note, empty log
        let data = migrate_from_legacy();
        assert_eq!(data.version, 1);
        // Config should have defaults
        assert_eq!(data.config.font_size, 14);
        // Note should be loaded from whatever exists (fresh default)
        assert!(data.note.text.is_empty());
        // Combined file should exist after migration
        assert!(exists());
        cleanup_test_files();
    }
}
