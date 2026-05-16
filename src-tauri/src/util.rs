use std::path::Path;

const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

#[cfg(windows)]
extern "system" {
    fn SetFileAttributesW(lpFileName: *const u16, dwFileAttributes: u32) -> i32;
}

pub fn write(path: &Path, contents: &str) -> Result<(), String> {
    let hidden = was_hidden(path);
    std::fs::write(path, contents)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    if hidden {
        set_hidden(path)?;
    }
    Ok(())
}

#[cfg(windows)]
fn was_hidden(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    path.metadata()
        .map(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn was_hidden(_path: &Path) -> bool {
    false
}

#[cfg(windows)]
fn set_hidden(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let ok = unsafe { SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN) };
    if ok == 0 {
        return Err(format!(
            "failed to restore hidden attribute on {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn set_hidden(_path: &Path) -> Result<(), String> {
    Ok(())
}

// ── Start with Windows (registry) ────────────────────────────────────

#[cfg(windows)]
extern "system" {
    fn RegOpenKeyExW(
        hKey: usize,
        lpSubKey: *const u16,
        ulOptions: u32,
        samDesired: u32,
        phkResult: *mut usize,
    ) -> i32;
    fn RegSetValueExW(
        hKey: usize,
        lpValueName: *const u16,
        Reserved: u32,
        dwType: u32,
        lpData: *const u8,
        cbData: i32,
    ) -> i32;
    fn RegDeleteValueW(hKey: usize, lpValueName: *const u16) -> i32;
    fn RegCloseKey(hKey: usize) -> i32;
}

const HKEY_CURRENT_USER: usize = 0x80000001;
const KEY_SET_VALUE: u32 = 0x0002;
const REG_SZ: u32 = 1;

/// Set or clear the Windows Run registry key for this app.
/// When enabled, writes `HKCU\...\Run\Notes` = current exe path.
/// When disabled, deletes that value.
/// Best-effort — silently ignores errors.
#[cfg(windows)]
pub fn set_startup_registry(enabled: bool) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let sub_key: Vec<u16> = OsStr::new("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut hkey: usize = 0;
    let rc = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, sub_key.as_ptr(), 0, KEY_SET_VALUE, &mut hkey) };
    if rc != 0 {
        return;
    }

    if enabled {
        let exe_path = std::env::current_exe().ok();
        if let Some(path) = exe_path {
            let wide: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let value_name: Vec<u16> = OsStr::new("Notes")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            // Include null terminator in data size (REG_SZ requires it)
            let bytes = wide.len() * 2;
            unsafe {
                RegSetValueExW(
                    hkey,
                    value_name.as_ptr(),
                    0,
                    REG_SZ,
                    wide.as_ptr() as *const u8,
                    bytes as i32,
                );
            }
        }
    } else {
        let value_name: Vec<u16> = OsStr::new("Notes")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            RegDeleteValueW(hkey, value_name.as_ptr());
        }
    }

    unsafe { RegCloseKey(hkey); }
}

/// Non-Windows stub.
#[cfg(not(windows))]
pub fn set_startup_registry(_enabled: bool) {}

/// Read back the current auto-start registry value.
/// Returns `Some(path)` if the key exists, `None` if unset or inaccessible.
#[cfg(windows)]
pub fn get_startup_registry() -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    const KEY_QUERY_VALUE: u32 = 0x0001;
    const REG_SZ: u32 = 1;
    const ERROR_SUCCESS: i32 = 0;

    let sub_key: Vec<u16> = OsStr::new("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    extern "system" {
        fn RegOpenKeyExW(
            hKey: usize,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut usize,
        ) -> i32;
        fn RegQueryValueExW(
            hKey: usize,
            lpValueName: *const u16,
            lpReserved: *mut u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut i32,
        ) -> i32;
        fn RegCloseKey(hKey: usize) -> i32;
    }

    let mut hkey: usize = 0;
    let rc = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, sub_key.as_ptr(), 0, KEY_QUERY_VALUE, &mut hkey) };
    if rc != ERROR_SUCCESS {
        return None;
    }

    let value_name: Vec<u16> = OsStr::new("Notes")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut buf = [0u16; 512];
    let mut buf_size: i32 = (buf.len() * 2) as i32;
    let mut value_type: u32 = 0;
    let rc = unsafe {
        RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            buf.as_mut_ptr() as *mut u8,
            &mut buf_size,
        )
    };

    unsafe { RegCloseKey(hkey); }

    if rc != ERROR_SUCCESS || value_type != REG_SZ || buf_size <= 2 {
        return None;
    }

    // buf_size is in bytes; convert to u16 count (including null terminator)
    let count = (buf_size as usize).saturating_sub(2) / 2;
    let result = String::from_utf16(&buf[..count]).ok()?;
    Some(result)
}

#[cfg(not(windows))]
pub fn get_startup_registry() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read() {
        let dir = std::env::temp_dir().join(format!("a-note-test-util-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.txt");

        write(&path, "hello").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello");

        write(&path, "world").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "world");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_empty() {
        let dir = std::env::temp_dir().join(format!("a-note-test-util-empty-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("empty.txt");

        write(&path, "").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn test_preserves_hidden() {
        use std::os::windows::ffi::OsStrExt;
        use std::os::windows::fs::MetadataExt;

        let dir = std::env::temp_dir().join(format!("a-note-test-hidden-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("hidden.txt");

        std::fs::write(&path, "original").unwrap();

        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN);
        }

        assert!(
            path.metadata()
                .map(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
                .unwrap_or(false)
        );

        write(&path, "rewritten").unwrap();

        assert!(
            path.metadata()
                .map(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
                .unwrap_or(false)
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "rewritten");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn test_does_not_mark_unhidden() {
        use std::os::windows::fs::MetadataExt;

        let dir = std::env::temp_dir().join(format!("a-note-test-nothidden-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("plain.txt");

        write(&path, "first write").unwrap();
        assert!(
            !path
                .metadata()
                .map(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
                .unwrap_or(true)
        );

        write(&path, "second write").unwrap();
        assert!(
            !path
                .metadata()
                .map(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
                .unwrap_or(true)
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_special_chars() {
        let dir = std::env::temp_dir().join(format!("a-note-test-special-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("special.txt");

        let content = "hello\nworld\n  tabs\there\n  unicode: 世界 🚀\n  quotes: \"'`\n  angle: <test>";
        write(&path, content).unwrap();
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, content);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_long_content() {
        let dir = std::env::temp_dir().join(format!("a-note-test-long-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("long.txt");

        // 50 KB of content
        let content = "The quick brown fox jumps over the lazy dog.\n".repeat(1200);
        assert!(content.len() > 50_000);
        write(&path, &content).unwrap();
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back.len(), content.len());
        assert_eq!(read_back, content);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_overwrite_with_shorter() {
        let dir = std::env::temp_dir().join(format!("a-note-test-overwrite-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("overwrite.txt");

        write(&path, "long content here that should be replaced").unwrap();
        write(&path, "short").unwrap();
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, "short");
        // File should not contain leftover bytes from previous content
        assert!(!read_back.contains("long content"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_new_file_creates() {
        let dir = std::env::temp_dir().join(format!("a-note-test-new-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("newly_created.txt");

        assert!(!path.exists());
        write(&path, "brand new").unwrap();
        assert!(path.exists());
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, "brand new");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_nonexistent_parent_returns_err() {
        // Writing to a path whose parent directory doesn't exist
        // should return an error instead of silently succeeding.
        let dir = std::env::temp_dir().join(format!("a-note-test-nonexist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir); // ensure it doesn't exist
        let path = dir.join("nested").join("file.txt");

        let result = write(&path, "should fail");
        assert!(result.is_err());
        // File should NOT exist since parent doesn't exist
        assert!(!path.exists());
    }

    // ── Auto-start registry tests (Windows only) ─────────────────
    // Note: serialized with a Mutex because all tests share the same
    // registry key (HKCU\...\Run\Notes) and run in parallel.

    #[cfg(windows)]
    use std::sync::Mutex;
    #[cfg(windows)]
    static STARTUP_LOCK: Mutex<()> = Mutex::new(());

    #[cfg(windows)]
    #[test]
    fn test_startup_registry_initial_clean() {
        let _lock = STARTUP_LOCK.lock().unwrap();
        // Clean up any leftover from previous runs
        set_startup_registry(false);
        // Initially, the registry value should not exist
        assert!(get_startup_registry().is_none(), "expected clean state");
    }

    #[cfg(windows)]
    #[test]
    fn test_startup_registry_set_and_read() {
        let _lock = STARTUP_LOCK.lock().unwrap();
        // Ensure clean state
        set_startup_registry(false);

        // Enable auto-start
        set_startup_registry(true);
        let value = get_startup_registry();
        assert!(value.is_some(), "registry value should exist after set");

        let path = value.unwrap();
        assert!(!path.is_empty(), "path should not be empty");
        // The path should end with ".exe"
        assert!(
            path.to_lowercase().ends_with(".exe"),
            "path should end with .exe, got: {path}"
        );
        // The path should be an absolute file path
        assert!(path.contains('\\'), "path should be absolute: {path}");
    }

    #[cfg(windows)]
    #[test]
    fn test_startup_registry_disable_clears() {
        let _lock = STARTUP_LOCK.lock().unwrap();
        // Ensure clean state
        set_startup_registry(false);
        assert!(get_startup_registry().is_none());

        // Enable then disable
        set_startup_registry(true);
        assert!(get_startup_registry().is_some(), "should exist after enable");
        set_startup_registry(false);
        assert!(
            get_startup_registry().is_none(),
            "should be cleared after disable"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_startup_registry_idempotent() {
        let _lock = STARTUP_LOCK.lock().unwrap();
        // Calling set_startup_registry multiple times with the same state
        // should not cause errors.
        set_startup_registry(false);
        set_startup_registry(false); // again — should be no-op
        assert!(get_startup_registry().is_none());

        set_startup_registry(true);
        let val1 = get_startup_registry();
        set_startup_registry(true); // again — should overwrite with same path
        let val2 = get_startup_registry();
        assert_eq!(val1, val2, "repeated enable should keep same path");
    }
}
