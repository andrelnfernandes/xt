//! A small SFTP (version 3) client: what Xylem needs to browse and read a
//! remote machine. It talks to any byte stream; in use, the stdin and stdout
//! of `ssh -s host sftp`.
//!
//! The protocol is draft-ietf-secsh-filexfer-02, the version every OpenSSH
//! server speaks: packets of `length (u32) type (u8) payload`, requests with
//! an id, big-endian integers, strings as `length (u32) bytes`. Paths and
//! names are bytes, as the server has them.

use std::io::{self, Read, Write};

const INIT: u8 = 1;
const VERSION: u8 = 2;
const OPEN: u8 = 3;
const CLOSE: u8 = 4;
const READ: u8 = 5;
const LSTAT: u8 = 7;
const OPENDIR: u8 = 11;
const READDIR: u8 = 12;
const REALPATH: u8 = 16;
const STAT: u8 = 17;
const READLINK: u8 = 19;
const STATUS: u8 = 101;
const HANDLE: u8 = 102;
const DATA: u8 = 103;
const NAME: u8 = 104;
const ATTRS: u8 = 105;

const FX_EOF: u32 = 1;
const FX_NO_SUCH_FILE: u32 = 2;
const FX_PERMISSION_DENIED: u32 = 3;

const ATTR_SIZE: u32 = 0x1;
const ATTR_UIDGID: u32 = 0x2;
const ATTR_PERMISSIONS: u32 = 0x4;
const ATTR_ACMODTIME: u32 = 0x8;
const ATTR_EXTENDED: u32 = 0x8000_0000;

const OPEN_READ: u32 = 0x1;

/// Largest packet accepted from the server (data replies are far smaller).
const MAX_PACKET: usize = 1024 * 1024;

/// What a READ asks for at most: every server accepts 32 KiB.
pub const READ_CHUNK: u32 = 32 * 1024;

/// File attributes; each one only if the server sent it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Attrs {
    pub size: Option<u64>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    /// Type and permission bits, as in `st_mode`.
    pub permissions: Option<u32>,
    /// Seconds since the epoch.
    pub mtime: Option<u32>,
}

impl Attrs {
    fn file_type(&self) -> Option<u32> {
        self.permissions.map(|p| p & 0o170000)
    }

    pub fn is_dir(&self) -> bool {
        self.file_type() == Some(0o040000)
    }

    pub fn is_symlink(&self) -> bool {
        self.file_type() == Some(0o120000)
    }

    pub fn is_file(&self) -> bool {
        // A server that sends no type: taken as a file.
        matches!(self.file_type(), Some(0o100000) | None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: Vec<u8>,
    pub attrs: Attrs,
}

/// An open file or directory on the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handle(Vec<u8>);

pub struct Sftp<R, W> {
    reader: R,
    writer: W,
    next_id: u32,
}

// ---------------------------------------------------------------- encoding

struct Out(Vec<u8>);

impl Out {
    fn new(kind: u8, id: Option<u32>) -> Self {
        let mut out = Out(vec![0, 0, 0, 0, kind]);
        if let Some(id) = id {
            out.u32(id);
        }
        out
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend(v.to_be_bytes());
        self
    }
    fn u64(&mut self, v: u64) -> &mut Self {
        self.0.extend(v.to_be_bytes());
        self
    }
    fn string(&mut self, s: &[u8]) -> &mut Self {
        self.u32(s.len() as u32);
        self.0.extend(s);
        self
    }
    fn finish(mut self) -> Vec<u8> {
        let len = (self.0.len() - 4) as u32;
        self.0[..4].copy_from_slice(&len.to_be_bytes());
        self.0
    }
}

struct In<'a>(&'a [u8]);

fn malformed() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "malformed SFTP reply")
}

impl In<'_> {
    fn take(&mut self, n: usize) -> io::Result<&[u8]> {
        if self.0.len() < n {
            return Err(malformed());
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn string(&mut self) -> io::Result<Vec<u8>> {
        let len = self.u32()? as usize;
        Ok(self.take(len)?.to_vec())
    }
    fn attrs(&mut self) -> io::Result<Attrs> {
        let flags = self.u32()?;
        let mut a = Attrs::default();
        if flags & ATTR_SIZE != 0 {
            a.size = Some(self.u64()?);
        }
        if flags & ATTR_UIDGID != 0 {
            a.uid = Some(self.u32()?);
            a.gid = Some(self.u32()?);
        }
        if flags & ATTR_PERMISSIONS != 0 {
            a.permissions = Some(self.u32()?);
        }
        if flags & ATTR_ACMODTIME != 0 {
            let _atime = self.u32()?;
            a.mtime = Some(self.u32()?);
        }
        if flags & ATTR_EXTENDED != 0 {
            for _ in 0..self.u32()? {
                self.string()?;
                self.string()?;
            }
        }
        Ok(a)
    }
}

/// A STATUS reply as an error (or `Ok` for SSH_FX_OK).
fn status(payload: &mut In) -> io::Result<u32> {
    let code = payload.u32()?;
    let message = String::from_utf8_lossy(&payload.string().unwrap_or_default()).into_owned();
    let kind = match code {
        0 | FX_EOF => return Ok(code),
        FX_NO_SUCH_FILE => io::ErrorKind::NotFound,
        FX_PERMISSION_DENIED => io::ErrorKind::PermissionDenied,
        _ => io::ErrorKind::Other,
    };
    let message = if message.is_empty() {
        format!("SFTP error {code}")
    } else {
        message
    };
    Err(io::Error::new(kind, message))
}

// ---------------------------------------------------------------- client

impl<R: Read, W: Write> Sftp<R, W> {
    /// Starts the session (INIT / VERSION).
    pub fn init(reader: R, writer: W) -> io::Result<Self> {
        let mut sftp = Sftp {
            reader,
            writer,
            next_id: 1,
        };
        let mut out = Out::new(INIT, None);
        out.u32(3);
        sftp.writer.write_all(&out.finish())?;
        sftp.writer.flush()?;
        let (kind, _) = sftp.read_packet()?;
        if kind != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the server does not speak SFTP",
            ));
        }
        Ok(sftp)
    }

    fn read_packet(&mut self) -> io::Result<(u8, Vec<u8>)> {
        let mut len = [0u8; 4];
        self.reader.read_exact(&mut len).map_err(|e| {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "the connection was closed",
                )
            } else {
                e
            }
        })?;
        let len = u32::from_be_bytes(len) as usize;
        if len == 0 || len > MAX_PACKET {
            return Err(malformed());
        }
        let mut body = vec![0u8; len];
        self.reader.read_exact(&mut body)?;
        let kind = body[0];
        body.remove(0);
        Ok((kind, body))
    }

    /// Sends a request built by `fill` and returns the reply's type and
    /// payload (after the id).
    fn request(&mut self, kind: u8, fill: impl FnOnce(&mut Out)) -> io::Result<(u8, Vec<u8>)> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let mut out = Out::new(kind, Some(id));
        fill(&mut out);
        self.writer.write_all(&out.finish())?;
        self.writer.flush()?;
        let (reply, body) = self.read_packet()?;
        let mut payload = In(&body);
        if payload.u32()? != id {
            return Err(malformed());
        }
        Ok((reply, payload.0.to_vec()))
    }

    /// A request answered with STATUS (OK) or `expected`.
    fn expect(
        &mut self,
        kind: u8,
        expected: u8,
        fill: impl FnOnce(&mut Out),
    ) -> io::Result<Vec<u8>> {
        let (reply, body) = self.request(kind, fill)?;
        match reply {
            r if r == expected => Ok(body),
            STATUS => {
                status(&mut In(&body))?;
                Err(io::Error::other("unexpected SFTP status"))
            }
            _ => Err(malformed()),
        }
    }

    /// The absolute, canonical form of `path` ("." is the home directory).
    pub fn realpath(&mut self, path: &[u8]) -> io::Result<Vec<u8>> {
        let body = self.expect(REALPATH, NAME, |o| {
            o.string(path);
        })?;
        let mut p = In(&body);
        if p.u32()? == 0 {
            return Err(malformed());
        }
        p.string()
    }

    pub fn stat(&mut self, path: &[u8]) -> io::Result<Attrs> {
        let body = self.expect(STAT, ATTRS, |o| {
            o.string(path);
        })?;
        In(&body).attrs()
    }

    /// Like `stat`, without following a final symlink.
    pub fn lstat(&mut self, path: &[u8]) -> io::Result<Attrs> {
        let body = self.expect(LSTAT, ATTRS, |o| {
            o.string(path);
        })?;
        In(&body).attrs()
    }

    pub fn read_link(&mut self, path: &[u8]) -> io::Result<Vec<u8>> {
        let body = self.expect(READLINK, NAME, |o| {
            o.string(path);
        })?;
        let mut p = In(&body);
        if p.u32()? == 0 {
            return Err(malformed());
        }
        p.string()
    }

    fn close(&mut self, handle: &Handle) -> io::Result<()> {
        let (reply, body) = self.request(CLOSE, |o| {
            o.string(&handle.0);
        })?;
        match reply {
            STATUS => status(&mut In(&body)).map(|_| ()),
            _ => Err(malformed()),
        }
    }

    /// The entries of directory `path`, without "." and "..". Attributes
    /// are those of the entries themselves (symlinks are not followed).
    pub fn list_dir(&mut self, path: &[u8]) -> io::Result<Vec<DirEntry>> {
        let body = self.expect(OPENDIR, HANDLE, |o| {
            o.string(path);
        })?;
        let handle = Handle(In(&body).string()?);
        let mut entries = Vec::new();
        let result = loop {
            let (reply, body) = match self.request(READDIR, |o| {
                o.string(&handle.0);
            }) {
                Ok(r) => r,
                Err(e) => break Err(e),
            };
            let mut p = In(&body);
            match reply {
                NAME => {
                    let count = p.u32()?;
                    for _ in 0..count {
                        let name = p.string()?;
                        let _long = p.string()?;
                        let attrs = p.attrs()?;
                        if name != b"." && name != b".." {
                            entries.push(DirEntry { name, attrs });
                        }
                    }
                }
                STATUS => match status(&mut p) {
                    Ok(_) => break Ok(()),
                    Err(e) => break Err(e),
                },
                _ => break Err(malformed()),
            }
        };
        let closed = self.close(&handle);
        result?;
        closed?;
        Ok(entries)
    }

    pub fn open(&mut self, path: &[u8]) -> io::Result<Handle> {
        let body = self.expect(OPEN, HANDLE, |o| {
            o.string(path).u32(OPEN_READ).u32(0);
        })?;
        Ok(Handle(In(&body).string()?))
    }

    /// Up to `len` bytes from `offset`; empty at the end of the file.
    pub fn read(&mut self, handle: &Handle, offset: u64, len: u32) -> io::Result<Vec<u8>> {
        let (reply, body) = self.request(READ, |o| {
            o.string(&handle.0).u64(offset).u32(len.min(READ_CHUNK));
        })?;
        let mut p = In(&body);
        match reply {
            DATA => p.string(),
            STATUS => status(&mut p).map(|_| Vec::new()),
            _ => Err(malformed()),
        }
    }

    pub fn close_file(&mut self, handle: &Handle) -> io::Result<()> {
        self.close(handle)
    }
}

#[cfg(all(test, unix))]
pub mod tests {
    use super::*;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;
    use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

    /// OpenSSH's own SFTP server, spoken to over stdio (no network).
    pub fn server() -> Option<(Child, Sftp<ChildStdout, ChildStdin>)> {
        let program = [
            "/usr/lib/openssh/sftp-server",
            "/usr/libexec/openssh/sftp-server",
            "/usr/lib/ssh/sftp-server",
            "/usr/libexec/sftp-server",
        ]
        .into_iter()
        .find(|p| Path::new(p).exists())?;
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let (stdout, stdin) = (child.stdout.take()?, child.stdin.take()?);
        Some((child, Sftp::init(stdout, stdin).expect("SFTP handshake")))
    }

    fn bytes(p: &Path) -> &[u8] {
        p.as_os_str().as_bytes()
    }

    #[test]
    fn lists_reads_and_reports_errors_against_openssh() {
        let Some((mut child, mut sftp)) = server() else {
            eprintln!("sftp-server not installed: skipped");
            return;
        };
        let tmp = crate::testing::tempdir();
        let dir = tmp.path();
        std::fs::create_dir(dir.join("sub")).unwrap();
        let big: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(dir.join("big.bin"), &big).unwrap();
        std::os::unix::fs::symlink("big.bin", dir.join("link")).unwrap();

        let real = sftp.realpath(bytes(dir)).unwrap();
        assert_eq!(real, bytes(&dir.canonicalize().unwrap()));

        let mut entries = sftp.list_dir(bytes(dir)).unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let names: Vec<&[u8]> = entries.iter().map(|e| e.name.as_slice()).collect();
        assert_eq!(names, [&b"big.bin"[..], b"link", b"sub"]);
        assert_eq!(entries[0].attrs.size, Some(100_000));
        assert!(entries[0].attrs.is_file() && entries[0].attrs.mtime.is_some());
        assert!(entries[1].attrs.is_symlink());
        assert!(entries[2].attrs.is_dir());
        assert_eq!(
            sftp.read_link(bytes(&dir.join("link"))).unwrap(),
            b"big.bin"
        );
        assert!(sftp.stat(bytes(&dir.join("link"))).unwrap().is_file());

        // Reading in chunks gives the file back.
        let handle = sftp.open(bytes(&dir.join("big.bin"))).unwrap();
        let mut data = Vec::new();
        loop {
            let chunk = sftp.read(&handle, data.len() as u64, READ_CHUNK).unwrap();
            if chunk.is_empty() {
                break;
            }
            data.extend(chunk);
        }
        sftp.close_file(&handle).unwrap();
        assert_eq!(data, big);

        let err = sftp.list_dir(bytes(&dir.join("nope"))).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        let err = sftp.open(bytes(&dir.join("nope"))).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        let _ = child.kill();
    }

    #[test]
    fn a_closed_connection_is_an_error_not_a_hang() {
        let Some((mut child, mut sftp)) = server() else {
            return;
        };
        child.kill().unwrap();
        let _ = child.wait();
        let err = sftp.realpath(b".").unwrap_err();
        assert!(
            matches!(
                err.kind(),
                io::ErrorKind::ConnectionAborted | io::ErrorKind::BrokenPipe
            ),
            "{err:?}"
        );
    }
}
