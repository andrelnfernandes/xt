//! What depends on the operating system: permissions, file identity,
//! disk space, mounted file systems, the machine (statistics screen) and
//! where configuration, state and temporary files live.
//!
//! Each system has its own module with the same functions; the rest of xt
//! only calls `platform::...`.

use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;

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

/// Physical memory or swap, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    pub total: u64,
    /// What can still be used without swapping out (Linux `MemAvailable`).
    pub available: u64,
}

/// The machine and xt's process, for the statistics screen (`?`). What the
/// system does not tell stays `None`.
#[derive(Debug, Default, Clone)]
pub struct System {
    /// Distribution or edition: "Ubuntu 24.04.1 LTS", "Windows 11 Pro 23H2".
    pub os: Option<String>,
    /// "Linux 6.8.0-45-generic", "Windows NT build 22631.4317".
    pub kernel: Option<String>,
    pub host: Option<String>,
    pub cpu: Option<String>,
    pub uptime: Option<Duration>,
    /// Load average over 1, 5 and 15 minutes (Unix).
    pub load: Option<[f64; 3]>,
    pub memory: Option<Memory>,
    pub swap: Option<Memory>,
    /// Memory used by xt itself (resident set / working set).
    pub xt_memory: Option<u64>,
}

/// The network: interfaces that are up (without loopback) and their
/// addresses, the default gateway and the DNS servers.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Network {
    pub interfaces: Vec<Interface>,
    pub gateway: Option<IpAddr>,
    pub dns: Vec<IpAddr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    /// Address and prefix length (the netmask in CIDR notation).
    pub addrs: Vec<(IpAddr, u8)>,
}

impl Network {
    /// Adds an address to its interface, leaving out the loopback and the
    /// IPv6 link-local addresses (fe80::/10, present on every interface).
    fn add(&mut self, name: &str, ip: IpAddr, prefix: u8) {
        let link_local = matches!(ip, IpAddr::V6(v6) if v6.segments()[0] & 0xffc0 == 0xfe80);
        if ip.is_loopback() || ip.is_unspecified() || link_local {
            return;
        }
        match self.interfaces.iter_mut().find(|i| i.name == name) {
            Some(i) => i.addrs.push((ip, prefix)),
            None => self.interfaces.push(Interface {
                name: name.to_string(),
                addrs: vec![(ip, prefix)],
            }),
        }
    }
}

/// Directory of xt's configuration (`config.toml`).
pub fn config_dir() -> Option<PathBuf> {
    Some(config_base()?.join("xt"))
}

/// Directory of the state kept between runs (history).
pub fn state_dir() -> Option<PathBuf> {
    Some(state_base()?.join("xt"))
}

/// Directory of xt's temporary files (`$XDG_CACHE_HOME/xt`, `~/.cache/xt`;
/// `%LOCALAPPDATA%\xt\cache` on Windows). Inside the user's own space, never
/// in the shared `/tmp`.
pub fn cache_dir() -> Option<PathBuf> {
    cache_base()
}

/// A folder of this run of xt inside `cache_dir`, readable only by the user
/// and removed on exit (by `main`): files extracted from ZIP/TAR, the
/// built-in terminal's script. Created if missing.
pub fn run_dir() -> std::io::Result<PathBuf> {
    let base = cache_dir().ok_or_else(|| std::io::Error::other("no home directory"))?;
    let dir = run_dir_path(&base);
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(&dir)?;
    Ok(dir)
}

/// Where `run_dir` is, without creating it (to remove it on exit).
pub fn run_dir_path(base: &std::path::Path) -> PathBuf {
    base.join(format!("run-{}", std::process::id()))
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Moves a file or directory to the system trash (FreeDesktop Trash on Linux,
/// Recycle Bin on Windows).
pub fn trash_file(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(all(
        unix,
        not(target_os = "macos"),
        not(target_os = "ios"),
        not(target_os = "android")
    ))]
    {
        // Try `gio trash` first if available, which immediately notifies GVFS / desktop
        // environments (GNOME, XFCE, MATE, Cinnamon) via D-Bus that the trash has changed.
        if let Ok(status) = std::process::Command::new("gio")
            .arg("trash")
            .arg(path)
            .status()
            && status.success()
        {
            return Ok(());
        }
    }
    trash::delete(path).map_err(|e| std::io::Error::other(e.to_string()))
}

/// An item in the trash that can be restored.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TrashEntry {
    pub name: String,
    pub original_path: PathBuf,
    pub time_deleted: i64,
}

/// Returns the most recently deleted item from the trash, if any.
pub fn latest_trash_item() -> Result<Option<TrashEntry>, String> {
    #[cfg(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    ))]
    {
        let mut items = trash::os_limited::list().map_err(|e| e.to_string())?;
        if items.is_empty() {
            return Ok(None);
        }
        items.sort_by_key(|i| std::cmp::Reverse(i.time_deleted));
        let item = &items[0];
        Ok(Some(TrashEntry {
            name: item.name.to_string_lossy().into_owned(),
            original_path: item.original_path(),
            time_deleted: item.time_deleted,
        }))
    }
    #[cfg(not(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    )))]
    {
        Err("Trash restore is not supported on this platform".into())
    }
}

/// Restores the trash items corresponding to the specified original paths.
pub fn restore_trash_paths(paths: &[PathBuf]) -> Result<usize, String> {
    #[cfg(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    ))]
    {
        let all_items = trash::os_limited::list().map_err(|e| e.to_string())?;
        let mut to_restore = Vec::new();
        for path in paths {
            let mut matching: Vec<_> = all_items
                .iter()
                .filter(|item| item.original_path() == *path)
                .cloned()
                .collect();
            matching.sort_by_key(|item| std::cmp::Reverse(item.time_deleted));
            if let Some(item) = matching.into_iter().next() {
                to_restore.push(item);
            }
        }
        if to_restore.is_empty() {
            return Err("No matching files found in trash".into());
        }
        let count = to_restore.len();
        trash::os_limited::restore_all(to_restore).map_err(|e| e.to_string())?;
        Ok(count)
    }
    #[cfg(not(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    )))]
    {
        Err("Trash restore is not supported on this platform".into())
    }
}
