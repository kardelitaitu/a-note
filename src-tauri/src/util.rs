use std::path::Path;

const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

#[cfg(windows)]
extern "system" {
    fn SetFileAttributesW(lpFileName: *const u16, dwFileAttributes: u32) -> i32;
}

pub fn write(path: &Path, contents: &str) {
    let hidden = was_hidden(path);
    let _ = std::fs::write(path, contents);
    if hidden {
        set_hidden(path);
    }
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
fn set_hidden(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN);
    }
}

#[cfg(not(windows))]
fn set_hidden(_path: &Path) {}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read() {
        let dir = std::env::temp_dir().join(format!("a-note-test-util-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.txt");

        write(&path, "hello");
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello");

        write(&path, "world");
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "world");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_empty() {
        let dir = std::env::temp_dir().join(format!("a-note-test-util-empty-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("empty.txt");

        write(&path, "");
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

        write(&path, "rewritten");

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

        write(&path, "first write");
        assert!(
            !path
                .metadata()
                .map(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
                .unwrap_or(true)
        );

        write(&path, "second write");
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
        write(&path, content);
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
        write(&path, &content);
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

        write(&path, "long content here that should be replaced");
        write(&path, "short");
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
        write(&path, "brand new");
        assert!(path.exists());
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, "brand new");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_nonexistent_parent_does_not_panic() {
        // Writing to a path whose parent directory doesn't exist
        // should fail silently (no panic), not crash.
        let dir = std::env::temp_dir().join(format!("a-note-test-nonexist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir); // ensure it doesn't exist
        let path = dir.join("nested").join("file.txt");

        // This should not panic — write() discards the error
        write(&path, "should fail silently");
        // File should NOT exist since parent doesn't exist
        assert!(!path.exists());
    }
}
