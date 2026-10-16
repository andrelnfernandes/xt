//! File operations: copy, move and delete.
//!
//! An [`Operation`] runs in small steps (`step`), so the UI can show progress,
//! ask about conflicts and allow cancelling. Files are copied in blocks to a
//! temporary file, which replaces the destination only at the end:
//! a cancellation or error never leaves a half-written destination.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use crate::archive;
use crate::platform;

const CHUNK: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Copy,
    Move,
    Delete,
    /// Deletes a branch: files and then directories, deepest first.
    Prune,
}

impl OpKind {
    pub fn verb(self) -> &'static str {
        match self {
            OpKind::Copy => "Copying",
            OpKind::Move => "Moving",
            OpKind::Delete => "Deleting",
            OpKind::Prune => "Pruning",
        }
    }

    pub fn past(self) -> &'static str {
        match self {
            OpKind::Copy => "Copied",
            OpKind::Move => "Moved",
            OpKind::Delete => "Deleted",
            OpKind::Prune => "Pruned",
        }
    }
}

/// A source file, with its size already known (for progress).
#[derive(Debug, Clone)]
pub struct Source {
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone)]
struct Item {
    src: PathBuf,
    /// Empty on Delete.
    dst: PathBuf,
    size: u64,
    /// Directory to remove (Prune only; it will already be empty).
    is_dir: bool,
}

/// User's answer to a conflict ("the destination already exists").
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

/// Origin of the files when they come from inside a ZIP/TAR.
struct ArchiveSource {
    reader: archive::Reader,
    /// Path of the archive: root of the virtual paths.
    root: PathBuf,
    entries: HashMap<PathBuf, archive::Entry>,
}

/// A copy of a file in progress.
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
    /// Item `next` already had its conflict resolved with "yes".
    approved: bool,
    /// Conflicting destination, waiting for [`Operation::answer`].
    pub conflict: Option<PathBuf>,
    pub to_trash: bool,
    /// Successfully trashed paths, for Oops restoration.
    pub trashed: Vec<PathBuf>,
    pub done_files: usize,
    pub skipped: usize,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub errors: Vec<String>,
    /// The source files are inside this archive.
    archive: Option<ArchiveSource>,
    /// Tests only: pretends source and destination are on different disks.
    #[cfg(test)]
    force_cross_device: bool,
}

impl Operation {
    pub fn delete(sources: Vec<Source>, to_trash: bool) -> Self {
        let items = sources
            .into_iter()
            .map(|s| Item {
                src: s.path,
                dst: PathBuf::new(),
                size: s.size,
                is_dir: false,
            })
            .collect();
        let mut op = Self::new(OpKind::Delete, items);
        op.to_trash = to_trash;
        op
    }

    /// Prepares a Prune: deletes `files` and then `dirs`, deepest first
    /// (each one will already be empty by its turn).
    pub fn prune(files: Vec<Source>, mut dirs: Vec<PathBuf>) -> Self {
        dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
        let mut op = Self::delete(files, false);
        op.kind = OpKind::Prune;
        op.items.extend(dirs.into_iter().map(|d| Item {
            src: d,
            dst: PathBuf::new(),
            size: 0,
            is_dir: true,
        }));
        op
    }

    /// Prepares a copy or move to `dest`. If `dest` is an existing directory,
    /// the files go inside it; with a single file, `dest` can also be the new
    /// name.
    ///
    /// With `paths_from`, each file goes to the same path relative to that
    /// directory (`base/a/b/x` → `dest/a/b/x`), creating whatever is missing.
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
                        is_dir: false,
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
                is_dir: false,
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
            to_trash: false,
            trashed: Vec::new(),
            done_files: 0,
            skipped: 0,
            done_bytes: 0,
            errors: Vec::new(),
            #[cfg(test)]
            force_cross_device: false,
            archive: None,
        }
    }

    /// Marks the sources as virtual paths inside `archive`
    /// (e.g. `/x/p.zip/src/a.rs`). Copy only: archives are
    /// read-only.
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

    /// File being processed right now.
    pub fn current(&self) -> Option<&Path> {
        self.items.get(self.next).map(|i| i.src.as_path())
    }

    /// Directories whose contents may have changed.
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

    /// Stops the operation, deleting the temporary file of a copy in progress.
    pub fn cancel(&mut self) {
        if let Some(t) = self.transfer.take() {
            drop(t.writer);
            let _ = fs::remove_file(&t.tmp);
        }
        self.next = self.items.len();
        self.conflict = None;
    }

    /// Works until done, `budget` runs out or an answer is needed.
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

        if matches!(self.kind, OpKind::Delete | OpKind::Prune) {
            let removed = if self.to_trash {
                platform::trash_file(&item.src)
            } else if item.is_dir {
                fs::remove_dir(&item.src)
            } else {
                platform::remove_file(&item.src)
            };
            match removed {
                Ok(()) => {
                    if self.to_trash {
                        self.trashed.push(item.src);
                    }
                    self.done_bytes += item.size;
                    self.finish_item();
                }
                Err(e) => self.fail(&item.src, e),
            }
            return;
        }

        // Inside an archive there is no metadata on disk.
        let src_meta = match self.archive {
            Some(_) => None,
            None => match fs::symlink_metadata(&item.src) {
                Ok(m) => Some(m),
                Err(e) => return self.fail(&item.src, e),
            },
        };
        if let Ok(dst_meta) = fs::symlink_metadata(&item.dst) {
            if let Some(src_meta) = &src_meta
                && let Some(dst_id) = platform::file_id(&item.dst, &dst_meta)
                && platform::file_id(&item.src, src_meta) == Some(dst_id)
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
                Err(e) if e.kind() == ErrorKind::CrossesDevices => {} // copy and delete
                Err(e) => return self.fail(&item.src, e),
            }
        }

        let ft = src_meta.file_type();
        if ft.is_symlink() {
            // Copies the link, not the file it points to.
            let result = fs::read_link(&item.src).and_then(|target| {
                let tmp = tmp_path(&item.dst);
                let is_dir = fs::metadata(&item.src).is_ok_and(|m| m.is_dir());
                platform::symlink(&target, &tmp, is_dir)?;
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

    /// Extracts an entry from the archive to the destination, via a
    /// temporary file, with the permissions and date stored in the archive.
    fn extract_item(&mut self, item: &Item) {
        let source = self.archive.as_mut().expect("archive source");
        let entry = item.src.strip_prefix(&source.root).unwrap_or(&item.src);
        let info = source.entries.get(entry).cloned();
        let tmp = tmp_path(&item.dst);
        let result = (|| {
            let mut file = File::create(&tmp)?;
            source.reader.extract(entry, &mut file, u64::MAX)?;
            if let Some(info) = &info {
                platform::set_file_mode(&file, info.mode)?;
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

    /// End of reading: copies permissions and date, and puts the file in place.
    fn finish_transfer(&mut self) {
        let t = self.transfer.take().unwrap();
        let item = self.items[self.next].clone();
        let result = (|| {
            let meta = t.reader.metadata()?;
            platform::set_file_mode(&t.writer, platform::unix_mode(&meta) & 0o7777)?;
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

    /// Copy done; on a cross-disk Move, the source still needs to be deleted.
    fn complete_copy(&mut self, src: &Path) {
        if self.kind == OpKind::Move
            && let Err(e) = platform::remove_file(src)
        {
            self.errors
                .push(format!("{}: copied but not removed: {e}", src.display()));
            self.advance();
            return;
        }
        self.finish_item();
    }

    /// Summary to show at the end.
    pub fn summary(&self) -> String {
        let noun = match self.kind {
            OpKind::Prune => "files and directories",
            _ => "files",
        };
        let action = if self.to_trash {
            "Trashed"
        } else {
            self.kind.past()
        };
        let mut s = format!(
            "{} {} of {} {noun}",
            action,
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

/// Hidden temporary file, in the same directory as the destination (so the final
/// rename is atomic).
fn tmp_path(dst: &Path) -> PathBuf {
    let name = dst.file_name().unwrap_or_default().to_string_lossy();
    dst.with_file_name(format!(".{name}.xt-{}.tmp", std::process::id()))
}

/// Interprets the typed destination: `~` becomes home, relative paths
/// start from `base`, and `.`/`..` are resolved in the text.
pub fn resolve_path(input: &str, base: &Path) -> PathBuf {
    let input = input.trim();
    let path = match (input.strip_prefix('~'), platform::home_dir()) {
        (Some(rest), Some(home))
            if rest.is_empty() || rest.starts_with(std::path::is_separator) =>
        {
            home.join(rest.trim_start_matches(std::path::is_separator))
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

/// Validates a typed file or directory name.
pub fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." {
        Err("Invalid name".into())
    } else if let Some(c) = name.chars().find(|&c| std::path::is_separator(c)) {
        Err(format!("Name cannot contain '{c}'"))
    } else {
        platform::check_name(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
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
        let tmp = crate::testing::tempdir();
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

    #[cfg(unix)]
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

        // Without paths_from, everything goes straight to the destination.
        let flat = tmp.path().join("flat");
        fs::create_dir(&flat).unwrap();
        let mut op = Operation::transfer(OpKind::Copy, sources.clone(), &flat, None).unwrap();
        run(&mut op);
        assert!(flat.join("deep.txt").exists());

        // With paths, the destination must be an existing directory.
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

    #[cfg(unix)]
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

    #[cfg(unix)]
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
        let mut op = Operation::delete(sources, false);
        run(&mut op);
        assert!(!a.join("one.txt").exists());
        assert_eq!(op.done_files, 1);
        assert_eq!(op.errors.len(), 1);
        assert!(op.summary().starts_with("Deleted 1 of 2 files. Error:"));
    }

    #[test]
    fn trashes_files() {
        let (_tmp, a, _b) = setup();
        let file = a.join("one.txt");
        let sources = vec![src(&file)];
        let mut op = Operation::delete(sources, true);
        run(&mut op);
        assert!(!file.exists());
        assert_eq!(op.done_files, 1);
        assert_eq!(op.trashed, vec![file.clone()]);
        assert_eq!(op.summary(), "Trashed 1 of 1 files");

        // Restore using platform::restore_trash_paths
        let restored = platform::restore_trash_paths(std::slice::from_ref(&file)).unwrap();
        assert_eq!(restored, 1);
        assert!(file.exists());
    }

    #[test]
    fn large_copy_runs_in_chunks_and_cancel_cleans_up() {
        let (_tmp, a, b) = setup();
        let big = a.join("big.bin");
        fs::write(&big, vec![7u8; CHUNK * 3 + 10]).unwrap();

        let mut op = Operation::transfer(OpKind::Copy, vec![src(&big)], &b, None).unwrap();
        op.step(Duration::ZERO); // opens the files
        op.step(Duration::ZERO); // first block
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

    #[cfg(unix)]
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

        // Archives are read-only.
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

    #[cfg(unix)]
    #[test]
    fn prunes_files_then_dirs_deepest_first() {
        let (tmp, a, _b) = setup();
        fs::create_dir_all(a.join("x/y")).unwrap();
        fs::write(a.join("x/y/z.txt"), "z").unwrap();
        symlink(tmp.path(), a.join("x/link-up")).unwrap();
        let files = ["one.txt", "two.txt", "x/y/z.txt", "x/link-up"]
            .map(|f| src(&a.join(f)))
            .to_vec();
        let dirs = vec![a.clone(), a.join("x"), a.join("x/y")];
        let mut op = Operation::prune(files, dirs);
        run(&mut op);
        assert!(op.errors.is_empty(), "{:?}", op.errors);
        assert_eq!(op.done_files, 7);
        assert!(!a.exists());
        // The link was removed, not what it points to.
        assert!(tmp.path().join("b").exists());
        assert!(
            op.summary()
                .starts_with("Pruned 7 of 7 files and directories")
        );
    }

    #[test]
    fn resolves_paths() {
        let base = Path::new("/base");
        assert_eq!(resolve_path("sub", base), Path::new("/base/sub"));
        assert_eq!(resolve_path(" /abs ", base), Path::new("/abs"));
        let home = platform::home_dir().unwrap();
        assert_eq!(resolve_path("~", base), home);
        assert_eq!(resolve_path("~/x", base), home.join("x"));
        assert_eq!(resolve_path("~x", base), Path::new("/base/~x"));
        assert_eq!(resolve_path("../up/./x", base), Path::new("/up/x"));
    }

    #[test]
    fn validates_names() {
        assert!(check_name("ok.txt").is_ok());
        assert!(check_name("").is_err());
        assert!(check_name("..").is_err());
        assert!(check_name("a/b").is_err());
    }
}
