//! Information screens, shown in the viewer: Info (Alt+I), Avail
//! (`A` in the tree, the mounted file systems) and statistics (`?`).

use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Local};
use nix::unistd::{Gid, Group, Uid, User};

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
    } else if ft.is_fifo() {
        "named pipe (FIFO)"
    } else if ft.is_socket() {
        "socket"
    } else if ft.is_block_device() {
        "block device"
    } else if ft.is_char_device() {
        "character device"
    } else {
        "other"
    };
    let row = |out: &mut String, k: &str, v: String| {
        let _ = writeln!(out, "  {k:<14}{v}");
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
    let mode = meta.mode() & 0o7777;
    row(
        &mut out,
        "Permissions",
        format!("{mode:04o}  {}", rwx(mode)),
    );
    row(&mut out, "Owner", user_name(meta.uid()));
    row(&mut out, "Group", group_name(meta.gid()));
    row(&mut out, "Modified", time(meta.modified().ok()));
    row(&mut out, "Accessed", time(meta.accessed().ok()));
    let changed =
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(meta.ctime().max(0) as u64);
    row(&mut out, "Changed", time(Some(changed)));
    row(&mut out, "Inode", meta.ino().to_string());
    row(&mut out, "Links", meta.nlink().to_string());
    row(
        &mut out,
        "Device",
        format!("{}:{}", major(meta.dev()), minor(meta.dev())),
    );
    out
}

/// The mounted file systems, like `df`.
pub fn avail() -> String {
    let mut out = String::from("AVAIL  mounted file systems\n\n");
    let _ = writeln!(
        out,
        "  {:>15} {:>15} {:>15} {:>5}  {:<10} Mounted on",
        "Total", "Used", "Free", "Use%", "Type"
    );
    let mounts = fs::read_to_string("/proc/self/mounts").unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    for line in mounts.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [_, point, fstype, ..] = fields[..] else {
            continue;
        };
        // "\040" is the space in /proc/mounts names.
        let point = point.replace("\\040", " ");
        let Ok(st) = nix::sys::statvfs::statvfs(Path::new(&point)) else {
            continue;
        };
        let frag = st.fragment_size();
        let total = st.blocks() * frag;
        // Virtual file systems (proc, sysfs, cgroup...) have no size.
        if total == 0 || !seen.insert(point.clone()) {
            continue;
        }
        let free = st.blocks_available() * frag;
        let used = total.saturating_sub(st.blocks_free() * frag);
        let pct = (used * 100).checked_div(used + free).unwrap_or(0);
        let _ = writeln!(
            out,
            "  {:>15} {:>15} {:>15} {:>4}%  {:<10} {point}",
            group(total),
            group(used),
            group(free),
            pct,
            fstype
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

fn user_name(uid: u32) -> String {
    match User::from_uid(Uid::from_raw(uid)) {
        Ok(Some(u)) => format!("{} ({uid})", u.name),
        _ => uid.to_string(),
    }
}

fn group_name(gid: u32) -> String {
    match Group::from_gid(Gid::from_raw(gid)) {
        Ok(Some(g)) => format!("{} ({gid})", g.name),
        _ => gid.to_string(),
    }
}

fn major(dev: u64) -> u64 {
    ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0xfff)
}

fn minor(dev: u64) -> u64 {
    ((dev >> 12) & 0xffff_ff00) | (dev & 0xff)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn avail_lists_the_root_file_system() {
        let text = avail();
        assert!(text.lines().any(|l| l.ends_with(" /")), "{text}");
    }

    #[test]
    fn helpers() {
        assert_eq!(group(1234567), "1,234,567");
        assert_eq!(rwx(0o750), "rwxr-x---");
        assert_eq!((major(0x803), minor(0x803)), (8, 3));
    }
}
