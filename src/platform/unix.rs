//! Linux (and other Unix systems).

use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::{DiskSpace, Mount, env_dir};

/// Permission bits (and file type), as in `st_mode`.
pub fn mode(meta: &Metadata) -> u32 {
    meta.permissions().mode()
}

pub fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

pub fn set_file_mode(file: &File, mode: u32) -> io::Result<()> {
    file.set_permissions(fs::Permissions::from_mode(mode))
}

/// Identifies a file regardless of the name used to reach it.
pub type FileId = (u64, u64);

pub fn file_id(_path: &Path, meta: &Metadata) -> Option<FileId> {
    Some((meta.dev(), meta.ino()))
}

/// Device of the file system that holds the file (to notice mount points).
pub fn device(meta: &Metadata) -> u64 {
    meta.dev()
}

/// Creates a symbolic link; `_is_dir` only matters on Windows.
pub fn symlink(target: &Path, link: &Path, _is_dir: bool) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

/// Makes the file being created readable only by its owner.
pub fn private(options: &mut OpenOptions) -> &mut OpenOptions {
    options.mode(0o600)
}

pub fn disk_space(path: &Path) -> Option<DiskSpace> {
    let s = nix::sys::statvfs::statvfs(path).ok()?;
    let frag = s.fragment_size() as u64;
    Some(DiskSpace {
        total: s.blocks() as u64 * frag,
        free: s.blocks_free() as u64 * frag,
        available: s.blocks_available() as u64 * frag,
    })
}

/// The mounted file systems, from `/proc/self/mounts`.
pub fn mounts() -> Vec<Mount> {
    let text = fs::read_to_string("/proc/self/mounts").unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [_, point, fstype, ..] = fields[..] else {
                return None;
            };
            // "\040" is the space in /proc/mounts names.
            Some(Mount {
                point: PathBuf::from(point.replace("\\040", " ")),
                fstype: fstype.to_string(),
            })
        })
        .collect()
}

pub fn home_dir() -> Option<PathBuf> {
    env_dir("HOME")
}

/// `$XDG_CONFIG_HOME` or `~/.config`.
pub(super) fn config_base() -> Option<PathBuf> {
    env_dir("XDG_CONFIG_HOME").or_else(|| Some(home_dir()?.join(".config")))
}

/// `$XDG_STATE_HOME` or `~/.local/state`.
pub(super) fn state_base() -> Option<PathBuf> {
    env_dir("XDG_STATE_HOME").or_else(|| Some(home_dir()?.join(".local").join("state")))
}
