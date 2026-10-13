//! Linux (and other Unix systems).

use std::ffi::OsStr;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{DiskSpace, Mount, env_dir};

/// Permission bits (and file type), as in `st_mode`.
pub fn mode(meta: &Metadata) -> u32 {
    meta.permissions().mode()
}

/// Permissions for archives and copies: the same thing on Unix.
pub fn unix_mode(meta: &Metadata) -> u32 {
    mode(meta)
}

pub fn from_unix_mode(unix: u32) -> u32 {
    unix
}

/// Hidden name: starts with a dot.
pub fn is_hidden(name: &OsStr, _mode: u32) -> bool {
    name.as_encoded_bytes().first() == Some(&b'.')
}

pub fn is_executable(_name: &OsStr, mode: u32) -> bool {
    mode & 0o111 != 0
}

/// `rwxr-x---`.
pub fn format_mode(mode: u32) -> String {
    let mut s = String::with_capacity(9);
    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 0o7;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        s.push(if bits & 1 != 0 { 'x' } else { '-' });
    }
    s
}

/// Label and value for the Info screen.
pub fn describe_mode(mode: u32) -> (&'static str, String) {
    let mode = mode & 0o7777;
    ("Permissions", format!("{mode:04o}  {}", format_mode(mode)))
}

pub fn mode_prompt(what: &str) -> String {
    format!("PERMISSIONS for {what} (octal): ")
}

/// Starting text of the prompt: `644`.
pub fn mode_text(mode: u32) -> String {
    format!("{:03o}", mode & 0o7777)
}

/// Octal permissions, e.g.: "644" or "0755".
pub fn parse_mode(text: &str, _current: u32) -> Result<u32, String> {
    let text = text.trim();
    let valid = (3..=4).contains(&text.len()) && text.chars().all(|c| ('0'..='7').contains(&c));
    if !valid {
        return Err("Permissions must be 3 or 4 octal digits, e.g. 644".into());
    }
    u32::from_str_radix(text, 8).map_err(|e| e.to_string())
}

pub fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

pub fn set_file_mode(file: &File, mode: u32) -> io::Result<()> {
    file.set_permissions(fs::Permissions::from_mode(mode))
}

pub fn remove_file(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

/// Any name without `/` is valid.
pub fn check_name(_name: &str) -> Result<(), String> {
    Ok(())
}

/// Changes the modification date (the owner can do it without write
/// permission).
pub fn set_modified(path: &Path, time: SystemTime) -> io::Result<()> {
    File::open(path)?.set_modified(time)
}

/// Identifies a file regardless of the name used to reach it.
pub type FileId = (u64, u64);

pub fn file_id(_path: &Path, meta: &Metadata) -> Option<FileId> {
    Some((meta.dev(), meta.ino()))
}

/// Device of the file system that holds the file (to notice mount points).
pub fn device(_path: &Path, meta: &Metadata) -> u64 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes() {
        assert_eq!(format_mode(0o750), "rwxr-x---");
        assert_eq!(describe_mode(0o100644).1, "0644  rw-r--r--");
        assert_eq!(mode_text(0o100644), "644");
        assert_eq!(parse_mode("644", 0), Ok(0o644));
        assert_eq!(parse_mode("4755", 0), Ok(0o4755));
        assert!(parse_mode("8", 0).is_err());
        assert!(parse_mode("rwx", 0).is_err());
        assert!(is_hidden(OsStr::new(".x"), 0) && !is_hidden(OsStr::new("x"), 0));
    }
}
