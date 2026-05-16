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
pub fn load() -> NoteData {
    let path = notes_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(fresh)
    } else {
        fresh()
    }
}

/// Save the combined NoteData to `{exe}.notes`.
pub fn save(data: &NoteData) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        crate::util::write(&notes_path(), &json);
    }
}

/// Load just the config from the combined file.
pub fn load_config() -> crate::config::Config {
    load().config
}

/// Save config through the combined file, flushing diagnostics log.
pub fn save_config(cfg: &crate::config::Config) {
    let mut data = load();
    data.config = cfg.clone();
    data.log = crate::diagnostics::flush_to_log_str();
    save(&data);
}

/// Load just the NoteFile from the combined file.
pub fn load_note_file() -> crate::note::NoteFile {
    load().note
}

/// Save NoteFile through the combined file, flushing diagnostics log.
pub fn save_note_file(nf: &crate::note::NoteFile) {
    let mut data = load();
    data.note = nf.clone();
    data.log = crate::diagnostics::flush_to_log_str();
    save(&data);
}

/// Atomically save both config and NoteFile (used by password operations).
pub fn save_config_and_note(cfg: &crate::config::Config, nf: &crate::note::NoteFile) {
    let mut data = load();
    data.config = cfg.clone();
    data.note = nf.clone();
    data.log = crate::diagnostics::flush_to_log_str();
    save(&data);
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

    let data = NoteData {
        version: CURRENT_VERSION,
        config,
        note,
        log,
    };

    // Write the combined file
    save(&data);

    // Remove old files silently
    let _ = std::fs::remove_file(&legacy_config_path());
    let _ = std::fs::remove_file(&legacy_log_path());

    data
}

