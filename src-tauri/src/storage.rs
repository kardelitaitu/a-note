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

#[cfg(test)]
pub(crate) static STORAGE_TEST_FILE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn notes_path() -> PathBuf {
    crate::paths::notes_path()
}

fn legacy_config_path() -> PathBuf {
    crate::paths::legacy_config_path()
}

fn legacy_log_path() -> PathBuf {
    crate::paths::legacy_log_path()
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

    use super::STORAGE_TEST_FILE_LOCK as FILE_LOCK;

    fn test_stem() -> String {
        crate::paths::exe_stem()
    }

    fn test_dir() -> PathBuf {
        crate::paths::exe_dir()
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

    // ── Additional coverage tests ─────────────────────────────────

    #[test]
    fn test_migrate_corrupt_config() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Write legacy config with invalid JSON
        std::fs::write(
            test_dir().join(format!("{}.config", test_stem())),
            "this is not json at all",
        )
        .unwrap();
        // No .notes or .log files — migration should use defaults

        let data = migrate_from_legacy();
        assert_eq!(data.version, CURRENT_VERSION);
        // Config should be default since corrupt JSON falls back to default
        assert_eq!(data.config.font_size, 14);
        assert!(!data.config.password_protected);
        assert!(data.note.text.is_empty());
        assert!(data.log.is_empty());
        assert!(exists());

        cleanup_test_files();
    }

    #[test]
    fn test_migrate_corrupt_note() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Write a valid legacy config
        let cfg = r#"{"width":400,"height":500,"left":50,"top":60,"font_size":18,"always_on_top":true}"#;
        std::fs::write(
            test_dir().join(format!("{}.config", test_stem())),
            cfg,
        )
        .unwrap();

        // Write a corrupt legacy .notes file
        std::fs::write(
            test_dir().join(format!("{}.notes", test_stem())),
            "not valid note json",
        )
        .unwrap();

        // No .log file

        let data = migrate_from_legacy();
        assert_eq!(data.version, CURRENT_VERSION);
        // Config should be preserved from the valid file
        assert_eq!(data.config.width, 400);
        // Note should fall back to default (empty) because corrupt
        assert!(data.note.text.is_empty());
        assert!(!data.note.encrypted);

        cleanup_test_files();
    }

    #[test]
    fn test_migrate_empty_legacy_files() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Write empty legacy files
        std::fs::write(
            test_dir().join(format!("{}.config", test_stem())),
            "",
        )
        .unwrap();
        std::fs::write(
            test_dir().join(format!("{}.notes", test_stem())),
            "",
        )
        .unwrap();
        std::fs::write(
            test_dir().join(format!("{}.log", test_stem())),
            "",
        )
        .unwrap();

        let data = migrate_from_legacy();
        assert_eq!(data.version, CURRENT_VERSION);
        // Empty config → defaults, empty notes → defaults, empty log → empty string
        assert!(!data.config.password_protected);
        assert!(data.note.text.is_empty());
        assert!(data.log.is_empty());
        assert!(exists());

        cleanup_test_files();
    }

    #[test]
    fn test_migrate_protected_no_salt() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Write legacy config with password_protected=true but empty salt
        let cfg = r#"{"password_protected":true,"password_salt":""}"#;
        std::fs::write(
            test_dir().join(format!("{}.config", test_stem())),
            cfg,
        )
        .unwrap();

        let data = migrate_from_legacy();
        assert_eq!(data.version, CURRENT_VERSION);
        // Migration should repair: set password_protected = false since no salt
        assert!(!data.config.password_protected, "should repair no-salt deadlock");
        assert!(data.config.password_salt.is_empty());

        cleanup_test_files();
    }

    #[test]
    fn test_migrate_encrypted_note() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Create an encrypted note scenario
        let salt = crate::crypto::generate_salt();
        let key = crate::crypto::derive_key("encrypted-migration-key", &salt).unwrap();
        let note = crate::note::Note {
            text: "encrypted during migration".to_string(),
            cursor_pos: 5,
            scroll_top: 2,
        };
        let nf = crate::note::NoteFile::from_encrypted(&note, &key).unwrap();
        let nf_json = serde_json::to_string(&nf).unwrap();

        // Write legacy config with password
        let mut cfg = crate::config::Config::default();
        cfg.password_protected = true;
        cfg.password_salt = hex::encode(salt);
        std::fs::write(
            test_dir().join(format!("{}.config", test_stem())),
            &serde_json::to_string_pretty(&cfg).unwrap(),
        )
        .unwrap();

        // Write encrypted note
        std::fs::write(
            test_dir().join(format!("{}.notes", test_stem())),
            &nf_json,
        )
        .unwrap();

        // Write empty log
        std::fs::write(
            test_dir().join(format!("{}.log", test_stem())),
            "",
        )
        .unwrap();

        let data = migrate_from_legacy();
        assert_eq!(data.version, CURRENT_VERSION);
        assert!(data.config.password_protected);
        assert!(!data.config.password_salt.is_empty());
        assert!(data.note.encrypted);

        // Verify decryption still works
        let migrated_salt = hex::decode(&data.config.password_salt).unwrap();
        let migrated_key = crate::crypto::derive_key("encrypted-migration-key", &migrated_salt).unwrap();
        let decrypted = data.note.decrypt_to_note(&migrated_key).unwrap();
        assert_eq!(decrypted.text, "encrypted during migration");

        cleanup_test_files();
    }

    #[test]
    fn test_migrate_idempotent() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Write old config file
        let old_config = r#"{"width":400,"height":500,"left":50,"top":60,"font_size":18,"always_on_top":true}"#;
        std::fs::write(
            test_dir().join(format!("{}.config", test_stem())),
            old_config,
        )
        .unwrap();

        // First migration
        let data1 = migrate_from_legacy();
        assert_eq!(data1.config.width, 400);
        // Old files should be gone
        assert!(!test_dir().join(format!("{}.config", test_stem())).exists());

        // Second migration — old files no longer exist, uses defaults
        let data2 = migrate_from_legacy();
        assert_eq!(data2.version, CURRENT_VERSION);
        // Config is default because no legacy files left
        assert_eq!(data2.config.width, 300);
        // Combined file should still exist and be intact
        assert!(exists());

        cleanup_test_files();
    }

    #[test]
    fn test_save_error_propagates() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Set forced write error
        crate::util::set_forced_write_error_for_current_thread(Some("simulated write failure".to_string()));

        let data = fresh();
        let result = save(&data);
        assert!(result.is_err(), "save should return Err when write fails");
        assert!(
            result.unwrap_err().contains("simulated write failure"),
            "error message should propagate"
        );

        // Reset forced error
        crate::util::set_forced_write_error_for_current_thread(None);

        cleanup_test_files();
    }

    #[test]
    fn test_try_load_corrupt_file() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Write a corrupt combined file
        std::fs::write(&notes_path(), "this is not valid json").unwrap();

        // try_load should return Err, not silently return a default
        let result = try_load();
        assert!(result.is_err(), "try_load should return Err for corrupt data");

        cleanup_test_files();
    }

    #[test]
    fn test_reconcile_both_invariants_violated() {
        // Start with: encrypted = true, password_protected = false, salt is empty.
        // Rule 1 fires: encrypted && !password_protected → password_protected = true.
        // Rule 2: !encrypted is false → skip.
        // Rule 3: !password_protected is now false → skip.
        // Result: only rule 1 fires, but reconciliation still succeeds.
        let mut data = NoteData {
            version: CURRENT_VERSION,
            config: crate::config::Config {
                password_protected: false,
                password_salt: String::new(),
                ..crate::config::Config::default()
            },
            note: crate::note::NoteFile {
                encrypted: true,
                nonce_hex: Some("aabbccdd".to_string()),
                ciphertext_hex: Some("11223344".to_string()),
                text: String::new(),
                cursor_pos: 0,
                scroll_top: 0,
            },
            log: String::new(),
        };

        let changed = reconcile_invariants(&mut data);
        assert!(changed, "reconcile should report changes were made");
        assert!(
            data.config.password_protected,
            "encrypted note must force password_protected=true"
        );
        assert!(data.note.encrypted, "note should remain encrypted");
    }

    #[test]
    fn test_reconcile_already_consistent() {
        // Normal state: plaintext note, no protection, no salt — no changes needed
        let mut data = NoteData {
            version: CURRENT_VERSION,
            config: crate::config::Config {
                password_protected: false,
                password_salt: String::new(),
                ..crate::config::Config::default()
            },
            note: crate::note::NoteFile {
                encrypted: false,
                nonce_hex: None,
                ciphertext_hex: None,
                text: "normal note".to_string(),
                cursor_pos: 0,
                scroll_top: 0,
            },
            log: String::new(),
        };

        let changed = reconcile_invariants(&mut data);
        assert!(!changed, "consistent state should not report changes");
        assert!(!data.config.password_protected);
        assert!(!data.note.encrypted);
        assert!(data.config.password_salt.is_empty());
    }

    #[test]
    fn test_migrate_only_config_exists() {
        let _lock = FILE_LOCK.lock().unwrap();
        cleanup_test_files();

        // Write only the legacy config file
        let old_config = r#"{"width":600,"height":700,"left":100,"top":200,"font_size":20,"always_on_top":false}"#;
        std::fs::write(
            test_dir().join(format!("{}.config", test_stem())),
            old_config,
        )
        .unwrap();
        // No .notes or .log files

        let data = migrate_from_legacy();
        assert_eq!(data.version, CURRENT_VERSION);
        assert_eq!(data.config.width, 600);
        assert_eq!(data.config.height, 700);
        // Note should be default (empty)
        assert!(data.note.text.is_empty());
        // Log should be empty
        assert!(data.log.is_empty());
        // Combined file should exist
        assert!(exists());

        cleanup_test_files();
    }

    #[test]
    fn test_fresh_returns_current_version() {
        let data = fresh();
        assert_eq!(
            data.version,
            CURRENT_VERSION,
            "fresh() should always return CURRENT_VERSION ({})",
            CURRENT_VERSION
        );
        assert!(!data.config.password_protected);
        assert!(!data.note.encrypted);
        assert!(data.note.text.is_empty());
        assert!(data.log.is_empty());
    }
}
