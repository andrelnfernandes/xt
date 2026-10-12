//! Leitura de arquivos compactados (ZIP e TAR), somente leitura.
//!
//! As entradas viram uma árvore virtual cuja raiz é o próprio arquivo:
//! `/x/pacote.zip/src/a.rs` é a entrada `src/a.rs` de `/x/pacote.zip`.
//!
//! Segurança: entradas com `..` ou caminho absoluto são ignoradas (senão
//! poderiam ser extraídas para fora do destino), e links simbólicos nunca
//! são extraídos (um link para `/etc` seguido de `link/passwd` escreveria
//! fora do destino).

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::{Local, NaiveDate, TimeZone};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Zip,
    Tar,
    TarGz,
}

impl Kind {
    /// Reconhece pelo nome do arquivo.
    pub fn detect(path: &Path) -> Option<Kind> {
        let name = path.file_name()?.to_string_lossy().to_lowercase();
        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            Some(Kind::TarGz)
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
    /// Caminho relativo, já validado (só componentes normais).
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub mode: u32,
    pub kind: EntryKind,
}

/// Um arquivo compactado aberto, para listar e extrair entradas.
pub struct Reader {
    path: PathBuf,
    kind: Kind,
    zip: Option<zip::ZipArchive<BufReader<File>>>,
    /// Caminho → índice no ZIP.
    zip_index: HashMap<PathBuf, usize>,
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
        };
        if kind == Kind::Zip {
            let file = BufReader::new(File::open(path)?);
            reader.zip = Some(zip::ZipArchive::new(file).map_err(io::Error::other)?);
        }
        Ok(reader)
    }

    /// Todas as entradas válidas do arquivo.
    pub fn list(&mut self) -> io::Result<Vec<Entry>> {
        match self.kind {
            Kind::Zip => self.list_zip(),
            Kind::Tar | Kind::TarGz => {
                let mut entries = Vec::new();
                self.each_tar_entry(|entry, _| {
                    entries.push(entry);
                    Ok(true)
                })?;
                Ok(entries)
            }
        }
    }

    /// Escreve o conteúdo de `entry` em `out`, até `limit` bytes.
    /// Devolve quantos bytes foram escritos.
    pub fn extract(&mut self, entry: &Path, out: &mut dyn Write, limit: u64) -> io::Result<u64> {
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
            Kind::Tar | Kind::TarGz => {
                let mut written = None;
                self.each_tar_entry(|info, data| {
                    if info.path != entry {
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

    fn list_zip(&mut self) -> io::Result<Vec<Entry>> {
        let zip = self.zip.as_mut().expect("zip archive is open");
        let mut entries = Vec::new();
        self.zip_index.clear();
        for i in 0..zip.len() {
            let file = zip.by_index_raw(i).map_err(io::Error::other)?;
            // `enclosed_name` já recusa ".." e caminhos absolutos.
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

    /// Percorre o TAR do início, chamando `f` com cada entrada válida e o
    /// seu conteúdo; `f` devolve `false` para parar.
    fn each_tar_entry(
        &self,
        mut f: impl FnMut(Entry, &mut dyn Read) -> io::Result<bool>,
    ) -> io::Result<()> {
        let file = BufReader::new(File::open(&self.path)?);
        let reader: Box<dyn Read> = match self.kind {
            Kind::TarGz => Box::new(flate2::read::GzDecoder::new(file)),
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

/// Escritor do arquivo compactado em criação.
enum Writer {
    Zip(Box<zip::ZipWriter<File>>),
    Tar(tar::Builder<File>),
    TarGz(tar::Builder<flate2::write::GzEncoder<File>>),
}

/// Um arquivo a pôr no compactado: o caminho no disco e o nome lá dentro.
pub struct ArchiveItem {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

/// Criação de um ZIP/TAR (F5), um arquivo por vez, num temporário que só
/// ganha o nome final no fim: cancelar ou falhar não deixa um arquivo
/// compactado pela metade.
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
        let kind = Kind::detect(dest).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "use a name ending in .zip, .tar, .tar.gz or .tgz",
            )
        })?;
        let name = dest.file_name().unwrap_or_default().to_string_lossy();
        let tmp = dest.with_file_name(format!(".{name}.xt-{}.tmp", std::process::id()));
        let file = File::create(&tmp)?;
        let writer = match kind {
            Kind::Zip => Writer::Zip(Box::new(zip::ZipWriter::new(file))),
            Kind::Tar => Writer::Tar(no_follow(tar::Builder::new(file))),
            Kind::TarGz => Writer::TarGz(no_follow(tar::Builder::new(
                flate2::write::GzEncoder::new(file, flate2::Compression::default()),
            ))),
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

    /// Acrescenta arquivos até estourar `budget`.
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

    /// Fecha o arquivo compactado e o põe no lugar.
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

    /// Desiste: apaga o temporário.
    pub fn cancel(mut self) {
        self.writer = None;
        let _ = std::fs::remove_file(&self.tmp);
    }
}

fn no_follow<W: Write>(mut b: tar::Builder<W>) -> tar::Builder<W> {
    // Links simbólicos entram como links, não como o arquivo apontado.
    b.follow_symlinks(false);
    b
}

fn add(writer: &mut Writer, item: &ArchiveItem) -> io::Result<()> {
    match writer {
        Writer::Tar(t) => t.append_path_with_name(&item.path, &item.name),
        Writer::TarGz(t) => t.append_path_with_name(&item.path, &item.name),
        Writer::Zip(z) => {
            let meta = std::fs::symlink_metadata(&item.path)?;
            let mut opts = zip::write::SimpleFileOptions::default().unix_permissions(
                std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o7777,
            );
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

/// Só componentes normais: nada de `..`, `/` no começo ou `C:`.
fn is_safe(path: &Path) -> bool {
    path.components().all(|c| matches!(c, Component::Normal(_)))
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

    /// ZIP com src/a.rs, README e uma entrada maliciosa "../evil".
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

    /// TAR(.gz) com docs/guide.txt, um link para /etc e "../evil".
    pub fn make_tar(path: &Path, gz: bool) {
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

            // `append_data` recusa "..": monta o cabeçalho na mão.
            let mut evil = tar::Header::new_old();
            evil.as_old_mut().name[..7].copy_from_slice(b"../evil");
            evil.set_size(1);
            evil.set_cksum();
            tar.append(&evil, Cursor::new(b"x")).unwrap();
            tar.finish().unwrap();
        }
        if gz {
            let mut enc = flate2::write::GzEncoder::new(
                File::create(path).unwrap(),
                flate2::Compression::default(),
            );
            enc.write_all(&data).unwrap();
            enc.finish().unwrap();
        } else {
            std::fs::write(path, data).unwrap();
        }
    }

    fn paths(entries: &[Entry]) -> Vec<String> {
        let mut p: Vec<String> = entries
            .iter()
            .map(|e| e.path.to_string_lossy().into_owned())
            .collect();
        p.sort();
        p
    }

    #[test]
    fn detects_kinds() {
        assert_eq!(Kind::detect(Path::new("a.ZIP")), Some(Kind::Zip));
        assert_eq!(Kind::detect(Path::new("a.tar.gz")), Some(Kind::TarGz));
        assert_eq!(Kind::detect(Path::new("a.tgz")), Some(Kind::TarGz));
        assert_eq!(Kind::detect(Path::new("a.tar")), Some(Kind::Tar));
        assert_eq!(Kind::detect(Path::new("a.txt")), None);
    }

    #[test]
    fn lists_and_extracts_zip_ignoring_unsafe_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("t.zip");
        make_zip(&path);
        let mut r = Reader::open(&path).unwrap();
        let entries = r.list().unwrap();
        assert_eq!(paths(&entries), ["README", "src", "src/a.rs"]);
        let a = entries
            .iter()
            .find(|e| e.path == Path::new("src/a.rs"))
            .unwrap();
        assert_eq!((a.size, a.mode, a.kind), (9, 0o755, EntryKind::File));
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
    fn lists_and_extracts_tar_and_tgz() {
        let tmp = tempfile::tempdir().unwrap();
        for gz in [false, true] {
            let path = tmp.path().join(if gz { "t.tgz" } else { "t.tar" });
            make_tar(&path, gz);
            let mut r = Reader::open(&path).unwrap();
            let entries = r.list().unwrap();
            assert_eq!(paths(&entries), ["docs/guide.txt", "etc-link"], "gz={gz}");
            let guide = &entries
                .iter()
                .find(|e| e.path.ends_with("guide.txt"))
                .unwrap();
            assert_eq!((guide.size, guide.mode), (5, 0o640));

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

    #[test]
    fn creates_archives_that_read_back() {
        let tmp = tempfile::tempdir().unwrap();
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
        // Cancelar não deixa nada; extensão desconhecida é recusada.
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
        let tmp = tempfile::tempdir().unwrap();
        assert!(Reader::open(&tmp.path().join("x.txt")).is_err());
        let broken = tmp.path().join("broken.zip");
        std::fs::write(&broken, "not a zip").unwrap();
        assert!(Reader::open(&broken).is_err());
    }
}
