//! Windows.
//!
//! First version: Unix permissions are approximated from the read-only
//! attribute, and mount points inside a branch are not detected (junctions
//! are already treated as links, so they are never entered).

use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_WRITE_ATTRIBUTES, GetDiskFreeSpaceExW, GetLogicalDrives,
};

use super::{DiskSpace, Mount, env_dir};

/// Unix-like permission bits: 644 (755 for directories), without the
/// write bits when the file is read-only.
pub fn mode(meta: &Metadata) -> u32 {
    let mode = if meta.is_dir() { 0o755 } else { 0o644 };
    if meta.permissions().readonly() {
        mode & !0o222
    } else {
        mode
    }
}

/// Only the write bits matter: without them, the file becomes read-only.
pub fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_readonly(mode & 0o222 == 0);
    fs::set_permissions(path, perms)
}

pub fn set_file_mode(file: &File, mode: u32) -> io::Result<()> {
    let mut perms = file.metadata()?.permissions();
    perms.set_readonly(mode & 0o222 == 0);
    file.set_permissions(perms)
}

/// Changes the modification date: needs a handle opened with permission to
/// write attributes (and, for directories, backup semantics).
pub fn set_modified(path: &Path, time: SystemTime) -> io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .access_mode(FILE_WRITE_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?
        .set_modified(time)
}

/// Identifies a file regardless of the name used to reach it: the
/// canonical path (stable Rust has no file index on Windows).
pub type FileId = PathBuf;

pub fn file_id(path: &Path, _meta: &Metadata) -> Option<FileId> {
    fs::canonicalize(path).ok()
}

/// Mount points are not detected yet: everything is on the same "device".
pub fn device(_meta: &Metadata) -> u64 {
    0
}

/// Creates a symbolic link (needs Developer Mode or administrator rights).
pub fn symlink(target: &Path, link: &Path, is_dir: bool) -> io::Result<()> {
    if is_dir {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

/// Files under the user profile are already private.
pub fn private(options: &mut OpenOptions) -> &mut OpenOptions {
    options
}

pub fn disk_space(path: &Path) -> Option<DiskSpace> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
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
