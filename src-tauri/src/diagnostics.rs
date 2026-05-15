//! Simple local crash reporting and event logging.
//!
//! - Panics caught via `set_hook` → written to `{exe}.crash`
//! - Major events appended to `{exe}.log` (startup, password ops, errors)
//! - No network calls — all data stays local on disk.

use std::path::PathBuf;

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

/// Initialise the crash reporter.
///
/// Call once at startup. Sets a panic hook that writes crash details
/// to `{exe}.crash`. Existing `.log` file is cleared on init.
pub fn init() {
    let stem = exe_stem();
    let dir = exe_dir();

    // Clear previous session log
    let _ = std::fs::write(dir.join(format!("{stem}.log")), "");

    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let stem = exe_stem();
        let dir = exe_dir();
        let bt = std::backtrace::Backtrace::capture();
        let report = format!(
            "=== CRASH ===\nTime: {}\nPanic: {}\n\nBacktrace:\n{:?}\n",
            timestamp(),
            info,
            bt
        );
        let _ = std::fs::write(dir.join(format!("{stem}.crash")), &report);
        eprintln!("{}", report);
        prev(info);
    }));

    event("startup", "Application started");
}

/// Append a major event to `{exe}.log`.
pub fn event(category: &str, message: &str) {
    let stem = exe_stem();
    let dir = exe_dir();
    let path = dir.join(format!("{stem}.log"));

    let line = format!("[{}] {}: {}\n", timestamp(), category, message);
    let mut log = std::fs::read_to_string(&path).unwrap_or_default();
    log.push_str(&line);

    // Keep log under ~100 KB — trim oldest lines
    if log.len() > 100_000 {
        if let Some(pos) = log.rfind('\n') {
            let trimmed = &log[log.len() - 90_000..];
            let first_newline = trimmed.find('\n').unwrap_or(0);
            log = format!("[log trimmed]\n{}", &trimmed[first_newline + 1..]);
        }
    }

    let _ = std::fs::write(&path, &log);
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
