//! Simple local crash reporting and event logging.
//!
//! - Panics caught via `set_hook` → written to `{exe}.crash`
//! - Events stored in-memory, flushed to the combined `.notes` file via
//!   `flush_to_log_str()` / `restore_from_log_str()`
//! - No network calls — all data stays local on disk.

use std::sync::Mutex;

static LOG_BUF: Mutex<Option<String>> = Mutex::new(None);

/// Initialise the crash reporter.
///
/// Call once at startup. Sets a panic hook that writes crash details
/// to `{exe}.crash`. Also logs a startup event to the in-memory buffer.
pub fn init() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let stem = crate::paths::exe_stem();
        let dir = crate::paths::exe_dir();
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

/// Append a major event to the in-memory log buffer.
pub fn event(category: &str, message: &str) {
    let line = format!("[{}] {}: {}\n", timestamp(), category, message);
    if let Ok(mut guard) = LOG_BUF.lock() {
        let buf = guard.get_or_insert_with(String::new);
        buf.push_str(&line);
    }
}

/// Flush the in-memory log to a string suitable for persisting.
/// The log is capped at ~10 KB, trimmed from the oldest entries.
/// The in-memory buffer is cleared after flushing.
pub fn flush_to_log_str() -> String {
    if let Ok(mut guard) = LOG_BUF.lock() {
        let buf = guard.take().unwrap_or_default();
        if buf.len() > 10_000 {
            if let Some(_pos) = buf.rfind('\n') {
                let trimmed = &buf[buf.len() - 9_000..];
                let first_newline = trimmed.find('\n').unwrap_or(0);
                return format!("[log trimmed]\n{}", &trimmed[first_newline + 1..]);
            }
        }
        buf
    } else {
        String::new()
    }
}

/// Restore previously persisted log entries back into the in-memory buffer.
/// New events will be appended after these.
pub fn restore_from_log_str(saved: &str) {
    if saved.is_empty() {
        return;
    }
    if let Ok(mut guard) = LOG_BUF.lock() {
        let buf = guard.get_or_insert_with(String::new);
        // Prepend old entries, then add the restored entries before existing ones
        // (flushing later includes everything)
        *buf = format!("{}\n{}", saved.trim(), buf);
    }
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static DIAG_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_event_appends_to_buffer() {
        let _lock = DIAG_LOCK.lock().unwrap();
        event("cat-a", "msg a");
        event("cat-b", "msg b");
        let log = flush_to_log_str();
        assert!(log.contains("cat-a: msg a"), "missing cat-a");
        assert!(log.contains("cat-b: msg b"), "missing cat-b");
        let lines: Vec<&str> = log.lines().collect();
        assert!(lines.len() >= 2, "expected ≥2 lines, got {}", lines.len());
        assert!(log.contains("["), "should have timestamp");
    }

    #[test]
    fn test_timestamp_reasonable() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let ts = timestamp();
        assert!(ts > 1_700_000_000, "timestamp seems too far in the past");
        assert!(ts <= now + 1, "timestamp is in the future");
    }

    #[test]
    fn test_init_does_not_panic() {
        let _lock = DIAG_LOCK.lock().unwrap();
        // Clear any previous state
        let _ = flush_to_log_str();
        // init() sets panic hook and writes a startup event
        init();
        let log = flush_to_log_str();
        assert!(log.contains("startup"), "init should write a startup event");
        assert!(log.contains("Application started"));
    }

    #[test]
    fn test_log_rotation_on_flush() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear

        // Write enough to trigger 10KB cap on flush
        let big_line = "x".repeat(1_000);
        for _ in 0..15 {
            event("bulk", &big_line);
        }
        let log = flush_to_log_str();
        assert!(
            log.len() <= 10_100,
            "log should be trimmed to ~10KB, got {}",
            log.len()
        );
    }

    #[test]
    fn test_event_with_newlines_in_message() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear

        event("multi", "line one\nline two\nline three");
        let log = flush_to_log_str();
        assert!(log.contains("line one"));
        assert!(log.contains("line two"));
        assert!(log.contains("line three"));
        let lines: Vec<&str> = log.lines().collect();
        assert_eq!(lines.len(), 3, "should have 3 lines for the 3-line message");
    }

    #[test]
    fn test_event_empty_category() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear

        event("", "just a message");
        let log = flush_to_log_str();
        assert!(log.contains(": just a message"));
    }

    #[test]
    fn test_restore_from_log_str() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear

        // Restore previous log
        restore_from_log_str("[100] prev: old event");
        // Add new event
        event("curr", "new event");
        let log = flush_to_log_str();
        assert!(log.contains("old event"), "restored entry should appear");
        assert!(log.contains("new event"), "new entry should appear");
    }
}
