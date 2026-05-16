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

    #[test]
    fn test_flush_empty_buffer() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear
        let log = flush_to_log_str();
        assert!(log.is_empty(), "flush on empty buffer should return empty string");
    }

    #[test]
    fn test_flush_clears_buffer() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear
        event("cat", "hello");
        let first = flush_to_log_str();
        assert!(!first.is_empty(), "first flush should contain the event");
        let second = flush_to_log_str();
        assert!(second.is_empty(), "second flush should return empty string");
    }

    #[test]
    fn test_double_restore() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear
        restore_from_log_str("[1] first: batch a");
        restore_from_log_str("[2] second: batch b");
        let log = flush_to_log_str();
        assert!(log.contains("batch a"), "first restored entry should appear");
        assert!(log.contains("batch b"), "second restored entry should appear");
    }

    #[test]
    fn test_restore_empty_string() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear
        // Restore empty string (should be a no-op)
        restore_from_log_str("");
        event("cat", "after empty restore");
        let log = flush_to_log_str();
        assert!(log.contains("after empty restore"), "event after empty restore should be present");
        assert!(!log.contains("[log trimmed]"), "no trimming prefix expected");
    }

    #[test]
    fn test_event_special_chars() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear
        event("café", "ñoño — 你好 — 👍 — <>&'\";");
        let log = flush_to_log_str();
        assert!(log.contains("café"), "category with non-ASCII");
        assert!(log.contains("ñoño"), "message with non-ASCII");
        assert!(log.contains("你好"), "message with CJK");
        assert!(log.contains("👍"), "message with emoji");
        assert!(log.contains("<>&'\";"), "message with special chars");
    }

    #[test]
    fn test_restore_after_flush() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear
        // Write and flush some events
        event("first", "batch");
        let _first_flush = flush_to_log_str();
        // Restore more entries
        restore_from_log_str("[1] restored: entry");
        event("second", "after restore");
        let combined = flush_to_log_str();
        // Both the restored entry and the post-restore event should be present
        assert!(combined.contains("restored: entry"), "restored entry should appear");
        assert!(combined.contains("after restore"), "event after restore should appear");
        // The first batch that was flushed earlier should NOT appear
        assert!(!combined.contains("first: batch"), "previously flushed events should not reappear");
    }

    #[test]
    fn test_log_boundary_trimming() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear
        // Fill buffer to just under 10 KB with known content
        // Each line: "[1234567890] bulk: XXXX...\n" — aim for ~200 bytes per line
        let msg = "X".repeat(180);
        // 50 events * ~200 bytes ≈ 10 KB — should be right at the boundary
        for _ in 0..55 {
            event("bulk", &msg);
        }
        // Manually check the buffer size by looking at the flushed output
        let log = flush_to_log_str();
        // The trimmed output should be at most ~10 KB plus the "[log trimmed]\n" prefix
        assert!(
            log.len() <= 10_500,
            "log too large ({}), trimming should have capped it",
            log.len()
        );
    }

    #[test]
    fn test_concurrent_events() {
        let _lock = DIAG_LOCK.lock().unwrap();
        let _ = flush_to_log_str(); // clear
        // Rapid-fire 50 events in sequence
        for i in 0..50 {
            event("conc", &format!("event number {i}"));
        }
        let log = flush_to_log_str();
        // All 50 should be present
        for i in 0..50 {
            assert!(
                log.contains(&format!("event number {i}")),
                "event {i} should be in the log"
            );
        }
        let lines: Vec<&str> = log.lines().collect();
        assert_eq!(lines.len(), 50, "all 50 events should produce 50 lines");
        // Each line should have a timestamp
        for line in &lines {
            assert!(line.starts_with('['), "every line should start with a timestamp: {line:?}");
        }
    }
}
