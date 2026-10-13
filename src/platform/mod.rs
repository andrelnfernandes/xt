//! What depends on the operating system: permissions, file identity,
//! disk space, mounted file systems and where configuration and state live.
//!
//! Each system has its own module with the same functions; the rest of xt
//! only calls `platform::...`.

use std::path::PathBuf;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

/// Space on a file system, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskSpace {
    pub total: u64,
    /// Free, including what is reserved for the superuser.
    pub free: u64,
    /// Free for the current user.
    pub available: u64,
}

/// A mounted file system (Linux) or a drive (Windows).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub point: PathBuf,
    pub fstype: String,
}

/// Directory of xt's configuration (`config.toml`).
pub fn config_dir() -> Option<PathBuf> {
    Some(config_base()?.join("xt"))
}

/// Directory of the state kept between runs (history).
pub fn state_dir() -> Option<PathBuf> {
    Some(state_base()?.join("xt"))
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}
