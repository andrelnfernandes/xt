//! Reading of archives (ZIP, 7z and TAR, plain or compressed with gzip, xz,
//! bzip2 or zstd), read-only; creating ZIP and TAR.
//!
//! The entries become a virtual tree whose root is the archive itself:
//! `/x/package.zip/src/a.rs` is the entry `src/a.rs` of `/x/package.zip`.
//!
//! Security: entries with `..` or an absolute path are ignored (otherwise they
//! could be extracted outside the destination), and symlinks are never
//! extracted (a link to `/etc` followed by `link/passwd` would write
//! outside the destination).

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::time::{Duration, SystemTime};

use chrono::{Local, NaiveDate, TimeZone};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Zip,
    SevenZ,
    Tar,
    TarGz,
    TarXz,
    TarBz2,
    TarZst,
}

impl Kind {
    /// Recognizes it by the file name.
    pub fn detect(path: &Path) -> Option<Kind> {
        let name = path.file_name()?.to_string_lossy().to_lowercase();
        let ends = |exts: &[&str]| exts.iter().any(|ext| name.ends_with(ext));
        if ends(&[".tar.gz", ".tgz"]) {
            Some(Kind::TarGz)
        } else if ends(&[".tar.xz", ".txz"]) {
            Some(Kind::TarXz)
        } else if ends(&[".tar.bz2", ".tbz2", ".tbz"]) {
            Some(Kind::TarBz2)
        } else if ends(&[".tar.zst", ".tzst"]) {
            Some(Kind::TarZst)
        } else if name.ends_with(".7z") {
            Some(Kind::SevenZ)
        } else if name.ends_with(".tar") {
            Some(Kind::Tar)
        } else if [".zip", ".jar", ".war", ".apk"]
            .iter()
            .any(|ext| name.ends_with(ext))
        {
            Some(Kind::Zip)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
    Other,
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// Relative path, already validated (normal components only).
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub mode: u32,
    pub kind: EntryKind,
}

/// An open archive, to list and extract entries.
pub struct Reader {
    path: PathBuf,
    kind: Kind,
    zip: Option<zip::ZipArchive<BufReader<File>>>,
    /// Path → index in the ZIP.
    zip_index: HashMap<PathBuf, usize>,
    sevenz: Option<sevenz_rust2::ArchiveReader<File>>,
    /// Path → name of the entry in the 7z, and its kind.
    sevenz_index: HashMap<PathBuf, (String, EntryKind)>,
}

impl Reader {
    pub fn open(path: &Path) -> io::Result<Self> {
        let kind = Kind::detect(path).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "not a supported archive")
        })?;
        let mut reader = Reader {
            path: path.to_path_buf(),
            kind,
            zip: None,
            zip_index: HashMap::new(),
            sevenz: None,
            sevenz_index: HashMap::new(),
        };
        match kind {
            Kind::Zip => {
                let file = BufReader::new(File::open(path)?);
                reader.zip = Some(zip::ZipArchive::new(file).map_err(io::Error::other)?);
            }
            Kind::SevenZ => {
                let file = File::open(path)?;
                let password = sevenz_rust2::Password::empty();
                let sevenz =
                    sevenz_rust2::ArchiveReader::new(file, password).map_err(sevenz_error)?;
                reader.sevenz = Some(sevenz);
            }
            _ => {}
        }
        Ok(reader)
    }

    /// All valid entries of the archive.
    pub fn list(&mut self) -> io::Result<Vec<Entry>> {
        match self.kind {
            Kind::Zip => self.list_zip(),
            Kind::SevenZ => Ok(self.list_7z()),
            _ => {
                let mut entries = Vec::new();
                self.each_tar_entry(|entry, _| {
                    entries.push(entry);
                    Ok(true)
                })?;
                Ok(entries)
            }
        }
    }

    /// Writes the contents of `entry` to `out`, up to `limit` bytes.
    /// Returns how many bytes were written.
    ///
    /// In a TAR or a solid 7z the entries before this one are decompressed
    /// too. With a `limit` (a preview), that stops at [`MAX_SKIP`] bytes:
    /// a huge entry at the start would hold the screen up. Between blocks,
    /// `out.flush()` failing stops it as well (see [`Streamed`]).
    pub fn extract(&mut self, entry: &Path, out: &mut dyn Write, limit: u64) -> io::Result<u64> {
        let mut skipped = 0u64;
        let mut skip_past = |size: u64| {
            skipped = skipped.saturating_add(size);
            if limit != u64::MAX && skipped > MAX_SKIP {
                return Err(io::Error::other(
                    "too deep inside the archive to preview; copy it out to see it",
                ));
            }
            Ok(())
        };
        match self.kind {
            Kind::Zip => {
                if self.zip_index.is_empty() {
                    self.list_zip()?;
                }
                let index = *self.zip_index.get(entry).ok_or_else(not_found)?;
                let zip = self.zip.as_mut().expect("zip archive is open");
                let mut file = zip.by_index(index).map_err(io::Error::other)?;
                if file.is_symlink() {
                    return Err(symlink_error());
                }
                io::copy(&mut (&mut file).take(limit), out)
            }
            Kind::SevenZ => {
                if self.sevenz_index.is_empty() {
                    self.list_7z();
                }
                let (name, kind) = self.sevenz_index.get(entry).ok_or_else(not_found)?.clone();
                if kind == EntryKind::Symlink {
                    return Err(symlink_error());
                }
                let sevenz = self.sevenz.as_mut().expect("7z archive is open");
                let mut written = None;
                // Solid archives only decode in order: walks up to the entry,
                // reading through the ones before it (in a solid block they
                // share one stream, so an unread entry would shift the next).
                sevenz
                    .for_each_entries(|info, data| {
                        if info.name() != name {
                            skip_past(info.size())?;
                            read_through(data, out)?;
                            return Ok(true);
                        }
                        written = Some(io::copy(&mut data.take(limit), out)?);
                        Ok(false)
                    })
                    .map_err(sevenz_error)?;
                written.ok_or_else(not_found)
            }
            _ => {
                let mut written = None;
                self.each_tar_entry(|info, data| {
                    if info.path != entry {
                        skip_past(info.size)?;
                        read_through(data, out)?;
                        return Ok(true);
                    }
                    if info.kind == EntryKind::Symlink {
                        return Err(symlink_error());
                    }
                    written = Some(io::copy(&mut data.take(limit), out)?);
                    Ok(false)
                })?;
                written.ok_or_else(not_found)
            }
        }
    }

    /// Extracts entries in a thread of its own, one after the other.
    pub fn into_extractor(mut self) -> Extractor {
        let (requests, queue) = mpsc::channel::<(PathBuf, Pipe)>();
        std::thread::spawn(move || {
            for (entry, mut pipe) in queue {
                let result = self.extract(&entry, &mut pipe, u64::MAX);
                pipe.finish(result);
            }
        });
        Extractor { requests }
    }

    fn list_zip(&mut self) -> io::Result<Vec<Entry>> {
        let zip = self.zip.as_mut().expect("zip archive is open");
        let mut entries = Vec::new();
        self.zip_index.clear();
        for i in 0..zip.len() {
            let file = zip.by_index_raw(i).map_err(io::Error::other)?;
            // `enclosed_name` already rejects ".." and absolute paths.
            let Some(path) = file.enclosed_name().filter(|p| is_safe(p)) else {
                continue;
            };
            let kind = if file.is_dir() {
                EntryKind::Dir
            } else if file.is_symlink() {
                EntryKind::Symlink
            } else {
                EntryKind::File
            };
            let modified = file.last_modified().and_then(|t| {
                let date =
                    NaiveDate::from_ymd_opt(t.year().into(), t.month().into(), t.day().into())?;
                let time =
                    date.and_hms_opt(t.hour().into(), t.minute().into(), t.second().into())?;
                Local
                    .from_local_datetime(&time)
                    .single()
                    .map(SystemTime::from)
            });
            self.zip_index.insert(path.clone(), i);
            entries.push(Entry {
                path,
                size: file.size(),
                modified,
                mode: file.unix_mode().unwrap_or(0o644) & 0o7777,
                kind,
            });
        }
        Ok(entries)
    }

    fn list_7z(&mut self) -> Vec<Entry> {
        let sevenz = self.sevenz.as_ref().expect("7z archive is open");
        let mut entries = Vec::new();
        self.sevenz_index.clear();
        for file in &sevenz.archive().files {
            if file.is_anti_item() {
                continue;
            }
            // Archives made on Windows may use `\`.
            let path: PathBuf = Path::new(&file.name().replace('\\', "/"))
                .components()
                .filter(|c| *c != Component::CurDir)
                .collect();
            if path.as_os_str().is_empty() || !is_safe(&path) {
                continue;
            }
            // Unix permissions, when the archive keeps them, are in the high
            // 16 bits of the attributes (7-Zip's convention, flag 0x8000).
            let unix = (file.has_windows_attributes && file.windows_attributes & 0x8000 != 0)
                .then_some(file.windows_attributes >> 16);
            let kind = if file.is_directory() {
                EntryKind::Dir
            } else if unix.is_some_and(|m| m & 0o170000 == 0o120000) {
                EntryKind::Symlink
            } else {
                EntryKind::File
            };
            self.sevenz_index
                .insert(path.clone(), (file.name().to_string(), kind));
            entries.push(Entry {
                path,
                size: file.size(),
                modified: file
                    .has_last_modified_date
                    .then(|| file.last_modified_date().into()),
                mode: unix.map_or(0o644, |m| m & 0o7777),
                kind,
            });
        }
        entries
    }

    /// Walks the TAR from the start, calling `f` with each valid entry and its
    /// contents; `f` returns `false` to stop.
    fn each_tar_entry(
        &self,
        mut f: impl FnMut(Entry, &mut dyn Read) -> io::Result<bool>,
    ) -> io::Result<()> {
        let file = BufReader::new(File::open(&self.path)?);
        let reader: Box<dyn Read> = match self.kind {
            Kind::TarGz => Box::new(flate2::read::GzDecoder::new(file)),
            Kind::TarXz => Box::new(crate::decompress::xz_reader(
                file,
                crate::decompress::VIEW_DICT,
            )),
            Kind::TarBz2 => Box::new(bzip2::read::MultiBzDecoder::new(file)),
            Kind::TarZst => Box::new(crate::decompress::zstd_reader(
                file,
                crate::decompress::VIEW_DICT,
            )?),
            _ => Box::new(file),
        };
        let mut archive = tar::Archive::new(reader);
        for item in archive.entries()? {
            let mut item = item?;
            let header = item.header();
            let Ok(path) = item.path().map(|p| p.into_owned()) else {
                continue;
            };
            let path: PathBuf = path
                .components()
                .filter(|c| *c != Component::CurDir)
                .collect();
            if path.as_os_str().is_empty() || !is_safe(&path) {
                continue;
            }
            let kind = match header.entry_type() {
                tar::EntryType::Regular | tar::EntryType::Continuous => EntryKind::File,
                tar::EntryType::Directory => EntryKind::Dir,
                tar::EntryType::Symlink => EntryKind::Symlink,
                _ => EntryKind::Other,
            };
            let entry = Entry {
                path,
                size: header.size().unwrap_or(0),
                modified: header
                    .mtime()
                    .ok()
                    .map(|s| SystemTime::UNIX_EPOCH + Duration::from_secs(s)),
                mode: header.mode().unwrap_or(0o644) & 0o7777,
                kind,
            };
            if !f(entry, &mut item)? {
                break;
            }
        }
        Ok(())
    }
}

/// Writer of the archive being created.
enum Writer {
    Zip(Box<zip::ZipWriter<File>>),
    Tar(tar::Builder<File>),
    TarGz(Box<tar::Builder<flate2::write::GzEncoder<File>>>),
}

/// Name inside an archive for a relative path: always with `/`, as ZIP and
/// TAR require (on Windows, `Path` uses `\`).
pub fn entry_name(relative: &Path) -> String {
    let parts: Vec<_> = relative.iter().map(|c| c.to_string_lossy()).collect();
    parts.join("/")
}

/// How much of the entries before the wanted one a preview decompresses
/// before giving up.
pub const MAX_SKIP: u64 = if cfg!(test) {
    1024 * 1024
} else {
    64 * 1024 * 1024
};

/// Reads an entry that comes before the wanted one, checking between
/// blocks that someone still wants the result.
fn read_through(data: &mut dyn Read, out: &mut dyn Write) -> io::Result<()> {
    let mut buf = vec![0; 64 * 1024];
    while data.read(&mut buf)? > 0 {
        out.flush()?;
    }
    Ok(())
}

/// Size of the blocks sent from the extracting thread.
const BLOCK: usize = 256 * 1024;

/// An archive being extracted in another thread ([`Reader::into_extractor`]).
pub struct Extractor {
    requests: mpsc::Sender<(PathBuf, Pipe)>,
}

impl Extractor {
    /// The contents of `entry`, as the thread extracts them (after the
    /// entries asked for before).
    pub fn extract(&self, entry: &Path) -> Streamed {
        let (tx, rx) = mpsc::sync_channel(4);
        let gone = Arc::new(AtomicBool::new(false));
        let pipe = Pipe {
            tx,
            block: Vec::new(),
            gone: gone.clone(),
        };
        if let Err(mpsc::SendError((_, pipe))) = self.requests.send((entry.to_path_buf(), pipe)) {
            pipe.finish(Err(io::Error::other("extraction stopped")));
        }
        Streamed {
            rx,
            block: Vec::new(),
            pos: 0,
            gone,
            wait: None,
        }
    }
}

/// Extracts `entry` of `archive` in another thread, for reading as a
/// stream: a big entry (or a decompression bomb) never sits whole in memory.
pub fn extract_streamed(archive: &Path, entry: &Path) -> io::Result<Streamed> {
    Ok(Reader::open(archive)?.into_extractor().extract(entry))
}

/// The extracting thread's end: sends what is written in blocks, and fails
/// once nobody reads (the copy was cancelled), which stops the extraction.
struct Pipe {
    tx: SyncSender<io::Result<Vec<u8>>>,
    block: Vec<u8>,
    gone: Arc<AtomicBool>,
}

impl Pipe {
    fn finish(mut self, result: io::Result<u64>) {
        let last = match result {
            Ok(_) => Ok(std::mem::take(&mut self.block)),
            Err(e) => Err(e),
        };
        let _ = self.tx.send(last);
    }
}

impl Write for Pipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.flush()?;
        self.block.extend_from_slice(buf);
        if self.block.len() >= BLOCK {
            let block = std::mem::take(&mut self.block);
            self.tx
                .send(Ok(block))
                .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))?;
        }
        Ok(buf.len())
    }

    /// Only tells whether anyone is still reading.
    fn flush(&mut self) -> io::Result<()> {
        if self.gone.load(Ordering::Relaxed) {
            return Err(io::Error::from(io::ErrorKind::BrokenPipe));
        }
        Ok(())
    }
}

/// An entry's contents as another thread extracts them. Dropping it stops
/// the extraction.
pub struct Streamed {
    rx: Receiver<io::Result<Vec<u8>>>,
    block: Vec<u8>,
    pos: usize,
    gone: Arc<AtomicBool>,
    wait: Option<Duration>,
}

impl Streamed {
    /// Instead of waiting for the next block, gives up after `wait` with
    /// `WouldBlock`: a copy in steps keeps the screen responsive.
    pub fn waiting_at_most(mut self, wait: Duration) -> Self {
        self.wait = Some(wait);
        self
    }
}

impl Read for Streamed {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.pos == self.block.len() {
            let next = match self.wait {
                None => self.rx.recv().ok(),
                Some(wait) => match self.rx.recv_timeout(wait) {
                    Ok(next) => Some(next),
                    Err(RecvTimeoutError::Timeout) => {
                        return Err(io::Error::from(io::ErrorKind::WouldBlock));
                    }
                    Err(RecvTimeoutError::Disconnected) => None,
                },
            };
            match next {
                Some(Ok(block)) if block.is_empty() => return Ok(0),
                Some(Ok(block)) => (self.block, self.pos) = (block, 0),
                Some(Err(e)) => return Err(e),
                None => return Ok(0),
            }
        }
        let n = buf.len().min(self.block.len() - self.pos);
        buf[..n].copy_from_slice(&self.block[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

impl Drop for Streamed {
    fn drop(&mut self) {
        self.gone.store(true, Ordering::Relaxed);
    }
}

/// A file to put in the archive: the path on disk and the name inside it.
pub struct ArchiveItem {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

/// Creation of a ZIP/TAR (F5), one file at a time, in a temporary file that only
/// gets its final name at the end: cancelling or failing does not leave a
/// half-written archive.
pub struct ArchiveJob {
    writer: Option<Writer>,
    items: Vec<ArchiveItem>,
    next: usize,
    pub dest: PathBuf,
    tmp: PathBuf,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub errors: Vec<String>,
}

impl ArchiveJob {
    pub fn new(dest: &Path, items: Vec<ArchiveItem>) -> io::Result<Self> {
        // Reads more formats than it creates.
        let creatable = |k: &Kind| matches!(k, Kind::Zip | Kind::Tar | Kind::TarGz);
        let kind = Kind::detect(dest).filter(creatable).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "use a name ending in .zip, .tar, .tar.gz or .tgz",
            )
        })?;
        let (file, tmp) = crate::platform::create_tmp(dest)?;
        let writer = match kind {
            Kind::Zip => Writer::Zip(Box::new(zip::ZipWriter::new(file))),
            Kind::Tar => Writer::Tar(no_follow(tar::Builder::new(file))),
            Kind::TarGz => Writer::TarGz(Box::new(no_follow(tar::Builder::new(
                flate2::write::GzEncoder::new(file, flate2::Compression::default()),
            )))),
            _ => unreachable!("filtered by `creatable`"),
        };
        Ok(ArchiveJob {
            writer: Some(writer),
            total_bytes: items.iter().map(|i| i.size).sum(),
            items,
            next: 0,
            dest: dest.to_path_buf(),
            tmp,
            done_bytes: 0,
            errors: Vec::new(),
        })
    }

    pub fn total(&self) -> usize {
        self.items.len()
    }

    pub fn done(&self) -> usize {
        self.next
    }

    pub fn current(&self) -> Option<&str> {
        self.items.get(self.next).map(|i| i.name.as_str())
    }

    pub fn is_done(&self) -> bool {
        self.next >= self.items.len()
    }

    /// Adds files until `budget` is exceeded.
    pub fn step(&mut self, budget: std::time::Duration) {
        let start = std::time::Instant::now();
        while !self.is_done() {
            let item = &self.items[self.next];
            let writer = self.writer.as_mut().expect("writer open until finish");
            if let Err(e) = add(writer, item) {
                self.errors.push(format!("{}: {e}", item.path.display()));
            }
            self.done_bytes += item.size;
            self.next += 1;
            if start.elapsed() >= budget {
                break;
            }
        }
    }

    /// Closes the archive and puts it in place.
    pub fn finish(mut self) -> io::Result<PathBuf> {
        let result = match self.writer.take().expect("not finished yet") {
            Writer::Zip(z) => z.finish().map(drop).map_err(io::Error::other),
            Writer::Tar(t) => t.into_inner().map(drop),
            Writer::TarGz(t) => t.into_inner()?.finish().map(drop),
        };
        match result.and_then(|()| std::fs::rename(&self.tmp, &self.dest)) {
            Ok(()) => Ok(self.dest.clone()),
            Err(e) => {
                let _ = std::fs::remove_file(&self.tmp);
                Err(e)
            }
        }
    }

    /// Gives up: deletes the temporary file.
    pub fn cancel(mut self) {
        self.writer = None;
        let _ = std::fs::remove_file(&self.tmp);
    }
}

fn no_follow<W: Write>(mut b: tar::Builder<W>) -> tar::Builder<W> {
    // Symlinks go in as links, not as the file they point to.
    b.follow_symlinks(false);
    b
}

fn add(writer: &mut Writer, item: &ArchiveItem) -> io::Result<()> {
    match writer {
        Writer::Tar(t) => t.append_path_with_name(&item.path, &item.name),
        Writer::TarGz(t) => t.append_path_with_name(&item.path, &item.name),
        Writer::Zip(z) => {
            let meta = std::fs::symlink_metadata(&item.path)?;
            let mut opts = zip::write::SimpleFileOptions::default()
                .unix_permissions(crate::platform::unix_mode(&meta) & 0o7777);
            if let Ok(t) = meta.modified() {
                use chrono::{Datelike, Timelike};
                let t = chrono::DateTime::<Local>::from(t);
                if let Ok(dt) = zip::DateTime::from_date_and_time(
                    t.year().clamp(1980, 2107) as u16,
                    t.month() as u8,
                    t.day() as u8,
                    t.hour() as u8,
                    t.minute() as u8,
                    t.second() as u8,
                ) {
                    opts = opts.last_modified_time(dt);
                }
            }
            if meta.is_symlink() {
                let target = std::fs::read_link(&item.path)?;
                z.add_symlink(&item.name, target.to_string_lossy(), opts)
                    .map_err(io::Error::other)
            } else {
                z.start_file(&item.name, opts).map_err(io::Error::other)?;
                io::copy(&mut File::open(&item.path)?, z).map(drop)
            }
        }
    }
}

/// Normal components only: no `..`, leading `/` or `C:`.
fn is_safe(path: &Path) -> bool {
    path.components().all(|c| matches!(c, Component::Normal(_)))
}

/// A 7z error as an I/O error, with a clear message for encrypted archives.
fn sevenz_error(e: sevenz_rust2::Error) -> io::Error {
    match e {
        sevenz_rust2::Error::PasswordRequired | sevenz_rust2::Error::MaybeBadPassword(_) => {
            io::Error::other("encrypted 7z archives are not supported")
        }
        sevenz_rust2::Error::Io(e, _) => e,
        e => io::Error::other(e.to_string()),
    }
}

fn not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "entry not found in archive")
}

fn symlink_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "symbolic links inside archives are not extracted",
    )
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Cursor;

    /// ZIP with src/a.rs, README and a malicious entry "../evil".
    pub fn make_zip(path: &Path) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.add_directory("src/", opts).unwrap();
        zip.start_file("src/a.rs", opts).unwrap();
        zip.write_all(b"fn a() {}").unwrap();
        zip.start_file("README", opts).unwrap();
        zip.write_all(b"leia-me").unwrap();
        zip.start_file("../evil", opts).unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
    }

    /// TAR(.gz) with docs/guide.txt, a link to /etc and "../evil".
    pub fn make_tar(path: &Path, gz: bool) {
        let kind = if gz { Kind::TarGz } else { Kind::Tar };
        make_tar_as(path, kind);
    }

    /// The test TAR (a file, a symlink and an entry with ".."),
    /// compressed as `kind` says.
    pub fn make_tar_as(path: &Path, kind: Kind) {
        let mut data = Vec::new();
        {
            let mut tar = tar::Builder::new(&mut data);
            let mut header = tar::Header::new_gnu();
            header.set_size(5);
            header.set_mode(0o640);
            header.set_mtime(1_000_000);
            header.set_cksum();
            tar.append_data(&mut header.clone(), "docs/guide.txt", Cursor::new(b"guide"))
                .unwrap();

            let mut link = tar::Header::new_gnu();
            link.set_entry_type(tar::EntryType::Symlink);
            link.set_size(0);
            tar.append_link(&mut link, "etc-link", "/etc").unwrap();

            // `append_data` rejects "..": builds the header by hand.
            let mut evil = tar::Header::new_old();
            evil.as_old_mut().name[..7].copy_from_slice(b"../evil");
            evil.set_size(1);
            evil.set_cksum();
            tar.append(&evil, Cursor::new(b"x")).unwrap();
            tar.finish().unwrap();
        }
        let mut file = File::create(path).unwrap();
        match kind {
            Kind::Tar => file.write_all(&data).unwrap(),
            Kind::TarGz => {
                let mut enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
                enc.write_all(&data).unwrap();
                enc.finish().unwrap();
            }
            Kind::TarXz => {
                let options = lzma_rust2::XzOptions::default();
                let mut enc = lzma_rust2::XzWriter::new(file, options).unwrap();
                enc.write_all(&data).unwrap();
                enc.finish().unwrap();
            }
            Kind::TarBz2 => {
                let mut enc = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
                enc.write_all(&data).unwrap();
                enc.finish().unwrap();
            }
            Kind::TarZst => ruzstd::encoding::compress(
                &data[..],
                file,
                ruzstd::encoding::CompressionLevel::Fastest,
            ),
            _ => panic!("not a TAR kind: {kind:?}"),
        }
    }

    /// A 7z with the same contents as the test TAR: `docs/guide.txt` (mode
    /// 640), a symlink `etc-link` and an entry with "..", all in one solid
    /// block, with the file last (reading it goes through the others).
    pub fn make_7z(path: &Path) {
        use sevenz_rust2::{ArchiveEntry, ArchiveWriter, SourceReader};
        // Unix mode in the high bits of the attributes, with 7-Zip's flag.
        let unix = |mode: u32| (mode << 16) | 0x8000;
        let mut writer = ArchiveWriter::create(path).unwrap();
        let mut guide = ArchiveEntry::new_file("docs/guide.txt");
        guide.has_windows_attributes = true;
        guide.windows_attributes = unix(0o100640);
        guide.has_last_modified_date = true;
        guide.last_modified_date = (SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000))
            .try_into()
            .unwrap();
        let mut link = ArchiveEntry::new_file("etc-link");
        link.has_windows_attributes = true;
        link.windows_attributes = unix(0o120777);
        let evil = ArchiveEntry::new_file("../evil");
        let data: [&[u8]; 3] = [b"/etc", b"x", b"guide"];
        let readers = data.into_iter().map(SourceReader::from).collect();
        writer
            .push_archive_entries(vec![link, evil, guide], readers)
            .unwrap();
        writer.finish().unwrap();
    }

    fn paths(entries: &[Entry]) -> Vec<String> {
        let mut p: Vec<String> = entries.iter().map(|e| entry_name(&e.path)).collect();
        p.sort();
        p
    }

    /// TAR with a 2 MiB `big` (past `MAX_SKIP` in tests), then `small`.
    fn deep_tar(path: &Path) {
        let mut tar = tar::Builder::new(File::create(path).unwrap());
        for (name, data) in [
            ("big", vec![0u8; 2 * 1024 * 1024]),
            ("small", b"s".to_vec()),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, name, &data[..]).unwrap();
        }
        tar.finish().unwrap();
    }

    #[test]
    fn previews_give_up_deep_inside_an_archive() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("deep.tar");
        deep_tar(&path);
        let mut r = Reader::open(&path).unwrap();
        let mut out = Vec::new();
        let err = r.extract(Path::new("small"), &mut out, 1000).unwrap_err();
        assert!(err.to_string().contains("too deep"), "{err}");
        // A copy (no limit) goes all the way.
        r.extract(Path::new("small"), &mut out, u64::MAX).unwrap();
        assert_eq!(out, b"s");
    }

    #[test]
    fn extracts_in_the_background_and_stops_when_dropped() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("deep.tar");
        deep_tar(&path);
        let extractor = Reader::open(&path).unwrap().into_extractor();
        let mut big = Vec::new();
        extractor
            .extract(Path::new("big"))
            .read_to_end(&mut big)
            .unwrap();
        assert_eq!(big.len(), 2 * 1024 * 1024);

        // Abandoned halfway: the thread goes on to the next request.
        let mut first = [0u8; 10];
        let mut abandoned = extractor.extract(Path::new("big"));
        abandoned.read_exact(&mut first).unwrap();
        drop(abandoned);
        let mut small = String::new();
        extractor
            .extract(Path::new("small"))
            .read_to_string(&mut small)
            .unwrap();
        assert_eq!(small, "s");

        let err = extractor
            .extract(Path::new("nope"))
            .read_to_end(&mut Vec::new())
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn detects_kinds() {
        assert_eq!(Kind::detect(Path::new("a.ZIP")), Some(Kind::Zip));
        assert_eq!(Kind::detect(Path::new("a.tar.gz")), Some(Kind::TarGz));
        assert_eq!(Kind::detect(Path::new("a.tgz")), Some(Kind::TarGz));
        assert_eq!(Kind::detect(Path::new("a.tar")), Some(Kind::Tar));
        assert_eq!(Kind::detect(Path::new("a.tar.xz")), Some(Kind::TarXz));
        assert_eq!(Kind::detect(Path::new("a.txz")), Some(Kind::TarXz));
        assert_eq!(Kind::detect(Path::new("a.tar.bz2")), Some(Kind::TarBz2));
        assert_eq!(Kind::detect(Path::new("a.tbz2")), Some(Kind::TarBz2));
        assert_eq!(Kind::detect(Path::new("a.tar.zst")), Some(Kind::TarZst));
        assert_eq!(Kind::detect(Path::new("A.7Z")), Some(Kind::SevenZ));
        assert_eq!(Kind::detect(Path::new("a.txt")), None);
        // A lone .xz/.bz2 (not a TAR) is not an archive.
        assert_eq!(Kind::detect(Path::new("a.log.xz")), None);
    }

    #[test]
    fn lists_and_extracts_zip_ignoring_unsafe_entries() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("t.zip");
        make_zip(&path);
        let mut r = Reader::open(&path).unwrap();
        let entries = r.list().unwrap();
        assert_eq!(paths(&entries), ["README", "src", "src/a.rs"]);
        let a = entries
            .iter()
            .find(|e| e.path == Path::new("src/a.rs"))
            .unwrap();
        assert_eq!((a.size, a.kind), (9, EntryKind::File));
        // On Windows, the zip crate records entries as made on DOS, without the
        // Unix permissions.
        #[cfg(unix)]
        assert_eq!(a.mode, 0o755);
        assert!(a.modified.is_some());

        let mut out = Vec::new();
        assert_eq!(
            r.extract(Path::new("src/a.rs"), &mut out, u64::MAX)
                .unwrap(),
            9
        );
        assert_eq!(out, b"fn a() {}");
        let mut out = Vec::new();
        r.extract(Path::new("README"), &mut out, 4).unwrap();
        assert_eq!(out, b"leia", "limit");
        assert!(
            r.extract(Path::new("../evil"), &mut Vec::new(), 10)
                .is_err()
        );
    }

    #[test]
    fn tar_xz_with_huge_dictionary_is_refused_not_allocated() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("big.tar.xz");
        std::fs::write(
            &path,
            crate::decompress::test_util::xz_header_with_dict_props(40),
        )
        .unwrap();
        let mut r = Reader::open(&path).unwrap();
        let err = r.list().unwrap_err();
        assert!(err.to_string().contains("mem_limit"), "{err}");
    }

    #[test]
    fn lists_and_extracts_tar_compressed_or_not_and_7z() {
        let tmp = crate::testing::tempdir();
        let names = [
            "t.tar",
            "t.tgz",
            "t.tar.xz",
            "t.tar.bz2",
            "t.tar.zst",
            "t.7z",
        ];
        for name in names {
            let path = tmp.path().join(name);
            match Kind::detect(&path).unwrap() {
                Kind::SevenZ => make_7z(&path),
                kind => make_tar_as(&path, kind),
            }
            let mut r = Reader::open(&path).unwrap();
            let entries = r.list().unwrap();
            assert_eq!(paths(&entries), ["docs/guide.txt", "etc-link"], "{name}");
            let guide = &entries
                .iter()
                .find(|e| e.path.ends_with("guide.txt"))
                .unwrap();
            assert_eq!((guide.size, guide.mode), (5, 0o640), "{name}");
            let mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
            assert_eq!(guide.modified, Some(mtime), "{name}");

            let mut out = Vec::new();
            r.extract(Path::new("docs/guide.txt"), &mut out, u64::MAX)
                .unwrap();
            assert_eq!(out, b"guide");
            let err = r
                .extract(Path::new("etc-link"), &mut Vec::new(), 10)
                .unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::Unsupported);
            assert!(r.extract(Path::new("nope"), &mut Vec::new(), 10).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn creates_archives_that_read_back() {
        let tmp = crate::testing::tempdir();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "alpha").unwrap();
        std::fs::write(src.join("sub/b.txt"), "beta").unwrap();
        std::os::unix::fs::symlink("a.txt", src.join("link")).unwrap();
        let items = || {
            ["a.txt", "sub/b.txt", "link"]
                .map(|n| ArchiveItem {
                    path: src.join(n),
                    name: n.into(),
                    size: 4,
                })
                .into_iter()
                .collect::<Vec<_>>()
        };
        for name in ["out.zip", "out.tar", "out.tar.gz"] {
            let dest = tmp.path().join(name);
            let mut job = ArchiveJob::new(&dest, items()).unwrap();
            while !job.is_done() {
                job.step(std::time::Duration::ZERO);
            }
            assert!(job.errors.is_empty(), "{name}: {:?}", job.errors);
            job.finish().unwrap();

            let mut r = Reader::open(&dest).unwrap();
            let entries = r.list().unwrap();
            assert_eq!(paths(&entries), ["a.txt", "link", "sub/b.txt"], "{name}");
            let link = entries.iter().find(|e| e.path.ends_with("link")).unwrap();
            assert_eq!(link.kind, EntryKind::Symlink, "{name}: stored as a link");
            let mut out = Vec::new();
            r.extract(Path::new("sub/b.txt"), &mut out, u64::MAX)
                .unwrap();
            assert_eq!(out, b"beta");
        }
        // Cancelling leaves nothing; unknown extension is rejected.
        let job = ArchiveJob::new(&tmp.path().join("x.zip"), items()).unwrap();
        job.cancel();
        let left: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(
            !left.iter().any(|n| n.to_string_lossy().contains("x.zip")),
            "{left:?}"
        );
        assert!(ArchiveJob::new(&tmp.path().join("x.rar"), items()).is_err());
    }

    #[test]
    fn rejects_unknown_or_broken_files() {
        let tmp = crate::testing::tempdir();
        assert!(Reader::open(&tmp.path().join("x.txt")).is_err());
        let broken = tmp.path().join("broken.zip");
        std::fs::write(&broken, "not a zip").unwrap();
        assert!(Reader::open(&broken).is_err());
    }
}
