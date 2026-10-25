//! Remote machines over SSH: their files browsed through SFTP, spoken over
//! the system's own `ssh` (so `~/.ssh/config`, keys, the agent and
//! `known_hosts` all apply, and nothing is installed on the remote side).
//!
//! A remote window's tree has a root like `andre@10.0.0.5:/home/andre`; the
//! remote path of anything below it is the remote root plus the components
//! joined with `/`, whatever the local system (as with archive entries).

pub mod askpass;
pub mod sftp;

use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use crate::model::tree::{FileEntry, FileKind, Lister};
use crate::platform;
use askpass::Askpass;
use sftp::{Attrs, Handle, Sftp};

/// A remote machine and where to start, as typed:
/// `ssh://[user@]host[:port][/path]` or `user@host:[path]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub user: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    /// Empty: the home directory.
    pub path: String,
}

impl Address {
    /// `None` for anything that is not a remote address. Plain `host:path`
    /// is not one: the user is required, so local names with a colon stay
    /// local.
    pub fn parse(text: &str) -> Option<Address> {
        let text = text.trim();
        if text.is_empty() || text.chars().any(char::is_whitespace) {
            return None;
        }
        if let Some(rest) = text.strip_prefix("ssh://") {
            let (authority, path) = match rest.find('/') {
                Some(i) => (&rest[..i], &rest[i..]),
                None => (rest, ""),
            };
            let (user, host_port) = match authority.rsplit_once('@') {
                Some((u, h)) => (Some(u), h),
                None => (None, authority),
            };
            let (host, port) = match host_port.rsplit_once(':') {
                Some((h, p)) => (h, Some(p.parse().ok()?)),
                None => (host_port, None),
            };
            return Self::build(user, host, port, path);
        }
        let (user, rest) = text.split_once('@')?;
        let (host, path) = rest.split_once(':')?;
        Self::build(Some(user), host, None, path)
    }

    fn build(user: Option<&str>, host: &str, port: Option<u16>, path: &str) -> Option<Address> {
        // Names only: nothing that ssh could take as an option.
        let valid = |s: &str| {
            !s.is_empty()
                && !s.starts_with('-')
                && s.chars().all(|c| c.is_alphanumeric() || "-._".contains(c))
        };
        if !valid(host) || user.is_some_and(|u| !valid(u)) {
            return None;
        }
        Some(Address {
            user: user.map(String::from),
            host: host.to_string(),
            port,
            path: path.to_string(),
        })
    }

    /// What `ssh` gets: `user@host` or `host`.
    pub fn destination(&self) -> String {
        match &self.user {
            Some(user) => format!("{user}@{}", self.host),
            None => self.host.clone(),
        }
    }

    /// `ssh` with the options every connection uses.
    fn ssh(&self, control: Option<&Path>, master: bool) -> Command {
        let mut cmd = Command::new(ssh_program());
        // A dead network ends ssh, which ends the calls waiting on it.
        cmd.args([
            "-o",
            "ServerAliveInterval=15",
            "-o",
            "ServerAliveCountMax=3",
        ]);
        if let Some(port) = self.port {
            cmd.arg("-p").arg(port.to_string());
        }
        if let Some(control) = control {
            let mode = if master { "auto" } else { "no" };
            cmd.arg("-o").arg(format!("ControlMaster={mode}"));
            cmd.arg("-o")
                .arg(format!("ControlPath={}", control.display()));
        }
        cmd
    }
}

/// The ssh program: `ssh`, or `$XY_SSH` (the tests point it at a wrapper
/// with a configuration of their own, away from the user's `~/.ssh`).
fn ssh_program() -> OsString {
    std::env::var_os("XY_SSH").unwrap_or_else(|| "ssh".into())
}

/// A connected remote machine.
pub struct Remote {
    pub address: Address,
    /// The window's root, as shown: `user@host:/home/user`.
    pub root: PathBuf,
    /// The remote path of the root.
    remote_root: Vec<u8>,
    sftp: Mutex<Sftp<ChildStdout, ChildStdin>>,
    child: Mutex<Child>,
    /// ssh's control socket, so the terminal reuses the login (Unix only).
    control: Option<PathBuf>,
}

type Session = (Sftp<ChildStdout, ChildStdin>, Vec<u8>);

/// A connection being made: ssh may be asking for a password meanwhile.
/// Dropping it before it is made kills ssh (Esc cancels).
pub struct Connecting {
    pub address: Address,
    pub askpass: Askpass,
    child: Option<Child>,
    control: Option<PathBuf>,
    result: Receiver<io::Result<Session>>,
}

impl Connecting {
    /// Starts `ssh` towards `address`; `run` is Xylem's private folder for
    /// this run and `n` numbers the connection.
    pub fn start(address: Address, run: &Path, n: usize) -> io::Result<Connecting> {
        let askpass = Askpass::new(run, n)?;
        // Unix sockets have a short path limit (ssh adds a suffix while
        // creating it); with a long home, the terminal logs in again instead.
        let control =
            Some(run.join(format!("cm-{n}"))).filter(|c| cfg!(unix) && c.as_os_str().len() <= 80);
        let mut cmd = address.ssh(control.as_deref(), true);
        cmd.arg("-s").arg(address.destination()).arg("sftp");
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // ssh asks through Xylem, never on the screen it is drawing.
        cmd.env("SSH_ASKPASS", std::env::current_exe()?)
            .env("SSH_ASKPASS_REQUIRE", "force")
            .env(askpass::DIR_VAR, askpass.dir());
        if std::env::var_os("DISPLAY").is_none() {
            // Older ssh only uses SSH_ASKPASS with a DISPLAY.
            cmd.env("DISPLAY", ":0");
        }
        detach(&mut cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| io::Error::new(e.kind(), format!("cannot run ssh: {e}")))?;
        let pipes = (
            child.stdout.take().expect("piped stdout"),
            child.stdin.take().expect("piped stdin"),
            child.stderr.take().expect("piped stderr"),
        );
        let (tx, rx) = mpsc::channel();
        let path = address.path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(handshake(pipes, &path));
        });
        Ok(Connecting {
            address,
            askpass,
            child: Some(child),
            control,
            result: rx,
        })
    }

    /// The connection, once made (or why it failed).
    pub fn poll(&mut self) -> Option<io::Result<Remote>> {
        let session = match self.result.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                Err(io::Error::other("the connection was interrupted"))
            }
        };
        let mut child = self.child.take()?;
        Some(match session {
            Ok((sftp, remote_root)) => {
                let root = PathBuf::from(format!(
                    "{}:{}",
                    self.address.destination(),
                    String::from_utf8_lossy(&remote_root)
                ));
                Ok(Remote {
                    address: self.address.clone(),
                    root,
                    remote_root,
                    sftp: Mutex::new(sftp),
                    child: Mutex::new(child),
                    control: self.control.take(),
                })
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(e)
            }
        })
    }
}

impl Drop for Connecting {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// On Unix, ssh runs in a session of its own, without the terminal: it can
/// only ask through SSH_ASKPASS.
#[cfg(unix)]
fn detach(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: only setsid() runs between fork and exec, and it is
    // async-signal-safe.
    unsafe {
        cmd.pre_exec(|| nix::unistd::setsid().map(|_| ()).map_err(io::Error::from));
    }
}

#[cfg(not(unix))]
fn detach(_cmd: &mut Command) {}

/// The SFTP session over ssh's pipes, starting at `path` (empty: home).
fn handshake(
    (stdout, stdin, mut stderr): (ChildStdout, ChildStdin, std::process::ChildStderr),
    path: &str,
) -> io::Result<Session> {
    let connected = Sftp::init(stdout, stdin).and_then(|mut sftp| {
        let start = if path.is_empty() { "." } else { path };
        let remote_root = sftp.realpath(start.as_bytes())?;
        if !sftp.stat(&remote_root)?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!(
                    "{} is not a directory",
                    String::from_utf8_lossy(&remote_root)
                ),
            ));
        }
        Ok((sftp, remote_root))
    });
    match connected {
        Ok(session) => {
            // Whatever ssh says from now on (warnings) must not fill the pipe.
            std::thread::spawn(move || io::copy(&mut stderr, &mut io::sink()));
            Ok(session)
        }
        Err(e) => {
            // ssh has ended (the pipe closed); its own words say more
            // ("Permission denied", "Could not resolve hostname").
            let mut said = String::new();
            let _ = stderr.read_to_string(&mut said);
            let last = said.lines().rev().find(|l| !l.trim().is_empty());
            Err(match last {
                Some(line) => io::Error::new(e.kind(), line.trim().to_string()),
                None => e,
            })
        }
    }
}

#[cfg(unix)]
fn os_bytes(s: &OsStr) -> Vec<u8> {
    std::os::unix::ffi::OsStrExt::as_bytes(s).to_vec()
}

#[cfg(not(unix))]
fn os_bytes(s: &OsStr) -> Vec<u8> {
    s.to_string_lossy().into_owned().into_bytes()
}

#[cfg(unix)]
fn bytes_os(b: Vec<u8>) -> OsString {
    std::os::unix::ffi::OsStringExt::from_vec(b)
}

#[cfg(not(unix))]
fn bytes_os(b: Vec<u8>) -> OsString {
    String::from_utf8_lossy(&b).into_owned().into()
}

/// A remote entry as a file of the tree.
fn file_entry(name: Vec<u8>, a: &Attrs) -> FileEntry {
    FileEntry {
        name: bytes_os(name),
        size: a.size.unwrap_or(0),
        modified: a
            .mtime
            .map(|s| SystemTime::UNIX_EPOCH + Duration::from_secs(s.into())),
        mode: platform::from_unix_mode(a.permissions.unwrap_or(0o100644)),
        kind: if a.is_symlink() {
            FileKind::Symlink
        } else if a.is_file() {
            FileKind::File
        } else {
            FileKind::Other
        },
        tagged: false,
    }
}

impl Remote {
    fn sftp(&self) -> MutexGuard<'_, Sftp<ChildStdout, ChildStdin>> {
        self.sftp.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The remote path of `path`, a path of the window's tree.
    pub fn remote_path(&self, path: &Path) -> Vec<u8> {
        let mut out = self.remote_root.clone();
        if let Ok(rel) = path.strip_prefix(&self.root) {
            for c in rel.components() {
                if let Component::Normal(name) = c {
                    if !out.ends_with(b"/") {
                        out.push(b'/');
                    }
                    out.extend(os_bytes(name));
                }
            }
        }
        out
    }

    /// The path in the window's tree for a remote path, if it is inside the
    /// window's root.
    pub fn virtual_path(&self, remote: &str) -> Option<PathBuf> {
        let root = String::from_utf8_lossy(&self.remote_root);
        let rel = remote.strip_prefix(root.as_ref())?;
        if !(rel.is_empty() || rel.starts_with('/') || root.ends_with('/')) {
            return None; // /home/andrex is not inside /home/andre
        }
        let mut path = self.root.clone();
        path.extend(rel.split('/').filter(|c| !c.is_empty()));
        Some(path)
    }

    /// A Windows machine: its paths start with a drive (`/C:/Users`).
    pub fn is_windows(&self) -> bool {
        let r = &self.remote_root;
        r.len() >= 3 && r[0] == b'/' && r[1].is_ascii_alphabetic() && r[2] == b':'
    }

    pub fn lstat(&self, path: &Path) -> io::Result<FileEntry> {
        let attrs = self.sftp().lstat(&self.remote_path(path))?;
        let name = path.file_name().map(os_bytes).unwrap_or_default();
        Ok(file_entry(name, &attrs))
    }

    /// The SFTP attributes of `path` (not following a symlink).
    pub fn attrs(&self, path: &Path) -> io::Result<Attrs> {
        self.sftp().lstat(&self.remote_path(path))
    }

    /// "uid:gid" of `path`: SFTP v3 sends ids, not names.
    pub fn owner(&self, path: &Path) -> Option<String> {
        let a = self.sftp().lstat(&self.remote_path(path)).ok()?;
        Some(format!("{}:{}", a.uid?, a.gid?))
    }

    /// Where a symlink points.
    pub fn read_link(&self, path: &Path) -> io::Result<String> {
        let target = self.sftp().read_link(&self.remote_path(path))?;
        Ok(String::from_utf8_lossy(&target).into_owned())
    }

    /// A file for reading, bit by bit.
    pub fn open(self: &Arc<Self>, path: &Path) -> io::Result<RemoteFile> {
        let handle = self.sftp().open(&self.remote_path(path))?;
        Ok(RemoteFile {
            remote: self.clone(),
            handle: Some(handle),
            offset: 0,
        })
    }

    /// The first `limit` bytes of a file.
    pub fn read(self: &Arc<Self>, path: &Path, limit: u64) -> io::Result<Vec<u8>> {
        let mut data = Vec::new();
        self.open(path)?.take(limit).read_to_end(&mut data)?;
        Ok(data)
    }

    /// The command for a terminal on this machine, starting in `dir` (a
    /// path of the window's tree): `ssh -t`, reusing the login when it can.
    pub fn terminal_command(&self, dir: &Path) -> (OsString, Vec<String>) {
        let mut args: Vec<String> = Vec::new();
        args.extend(
            [
                "-o",
                "ServerAliveInterval=15",
                "-o",
                "ServerAliveCountMax=3",
            ]
            .map(String::from),
        );
        if let Some(port) = self.address.port {
            args.extend(["-p".into(), port.to_string()]);
        }
        if let Some(control) = &self.control {
            args.extend([
                "-o".into(),
                "ControlMaster=no".into(),
                "-o".into(),
                format!("ControlPath={}", control.display()),
            ]);
        }
        args.push("-t".into());
        args.push(self.address.destination());
        // A Unix shell starts in the directory; a Windows one, as it is.
        if !self.is_windows() {
            let dir = String::from_utf8_lossy(&self.remote_path(dir)).replace('\'', "'\\''");
            args.push(format!(
                "cd '{dir}' 2>/dev/null; exec \"${{SHELL:-/bin/sh}}\" -l"
            ));
        }
        (ssh_program(), args)
    }
}

impl Lister for Remote {
    fn list(&self, path: &Path) -> io::Result<(Vec<(OsString, bool)>, Vec<FileEntry>)> {
        let entries = self.sftp().list_dir(&self.remote_path(path))?;
        let (mut dirs, mut files) = (Vec::new(), Vec::new());
        for e in entries {
            if e.attrs.is_dir() {
                let hidden = e.name.first() == Some(&b'.');
                dirs.push((bytes_os(e.name), hidden));
            } else {
                files.push(file_entry(e.name, &e.attrs));
            }
        }
        Ok((dirs, files))
    }
}

impl Drop for Remote {
    fn drop(&mut self) {
        let mut child = self.child.lock().unwrap_or_else(|e| e.into_inner());
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// A remote file being read.
pub struct RemoteFile {
    remote: Arc<Remote>,
    handle: Option<Handle>,
    offset: u64,
}

impl Read for RemoteFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(handle) = &self.handle else {
            return Ok(0);
        };
        let want = buf.len().min(sftp::READ_CHUNK as usize) as u32;
        let data = self.remote.sftp().read(handle, self.offset, want)?;
        buf[..data.len()].copy_from_slice(&data);
        self.offset += data.len() as u64;
        Ok(data.len())
    }
}

impl Drop for RemoteFile {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = self.remote.sftp().close_file(&handle);
        }
    }
}

/// For tests: a "remote machine" that is OpenSSH's sftp-server spoken to
/// directly (no ssh, no network), rooted at `dir`. `None` without it.
#[cfg(all(test, unix))]
pub fn for_tests(dir: &Path) -> Option<Arc<Remote>> {
    let (child, mut sftp) = sftp::tests::server()?;
    let dir = os_bytes(dir.as_os_str());
    let remote_root = sftp.realpath(&dir).ok()?;
    let root = PathBuf::from(format!(
        "test@testhost:{}",
        String::from_utf8_lossy(&remote_root)
    ));
    Some(Arc::new(Remote {
        address: Address::parse("test@testhost:").unwrap(),
        root,
        remote_root,
        sftp: Mutex::new(sftp),
        child: Mutex::new(child),
        control: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_paths_both_ways() {
        let tmp = crate::testing::tempdir();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        let Some(remote) = for_tests(tmp.path()) else {
            return;
        };
        let inside = remote.root.join("sub").join("a b.txt");
        let real = tmp.path().canonicalize().unwrap();
        assert_eq!(
            remote.remote_path(&inside),
            format!("{}/sub/a b.txt", real.display()).into_bytes()
        );
        let back = remote.virtual_path(&format!("{}/sub", real.display()));
        assert_eq!(back, Some(remote.root.join("sub")));
        assert_eq!(remote.virtual_path("/etc"), None);
        // A sibling whose name starts the same is not inside.
        assert_eq!(remote.virtual_path(&format!("{}x", real.display())), None);
        assert!(!remote.is_windows());
    }

    #[test]
    fn parses_addresses() {
        let a = Address::parse("andre@10.0.0.5:/srv/www").unwrap();
        assert_eq!(a.destination(), "andre@10.0.0.5");
        assert_eq!((a.port, a.path.as_str()), (None, "/srv/www"));
        let a = Address::parse("andre@host:").unwrap();
        assert_eq!(a.path, "", "the home directory");
        let a = Address::parse("ssh://andre@host.lan:2222/tmp").unwrap();
        assert_eq!((a.port, a.path.as_str()), (Some(2222), "/tmp"));
        let a = Address::parse("ssh://myserver").unwrap();
        assert_eq!(
            (a.destination().as_str(), a.path.as_str()),
            ("myserver", "")
        );
        // Local paths stay local.
        for local in [
            "/home/a",
            "notes:2026.txt",
            "C:\\Users",
            "~/x",
            "a b@c:d",
            "@host:x",
        ] {
            assert_eq!(Address::parse(local), None, "{local}");
        }
        // Nothing ssh would read as an option.
        assert_eq!(Address::parse("ssh://-oProxyCommand=x"), None);
        assert_eq!(Address::parse("-oX@host:/"), None);
    }
}
