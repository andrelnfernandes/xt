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
    fn rejects_unknown_or_broken_files() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Reader::open(&tmp.path().join("x.txt")).is_err());
        let broken = tmp.path().join("broken.zip");
        std::fs::write(&broken, "not a zip").unwrap();
        assert!(Reader::open(&broken).is_err());
    }
}
