//! Windows.
//!
//! The native "mode" of a file is its attributes (read-only, hidden, system,
//! archive), as in ZTreeWin. Archives and copies still deal in Unix
//! permissions, approximated from the read-only attribute.

use std::ffi::OsStr;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_HIDDEN,
    FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_NOT_CONTENT_INDEXED, FILE_ATTRIBUTE_OFFLINE,
    FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_SYSTEM, FILE_ATTRIBUTE_TEMPORARY,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_WRITE_ATTRIBUTES,
    GetDiskFreeSpaceExW, GetFileInformationByHandle, GetLogicalDrives, SetFileAttributesW,
};

use super::{DiskSpace, Mount, env_dir};

/// The attributes shown and edited by xt, in display order.
const RHSA: [(u32, char); 4] = [
    (FILE_ATTRIBUTE_READONLY, 'r'),
    (FILE_ATTRIBUTE_HIDDEN, 'h'),
    (FILE_ATTRIBUTE_SYSTEM, 's'),
    (FILE_ATTRIBUTE_ARCHIVE, 'a'),
];
const RHSA_MASK: u32 = FILE_ATTRIBUTE_READONLY
    | FILE_ATTRIBUTE_HIDDEN
    | FILE_ATTRIBUTE_SYSTEM
    | FILE_ATTRIBUTE_ARCHIVE;

/// The file attributes.
pub fn mode(meta: &Metadata) -> u32 {
    meta.file_attributes()
}

/// Unix-like permission bits, for archives and copies: 644 (755 for
/// directories), without the write bits when the file is read-only.
pub fn unix_mode(meta: &Metadata) -> u32 {
    let mode = if meta.is_dir() { 0o755 } else { 0o644 };
    if meta.permissions().readonly() {
        mode & !0o222
    } else {
        mode
    }
}

/// Attributes for an archive entry with Unix permissions.
pub fn from_unix_mode(unix: u32) -> u32 {
    if unix & 0o222 == 0 {
        FILE_ATTRIBUTE_READONLY
    } else {
        0
    }
}

/// Hidden by the H attribute (names starting with a dot are ordinary here).
pub fn is_hidden(_name: &OsStr, mode: u32) -> bool {
    mode & FILE_ATTRIBUTE_HIDDEN != 0
}

/// Programs are known by the extension.
pub fn is_executable(name: &OsStr, _mode: u32) -> bool {
    let ext = Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    matches!(
        ext.as_deref(),
        Some("exe" | "com" | "bat" | "cmd" | "ps1" | "msi")
    )
}

/// `r-sa`: one letter per attribute, `-` when off.
pub fn format_mode(mode: u32) -> String {
    RHSA.iter()
        .map(|&(bit, c)| if mode & bit != 0 { c } else { '-' })
        .collect()
}

/// Label and value for the Info screen.
pub fn describe_mode(mode: u32) -> (&'static str, String) {
    ("Attributes", format_mode(mode))
}

pub fn mode_prompt(what: &str) -> String {
    format!("ATTRIBUTES for {what} (RHSA, or +/-): ")
}

/// Starting text of the prompt: the attributes that are on (`RA`).
pub fn mode_text(mode: u32) -> String {
    RHSA.iter()
        .filter(|&&(bit, _)| mode & bit != 0)
        .map(|&(_, c)| c.to_ascii_uppercase())
        .collect()
}

/// `RA` sets exactly those attributes; `+R -A` turns some on or off and
/// keeps the rest. Other attributes (directory, compressed...) are kept.
pub fn parse_mode(text: &str, current: u32) -> Result<u32, String> {
    let error = || "Attributes are R, H, S and A, e.g. RA or +R -A".to_string();
    let relative = text.contains(['+', '-']);
    let mut mode = if relative {
        current
    } else {
        current & !RHSA_MASK
    };
    let mut on = true;
    for c in text.chars() {
        match c.to_ascii_lowercase() {
            '+' => on = true,
            '-' => on = false,
            ' ' => {}
            c => {
                let &(bit, _) = RHSA.iter().find(|&&(_, l)| l == c).ok_or_else(error)?;
                if on {
                    mode |= bit;
                } else {
                    mode &= !bit;
                }
            }
        }
    }
    Ok(mode)
}

pub fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    let settable = RHSA_MASK
        | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED
        | FILE_ATTRIBUTE_OFFLINE
        | FILE_ATTRIBUTE_TEMPORARY;
    let attrs = match mode & settable {
        0 => FILE_ATTRIBUTE_NORMAL,
        a => a,
    };
    let wide = wide(path);
    // SAFETY: `wide` is NUL-terminated.
    if unsafe { SetFileAttributesW(wide.as_ptr(), attrs) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Applies Unix permissions: only the write bits matter (read-only).
pub fn set_file_mode(file: &File, unix: u32) -> io::Result<()> {
    let mut perms = file.metadata()?.permissions();
    perms.set_readonly(unix & 0o222 == 0);
    file.set_permissions(perms)
}

/// Changes the modification date: needs a handle opened with permission to
/// write attributes (and, for directories, backup semantics).
pub fn set_modified(path: &Path, time: SystemTime) -> io::Result<()> {
    OpenOptions::new()
        .access_mode(FILE_WRITE_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?
        .set_modified(time)
}

/// Deletes a file or link. Like `rm` on Linux, a read-only file is deleted
/// too; a link to a directory (or junction) is removed as a directory.
pub fn remove_file(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink_dir() {
        return fs::remove_dir(path);
    }
    if meta.permissions().readonly() {
        set_mode(path, mode(&meta) & !FILE_ATTRIBUTE_READONLY)?;
    }
    fs::remove_file(path)
}

/// Volume serial number and file index: identify a file regardless of the
/// name used to reach it. Links are not followed.
pub type FileId = (u32, u64);

pub fn file_id(path: &Path, _meta: &Metadata) -> Option<FileId> {
    let file = OpenOptions::new()
        .access_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .ok()?;
    // SAFETY: plain C struct, filled in by the call below.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: the handle is open for the whole call; `info` is a local.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return None;
    }
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Some((info.dwVolumeSerialNumber, index))
}

/// Volume of the file (to notice other volumes mounted inside a branch).
pub fn device(path: &Path, meta: &Metadata) -> u64 {
    file_id(path, meta).map_or(0, |(volume, _)| u64::from(volume))
}

/// Creates a symbolic link (needs Developer Mode or administrator rights).
pub fn symlink(target: &Path, link: &Path, is_dir: bool) -> io::Result<()> {
    if is_dir {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

/// Characters and names that Windows does not accept in file names.
pub fn check_name(name: &str) -> Result<(), String> {
    if let Some(c) = name
        .chars()
        .find(|&c| c < ' ' || r#"<>:"/\|?*"#.contains(c))
    {
        return Err(format!("Name cannot contain '{}'", c.escape_default()));
    }
    if name.ends_with(['.', ' ']) {
        return Err("Name cannot end with a dot or a space".into());
    }
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    let reserved = ["CON", "PRN", "AUX", "NUL"]
        .iter()
        .any(|r| stem.eq_ignore_ascii_case(r))
        || (stem.len() == 4
            && ["COM", "LPT"]
                .iter()
                .any(|p| stem.as_bytes()[..3].eq_ignore_ascii_case(p.as_bytes()))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'));
    if reserved {
        return Err(format!("{stem} is a reserved name on Windows"));
    }
    Ok(())
}

/// Files under the user profile are already private.
pub fn private(options: &mut OpenOptions) -> &mut OpenOptions {
    options
}

pub fn disk_space(path: &Path) -> Option<DiskSpace> {
    let wide = wide(path);
    let (mut available, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: `wide` is NUL-terminated and the pointers are to local u64s.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) };
    (ok != 0).then_some(DiskSpace {
        total,
        free,
        available,
    })
}

/// The drives (`C:\`, `D:\`...).
pub fn mounts() -> Vec<Mount> {
    // SAFETY: no arguments; returns a bit mask, A = bit 0.
    let mask = unsafe { GetLogicalDrives() };
    (0..26)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| Mount {
            point: PathBuf::from(format!("{}:\\", char::from(b'A' + i as u8))),
            fstype: String::new(),
        })
        .collect()
}

pub fn home_dir() -> Option<PathBuf> {
    env_dir("USERPROFILE")
}

/// `%APPDATA%` (roaming: goes with the user to other machines).
pub(super) fn config_base() -> Option<PathBuf> {
    env_dir("APPDATA")
}

/// `%LOCALAPPDATA%`.
pub(super) fn state_base() -> Option<PathBuf> {
    env_dir("LOCALAPPDATA")
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain([0]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes() {
        let ra = FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_ARCHIVE;
        assert_eq!(format_mode(ra), "r--a");
        assert_eq!(mode_text(ra), "RA");
        assert_eq!(
            parse_mode("hs", ra),
            Ok(FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM)
        );
        assert_eq!(
            parse_mode("+h -r", ra),
            Ok(FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_ARCHIVE)
        );
        assert_eq!(parse_mode("", ra), Ok(0));
        assert!(parse_mode("x", ra).is_err());
        // The directory bit survives.
        assert_eq!(parse_mode("r", 0x10), Ok(0x10 | FILE_ATTRIBUTE_READONLY));
    }

    #[test]
    fn sets_and_reads_attributes_and_deletes_read_only() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("a.txt");
        fs::write(&f, "x").unwrap();
        set_mode(&f, FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_HIDDEN).unwrap();
        let m = mode(&fs::metadata(&f).unwrap());
        assert_eq!(format_mode(m), "rh--");
        assert!(is_hidden(OsStr::new("a.txt"), m));
        remove_file(&f).unwrap();
        assert!(!f.exists());
    }

    #[test]
    fn file_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        let id = |p: &Path| file_id(p, &fs::metadata(p).unwrap()).unwrap();
        assert_eq!(id(&a), id(&tmp.path().join(".").join("a")));
        assert_ne!(id(&a), id(&b));
    }

    #[test]
    fn names() {
        assert!(check_name("ok.txt").is_ok());
        assert!(check_name("a:b").is_err());
        assert!(check_name("nul.txt").is_err());
        assert!(check_name("COM1").is_err());
        assert!(check_name("COM10").is_ok());
        assert!(check_name("end.").is_err());
        assert!(is_executable(OsStr::new("Setup.EXE"), 0));
        assert!(!is_executable(OsStr::new("notes.txt"), 0));
    }
}
