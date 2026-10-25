//! Linux (and other Unix systems).

use std::ffi::OsStr;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::{DiskSpace, Memory, Mount, Network, System, env_dir};

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
            let [source, point, fstype, ..] = fields[..] else {
                return None;
            };
            // "\040" is the space in /proc/mounts names.
            Some(Mount {
                source: source.replace("\\040", " "),
                point: PathBuf::from(point.replace("\\040", " ")),
                fstype: fstype.to_string(),
            })
        })
        .collect()
}

/// "user:group" of `path` (not following symlinks), for the Owner display of
/// the file window. The names are looked up once per id and remembered.
pub fn owner(path: &Path) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use nix::unistd::{Gid, Group, Uid, User};

    // (is a group, id) → name.
    static NAMES: Mutex<Option<HashMap<(bool, u32), String>>> = Mutex::new(None);
    let meta = fs::symlink_metadata(path).ok()?;
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let names = names.get_or_insert_with(HashMap::new);
    let mut name = |group: bool, id: u32| {
        names
            .entry((group, id))
            .or_insert_with(|| {
                let found = if group {
                    Group::from_gid(Gid::from_raw(id))
                        .ok()
                        .flatten()
                        .map(|g| g.name)
                } else {
                    User::from_uid(Uid::from_raw(id))
                        .ok()
                        .flatten()
                        .map(|u| u.name)
                };
                found.unwrap_or_else(|| id.to_string())
            })
            .clone()
    };
    Some(format!(
        "{}:{}",
        name(false, meta.uid()),
        name(true, meta.gid())
    ))
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

/// `$XDG_CACHE_HOME/xylem` or `~/.cache/xylem`.
pub(super) fn cache_base() -> Option<PathBuf> {
    let base = env_dir("XDG_CACHE_HOME").or_else(|| Some(home_dir()?.join(".cache")))?;
    Some(base.join(super::DIR_NAME))
}

/// Where fish keeps its history files: `$XDG_DATA_HOME/fish` or
/// `~/.local/share/fish`.
pub(super) fn fish_data_dir() -> Option<PathBuf> {
    let base =
        env_dir("XDG_DATA_HOME").or_else(|| Some(home_dir()?.join(".local").join("share")))?;
    Some(base.join("fish"))
}

/// The machine, from `/proc` and `/etc/os-release` (Linux; elsewhere most
/// of it stays empty).
pub fn system() -> System {
    let read = |path: &str| fs::read_to_string(path).ok();
    let line = |path: &str| {
        read(path)
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
    };
    let meminfo = read("/proc/meminfo").unwrap_or_default();
    let (memory, swap) = parse_meminfo(&meminfo);
    let kernel = match (
        line("/proc/sys/kernel/ostype"),
        line("/proc/sys/kernel/osrelease"),
    ) {
        (Some(os), Some(release)) => {
            // WSL kernels say so in the release: "...-microsoft-standard-WSL2".
            let wsl = release.to_lowercase().contains("microsoft");
            Some(format!("{os} {release}{}", if wsl { " (WSL)" } else { "" }))
        }
        _ => None,
    };
    System {
        os: read("/etc/os-release").and_then(|t| os_release_name(&t)),
        kernel,
        host: line("/proc/sys/kernel/hostname").or_else(|| std::env::var("HOSTNAME").ok()),
        cpu: read("/proc/cpuinfo").and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("model name")?.split_once(':'))
                .map(|(_, v)| v.split_whitespace().collect::<Vec<_>>().join(" "))
        }),
        uptime: read("/proc/uptime").and_then(|t| {
            let secs: f64 = t.split_whitespace().next()?.parse().ok()?;
            Some(Duration::from_secs(secs as u64))
        }),
        load: read("/proc/loadavg").and_then(|t| {
            let mut it = t.split_whitespace().map(|v| v.parse::<f64>().ok());
            Some([it.next()??, it.next()??, it.next()??])
        }),
        memory,
        swap,
        xt_memory: kb_field(&read("/proc/self/status").unwrap_or_default(), "VmRSS"),
    }
}

/// Interfaces from `getifaddrs`, gateway from `/proc/net/route` and DNS
/// from `/etc/resolv.conf`.
pub fn network() -> Network {
    use nix::net::if_::InterfaceFlags;
    let mut net = Network::default();
    for a in nix::ifaddrs::getifaddrs().into_iter().flatten() {
        if !a.flags.contains(InterfaceFlags::IFF_UP)
            || a.flags.contains(InterfaceFlags::IFF_LOOPBACK)
        {
            continue;
        }
        let (Some(addr), mask) = (a.address, a.netmask) else {
            continue;
        };
        let (ip, prefix) = if let Some(v4) = addr.as_sockaddr_in() {
            let mask = mask.as_ref().and_then(|m| m.as_sockaddr_in());
            (
                IpAddr::V4(v4.ip()),
                mask.map_or(32, |m| m.ip().to_bits().count_ones()),
            )
        } else if let Some(v6) = addr.as_sockaddr_in6() {
            let mask = mask.as_ref().and_then(|m| m.as_sockaddr_in6());
            (
                IpAddr::V6(v6.ip()),
                mask.map_or(128, |m| m.ip().to_bits().count_ones()),
            )
        } else {
            continue;
        };
        net.add(&a.interface_name, ip, prefix as u8);
    }
    let read = |path: &str| fs::read_to_string(path).unwrap_or_default();
    net.gateway = default_gateway(&read("/proc/net/route"));
    net.dns = nameservers(&read("/etc/resolv.conf"));
    net
}

/// The gateway of the default route (destination 0) in `/proc/net/route`,
/// where addresses are hexadecimal in the machine's byte order.
fn default_gateway(route: &str) -> Option<IpAddr> {
    route.lines().skip(1).find_map(|l| {
        let fields: Vec<&str> = l.split_whitespace().collect();
        let [_, "00000000", gateway, ..] = fields[..] else {
            return None;
        };
        let bits = u32::from_str_radix(gateway, 16).ok().filter(|&g| g != 0)?;
        Some(IpAddr::V4(Ipv4Addr::from(bits.to_ne_bytes())))
    })
}

/// The `nameserver` lines of `/etc/resolv.conf`.
fn nameservers(resolv: &str) -> Vec<IpAddr> {
    resolv
        .lines()
        .filter_map(|l| l.trim().strip_prefix("nameserver")?.trim().parse().ok())
        .collect()
}

/// `PRETTY_NAME` (or `NAME`) of `/etc/os-release`.
fn os_release_name(text: &str) -> Option<String> {
    let field = |key: &str| {
        text.lines().find_map(|l| {
            let v = l.strip_prefix(key)?.strip_prefix('=')?;
            let v = v.trim().trim_matches(|c| c == '"' || c == '\'');
            (!v.is_empty()).then(|| v.to_string())
        })
    };
    field("PRETTY_NAME").or_else(|| field("NAME"))
}

/// Memory and swap, from `/proc/meminfo`.
fn parse_meminfo(text: &str) -> (Option<Memory>, Option<Memory>) {
    let pair = |total: &str, available: &str| {
        let total = kb_field(text, total).filter(|&t| t > 0)?;
        Some(Memory {
            total,
            available: kb_field(text, available)?,
        })
    };
    (
        pair("MemTotal", "MemAvailable").or_else(|| pair("MemTotal", "MemFree")),
        pair("SwapTotal", "SwapFree"),
    )
}

/// "Key:   1234 kB" → 1234 × 1024.
fn kb_field(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|l| {
        let v = l.strip_prefix(key)?.strip_prefix(':')?;
        let kb: u64 = v.split_whitespace().next()?.parse().ok()?;
        Some(kb * 1024)
    })
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

    #[test]
    fn system_files() {
        let os = "NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\n";
        assert_eq!(os_release_name(os).as_deref(), Some("Ubuntu 24.04.1 LTS"));
        assert_eq!(os_release_name("NAME=Arch\n").as_deref(), Some("Arch"));
        let mem = "MemTotal:  2048 kB\nMemFree: 100 kB\nMemAvailable: 1024 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n";
        let (memory, swap) = parse_meminfo(mem);
        assert_eq!(
            memory,
            Some(Memory {
                total: 2048 * 1024,
                available: 1024 * 1024
            })
        );
        assert_eq!(swap, None, "no swap configured");
        assert_eq!(parse_meminfo(""), (None, None));
    }

    #[test]
    fn network_files() {
        let route = "Iface\tDestination\tGateway \tFlags\n\
                     eth0\t0014A8C0\t00000000\t0001\n\
                     eth0\t00000000\t0114A8C0\t0003\n";
        let expected: IpAddr = if cfg!(target_endian = "little") {
            "192.168.20.1".parse().unwrap()
        } else {
            "1.20.168.192".parse().unwrap()
        };
        assert_eq!(default_gateway(route), Some(expected));
        assert_eq!(default_gateway("Iface\n"), None);
        let resolv = "# comment\nnameserver 10.0.0.53\nsearch lan\nnameserver ::1\n";
        let dns: Vec<String> = nameservers(resolv)
            .iter()
            .map(|ip| ip.to_string())
            .collect();
        assert_eq!(dns, ["10.0.0.53", "::1"]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn system_on_linux() {
        let s = system();
        assert!(s.kernel.unwrap().starts_with("Linux"));
        assert!(s.memory.unwrap().total > 0);
        assert!(s.xt_memory.unwrap() > 0);
    }
}
