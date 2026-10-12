//! Operações em arquivos: copiar, mover e apagar.
//!
//! Uma [`Operation`] é executada aos poucos (`step`), para a interface mostrar
//! o progresso, perguntar sobre conflitos e permitir cancelar. Arquivos são
//! copiados em blocos para um temporário, que só no fim substitui o destino:
//! um cancelamento ou erro nunca deixa um destino pela metade.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use crate::archive;

const CHUNK: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Copy,
    Move,
    Delete,
}

impl OpKind {
    pub fn verb(self) -> &'static str {
        match self {
            OpKind::Copy => "Copying",
            OpKind::Move => "Moving",
            OpKind::Delete => "Deleting",
        }
    }

    pub fn past(self) -> &'static str {
        match self {
            OpKind::Copy => "Copied",
            OpKind::Move => "Moved",
            OpKind::Delete => "Deleted",
        }
    }
}

/// Um arquivo de origem, com o tamanho já conhecido (para o progresso).
#[derive(Debug, Clone)]
pub struct Source {
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone)]
struct Item {
    src: PathBuf,
    /// Vazio no Delete.
    dst: PathBuf,
    size: u64,
}

/// Resposta do usuário a um conflito ("o destino já existe").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Yes,
    No,
    All,
    SkipAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overwrite {
    Ask,
    Always,
    Never,
}

/// Origem dos arquivos quando eles saem de dentro de um ZIP/TAR.
struct ArchiveSource {
    reader: archive::Reader,
    /// Caminho do arquivo compactado: raiz dos caminhos virtuais.
    root: PathBuf,
    entries: HashMap<PathBuf, archive::Entry>,
}

/// Cópia de um arquivo em andamento.
struct Transfer {
    reader: File,
    writer: File,
    tmp: PathBuf,
}

pub struct Operation {
    pub kind: OpKind,
    items: Vec<Item>,
    next: usize,
    transfer: Option<Transfer>,
    overwrite: Overwrite,
    /// O item `next` já teve o conflito resolvido com "sim".
    approved: bool,
    /// Destino em conflito, esperando [`Operation::answer`].
    pub conflict: Option<PathBuf>,
    pub done_files: usize,
    pub skipped: usize,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub errors: Vec<String>,
    /// Os arquivos de origem estão dentro deste arquivo compactado.
    archive: Option<ArchiveSource>,
    /// Só para testes: finge que origem e destino estão em discos diferentes.
    #[cfg(test)]
    force_cross_device: bool,
}

impl Operation {
    pub fn delete(sources: Vec<Source>) -> Self {
        let items = sources
            .into_iter()
            .map(|s| Item {
                src: s.path,
                dst: PathBuf::new(),
                size: s.size,
            })
            .collect();
        Self::new(OpKind::Delete, items)
    }

    /// Prepara uma cópia ou movimentação para `dest`. Se `dest` for um
    /// diretório existente, os arquivos vão para dentro dele; com um único
    /// arquivo, `dest` também pode ser o novo nome.
    ///
    /// Com `paths_from`, cada arquivo vai para o mesmo caminho relativo a
    /// esse diretório (`base/a/b/x` → `dest/a/b/x`), criando o que faltar.
    pub fn transfer(
        kind: OpKind,
        sources: Vec<Source>,
        dest: &Path,
        paths_from: Option<&Path>,
    ) -> Result<Self, String> {
        let items = if dest.is_dir() {
            sources
                .into_iter()
                .map(|s| {
                    let rel = match (paths_from, s.path.parent()) {
                        (Some(base), Some(dir)) => dir.strip_prefix(base).unwrap_or(Path::new("")),
                        _ => Path::new(""),
                    };
                    Item {
                        dst: dest.join(rel).join(s.path.file_name().unwrap_or_default()),
                        src: s.path,
                        size: s.size,
                    }
                })
                .collect()
        } else if paths_from.is_none()
            && sources.len() == 1
            && dest.parent().is_some_and(Path::is_dir)
        {
            let s = sources.into_iter().next().unwrap();
            vec![Item {
                src: s.path,
                dst: dest.to_path_buf(),
                size: s.size,
            }]
        } else {
            return Err(format!(
                "Destination directory does not exist: {}",
                dest.display()
            ));
        };
        Ok(Self::new(kind, items))
    }

    fn new(kind: OpKind, items: Vec<Item>) -> Self {
        Operation {
            kind,
            total_bytes: items.iter().map(|i| i.size).sum(),
            items,
            next: 0,
            transfer: None,
            overwrite: Overwrite::Ask,
            approved: false,
            conflict: None,
            done_files: 0,
            skipped: 0,
            done_bytes: 0,
            errors: Vec::new(),
            #[cfg(test)]
            force_cross_device: false,
            archive: None,
        }
    }

    /// Indica que as origens são caminhos virtuais dentro de `archive`
    /// (ex.: `/x/p.zip/src/a.rs`). Só cópia: arquivos compactados são
    /// somente leitura.
    pub fn reading_archive(mut self, archive: &Path) -> Result<Self, String> {
        if self.kind != OpKind::Copy {
            return Err("Archives are read-only".into());
        }
        let mut reader = archive::Reader::open(archive).map_err(|e| e.to_string())?;
        let entries = reader
            .list()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|e| (e.path.clone(), e))
            .collect();
        self.archive = Some(ArchiveSource {
            reader,
            root: archive.to_path_buf(),
            entries,
        });
        Ok(self)
    }

    pub fn total_files(&self) -> usize {
        self.items.len()
    }

    pub fn is_done(&self) -> bool {
        self.next >= self.items.len() && self.transfer.is_none()
    }

    /// Arquivo sendo processado agora.
    pub fn current(&self) -> Option<&Path> {
        self.items.get(self.next).map(|i| i.src.as_path())
    }

    /// Diretórios cujo conteúdo pode ter mudado.
    pub fn touched_dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self
            .items
            .iter()
            .flat_map(|i| [i.src.parent(), i.dst.parent()])
            .flatten()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .collect();
        dirs.sort();
        dirs.dedup();
        dirs
    }

    pub fn answer(&mut self, answer: Answer) {
        if self.conflict.take().is_none() {
            return;
        }
        match answer {
            Answer::Yes => self.approved = true,
            Answer::All => {
                self.overwrite = Overwrite::Always;
                self.approved = true;
            }
            Answer::No => self.skip(),
            Answer::SkipAll => {
                self.overwrite = Overwrite::Never;
                self.skip();
            }
        }
    }

    /// Interrompe a operação, apagando o temporário de uma cópia em curso.
    pub fn cancel(&mut self) {
        if let Some(t) = self.transfer.take() {
            drop(t.writer);
            let _ = fs::remove_file(&t.tmp);
        }
        self.next = self.items.len();
        self.conflict = None;
    }

    /// Trabalha até terminar, estourar `budget` ou precisar de uma resposta.
    pub fn step(&mut self, budget: Duration) {
        let start = Instant::now();
        while self.conflict.is_none() && !self.is_done() {
            if self.transfer.is_some() {
                self.copy_chunk();
            } else {
                self.start_item();
            }
            if start.elapsed() >= budget {
                break;
            }
        }
    }

    fn skip(&mut self) {
        self.skipped += 1;
        self.done_bytes += self.items[self.next].size;
        self.advance();
    }

    fn advance(&mut self) {
        self.next += 1;
        self.approved = false;
    }

    fn fail(&mut self, path: &Path, e: impl std::fmt::Display) {
        self.errors.push(format!("{}: {e}", path.display()));
        self.done_bytes += self.items[self.next].size;
        self.advance();
    }

    fn finish_item(&mut self) {
        self.done_files += 1;
        self.advance();
    }

    fn start_item(&mut self) {
        let item = self.items[self.next].clone();

        if self.kind == OpKind::Delete {
            match fs::remove_file(&item.src) {
                Ok(()) => {
                    self.done_bytes += item.size;
                    self.finish_item();
                }
                Err(e) => self.fail(&item.src, e),
            }
            return;
        }

        // Dentro de um arquivo compactado não há metadados no disco.
        let src_meta = match self.archive {
            Some(_) => None,
            None => match fs::symlink_metadata(&item.src) {
                Ok(m) => Some(m),
                Err(e) => return self.fail(&item.src, e),
            },
        };
        if let Ok(dst_meta) = fs::symlink_metadata(&item.dst) {
            if let Some(src_meta) = &src_meta
                && (dst_meta.dev(), dst_meta.ino()) == (src_meta.dev(), src_meta.ino())
            {
                return self.fail(&item.src, "source and destination are the same file");
            }
            if dst_meta.is_dir() {
                return self.fail(&item.dst, "a directory with this name already exists");
            }
            if !self.approved {
                match self.overwrite {
                    Overwrite::Ask => {
                        self.conflict = Some(item.dst);
                        return;
                    }
                    Overwrite::Never => return self.skip(),
                    Overwrite::Always => {}
                }
            }
        }

        if let Some(parent) = item.dst.parent()
            && !parent.is_dir()
            && let Err(e) = fs::create_dir_all(parent)
        {
            return self.fail(parent, e);
        }

        let Some(src_meta) = src_meta else {
            return self.extract_item(&item);
        };

        if self.kind == OpKind::Move {
            match self.rename(&item.src, &item.dst) {
                Ok(()) => {
                    self.done_bytes += item.size;
                    return self.finish_item();
                }
                Err(e) if e.kind() == ErrorKind::CrossesDevices => {} // copia e apaga
                Err(e) => return self.fail(&item.src, e),
            }
        }

        let ft = src_meta.file_type();
        if ft.is_symlink() {
            // Copia o link, não o arquivo para onde ele aponta.
            let result = fs::read_link(&item.src).and_then(|target| {
                let tmp = tmp_path(&item.dst);
                std::os::unix::fs::symlink(target, &tmp)?;
                fs::rename(&tmp, &item.dst)
            });
            match result {
                Ok(()) => {
                    self.done_bytes += item.size;
                    self.complete_copy(&item.src);
                }
                Err(e) => self.fail(&item.src, e),
            }
        } else if ft.is_file() {
            let tmp = tmp_path(&item.dst);
            let opened = File::open(&item.src).and_then(|reader| {
                let writer = File::create(&tmp)?;
                Ok(Transfer {
                    reader,
                    writer,
                    tmp,
                })
            });
            match opened {
                Ok(t) => self.transfer = Some(t),
                Err(e) => self.fail(&item.src, e),
            }
        } else {
            self.fail(&item.src, "special files are not supported");
        }
    }

    /// Extrai uma entrada do arquivo compactado para o destino, via
    /// temporário, com as permissões e a data guardadas no arquivo.
    fn extract_item(&mut self, item: &Item) {
        let source = self.archive.as_mut().expect("archive source");
        let entry = item.src.strip_prefix(&source.root).unwrap_or(&item.src);
        let info = source.entries.get(entry).cloned();
        let tmp = tmp_path(&item.dst);
        let result = (|| {
            let mut file = File::create(&tmp)?;
            source.reader.extract(entry, &mut file, u64::MAX)?;
            if let Some(info) = &info {
                file.set_permissions(fs::Permissions::from_mode(info.mode))?;
                if let Some(t) = info.modified {
                    file.set_modified(t)?;
                }
            }
            drop(file);
            fs::rename(&tmp, &item.dst)
        })();
        match result {
            Ok(()) => {
                self.done_bytes += item.size;
                self.finish_item();
            }
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                self.fail(&item.src, e);
            }
        }
    }

    fn rename(&self, src: &Path, dst: &Path) -> io::Result<()> {
        #[cfg(test)]
        if self.force_cross_device {
            return Err(io::Error::from(ErrorKind::CrossesDevices));
        }
        fs::rename(src, dst)
    }

    fn copy_chunk(&mut self) {
        let t = self.transfer.as_mut().expect("transfer in progress");
        let mut buf = vec![0; CHUNK];
        let result = t.reader.read(&mut buf).and_then(|n| {
            t.writer.write_all(&buf[..n])?;
            Ok(n)
        });
        match result {
            Ok(0) => self.finish_transfer(),
            Ok(n) => self.done_bytes += n as u64,
            Err(e) => {
                let t = self.transfer.take().unwrap();
                let _ = fs::remove_file(&t.tmp);
                let src = self.items[self.next].src.clone();
                self.errors.push(format!("{}: {e}", src.display()));
                self.advance();
            }
        }
    }

    /// Fim da leitura: copia permissões e data, e põe o arquivo no lugar.
    fn finish_transfer(&mut self) {
        let t = self.transfer.take().unwrap();
        let item = self.items[self.next].clone();
        let result = (|| {
            let meta = t.reader.metadata()?;
            t.writer.set_permissions(fs::Permissions::from_mode(
                meta.permissions().mode() & 0o7777,
            ))?;
            if let Ok(mtime) = meta.modified() {
                t.writer.set_modified(mtime)?;
            }
            drop(t.writer);
            fs::rename(&t.tmp, &item.dst)
        })();
        match result {
            Ok(()) => self.complete_copy(&item.src),
            Err(e) => {
                let _ = fs::remove_file(&t.tmp);
                self.errors.push(format!("{}: {e}", item.src.display()));
                self.advance();
            }
        }
    }

    /// Cópia pronta; no Move entre discos, falta apagar a origem.
    fn complete_copy(&mut self, src: &Path) {
        if self.kind == OpKind::Move
            && let Err(e) = fs::remove_file(src)
        {
            self.errors
                .push(format!("{}: copied but not removed: {e}", src.display()));
            self.advance();
            return;
        }
        self.finish_item();
    }

    /// Resumo para mostrar no fim.
    pub fn summary(&self) -> String {
        let mut s = format!(
            "{} {} of {} files",
            self.kind.past(),
            self.done_files,
            self.items.len()
        );
        if self.skipped > 0 {
            s.push_str(&format!(", {} skipped", self.skipped));
        }
        match self.errors.as_slice() {
            [] => {}
            [e] => s.push_str(&format!(". Error: {e}")),
            [e, rest @ ..] => s.push_str(&format!(". {} errors, first: {e}", rest.len() + 1)),
        }
        s
    }
}

/// Temporário escondido, no mesmo diretório do destino (para o rename final
/// ser atômico).
fn tmp_path(dst: &Path) -> PathBuf {
    let name = dst.file_name().unwrap_or_default().to_string_lossy();
    dst.with_file_name(format!(".{name}.xt-{}.tmp", std::process::id()))
}

/// Interpreta o destino digitado: `~` vira o home, caminhos relativos
/// partem de `base`, e `.`/`..` são resolvidos no texto.
pub fn resolve_path(input: &str, base: &Path) -> PathBuf {
    let input = input.trim();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let path = match (input.strip_prefix('~'), home) {
        (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(input),
    };
    let mut out = PathBuf::new();
    for comp in base.join(path).components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// Valida um nome de arquivo ou diretório digitado.
pub fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." {
        Err("Invalid name".into())
    } else if name.contains('/') {
        Err("Name cannot contain '/'".into())
    } else {
        Ok(())
    }
}

/// Interpreta permissões em octal, ex.: "644" ou "0755".
pub fn parse_mode(text: &str) -> Result<u32, String> {
    let text = text.trim();
    let valid = (3..=4).contains(&text.len()) && text.chars().all(|c| ('0'..='7').contains(&c));
    if !valid {
        return Err("Permissions must be 3 or 4 octal digits, e.g. 644".into());
    }
    u32::from_str_radix(text, 8).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn src(path: &Path) -> Source {
        Source {
            path: path.to_path_buf(),
            size: fs::symlink_metadata(path).map(|m| m.len()).unwrap_or(0),
        }
    }

    fn run(op: &mut Operation) {
        op.step(Duration::MAX);
    }

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        fs::write(a.join("one.txt"), "one").unwrap();
        fs::write(a.join("two.txt"), "two!").unwrap();
        (tmp, a, b)
    }

    fn no_temp_files(dir: &Path) -> bool {
        fs::read_dir(dir)
            .unwrap()
            .all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".tmp"))
    }

    #[test]
    fn copies_into_directory_preserving_metadata() {
        let (_tmp, a, b) = setup();
        fs::set_permissions(a.join("one.txt"), fs::Permissions::from_mode(0o751)).unwrap();
        let mtime = fs::metadata(a.join("one.txt")).unwrap().modified().unwrap();

        let sources = vec![src(&a.join("one.txt")), src(&a.join("two.txt"))];
        let mut op = Operation::transfer(OpKind::Copy, sources, &b, None).unwrap();
        assert_eq!(op.total_bytes, 7);
        run(&mut op);

        assert!(op.is_done());
        assert_eq!(op.done_files, 2);
        assert_eq!(op.done_bytes, 7);
        assert_eq!(fs::read_to_string(b.join("two.txt")).unwrap(), "two!");
        let meta = fs::metadata(b.join("one.txt")).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o751);
        assert_eq!(meta.modified().unwrap(), mtime);
        assert!(a.join("one.txt").exists());
        assert!(no_temp_files(&b));
        assert_eq!(op.touched_dirs(), [a, b]);
    }

    #[test]
    fn copies_single_file_to_new_name() {
        let (_tmp, a, b) = setup();
        let mut op = Operation::transfer(
            OpKind::Copy,
            vec![src(&a.join("one.txt"))],
            &b.join("new.txt"),
            None,
        )
        .unwrap();
        run(&mut op);
        assert_eq!(fs::read_to_string(b.join("new.txt")).unwrap(), "one");
    }

    #[test]
    fn rejects_missing_destination_for_many_files() {
        let (_tmp, a, b) = setup();
        let sources = vec![src(&a.join("one.txt")), src(&a.join("two.txt"))];
        assert!(Operation::transfer(OpKind::Copy, sources, &b.join("nope"), None).is_err());
        let one = vec![src(&a.join("one.txt"))];
        assert!(Operation::transfer(OpKind::Copy, one, &b.join("x/y"), None).is_err());
    }

    #[test]
    fn copies_with_relative_paths() {
        let (tmp, a, b) = setup();
        fs::create_dir_all(a.join("x/y")).unwrap();
        fs::write(a.join("x/y/deep.txt"), "deep").unwrap();
        let sources = vec![src(&a.join("one.txt")), src(&a.join("x/y/deep.txt"))];

        let mut op = Operation::transfer(OpKind::Copy, sources.clone(), &b, Some(&a)).unwrap();
        run(&mut op);
        assert_eq!(op.done_files, 2);
        assert!(b.join("one.txt").exists());
        assert_eq!(fs::read_to_string(b.join("x/y/deep.txt")).unwrap(), "deep");
        assert!(op.touched_dirs().contains(&b.join("x/y")));

        // Sem paths_from, tudo vai direto para o destino.
        let flat = tmp.path().join("flat");
        fs::create_dir(&flat).unwrap();
        let mut op = Operation::transfer(OpKind::Copy, sources.clone(), &flat, None).unwrap();
        run(&mut op);
        assert!(flat.join("deep.txt").exists());

        // Com caminhos, o destino precisa ser um diretório existente.
        assert!(Operation::transfer(OpKind::Copy, sources, &b.join("nope"), Some(&a)).is_err());
    }

    #[test]
    fn moves_with_relative_paths() {
        let (_tmp, a, b) = setup();
        fs::create_dir_all(a.join("x")).unwrap();
        fs::write(a.join("x/m.txt"), "m").unwrap();
        let mut op =
            Operation::transfer(OpKind::Move, vec![src(&a.join("x/m.txt"))], &b, Some(&a)).unwrap();
        run(&mut op);
        assert!(!a.join("x/m.txt").exists());
        assert!(b.join("x/m.txt").exists());
    }

    #[test]
    fn asks_before_overwriting() {
        let (_tmp, a, b) = setup();
        fs::write(b.join("one.txt"), "old").unwrap();
        fs::write(b.join("two.txt"), "old").unwrap();
        let sources = vec![src(&a.join("one.txt")), src(&a.join("two.txt"))];
        let mut op = Operation::transfer(OpKind::Copy, sources, &b, None).unwrap();

        run(&mut op);
        assert_eq!(op.conflict.as_deref(), Some(b.join("one.txt").as_path()));
        op.answer(Answer::No);
        run(&mut op);
        assert_eq!(op.conflict.as_deref(), Some(b.join("two.txt").as_path()));
        op.answer(Answer::Yes);
        run(&mut op);

        assert!(op.is_done());
        assert_eq!((op.done_files, op.skipped), (1, 1));
        assert_eq!(fs::read_to_string(b.join("one.txt")).unwrap(), "old");
        assert_eq!(fs::read_to_string(b.join("two.txt")).unwrap(), "two!");
    }

    #[test]
    fn overwrite_all_and_skip_all() {
        let (_tmp, a, b) = setup();
        for answer in [Answer::All, Answer::SkipAll] {
            fs::write(b.join("one.txt"), "old").unwrap();
            fs::write(b.join("two.txt"), "old").unwrap();
            let sources = vec![src(&a.join("one.txt")), src(&a.join("two.txt"))];
            let mut op = Operation::transfer(OpKind::Copy, sources, &b, None).unwrap();
            run(&mut op);
            op.answer(answer);
            run(&mut op);
            assert!(op.is_done(), "{answer:?}");
            let expected = if answer == Answer::All { "two!" } else { "old" };
            assert_eq!(fs::read_to_string(b.join("two.txt")).unwrap(), expected);
        }
    }

    #[test]
    fn refuses_to_copy_file_onto_itself() {
        let (_tmp, a, _b) = setup();
        let mut op =
            Operation::transfer(OpKind::Copy, vec![src(&a.join("one.txt"))], &a, None).unwrap();
        run(&mut op);
        assert_eq!(op.errors.len(), 1);
        assert!(op.conflict.is_none());
        assert_eq!(fs::read_to_string(a.join("one.txt")).unwrap(), "one");
    }

    #[test]
    fn overwriting_a_symlink_replaces_the_link_not_its_target() {
        let (tmp, a, b) = setup();
        let precious = tmp.path().join("precious");
        fs::write(&precious, "keep").unwrap();
        symlink(&precious, b.join("one.txt")).unwrap();

        let mut op =
            Operation::transfer(OpKind::Copy, vec![src(&a.join("one.txt"))], &b, None).unwrap();
        run(&mut op);
        op.answer(Answer::Yes);
        run(&mut op);

        assert_eq!(fs::read_to_string(&precious).unwrap(), "keep");
        assert!(
            !fs::symlink_metadata(b.join("one.txt"))
                .unwrap()
                .is_symlink()
        );
    }

    #[test]
    fn copies_symlinks_as_links() {
        let (_tmp, a, b) = setup();
        symlink("one.txt", a.join("link")).unwrap();
        let mut op =
            Operation::transfer(OpKind::Copy, vec![src(&a.join("link"))], &b, None).unwrap();
        run(&mut op);
        assert_eq!(fs::read_link(b.join("link")).unwrap(), Path::new("one.txt"));
    }

    #[test]
    fn moves_files() {
        let (_tmp, a, b) = setup();
        let mut op =
            Operation::transfer(OpKind::Move, vec![src(&a.join("one.txt"))], &b, None).unwrap();
        run(&mut op);
        assert!(!a.join("one.txt").exists());
        assert_eq!(fs::read_to_string(b.join("one.txt")).unwrap(), "one");
    }

    #[test]
    fn moves_across_devices_by_copying() {
        let (_tmp, a, b) = setup();
        let mut op =
            Operation::transfer(OpKind::Move, vec![src(&a.join("two.txt"))], &b, None).unwrap();
        op.force_cross_device = true;
        run(&mut op);
        assert_eq!(op.done_files, 1);
        assert!(!a.join("two.txt").exists());
        assert_eq!(fs::read_to_string(b.join("two.txt")).unwrap(), "two!");
    }

    #[test]
    fn deletes_and_reports_errors() {
        let (_tmp, a, _b) = setup();
        let sources = vec![src(&a.join("one.txt")), src(&a.join("missing"))];
        let mut op = Operation::delete(sources);
        run(&mut op);
        assert!(!a.join("one.txt").exists());
        assert_eq!(op.done_files, 1);
        assert_eq!(op.errors.len(), 1);
        assert!(op.summary().starts_with("Deleted 1 of 2 files. Error:"));
    }

    #[test]
    fn large_copy_runs_in_chunks_and_cancel_cleans_up() {
        let (_tmp, a, b) = setup();
        let big = a.join("big.bin");
        fs::write(&big, vec![7u8; CHUNK * 3 + 10]).unwrap();

        let mut op = Operation::transfer(OpKind::Copy, vec![src(&big)], &b, None).unwrap();
        op.step(Duration::ZERO); // abre os arquivos
        op.step(Duration::ZERO); // primeiro bloco
        assert_eq!(op.done_bytes, CHUNK as u64);
        assert!(!op.is_done());

        op.cancel();
        assert!(op.is_done());
        assert!(!b.join("big.bin").exists());
        assert!(no_temp_files(&b));

        let mut op = Operation::transfer(OpKind::Copy, vec![src(&big)], &b, None).unwrap();
        run(&mut op);
        assert_eq!(
            fs::metadata(b.join("big.bin")).unwrap().len(),
            CHUNK as u64 * 3 + 10
        );
    }

    #[test]
    fn extracts_from_archives_with_or_without_paths() {
        let (tmp, _a, b) = setup();
        let zip = tmp.path().join("p.zip");
        archive::tests::make_zip(&zip);
        let virt = |p: &str| Source {
            path: zip.join(p),
            size: 0,
        };

        let sources = vec![virt("src/a.rs"), virt("README")];
        let mut op = Operation::transfer(OpKind::Copy, sources, &b, Some(&zip))
            .unwrap()
            .reading_archive(&zip)
            .unwrap();
        run(&mut op);
        assert_eq!(op.done_files, 2, "{:?}", op.errors);
        assert_eq!(fs::read_to_string(b.join("src/a.rs")).unwrap(), "fn a() {}");
        let mode = fs::metadata(b.join("README")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
        assert!(no_temp_files(&b));

        // Archives são somente leitura.
        let moving = Operation::transfer(OpKind::Move, vec![virt("README")], &b, None).unwrap();
        assert!(moving.reading_archive(&zip).is_err());
    }

    #[test]
    fn never_extracts_symlinks_from_archives() {
        let (tmp, _a, b) = setup();
        let tar = tmp.path().join("p.tar");
        archive::tests::make_tar(&tar, false);
        let link = Source {
            path: tar.join("etc-link"),
            size: 0,
        };
        let mut op = Operation::transfer(OpKind::Copy, vec![link], &b, None)
            .unwrap()
            .reading_archive(&tar)
            .unwrap();
        run(&mut op);
        assert_eq!(op.errors.len(), 1);
        assert!(fs::symlink_metadata(b.join("etc-link")).is_err());
        assert!(no_temp_files(&b));
    }

    #[test]
    fn resolves_paths() {
        let base = Path::new("/base");
        assert_eq!(resolve_path("sub", base), Path::new("/base/sub"));
        assert_eq!(resolve_path(" /abs ", base), Path::new("/abs"));
        let home = PathBuf::from(std::env::var_os("HOME").unwrap());
        assert_eq!(resolve_path("~", base), home);
        assert_eq!(resolve_path("~/x", base), home.join("x"));
        assert_eq!(resolve_path("~x", base), Path::new("/base/~x"));
        assert_eq!(resolve_path("../up/./x", base), Path::new("/up/x"));
    }

    #[test]
    fn validates_names_and_modes() {
        assert!(check_name("ok.txt").is_ok());
        assert!(check_name("").is_err());
        assert!(check_name("..").is_err());
        assert!(check_name("a/b").is_err());
        assert_eq!(parse_mode("644"), Ok(0o644));
        assert_eq!(parse_mode("4755"), Ok(0o4755));
        assert!(parse_mode("8").is_err());
        assert!(parse_mode("rwx").is_err());
    }
}
