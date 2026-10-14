//! Information screens, shown in the viewer: Info (Alt+I), Avail
//! (`A` in the tree, the mounted file systems) and statistics (`?`, with
//! the machine and xt itself).

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Local};

use crate::platform;

/// Everything about a file or directory.
pub fn path_info(path: &Path) -> String {
    let mut out = format!("INFO  {}\n\n", path.display());
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) => {
            let _ = writeln!(out, "Cannot read: {e}");
            return out;
        }
    };
    let ft = meta.file_type();
    let kind = if ft.is_dir() {
        "directory"
    } else if ft.is_symlink() {
        "symbolic link"
    } else if ft.is_file() {
        "regular file"
    } else {
        special_kind(&ft)
    };
    row(&mut out, "Type", kind.into());
    if ft.is_symlink() {
        let target = fs::read_link(path).map(|t| t.display().to_string());
        row(
            &mut out,
            "Points to",
            target.unwrap_or_else(|e| e.to_string()),
        );
        let broken = fs::metadata(path).is_err();
        if broken {
            row(&mut out, "", "(broken: the target does not exist)".into());
        }
    }
    row(&mut out, "Size", format!("{} bytes", group(meta.len())));
    let (label, mode) = platform::describe_mode(platform::mode(&meta));
    row(&mut out, label, mode);
    system_rows(&mut out, &meta);
    out
}

fn row(out: &mut String, k: &str, v: String) {
    let _ = writeln!(out, "  {k:<14}{v}");
}

#[cfg(unix)]
fn special_kind(ft: &fs::FileType) -> &'static str {
    use std::os::unix::fs::FileTypeExt;
    if ft.is_fifo() {
        "named pipe (FIFO)"
    } else if ft.is_socket() {
        "socket"
    } else if ft.is_block_device() {
        "block device"
    } else if ft.is_char_device() {
        "character device"
    } else {
        "other"
    }
}

#[cfg(windows)]
fn special_kind(_ft: &fs::FileType) -> &'static str {
    "other"
}

/// Owner, dates, inode and device.
#[cfg(unix)]
fn system_rows(out: &mut String, meta: &fs::Metadata) {
    use std::os::unix::fs::MetadataExt;
    row(out, "Owner", user_name(meta.uid()));
    row(out, "Group", group_name(meta.gid()));
    row(out, "Modified", time(meta.modified().ok()));
    row(out, "Accessed", time(meta.accessed().ok()));
    let changed =
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(meta.ctime().max(0) as u64);
    row(out, "Changed", time(Some(changed)));
    row(out, "Inode", meta.ino().to_string());
    row(out, "Links", meta.nlink().to_string());
    row(
        out,
        "Device",
        format!("{}:{}", major(meta.dev()), minor(meta.dev())),
    );
}

/// Dates.
#[cfg(windows)]
fn system_rows(out: &mut String, meta: &fs::Metadata) {
    row(out, "Created", time(meta.created().ok()));
    row(out, "Modified", time(meta.modified().ok()));
    row(out, "Accessed", time(meta.accessed().ok()));
}

/// The mounted file systems, like `df`.
pub fn avail() -> String {
    let mut out = String::from("AVAIL  mounted file systems\n\n");
    let _ = writeln!(
        out,
        "  {:>15} {:>15} {:>15} {:>5}  {:<10} Mounted on",
        "Total", "Used", "Free", "Use%", "Type"
    );
    let mut seen = std::collections::HashSet::new();
    for mount in platform::mounts() {
        let Some(disk) = platform::disk_space(&mount.point) else {
            continue;
        };
        // Virtual file systems (proc, sysfs, cgroup...) have no size.
        if disk.total == 0 || !seen.insert(mount.point.clone()) {
            continue;
        }
        let free = disk.available;
        let used = disk.total.saturating_sub(disk.free);
        let pct = (used * 100).checked_div(used + free).unwrap_or(0);
        let _ = writeln!(
            out,
            "  {:>15} {:>15} {:>15} {:>4}%  {:<10} {}",
            group(disk.total),
            group(used),
            group(free),
            pct,
            mount.fstype,
            mount.point.display()
        );
    }
    out
}

/// The SYSTEM and XT sections of the statistics screen (`?`).
pub fn system() -> String {
    let s = platform::system();
    let mut out = String::from("SYSTEM\n");
    let mut opt = |k: &str, v: Option<String>| {
        if let Some(v) = v {
            row(&mut out, k, v);
        }
    };
    let env = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    opt("OS", s.os);
    opt("Kernel", s.kernel);
    opt("Host", s.host);
    opt("User", env("USER").or_else(|| env("USERNAME")));
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get());
    let cpu = match s.cpu {
        Some(model) => format!("{model}, {threads} threads"),
        None => format!("{threads} threads"),
    };
    opt("CPU", Some(format!("{cpu} ({})", std::env::consts::ARCH)));
    opt("Uptime", s.uptime.map(uptime));
    opt(
        "Load average",
        s.load
            .map(|l| format!("{:.2}  {:.2}  {:.2}", l[0], l[1], l[2])),
    );
    opt("Memory", s.memory.map(|m| memory(m, "available")));
    opt("Swap", s.swap.map(|m| memory(m, "free")));
    opt("Shell", env("SHELL").or_else(|| env("COMSPEC")));
    opt("Terminal", env("TERM_PROGRAM").or_else(|| env("TERM")));

    out.push_str("\nNETWORK\n");
    network(&mut out, &platform::network());

    out.push_str("\nXT\n");
    let mut opt = |k: &str, v: Option<String>| {
        if let Some(v) = v {
            row(&mut out, k, v);
        }
    };
    opt("Version", Some(env!("CARGO_PKG_VERSION").into()));
    opt("Process", Some(format!("PID {}", std::process::id())));
    opt(
        "Memory",
        s.xt_memory.map(|b| format!("{} in use", human(b))),
    );
    opt(
        "Executable",
        std::env::current_exe()
            .ok()
            .map(|p| p.display().to_string()),
    );
    opt(
        "Config",
        crate::config::Config::path().map(|p| {
            let state = if p.exists() { "" } else { " (not created)" };
            format!("{}{state}", p.display())
        }),
    );
    out
}

/// Interfaces with their addresses (one per line), gateway and DNS.
fn network(out: &mut String, net: &platform::Network) {
    if net.interfaces.is_empty() {
        row(out, "Interfaces", "none connected".into());
    }
    for i in &net.interfaces {
        // A long name ("vEthernet (WSL)") gets its own line.
        let mut key = i.name.as_str();
        if key.chars().count() > 13 {
            row(out, key, String::new());
            key = "";
        }
        for (ip, prefix) in &i.addrs {
            row(out, key, format!("{ip}/{prefix}"));
            key = "";
        }
    }
    if let Some(gw) = net.gateway {
        row(out, "Gateway", gw.to_string());
    }
    if !net.dns.is_empty() {
        let dns: Vec<String> = net.dns.iter().map(|ip| ip.to_string()).collect();
        row(out, "DNS", dns.join("  "));
    }
}

/// The disk of `path`: where its file system is mounted, free and used.
pub fn disk(path: &Path) -> String {
    let Some(d) = platform::disk_space(path) else {
        return String::new();
    };
    let mut out = String::new();
    // The deepest mount point containing the path (the last one wins when a
    // point is mounted over).
    let mount = platform::mounts()
        .into_iter()
        .filter(|m| path.starts_with(&m.point))
        .max_by_key(|m| m.point.as_os_str().len());
    let used = d.total.saturating_sub(d.free);
    let pct = (used * 100).checked_div(used + d.available).unwrap_or(0);
    let usage = format!(
        "{} free, {} used of {} ({pct}% used)",
        human(d.available),
        human(used),
        human(d.total)
    );
    match mount {
        Some(m) => {
            let fstype = if m.fstype.is_empty() {
                String::new()
            } else {
                format!(" ({})", m.fstype)
            };
            row(&mut out, "Disk", format!("{}{fstype}", m.point.display()));
            row(&mut out, "", usage);
        }
        None => row(&mut out, "Disk", usage),
    }
    out
}

/// "8.0 GiB available of 15.5 GiB (48% used)".
fn memory(m: platform::Memory, free: &str) -> String {
    let used = m.total.saturating_sub(m.available);
    let pct = (used * 100).checked_div(m.total).unwrap_or(0);
    format!(
        "{} {free} of {} ({pct}% used)",
        human(m.available),
        human(m.total)
    )
}

/// "3 days, 4:05" (hours:minutes), like `uptime`.
fn uptime(d: std::time::Duration) -> String {
    let mins = d.as_secs() / 60;
    let (days, h, m) = (mins / 1440, mins / 60 % 24, mins % 60);
    match days {
        0 => format!("{h}:{m:02}"),
        1 => format!("1 day, {h}:{m:02}"),
        _ => format!("{days} days, {h}:{m:02}"),
    }
}

/// Sizes in binary units: "512 bytes", "45.2 MiB", "15.5 GiB".
fn human(n: u64) -> String {
    const UNITS: [&str; 5] = ["KiB", "MiB", "GiB", "TiB", "PiB"];
    if n < 1024 {
        return format!("{n} bytes");
    }
    let mut v = n as f64 / 1024.0;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    format!("{v:.1} {}", UNITS[unit])
}

/// "1234567" → "1,234,567" (the information screens always use the comma,
/// like `df`, to align).
pub fn group(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn time(t: Option<SystemTime>) -> String {
    t.map(|t| {
        DateTime::<Local>::from(t)
            .format("%Y-%m-%d %H:%M:%S %z")
            .to_string()
    })
    .unwrap_or_else(|| "-".into())
}

#[cfg(unix)]
fn user_name(uid: u32) -> String {
    use nix::unistd::{Uid, User};
    match User::from_uid(Uid::from_raw(uid)) {
        Ok(Some(u)) => format!("{} ({uid})", u.name),
        _ => uid.to_string(),
    }
}

#[cfg(unix)]
fn group_name(gid: u32) -> String {
    use nix::unistd::{Gid, Group};
    match Group::from_gid(Gid::from_raw(gid)) {
        Ok(Some(g)) => format!("{} ({gid})", g.name),
        _ => gid.to_string(),
    }
}

#[cfg(unix)]
fn major(dev: u64) -> u64 {
    ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0xfff)
}

#[cfg(unix)]
fn minor(dev: u64) -> u64 {
    ((dev >> 12) & 0xffff_ff00) | (dev & 0xff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn file_info_has_the_basics() {
        let tmp = crate::testing::tempdir();
        let f = tmp.path().join("a.txt");
        fs::write(&f, "12345").unwrap();
        std::os::unix::fs::symlink("nope", tmp.path().join("broken")).unwrap();
        let text = path_info(&f);
        assert!(text.contains("regular file"));
        assert!(text.contains("5 bytes"));
        assert!(text.contains("Permissions"));
        let link = path_info(&tmp.path().join("broken"));
        assert!(
            link.contains("symbolic link") && link.contains("Points to") && link.contains("broken")
        );
        assert!(path_info(&tmp.path().join("missing")).contains("Cannot read"));
    }

    #[cfg(unix)]
    #[test]
    fn avail_lists_the_root_file_system() {
        let text = avail();
        assert!(text.lines().any(|l| l.ends_with(" /")), "{text}");
    }

    #[test]
    fn network_rows() {
        let ip = |s: &str| s.parse::<std::net::IpAddr>().unwrap();
        let net = platform::Network {
            interfaces: vec![
                platform::Interface {
                    name: "eth0".into(),
                    addrs: vec![(ip("192.168.0.10"), 24), (ip("2001:db8::10"), 64)],
                },
                platform::Interface {
                    name: "vEthernet (WSL)".into(),
                    addrs: vec![(ip("172.20.0.1"), 20)],
                },
            ],
            gateway: Some(ip("192.168.0.1")),
            dns: vec![ip("192.168.0.1"), ip("8.8.8.8")],
        };
        let mut out = String::new();
        network(&mut out, &net);
        assert_eq!(
            out,
            "  eth0          192.168.0.10/24\n\
             \x20               2001:db8::10/64\n\
             \x20 vEthernet (WSL)\n\
             \x20               172.20.0.1/20\n\
             \x20 Gateway       192.168.0.1\n\
             \x20 DNS           192.168.0.1  8.8.8.8\n"
        );
        let mut out = String::new();
        network(&mut out, &platform::Network::default());
        assert!(out.contains("none connected"));
    }

    #[test]
    fn disk_of_a_directory() {
        let tmp = crate::testing::tempdir();
        let text = disk(tmp.path());
        assert!(text.contains("free,") && text.contains("used of"), "{text}");
    }

    #[test]
    fn helpers() {
        assert_eq!(group(1234567), "1,234,567");
        assert_eq!(human(512), "512 bytes");
        assert_eq!(human(1536), "1.5 KiB");
        assert_eq!(human(16 * 1024 * 1024 * 1024), "16.0 GiB");
        let gib = 1024 * 1024 * 1024;
        assert_eq!(
            memory(
                platform::Memory {
                    total: 4 * gib,
                    available: gib
                },
                "available"
            ),
            "1.0 GiB available of 4.0 GiB (75% used)"
        );
        let min = std::time::Duration::from_secs(60);
        assert_eq!(uptime(min * 65), "1:05");
        assert_eq!(uptime(min * (1440 + 3)), "1 day, 0:03");
        assert_eq!(uptime(min * (3 * 1440 + 125)), "3 days, 2:05");
        #[cfg(unix)]
        assert_eq!((major(0x803), minor(0x803)), (8, 3));
    }
}
