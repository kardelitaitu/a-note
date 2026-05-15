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
