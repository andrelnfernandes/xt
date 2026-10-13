//! What depends on the operating system: permissions, file identity,
//! disk space, mounted file systems, the machine (statistics screen) and
//! where configuration and state live.
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

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}
