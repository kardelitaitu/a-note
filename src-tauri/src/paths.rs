//! Shared path utilities for the portable sticky-notes app.
//!
//! All data file paths are derived from the executable's location,
//! making the app fully portable — copy the `.exe` anywhere and
//! its notes, config, and logs follow.
//!
//! Path functions **panic** if the executable path is unavailable because
//! no I/O path can function without it. This is a fail-fast decision:
//! `current_exe()` is guaranteed by the OS on Windows and should never
//! fail in normal operation.

use std::path::PathBuf;

/// Returns the executable's file stem (filename without `.exe` extension).
///
/// # Panics
/// Panics if the executable path cannot be determined.
pub fn exe_stem() -> String {
    std::env::current_exe()
        .expect("failed to get exe path")
        .file_stem()
        .expect("failed to get exe stem")
        .to_string_lossy()
        .to_string()
}

/// Returns the directory containing the executable.
///
/// # Panics
/// Panics if the executable path cannot be determined.
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .expect("failed to get exe path")
        .parent()
        .expect("failed to get exe parent")
        .to_path_buf()
}

/// Path to the combined `.notes` data file (config + note + log).
pub fn notes_path() -> PathBuf {
    exe_dir().join(format!("{}.notes", exe_stem()))
}

/// Path to the legacy `.config` file (v0.1.x format, used during migration).
pub fn legacy_config_path() -> PathBuf {
    exe_dir().join(format!("{}.config", exe_stem()))
}

/// Path to the legacy `.log` file (v0.1.x format, used during migration).
pub fn legacy_log_path() -> PathBuf {
    exe_dir().join(format!("{}.log", exe_stem()))
}

/// Path to the crash report file.
pub fn crash_path() -> PathBuf {
    exe_dir().join(format!("{}.crash", exe_stem()))
}
