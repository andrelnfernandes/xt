//! What depends on the operating system: permissions, file identity,
//! disk space, mounted file systems, the machine (statistics screen) and
//! where configuration, state and temporary files live.
//!
//! Each system has its own module with the same functions; the rest of Xylem
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
    /// What is mounted: the device, or what a network or FUSE file system
    /// serves (for rclone, the remote: `gdrive:`). Empty on Windows.
    pub source: String,
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

/// The machine and Xylem's process, for the statistics screen (`?`). What the
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
    /// Memory used by Xylem itself (resident set / working set).
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

/// Name of Xylem's folders (configuration, state, temporary files).
const DIR_NAME: &str = "xylem";

/// Name of the folders before Xylem was called Xylem: see
/// [`migrate_from_xt`].
const OLD_DIR_NAME: &str = "xt";

/// Directory of Xylem's configuration (`config.toml`).
pub fn config_dir() -> Option<PathBuf> {
    Some(config_base()?.join(DIR_NAME))
}

/// Directory of the state kept between runs (history).
pub fn state_dir() -> Option<PathBuf> {
    Some(state_base()?.join(DIR_NAME))
}

/// Xylem used to be called xt: moves xt's configuration and state (and the
/// built-in terminal's fish history) to Xylem's names, when the old one
/// exists and the new one doesn't. Each pair has the same parent, so it is
/// a plain rename. Returns what was moved, `(from, to)`.
pub fn migrate_from_xt() -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    for base in [config_base(), state_base()].into_iter().flatten() {
        pairs.push((base.join(OLD_DIR_NAME), base.join(DIR_NAME)));
    }
    if let Some(fish) = fish_data_dir() {
        pairs.push((fish.join("xt_history"), fish.join("xylem_history")));
    }
    move_missing(pairs)
}

/// Renames each `(old, new)` whose `old` exists and `new` doesn't.
fn move_missing(pairs: Vec<(PathBuf, PathBuf)>) -> Vec<(PathBuf, PathBuf)> {
    pairs
        .into_iter()
        .filter(|(old, new)| old.exists() && !new.exists() && std::fs::rename(old, new).is_ok())
        .collect()
}

/// Directory of Xylem's temporary files (`$XDG_CACHE_HOME/xylem`,
/// `~/.cache/xylem`; `%LOCALAPPDATA%\xylem\cache` on Windows). Inside the
/// user's own space, never in the shared `/tmp`.
pub fn cache_dir() -> Option<PathBuf> {
    cache_base()
}

/// A folder of this run of Xylem inside `cache_dir`, readable only by the user
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

/// Hidden temporary file next to `dst` (so the final rename is atomic),
/// named after this run.
pub fn tmp_path(dst: &std::path::Path) -> PathBuf {
    let name = dst.file_name().unwrap_or_default().to_string_lossy();
    dst.with_file_name(format!(".{name}.xy-{}.tmp", std::process::id()))
}

/// Creates `tmp_path(dst)` for writing, never opening what is already
/// there: in a shared folder someone else could have put a link with that
/// name, pointing to one of the user's files. Whatever is there is removed
/// (a link, not its target) and the creation tried once more.
pub fn create_tmp(dst: &std::path::Path) -> std::io::Result<(std::fs::File, PathBuf)> {
    let tmp = tmp_path(dst);
    let create = || {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
    };
    let file = match create() {
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            std::fs::remove_file(&tmp)?;
            create()?
        }
        other => other?,
    };
    Ok((file, tmp))
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Moves a file or directory to the system trash (FreeDesktop Trash on Linux,
/// Recycle Bin on Windows). In tests, to a trash of the test's own.
pub fn trash_file(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(test)]
    {
        test_trash::trash_file(path)
    }
    #[cfg(not(test))]
    system_trash_file(path)
}

/// Returns the most recently deleted item from the trash, if any.
pub fn latest_trash_item() -> Result<Option<TrashEntry>, String> {
    #[cfg(test)]
    {
        Ok(test_trash::latest_item())
    }
    #[cfg(not(test))]
    system_latest_trash_item()
}

/// Restores the trash items corresponding to the specified original paths.
pub fn restore_trash_paths(paths: &[PathBuf]) -> Result<usize, String> {
    #[cfg(test)]
    {
        test_trash::restore_paths(paths)
    }
    #[cfg(not(test))]
    system_restore_trash_paths(paths)
}

fn system_trash_file(path: &std::path::Path) -> std::io::Result<()> {
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
            // Its error messages would land on top of the interface.
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
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

fn system_latest_trash_item() -> Result<Option<TrashEntry>, String> {
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

fn system_restore_trash_paths(paths: &[PathBuf]) -> Result<usize, String> {
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

/// In tests the user's trash is left alone: trashed items move to a
/// temporary folder of the test (its thread), which remembers where they
/// came from.
#[cfg(test)]
mod test_trash {
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::TrashEntry;

    struct Trash {
        dir: tempfile::TempDir,
        /// (original path, where it is now), oldest first.
        items: Vec<(PathBuf, PathBuf)>,
    }

    thread_local! {
        static TRASH: RefCell<Option<Trash>> = const { RefCell::new(None) };
    }

    pub fn trash_file(path: &Path) -> std::io::Result<()> {
        TRASH.with(|t| {
            let mut t = t.borrow_mut();
            let t = t.get_or_insert_with(|| Trash {
                dir: crate::testing::tempdir(),
                items: Vec::new(),
            });
            let stored = t.dir.path().join(t.items.len().to_string());
            fs::rename(path, &stored)?;
            t.items.push((path.to_path_buf(), stored));
            Ok(())
        })
    }

    pub fn latest_item() -> Option<TrashEntry> {
        TRASH.with(|t| {
            let t = t.borrow();
            let (original, _) = t.as_ref()?.items.last()?;
            Some(TrashEntry {
                name: original.file_name()?.to_string_lossy().into_owned(),
                original_path: original.clone(),
                time_deleted: 0,
            })
        })
    }

    pub fn restore_paths(paths: &[PathBuf]) -> Result<usize, String> {
        TRASH.with(|t| {
            let mut t = t.borrow_mut();
            let mut count = 0;
            if let Some(t) = t.as_mut() {
                for path in paths {
                    if let Some(i) = t.items.iter().rposition(|(o, _)| o == path) {
                        let (original, stored) = t.items.remove(i);
                        fs::rename(stored, original).map_err(|e| e.to_string())?;
                        count += 1;
                    }
                }
            }
            if count == 0 {
                return Err("No matching files found in trash".into());
            }
            Ok(count)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_planted_link_is_replaced_not_followed() {
        let tmp = crate::testing::tempdir();
        let victim = tmp.path().join("victim");
        std::fs::write(&victim, "keep").unwrap();
        let dst = tmp.path().join("out.txt");
        std::os::unix::fs::symlink(&victim, tmp_path(&dst)).unwrap();
        let (mut file, path) = create_tmp(&dst).unwrap();
        std::io::Write::write_all(&mut file, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "keep");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert!(!std::fs::symlink_metadata(&path).unwrap().is_symlink());
    }

    #[test]
    fn moves_xt_folders_only_when_new_ones_are_missing() {
        let tmp = crate::testing::tempdir();
        let dir = |name: &str| tmp.path().join(name);
        for name in ["config/xt", "state/xt", "state/xylem"] {
            std::fs::create_dir_all(dir(name)).unwrap();
        }
        std::fs::write(dir("config/xt/config.toml"), "x").unwrap();
        std::fs::write(dir("state/xt/history.toml"), "old").unwrap();
        let pairs = vec![
            (dir("config/xt"), dir("config/xylem")),
            (dir("state/xt"), dir("state/xylem")),
            (dir("fish/xt_history"), dir("fish/xylem_history")),
        ];
        let moved = move_missing(pairs);
        assert_eq!(moved, vec![(dir("config/xt"), dir("config/xylem"))]);
        assert!(dir("config/xylem/config.toml").exists() && !dir("config/xt").exists());
        // Xylem already has its own state: xt's stays where it is.
        assert!(dir("state/xt/history.toml").exists());
    }

    /// Uses the real system trash, so it only runs on request:
    /// `cargo test -- --ignored system_trash_and_restore`.
    #[test]
    #[ignore]
    fn system_trash_and_restore() {
        let tmp = crate::testing::tempdir();
        let file = tmp.path().join("xy-trash-test.txt");
        std::fs::write(&file, "x").unwrap();
        system_trash_file(&file).unwrap();
        assert!(!file.exists());
        let latest = system_latest_trash_item().unwrap().expect("empty trash");
        assert_eq!(latest.original_path, file);
        assert_eq!(
            system_restore_trash_paths(std::slice::from_ref(&file)),
            Ok(1)
        );
        assert!(file.exists());
    }
}
