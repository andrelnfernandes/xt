//! Information screens, shown in the viewer: Info (Alt+I), Avail
//! (`A` in the tree, the mounted file systems) and statistics (`?`).

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
    let mode = platform::mode(&meta) & 0o7777;
    row(
        &mut out,
        "Permissions",
        format!("{mode:04o}  {}", rwx(mode)),
    );
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

fn rwx(mode: u32) -> String {
    let mut s = String::new();
    for shift in [6, 3, 0] {
        let b = (mode >> shift) & 7;
        s.push(if b & 4 != 0 { 'r' } else { '-' });
        s.push(if b & 2 != 0 { 'w' } else { '-' });
        s.push(if b & 1 != 0 { 'x' } else { '-' });
    }
    s
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
        let tmp = tempfile::tempdir().unwrap();
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
    fn helpers() {
        assert_eq!(group(1234567), "1,234,567");
        assert_eq!(rwx(0o750), "rwxr-x---");
        #[cfg(unix)]
        assert_eq!((major(0x803), minor(0x803)), (8, 3));
    }
}
