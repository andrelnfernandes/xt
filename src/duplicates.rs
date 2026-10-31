//! Duplicates (Alt+Y): files with the same contents, whatever their names, and
//! whole folders with the same files. The sizes come from the tree in memory;
//! only files of a size shared with another one are read, in a thread of its
//! own, and a group is split as soon as a part of its files differs.
//!
//! Each round hashes the next part of every file of a group and separates them
//! by the result, so a group of any size takes one open file at a time and 32
//! bytes per file. Files are hostile input like any other: a file swapped for
//! a link or a pipe after the tree read it is left out, not followed.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::model::tree::{FileKind, NodeId, Tree};
use crate::platform;

/// Read from each end of a file first: most files of the same size already
/// differ there (headers, trailers).
const ENDS: u64 = 64 * 1024;
/// How much of each file of a group is read before comparing them again.
const ROUND: u64 = 8 * 1024 * 1024;
const BUFFER: usize = 256 * 1024;
/// The class of empty files, which need no reading to be equal.
const EMPTY: u64 = u64::MAX;

type Hash = [u8; 32];

/// A regular file of the analysis.
#[derive(Debug, Clone)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    disk: u64,
    modified: Option<SystemTime>,
    /// Inode, when it has other hard links.
    link: Option<u64>,
}

/// A folder of the analysis; `parent` comes before it in [`Scan::folders`].
#[derive(Debug)]
struct Folder {
    path: PathBuf,
    parent: Option<usize>,
    files: Vec<(OsString, usize)>,
    /// Symbolic links: their names, and where they point once read.
    links: Vec<(OsString, Option<PathBuf>)>,
    children: Vec<(OsString, usize)>,
    /// Everything in it is known and counted, so it can be called equal to
    /// another. Not so with an unreadable or ignored folder inside, another
    /// disk mounted inside, or a special file: deleting a "copy" would lose
    /// what was not compared.
    comparable: bool,
}

/// What the tree knows about a branch, taken before going to the thread.
#[derive(Debug)]
pub struct Scan {
    files: Vec<FileInfo>,
    folders: Vec<Folder>,
    min_size: u64,
}

impl Scan {
    /// The branch of `root`, on one disk. Folders named in `ignore` are left
    /// out (and whatever holds them cannot be called equal to anything).
    pub fn of(tree: &Tree, root: NodeId, min_size: u64, ignore: &[String]) -> Scan {
        let mut scan = Scan {
            files: Vec::new(),
            folders: Vec::new(),
            min_size,
        };
        let device = tree.device(root);
        let mut stack: Vec<(NodeId, Option<(usize, OsString)>)> = vec![(root, None)];
        while let Some((id, parent)) = stack.pop() {
            let node = tree.node(id);
            let at = scan.folders.len();
            let mut folder = Folder {
                path: tree.path(id),
                parent: parent.as_ref().map(|(p, _)| *p),
                files: Vec::new(),
                links: Vec::new(),
                children: Vec::new(),
                comparable: node.scanned && node.error.is_none(),
            };
            if let Some((p, name)) = parent {
                scan.folders[p].children.push((name, at));
            }
            for f in &node.files {
                match f.kind {
                    FileKind::File => {
                        folder.files.push((f.name.clone(), scan.files.len()));
                        scan.files.push(FileInfo {
                            path: folder.path.join(&f.name),
                            size: f.size,
                            disk: f.disk,
                            modified: f.modified,
                            link: f.link,
                        });
                    }
                    FileKind::Symlink => folder.links.push((f.name.clone(), None)),
                    FileKind::Other => folder.comparable = false,
                }
            }
            for &c in node.children.iter().rev() {
                let child = tree.node(c);
                let name = child.name.to_string_lossy();
                let other_disk = device.is_some() && tree.device(c) != device;
                if other_disk || ignore.iter().any(|i| *i == name) {
                    folder.comparable = false;
                } else {
                    stack.push((c, Some((at, child.name.clone()))));
                }
            }
            scan.folders.push(folder);
        }
        for folder in &mut scan.folders {
            folder.files.sort_by(|a, b| a.0.cmp(&b.0));
            folder.links.sort_by(|a, b| a.0.cmp(&b.0));
            folder.children.sort_by(|a, b| a.0.cmp(&b.0));
        }
        scan
    }

    /// For each file, the first file found with the same inode (itself when
    /// it has no other hard link here): those share their space, so they are
    /// one file, never a duplicate of each other.
    fn first_links(&self) -> Vec<usize> {
        let mut seen = HashMap::new();
        self.files
            .iter()
            .enumerate()
            .map(|(i, f)| match f.link {
                Some(ino) => *seen.entry(ino).or_insert(i),
                None => i,
            })
            .collect()
    }

    /// The files among `which` that share a size with another one.
    fn same_size(&self, which: impl Iterator<Item = usize>, links: &[usize]) -> Vec<Vec<usize>> {
        let mut by_size: HashMap<u64, Vec<usize>> = HashMap::new();
        for i in which {
            let f = &self.files[i];
            if links[i] == i && f.size > 0 {
                by_size.entry(f.size).or_default().push(i);
            }
        }
        let mut sets: Vec<Vec<usize>> = by_size.into_values().filter(|s| s.len() > 1).collect();
        // Largest first: the most to gain comes first if it is cancelled.
        sets.sort_by_key(|s| std::cmp::Reverse(self.files[s[0]].size));
        sets
    }

    fn big_sets(&self, links: &[usize]) -> Vec<Vec<usize>> {
        let big = (0..self.files.len()).filter(|&i| self.files[i].size >= self.min_size);
        self.same_size(big, links)
    }

    /// How much comparing the files would read at most: for the question
    /// asked before reading through a network.
    pub fn to_read(&self) -> u64 {
        self.big_sets(&self.first_links())
            .iter()
            .map(|s| self.files[s[0]].size * s.len() as u64)
            .sum()
    }

    /// The hash of each folder from what is known of it: the names and sizes
    /// in it, the classes of its files, and the same for the folders inside.
    /// `None` for what cannot be called equal to anything.
    fn signatures(&self, class: &[Option<u64>]) -> Vec<Option<Hash>> {
        let mut signs: Vec<Option<Hash>> = vec![None; self.folders.len()];
        for (at, folder) in self.folders.iter().enumerate().rev() {
            if !folder.comparable {
                continue;
            }
            let mut hash = Sha256::new();
            let name = |hash: &mut Sha256, kind: &[u8], name: &OsString| {
                let bytes = name.as_encoded_bytes();
                hash.update(kind);
                hash.update((bytes.len() as u64).to_le_bytes());
                hash.update(bytes);
            };
            for (n, f) in &folder.files {
                name(&mut hash, b"f", n);
                hash.update(self.files[*f].size.to_le_bytes());
                hash.update(class[*f].map_or([0xff; 9], |c| {
                    let mut out = [0; 9];
                    out[1..].copy_from_slice(&c.to_le_bytes());
                    out
                }));
            }
            for (n, target) in &folder.links {
                name(&mut hash, b"l", n);
                if let Some(t) = target {
                    let bytes = t.as_os_str().as_encoded_bytes();
                    hash.update((bytes.len() as u64).to_le_bytes());
                    hash.update(bytes);
                }
            }
            let mut whole = true;
            for (n, c) in &folder.children {
                match signs[*c] {
                    Some(sign) => {
                        name(&mut hash, b"d", n);
                        hash.update(sign);
                    }
                    None => whole = false,
                }
            }
            if whole {
                signs[at] = Some(hash.finalize().into());
            }
        }
        signs
    }

    /// The files of a folder and of the folders inside it.
    fn files_below(&self, at: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![at];
        while let Some(n) = stack.pop() {
            let folder = &self.folders[n];
            out.extend(folder.files.iter().map(|(_, f)| *f));
            stack.extend(folder.children.iter().map(|(_, c)| *c));
        }
        out
    }

    /// The folder and the folders inside it.
    fn folders_below(&self, at: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![at];
        while let Some(n) = stack.pop() {
            out.push(n);
            stack.extend(self.folders[n].children.iter().map(|(_, c)| *c));
        }
        out
    }

    /// What a walk of the folder on the disk should find ([`Snapshot::read`]).
    fn snapshot(&self, at: usize) -> Snapshot {
        let mut snap = Snapshot::default();
        for n in self.folders_below(at) {
            snap.files += self.folders[n].links.len() as u64;
        }
        for f in self.files_below(at) {
            let file = &self.files[f];
            snap.files += 1;
            snap.bytes += file.size;
            snap.newest = snap.newest.max(file.modified);
        }
        snap
    }
}

/// What a folder held at the analysis, to see that it did not change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Snapshot {
    files: u64,
    bytes: u64,
    newest: Option<SystemTime>,
}

impl Snapshot {
    /// Walks the folder without following links. `None` if it cannot.
    fn read(path: &Path) -> Option<Snapshot> {
        let mut snap = Snapshot::default();
        let mut stack = vec![path.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).ok()? {
                let entry = entry.ok()?;
                let meta = std::fs::symlink_metadata(entry.path()).ok()?;
                if meta.is_dir() {
                    stack.push(entry.path());
                } else if meta.is_file() {
                    snap.files += 1;
                    snap.bytes += meta.len();
                    snap.newest = snap.newest.max(meta.modified().ok());
                } else {
                    snap.files += 1;
                }
            }
        }
        Some(snap)
    }
}

/// One file or folder of a group of equals.
#[derive(Debug, Clone)]
pub struct Item {
    pub path: PathBuf,
    pub size: u64,
    pub disk: u64,
    pub modified: Option<SystemTime>,
    pub tagged: bool,
    /// A folder's contents as of the analysis.
    pub snapshot: Option<Snapshot>,
    /// In the cloud, shared by someone else: deleting it frees nothing of
    /// the user's own, so tagging all but one leaves it alone.
    pub shared: bool,
    /// The date on the disk can be compared with `modified` (not through a
    /// mount, which may round it).
    pub check_date: bool,
}

impl Item {
    /// It is still as it was analyzed.
    fn unchanged(&self) -> bool {
        match &self.snapshot {
            Some(snap) => Snapshot::read(&self.path).as_ref() == Some(snap),
            None => std::fs::symlink_metadata(&self.path).is_ok_and(|m| {
                m.is_file()
                    && m.len() == self.size
                    && (!self.check_date || m.modified().ok() == self.modified)
            }),
        }
    }
}

/// Files or folders with the same contents.
#[derive(Debug, Clone)]
pub struct Group {
    pub items: Vec<Item>,
    /// What removing all but one would free.
    pub free: u64,
}

impl Group {
    fn new(mut items: Vec<Item>) -> Group {
        items.sort_by(|a, b| a.path.cmp(&b.path));
        let total: u64 = items.iter().map(|i| i.disk).sum();
        let largest = items.iter().map(|i| i.disk).max().unwrap_or(0);
        Group {
            items,
            free: total - largest,
        }
    }
}

/// What could not be compared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Notes {
    pub unreadable: u64,
    /// Changed on the disk since the tree read them.
    pub changed: u64,
    /// In the cloud: files the service gives no hash for (Google Docs...).
    pub no_hash: u64,
    /// In the cloud: names repeated in one folder, which a mount cannot tell
    /// apart.
    pub same_name: u64,
    /// In the cloud: other names of a file that is one (same ID).
    pub same_id: u64,
}

/// The result of an analysis.
#[derive(Debug, Default)]
pub struct Found {
    pub files: Vec<Group>,
    pub folders: Vec<Group>,
    pub notes: Notes,
    /// Compared by the hashes of this rclone remote, without reading.
    pub cloud: Option<crate::rclone::Remote>,
}

/// How far the thread went, for the footer.
#[derive(Debug, Default)]
pub struct Progress {
    pub read: AtomicU64,
    pub total: AtomicU64,
    pub current: Mutex<String>,
    /// Files listed by rclone, while nothing is being read.
    pub listed: AtomicU64,
    pub listing: AtomicBool,
}

/// Why the thread did not find anything.
#[derive(Debug, PartialEq, Eq)]
pub enum Fail {
    Cancelled,
    /// Why it stopped.
    Stopped(String),
    /// The cloud's hashes cannot be had: reading is the way left.
    NoHashes(String),
}

/// The analysis running in its thread.
pub struct Finder {
    rx: mpsc::Receiver<Result<Found, Fail>>,
    pub progress: Arc<Progress>,
    cancel: Arc<AtomicBool>,
    /// The rclone running, to stop it.
    child: crate::rclone::Running,
}

impl Finder {
    fn run(
        work: impl FnOnce(
            &Arc<Progress>,
            &Arc<AtomicBool>,
            &crate::rclone::Running,
        ) -> Result<Found, Fail>
        + Send
        + 'static,
    ) -> Finder {
        let (tx, rx) = mpsc::channel();
        let progress = Arc::new(Progress::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let child = crate::rclone::Running::default();
        let (p, c, r) = (progress.clone(), cancel.clone(), child.clone());
        std::thread::spawn(move || {
            let _ = tx.send(work(&p, &c, &r));
        });
        Finder {
            rx,
            progress,
            cancel,
            child,
        }
    }

    pub fn start(scan: Scan) -> Finder {
        Finder::run(move |progress, cancel, _| {
            Worker {
                progress: progress.clone(),
                cancel: cancel.clone(),
                buffer: vec![0; BUFFER],
                notes: Notes::default(),
            }
            .find(scan)
        })
    }

    /// Compares the files of a folder mounted by rclone by the hashes the
    /// service keeps, without downloading them. `dir` is the folder on this
    /// machine, and `spec` the same folder as the remote calls it.
    pub fn start_cloud(
        remote: crate::rclone::Remote,
        spec: String,
        dir: PathBuf,
        min_size: u64,
        ignore: Vec<String>,
    ) -> Finder {
        Finder::run(move |progress, cancel, child| {
            progress.listing.store(true, Ordering::Relaxed);
            let found = crate::rclone::list_hashes(
                "rclone", &remote, &spec, &dir, min_size, &ignore, progress, cancel, child,
            );
            if cancel.load(Ordering::Relaxed) {
                return Err(Fail::Cancelled);
            }
            found
        })
    }

    /// The result, once there is one.
    pub fn poll(&self) -> Option<Result<Found, Fail>> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err(Fail::Stopped("the analysis stopped".into())))
            }
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.child.stop();
    }
}

impl Drop for Finder {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Why a file left its group.
enum Left {
    Cancelled,
    Unreadable,
    Changed,
}

struct Worker {
    progress: Arc<Progress>,
    cancel: Arc<AtomicBool>,
    buffer: Vec<u8>,
    notes: Notes,
}

impl Worker {
    fn find(&mut self, mut scan: Scan) -> Result<Found, Fail> {
        let links = scan.first_links();
        let min_size = scan.min_size;
        // First, the files large enough to show.
        let sets = self.compare(&scan, scan.big_sets(&links))?;
        let mut class: Vec<Option<u64>> = scan
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| match f.size {
                0 => Some(EMPTY),
                s if s >= min_size => Some(i as u64),
                _ => None,
            })
            .collect();
        let mark = |class: &mut Vec<Option<u64>>, sets: &[Vec<usize>]| {
            for set in sets {
                for &m in set {
                    class[m] = Some(set[0] as u64);
                }
            }
            for (i, &first) in links.iter().enumerate() {
                if first != i {
                    class[i] = class[first];
                }
            }
        };
        mark(&mut class, &sets);
        let files = sets
            .iter()
            .map(|set| {
                Group::new(
                    set.iter()
                        .map(|&i| {
                            let f = &scan.files[i];
                            Item {
                                path: f.path.clone(),
                                size: f.size,
                                disk: f.disk,
                                modified: f.modified,
                                tagged: false,
                                snapshot: None,
                                shared: false,
                                check_date: true,
                            }
                        })
                        .collect(),
                )
            })
            .collect();

        // Then the folders: names and sizes first, and only what could still
        // be equal has its small files and links read.
        let mut bytes = vec![0u64; scan.folders.len()];
        for (at, folder) in scan.folders.iter().enumerate().rev() {
            bytes[at] += folder
                .files
                .iter()
                .map(|(_, f)| scan.files[*f].size)
                .sum::<u64>();
            if let Some(parent) = folder.parent {
                bytes[parent] += bytes[at];
            }
        }
        let mut candidates: HashMap<Hash, Vec<usize>> = HashMap::new();
        for (at, sign) in scan.signatures(&class).into_iter().enumerate() {
            if let Some(sign) = sign
                && bytes[at] >= min_size.max(1)
            {
                candidates.entry(sign).or_default().push(at);
            }
        }
        candidates.retain(|_, members| members.len() > 1);
        let mut inside = HashSet::new();
        for members in candidates.values() {
            for &m in members {
                inside.extend(scan.folders_below(m));
            }
        }
        for &at in &inside {
            let folder = &mut scan.folders[at];
            for (name, target) in &mut folder.links {
                *target = std::fs::read_link(folder.path.join(name.as_os_str())).ok();
            }
            if folder.links.iter().any(|(_, target)| target.is_none()) {
                folder.comparable = false;
            }
        }
        let small = inside
            .iter()
            .flat_map(|&at| scan.folders[at].files.iter().map(|(_, f)| *f))
            .filter(|&f| scan.files[f].size < min_size);
        let small_sets = scan.same_size(small, &links);
        for set in &small_sets {
            for &m in set {
                class[m] = Some(m as u64);
            }
        }
        // A small file that shares its size with nothing is unique too.
        for &at in &inside {
            for (_, f) in &scan.folders[at].files {
                if class[*f].is_none() {
                    class[*f] = Some(*f as u64);
                }
            }
        }
        let small_sets = self.compare(&scan, small_sets)?;
        mark(&mut class, &small_sets);

        let signs = scan.signatures(&class);
        let mut equal: HashMap<Hash, Vec<usize>> = HashMap::new();
        for members in candidates.values() {
            for &m in members {
                if let Some(sign) = signs[m] {
                    equal.entry(sign).or_default().push(m);
                }
            }
        }
        equal.retain(|_, members| members.len() > 1);
        let folders = equal
            .values()
            // Inside folders that are already equal as a whole: nothing new.
            .filter(|members| {
                let parents: HashSet<Option<Hash>> = members
                    .iter()
                    .map(|&m| {
                        scan.folders[m]
                            .parent
                            .and_then(|p| signs[p].filter(|s| equal.contains_key(s)))
                    })
                    .collect();
                parents.len() != 1 || parents.contains(&None)
            })
            .filter_map(|members| folder_group(&scan, members))
            .collect();
        Ok(Found {
            files: sorted(files),
            folders: sorted(folders),
            notes: self.notes,
            cloud: None,
        })
    }

    /// Splits `sets` of files of the same size into the sets with the same
    /// contents.
    fn compare(&mut self, scan: &Scan, sets: Vec<Vec<usize>>) -> Result<Vec<Vec<usize>>, Fail> {
        let sum: u64 = sets
            .iter()
            .map(|s| scan.files[s[0]].size * s.len() as u64)
            .sum();
        self.progress.total.fetch_add(sum, Ordering::Relaxed);
        let mut equal = Vec::new();
        for set in sets {
            let size = scan.files[set[0]].size;
            let parts = parts(size);
            let start: Vec<(usize, Hash)> = set.into_iter().map(|i| (i, [0; 32])).collect();
            let mut work = vec![(start, 0)];
            while let Some((members, at)) = work.pop() {
                let Some(&part) = parts.get(at) else {
                    equal.push(members.into_iter().map(|(i, _)| i).collect());
                    continue;
                };
                // What this file would still have read, past this part.
                let later: u64 = parts[at + 1..].iter().map(|p| p.1).sum();
                let mut split: HashMap<Hash, Vec<(usize, Hash)>> = HashMap::new();
                for (i, before) in members {
                    match self.absorb(&scan.files[i], part, &before) {
                        Ok(hash) => split.entry(hash).or_default().push((i, hash)),
                        Err(Left::Cancelled) => return Err(Fail::Cancelled),
                        Err(left) => {
                            match left {
                                Left::Unreadable => self.notes.unreadable += 1,
                                _ => self.notes.changed += 1,
                            }
                            self.forget(later + part.1);
                        }
                    }
                }
                for part in split.into_values() {
                    if part.len() > 1 {
                        work.push((part, at + 1));
                    } else {
                        self.forget(later);
                    }
                }
            }
        }
        Ok(equal)
    }

    /// Takes off the total what will not be read after all.
    fn forget(&self, bytes: u64) {
        let total = self.progress.total.load(Ordering::Relaxed);
        self.progress
            .total
            .store(total.saturating_sub(bytes), Ordering::Relaxed);
    }

    /// The hash of `before` and of the part `(start, len)` of the file.
    fn absorb(
        &mut self,
        f: &FileInfo,
        (start, len): (u64, u64),
        before: &Hash,
    ) -> Result<Hash, Left> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(Left::Cancelled);
        }
        if let Ok(mut current) = self.progress.current.lock() {
            current.clone_from(&f.path.to_string_lossy().into_owned());
        }
        let mut file = platform::open_no_follow(&f.path).map_err(|_| Left::Unreadable)?;
        let meta = file.metadata().map_err(|_| Left::Unreadable)?;
        if !meta.is_file() || meta.len() != f.size || meta.modified().ok() != f.modified {
            return Err(Left::Changed);
        }
        file.seek(SeekFrom::Start(start))
            .map_err(|_| Left::Unreadable)?;
        let mut hash = Sha256::new();
        hash.update(before);
        let mut left = len;
        while left > 0 {
            if self.cancel.load(Ordering::Relaxed) {
                return Err(Left::Cancelled);
            }
            let want = left.min(self.buffer.len() as u64) as usize;
            let n = file
                .read(&mut self.buffer[..want])
                .map_err(|_| Left::Unreadable)?;
            if n == 0 {
                return Err(Left::Changed);
            }
            hash.update(&self.buffer[..n]);
            left -= n as u64;
            self.progress.read.fetch_add(n as u64, Ordering::Relaxed);
        }
        Ok(hash.finalize().into())
    }
}

/// The order a file is read in, as `(start, length)`: both ends, then the
/// middle in rounds.
fn parts(size: u64) -> Vec<(u64, u64)> {
    let head = size.min(ENDS);
    let mut parts = vec![(0, head)];
    if size > head {
        let tail = (size - head).min(ENDS);
        parts.push((size - tail, tail));
        let mut at = head;
        let end = size - tail;
        while at < end {
            let len = (end - at).min(ROUND);
            parts.push((at, len));
            at += len;
        }
    }
    parts
}

/// A group of equal folders, unless removing one would free nothing because
/// their files are hard links of each other.
fn folder_group(scan: &Scan, members: &[usize]) -> Option<Group> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let mut unique = 0;
    for &m in members {
        let mut size = 0;
        let mut disk = 0;
        let mut newest = None;
        for f in scan.files_below(m) {
            let file = &scan.files[f];
            size += file.size;
            disk += file.disk;
            newest = newest.max(file.modified);
            if file.link.is_none_or(|ino| seen.insert(ino)) {
                unique += file.disk;
            }
        }
        items.push(Item {
            path: scan.folders[m].path.clone(),
            size,
            disk,
            modified: newest,
            tagged: false,
            snapshot: Some(scan.snapshot(m)),
            shared: false,
            check_date: true,
        });
    }
    let mut group = Group::new(items);
    let largest = group.items.iter().map(|i| i.disk).max().unwrap_or(0);
    group.free = unique.max(largest) - largest;
    (group.free > 0 || unique == 0).then_some(group)
}

/// Groups what rclone listed. `shared` has the IDs of what others shared.
pub fn cloud_groups(
    dir: &Path,
    listed: Vec<crate::rclone::Listed>,
    shared: &HashSet<String>,
    notes: &mut Notes,
) -> Vec<Group> {
    let mut by_hash: HashMap<(&'static str, String, u64), Vec<crate::rclone::Listed>> =
        HashMap::new();
    for file in listed {
        if let Some((kind, hash)) = file.hash.clone() {
            by_hash
                .entry((kind, hash, file.size))
                .or_default()
                .push(file);
        }
    }
    let mut groups = Vec::new();
    for (_, files) in by_hash {
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        let mut items = Vec::new();
        for file in files {
            // The same ID twice is one file in two folders: "the other copy"
            // is the only one. Two with one name cannot be told apart through
            // the mount: deleting "that name" could be either.
            if file.id.as_ref().is_some_and(|id| !ids.insert(id.clone())) {
                notes.same_id += 1;
                continue;
            }
            if !names.insert(file.path.clone()) {
                notes.same_name += 1;
                continue;
            }
            items.push(Item {
                path: dir.join(&file.path),
                size: file.size,
                disk: file.size,
                modified: file.modified,
                tagged: false,
                snapshot: None,
                shared: file.id.as_ref().is_some_and(|id| shared.contains(id)),
                check_date: false,
            });
        }
        if items.len() > 1 {
            groups.push(Group::new(items));
        }
    }
    sorted(groups)
}

/// What frees the most first; equal ones in the order of their paths.
fn sorted(mut groups: Vec<Group>) -> Vec<Group> {
    groups.sort_by(|a, b| {
        b.free
            .cmp(&a.free)
            .then_with(|| a.items[0].path.cmp(&b.items[0].path))
    });
    groups
}

/// Which copy of each group stays when tagging the others.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Keep {
    #[default]
    Oldest,
    Newest,
    ShortestPath,
}

impl Keep {
    pub fn next(self) -> Keep {
        match self {
            Keep::Oldest => Keep::Newest,
            Keep::Newest => Keep::ShortestPath,
            Keep::ShortestPath => Keep::Oldest,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Keep::Oldest => "the oldest",
            Keep::Newest => "the newest",
            Keep::ShortestPath => "the shortest path",
        }
    }
}

/// A row of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Group(usize),
    Item(usize, usize),
}

pub struct DuplicatesView {
    /// Where the analysis started.
    pub root: PathBuf,
    pub files: Vec<Group>,
    pub folders: Vec<Group>,
    /// Showing the folders, not the files (`G`).
    pub show_folders: bool,
    pub notes: Notes,
    /// The rclone remote, when the hashes came from it.
    pub cloud: Option<crate::rclone::Remote>,
    /// What the next `A` keeps.
    keep: Keep,
    /// Index of the row under the cursor, and of the first one on screen.
    pub at: usize,
    pub offset: usize,
    /// Height of the list as of the last draw (for PgUp/PgDn).
    pub page: usize,
}

impl DuplicatesView {
    pub fn new(root: PathBuf, found: Found) -> Self {
        DuplicatesView {
            root,
            // Folders explain more at once, but it is the files that always
            // have something to show.
            files: found.files,
            folders: found.folders,
            show_folders: false,
            notes: found.notes,
            cloud: found.cloud,
            keep: Keep::default(),
            at: 0,
            offset: 0,
            page: 10,
        }
    }

    pub fn groups(&self) -> &[Group] {
        if self.show_folders {
            &self.folders
        } else {
            &self.files
        }
    }

    fn groups_mut(&mut self) -> &mut Vec<Group> {
        if self.show_folders {
            &mut self.folders
        } else {
            &mut self.files
        }
    }

    pub fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for (g, group) in self.groups().iter().enumerate() {
            rows.push(Row::Group(g));
            rows.extend((0..group.items.len()).map(|i| Row::Item(g, i)));
        }
        rows
    }

    pub fn selected(&self) -> Option<&Item> {
        match self.rows().get(self.at)? {
            Row::Item(g, i) => self.groups().get(*g)?.items.get(*i),
            Row::Group(g) => self.groups().get(*g)?.items.first(),
        }
    }

    /// Group and item under the cursor (a group's title counts as its first).
    fn cursor(&self) -> Option<(usize, usize)> {
        match self.rows().get(self.at)? {
            Row::Item(g, i) => Some((*g, *i)),
            Row::Group(g) => Some((*g, 0)),
        }
    }

    pub fn move_by(&mut self, delta: isize) {
        let last = self.rows().len().saturating_sub(1) as isize;
        self.at = (self.at as isize + delta).clamp(0, last.max(0)) as usize;
    }

    /// Goes between files and folders.
    pub fn toggle_folders(&mut self) {
        self.show_folders = !self.show_folders;
        self.at = 0;
        self.offset = 0;
    }

    /// Tags or untags the item under the cursor. A group cannot have all its
    /// copies tagged: `false` when this one would be the last.
    pub fn toggle(&mut self) -> bool {
        let Some((g, i)) = self.cursor() else {
            return true;
        };
        let group = &mut self.groups_mut()[g];
        let others = group
            .items
            .iter()
            .enumerate()
            .any(|(n, item)| n != i && !item.tagged);
        let item = &mut group.items[i];
        if !item.tagged && !others {
            return false;
        }
        item.tagged = !item.tagged;
        true
    }

    /// Tags every copy but the one under the cursor.
    pub fn keep_selected(&mut self) {
        let Some((g, i)) = self.cursor() else { return };
        for (n, item) in self.groups_mut()[g].items.iter_mut().enumerate() {
            item.tagged = n != i && !item.shared;
        }
    }

    /// Tags all but one in every group, and says which one stayed; the next
    /// call keeps another one.
    pub fn tag_all_but_one(&mut self) -> Keep {
        let keep = self.keep;
        self.keep = keep.next();
        for group in self.groups_mut() {
            let best = (0..group.items.len())
                .filter(|&i| !group.items[i].shared)
                .min_by_key(|&i| {
                    let item = &group.items[i];
                    // No date goes last, whichever the order.
                    let time = item.modified;
                    let by_date = match keep {
                        Keep::Oldest => time.map(|t| (false, Some(t), None)),
                        Keep::Newest => time.map(|t| (false, None, Some(std::cmp::Reverse(t)))),
                        Keep::ShortestPath => Some((false, None, None)),
                    };
                    let by_date = by_date.unwrap_or((true, None, None));
                    let length = item.path.as_os_str().len();
                    (by_date, length, item.path.clone())
                });
            for (n, item) in group.items.iter_mut().enumerate() {
                item.tagged = Some(n) != best && !item.shared;
            }
        }
        keep
    }

    /// In every group with a copy in the folder of the item under the cursor,
    /// keeps that copy and tags the others. How many groups that was.
    pub fn keep_in_folder(&mut self) -> usize {
        let Some(folder) = self
            .selected()
            .and_then(|i| i.path.parent())
            .map(Path::to_path_buf)
        else {
            return 0;
        };
        let mut count = 0;
        for group in self.groups_mut() {
            let Some(best) = group
                .items
                .iter()
                .position(|i| i.path.parent() == Some(folder.as_path()))
            else {
                continue;
            };
            count += 1;
            for (n, item) in group.items.iter_mut().enumerate() {
                item.tagged = n != best && !item.shared;
            }
        }
        count
    }

    pub fn untag_all(&mut self) {
        for group in self.groups_mut() {
            for item in &mut group.items {
                item.tagged = false;
            }
        }
    }

    /// How many are tagged, and how much space they take.
    pub fn tagged(&self) -> (u64, u64) {
        self.groups()
            .iter()
            .flat_map(|g| &g.items)
            .filter(|i| i.tagged)
            .fold((0, 0), |(n, bytes), i| (n + 1, bytes + i.disk))
    }

    /// What may go, now: the tagged items of the groups where everything is as
    /// analyzed and one copy really stays, out of the tagged ones and of any
    /// tagged folder. The tags of the other groups are removed. Returns the
    /// items and how many groups were refused.
    pub fn check_tagged(&mut self) -> (Vec<Item>, usize) {
        let tagged: Vec<PathBuf> = self
            .groups()
            .iter()
            .flat_map(|g| &g.items)
            .filter(|i| i.tagged)
            .map(|i| i.path.clone())
            .collect();
        let inside_tagged = |path: &Path| tagged.iter().any(|t| path.starts_with(t));
        let mut out = Vec::new();
        let mut refused = 0;
        for group in self.groups_mut() {
            if !group.items.iter().any(|i| i.tagged) {
                continue;
            }
            let stays = group
                .items
                .iter()
                .any(|i| !i.tagged && !inside_tagged(&i.path) && i.unchanged());
            if stays && group.items.iter().filter(|i| i.tagged).all(Item::unchanged) {
                out.extend(group.items.iter().filter(|i| i.tagged).cloned());
            } else {
                refused += 1;
                for item in &mut group.items {
                    item.tagged = false;
                }
            }
        }
        (out, refused)
    }

    /// After deleting or moving: what is gone leaves its group, and a group
    /// with one copy is no longer a group.
    pub fn drop_missing(&mut self) {
        for groups in [&mut self.files, &mut self.folders] {
            groups.retain_mut(|group| {
                let before = group.items.len();
                group
                    .items
                    .retain(|i| std::fs::symlink_metadata(&i.path).is_ok());
                if group.items.len() != before {
                    let items = std::mem::take(&mut group.items);
                    *group = Group::new(items);
                }
                group.items.len() > 1
            });
        }
        self.move_by(0);
    }

    /// What to say after deleting in the cloud.
    pub fn after_delete(&self) -> Option<String> {
        let cloud = self.cloud.as_ref()?;
        (cloud.service.as_deref() == Some("Google Drive")).then(|| {
            format!(
                ". The space only returns when the Drive's trash is emptied: rclone cleanup {}",
                cloud.spec
            )
        })
    }

    /// The list as text, one line per copy and a blank line between groups:
    /// size, date (seconds since 1970), whether it is tagged, and the path.
    /// Control characters in names are escaped, so a name cannot fake a line.
    pub fn export(&self) -> String {
        let mut out = String::new();
        for group in self.groups() {
            for item in &group.items {
                let time = item
                    .modified
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                let path = item.path.to_string_lossy();
                out.push_str(&format!(
                    "{}\t{time}\t{}\t{}\n",
                    item.size,
                    if item.tagged { "T" } else { "-" },
                    crate::bidi::escape(&path),
                ));
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tree::ROOT;
    use std::fs;
    use std::time::Duration;

    fn logged(dir: &Path) -> Tree {
        let mut tree = Tree::new(dir.to_path_buf());
        tree.expand_all(ROOT);
        tree
    }

    fn found(dir: &Path, min: u64, ignore: &[&str]) -> Found {
        let ignore: Vec<String> = ignore.iter().map(|s| s.to_string()).collect();
        let finder = Finder::start(Scan::of(&logged(dir), ROOT, min, &ignore));
        loop {
            if let Some(result) = finder.poll() {
                return result.unwrap();
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn names(dir: &Path, groups: &[Group]) -> Vec<Vec<String>> {
        groups
            .iter()
            .map(|g| {
                g.items
                    .iter()
                    .map(|i| {
                        i.path
                            .strip_prefix(dir)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned()
                    })
                    .collect()
            })
            .collect()
    }

    fn sets(mut groups: Vec<Vec<String>>) -> Vec<Vec<String>> {
        groups.sort();
        groups
    }

    #[test]
    fn same_contents_whatever_the_names() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::create_dir_all(p.join("a/b")).unwrap();
        fs::write(p.join("one"), vec![7u8; 5000]).unwrap();
        fs::write(p.join("a/two"), vec![7u8; 5000]).unwrap();
        fs::write(p.join("a/b/other-size"), vec![7u8; 6000]).unwrap();
        fs::write(p.join("a/b/same-size"), vec![8u8; 5000]).unwrap();
        let f = found(p, 1024, &[]);
        assert_eq!(names(p, &f.files), [["a/two", "one"]]);
    }

    #[test]
    fn differences_at_the_end_or_in_the_middle_count() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        let size = (ENDS * 2 + ROUND + 1000) as usize;
        let base = vec![1u8; size];
        let mut end = base.clone();
        end[size - 1] = 2;
        let mut middle = base.clone();
        middle[ENDS as usize + ROUND as usize / 2] = 2;
        fs::write(p.join("base"), &base).unwrap();
        fs::write(p.join("copy"), &base).unwrap();
        fs::write(p.join("end"), &end).unwrap();
        fs::write(p.join("middle"), &middle).unwrap();
        let f = found(p, 1024, &[]);
        assert_eq!(names(p, &f.files), [["base", "copy"]]);
    }

    #[test]
    fn a_group_of_three_splits_into_two_and_one() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::write(p.join("a"), vec![1u8; 2000]).unwrap();
        fs::write(p.join("b"), vec![1u8; 2000]).unwrap();
        fs::write(p.join("c"), vec![2u8; 2000]).unwrap();
        assert_eq!(names(p, &found(p, 1024, &[]).files), [["a", "b"]]);
    }

    #[test]
    fn small_and_empty_files_are_left_out() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::write(p.join("a"), b"small").unwrap();
        fs::write(p.join("b"), b"small").unwrap();
        fs::write(p.join("e1"), b"").unwrap();
        fs::write(p.join("e2"), b"").unwrap();
        assert!(found(p, 1024, &[]).files.is_empty());
        assert_eq!(names(p, &found(p, 1, &[]).files), [["a", "b"]]);
    }

    #[cfg(unix)]
    #[test]
    fn hard_links_and_symlinks_are_not_duplicates() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::write(p.join("a"), vec![1u8; 2000]).unwrap();
        fs::hard_link(p.join("a"), p.join("link")).unwrap();
        std::os::unix::fs::symlink(p.join("a"), p.join("sym")).unwrap();
        assert!(found(p, 1024, &[]).files.is_empty());
        // A real copy makes a group, with only one of the two names.
        fs::write(p.join("copy"), vec![1u8; 2000]).unwrap();
        assert_eq!(names(p, &found(p, 1024, &[]).files), [["a", "copy"]]);
    }

    #[test]
    fn ignored_folders_are_not_read() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::create_dir_all(p.join(".git")).unwrap();
        fs::write(p.join("a"), vec![1u8; 2000]).unwrap();
        fs::write(p.join(".git/b"), vec![1u8; 2000]).unwrap();
        assert!(found(p, 1024, &[".git"]).files.is_empty());
        assert_eq!(found(p, 1024, &[]).files.len(), 1);
    }

    #[test]
    fn the_largest_gain_comes_first() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        for name in ["s1", "s2"] {
            fs::write(p.join(name), vec![1u8; 2000]).unwrap();
        }
        for name in ["b1", "b2"] {
            fs::write(p.join(name), vec![2u8; 90_000]).unwrap();
        }
        let f = found(p, 1024, &[]);
        assert_eq!(f.files[0].items[0].size, 90_000);
        assert!(f.files[0].free >= f.files[1].free);
    }

    /// Two folders with the same names and contents, large and small files.
    fn twins(p: &Path) {
        for top in ["backup1", "backup2"] {
            fs::create_dir_all(p.join(top).join("sub")).unwrap();
            fs::write(p.join(top).join("big"), vec![1u8; 4000]).unwrap();
            fs::write(p.join(top).join("sub/small"), b"tiny").unwrap();
            fs::write(p.join(top).join("sub/empty"), b"").unwrap();
        }
    }

    #[test]
    fn whole_folders_only_at_the_top() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        twins(p);
        let f = found(p, 1024, &[]);
        assert_eq!(names(p, &f.folders), [["backup1", "backup2"]]);
        assert_eq!(f.folders[0].items[0].size, 4004);
    }

    #[test]
    fn a_small_file_that_differs_makes_folders_different() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        twins(p);
        fs::write(p.join("backup2/sub/small"), b"TINY").unwrap();
        let f = found(p, 1024, &[]);
        assert!(f.folders.is_empty());
        assert_eq!(f.files.len(), 1, "the large file is still a duplicate");
    }

    #[test]
    fn a_name_that_differs_makes_folders_different() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        twins(p);
        fs::rename(p.join("backup2/big"), p.join("backup2/bigger")).unwrap();
        assert!(found(p, 1024, &[]).folders.is_empty());
    }

    #[test]
    fn folders_with_something_ignored_inside_are_not_equal() {
        // Two clones with different histories have the same working files:
        // deleting "a copy" would lose its .git.
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        twins(p);
        fs::create_dir_all(p.join("backup1/.git")).unwrap();
        fs::write(p.join("backup1/.git/x"), b"one").unwrap();
        fs::create_dir_all(p.join("backup2/.git")).unwrap();
        fs::write(p.join("backup2/.git/x"), b"two").unwrap();
        let f = found(p, 1, &[".git"]);
        assert_eq!(
            sets(names(p, &f.folders)),
            [["backup1/sub", "backup2/sub"]],
            "the folders that hold it are not equal; those beside it may be"
        );
    }

    #[cfg(unix)]
    #[test]
    fn links_must_point_to_the_same_place() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        twins(p);
        std::os::unix::fs::symlink("big", p.join("backup1/l")).unwrap();
        std::os::unix::fs::symlink("big", p.join("backup2/l")).unwrap();
        assert_eq!(found(p, 1024, &[]).folders.len(), 1);
        fs::remove_file(p.join("backup2/l")).unwrap();
        std::os::unix::fs::symlink("elsewhere", p.join("backup2/l")).unwrap();
        assert!(found(p, 1024, &[]).folders.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn folders_of_hard_links_free_nothing() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::create_dir_all(p.join("a")).unwrap();
        fs::create_dir_all(p.join("b")).unwrap();
        fs::write(p.join("a/f"), vec![1u8; 9000]).unwrap();
        fs::hard_link(p.join("a/f"), p.join("b/f")).unwrap();
        assert!(found(p, 1024, &[]).folders.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_file_swapped_for_a_link_is_not_followed() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::write(p.join("a"), vec![1u8; 2000]).unwrap();
        fs::write(p.join("b"), vec![1u8; 2000]).unwrap();
        let scan = Scan::of(&logged(p), ROOT, 1024, &[]);
        fs::remove_file(p.join("b")).unwrap();
        std::os::unix::fs::symlink(p.join("a"), p.join("b")).unwrap();
        let finder = Finder::start(scan);
        let f = loop {
            if let Some(r) = finder.poll() {
                break r.unwrap();
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        assert!(f.files.is_empty());
        assert_eq!(f.notes.unreadable + f.notes.changed, 1);
    }

    #[test]
    fn cancelling_stops() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::write(p.join("a"), vec![1u8; 2000]).unwrap();
        fs::write(p.join("b"), vec![1u8; 2000]).unwrap();
        let mut worker = Worker {
            progress: Arc::new(Progress::default()),
            cancel: Arc::new(AtomicBool::new(true)),
            buffer: vec![0; BUFFER],
            notes: Notes::default(),
        };
        let scan = Scan::of(&logged(p), ROOT, 1024, &[]);
        assert_eq!(worker.find(scan).unwrap_err(), Fail::Cancelled);
    }

    #[test]
    fn parts_cover_the_file_once() {
        for size in [1, ENDS, ENDS + 1, ENDS * 2, ENDS * 2 + 1, ROUND * 3 + 17] {
            let mut parts = parts(size);
            assert_eq!(parts[0].0, 0);
            parts.sort();
            let mut at = 0;
            for (start, len) in parts {
                assert_eq!(start, at, "size {size}");
                at += len;
            }
            assert_eq!(at, size);
        }
    }

    fn view(p: &Path) -> DuplicatesView {
        DuplicatesView::new(p.to_path_buf(), found(p, 1024, &[]))
    }

    fn three(p: &Path) {
        fs::create_dir_all(p.join("backup")).unwrap();
        let at = |secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        for (name, secs) in [("long/er/name", 2000), ("backup/b", 1000), ("c", 3000)] {
            let path = p.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, vec![1u8; 2000]).unwrap();
            fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(at(secs))
                .unwrap();
        }
    }

    fn kept(v: &DuplicatesView, p: &Path) -> Vec<String> {
        v.groups()[0]
            .items
            .iter()
            .filter(|i| !i.tagged)
            .map(|i| {
                i.path
                    .strip_prefix(p)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn all_but_one_by_each_rule() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        three(p);
        let mut v = view(p);
        assert_eq!(v.tag_all_but_one(), Keep::Oldest);
        assert_eq!(kept(&v, p), ["backup/b"]);
        assert_eq!(v.tag_all_but_one(), Keep::Newest);
        assert_eq!(kept(&v, p), ["c"]);
        assert_eq!(v.tag_all_but_one(), Keep::ShortestPath);
        assert_eq!(kept(&v, p), ["c"]);
        assert_eq!(v.tag_all_but_one(), Keep::Oldest);
        v.untag_all();
        assert_eq!(v.tagged().0, 0);
    }

    #[test]
    fn keeping_the_copy_under_the_cursor_or_in_its_folder() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        three(p);
        let mut v = view(p);
        v.move_by(1); // The first copy: backup/b.
        v.keep_selected();
        assert_eq!(kept(&v, p), ["backup/b"]);
        v.untag_all();
        v.move_by(1); // c, in the root.
        assert_eq!(v.keep_in_folder(), 1);
        assert_eq!(kept(&v, p), ["c"]);
    }

    #[test]
    fn the_last_copy_cannot_be_tagged() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::write(p.join("a"), vec![1u8; 2000]).unwrap();
        fs::write(p.join("b"), vec![1u8; 2000]).unwrap();
        let mut v = view(p);
        v.move_by(1);
        assert!(v.toggle());
        v.move_by(1);
        assert!(!v.toggle(), "the other one is already tagged");
        assert_eq!(v.tagged().0, 1);
    }

    #[test]
    fn nothing_goes_when_a_file_changed() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::write(p.join("a"), vec![1u8; 2000]).unwrap();
        fs::write(p.join("b"), vec![1u8; 2000]).unwrap();
        let mut v = view(p);
        v.tag_all_but_one();
        let (items, refused) = v.check_tagged();
        assert_eq!((items.len(), refused), (1, 0));
        // The copy that stays changes: it is no longer a copy.
        let stays = v.groups()[0].items.iter().find(|i| !i.tagged).unwrap();
        fs::write(&stays.path, vec![2u8; 3000]).unwrap();
        let (items, refused) = v.check_tagged();
        assert_eq!((items.len(), refused), (0, 1));
        assert_eq!(v.tagged().0, 0, "the tags of that group went");
    }

    #[test]
    fn nothing_goes_when_the_copy_that_stays_is_gone() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::write(p.join("a"), vec![1u8; 2000]).unwrap();
        fs::write(p.join("b"), vec![1u8; 2000]).unwrap();
        let mut v = view(p);
        v.tag_all_but_one();
        let stays = v.groups()[0].items.iter().find(|i| !i.tagged).unwrap();
        fs::remove_file(&stays.path).unwrap();
        assert_eq!(v.check_tagged().1, 1);
    }

    #[test]
    fn a_copy_inside_a_tagged_folder_does_not_stay() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        for dir in ["A/sub", "B/sub", "C"] {
            fs::create_dir_all(p.join(dir)).unwrap();
        }
        let item = |name: &str, tagged| Item {
            path: p.join(name),
            size: 0,
            disk: 0,
            modified: None,
            tagged,
            snapshot: Snapshot::read(&p.join(name)),
            shared: false,
            check_date: true,
        };
        // Two groups: A = B, and A/sub = C. With A and C tagged, what is meant
        // to stay of the second group goes with A.
        let mut v = DuplicatesView::new(p.to_path_buf(), Found::default());
        v.folders = vec![
            Group::new(vec![item("A", true), item("B", false)]),
            Group::new(vec![item("A/sub", false), item("C", true)]),
        ];
        v.show_folders = true;
        let (items, refused) = v.check_tagged();
        assert_eq!(refused, 1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, p.join("A"));
    }

    #[test]
    fn folders_that_changed_are_refused() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        twins(p);
        let mut v = view(p);
        v.toggle_folders();
        v.tag_all_but_one();
        assert_eq!(v.check_tagged().1, 0);
        fs::write(p.join("backup1/new"), b"x").unwrap();
        assert_eq!(v.check_tagged().1, 1);
    }

    #[test]
    fn what_is_gone_leaves_its_group() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        three(p);
        let mut v = view(p);
        fs::remove_file(p.join("c")).unwrap();
        v.drop_missing();
        assert_eq!(v.groups()[0].items.len(), 2);
        fs::remove_file(p.join("backup/b")).unwrap();
        v.drop_missing();
        assert!(v.groups().is_empty(), "one copy is no longer a group");
    }

    #[test]
    fn the_export_cannot_be_faked_by_a_name() {
        let mut v = DuplicatesView::new(PathBuf::from("/x"), Found::default());
        let item = |path: &str| Item {
            path: PathBuf::from(path),
            size: 5,
            disk: 5,
            modified: None,
            tagged: false,
            snapshot: None,
            shared: false,
            check_date: true,
        };
        v.files = vec![Group::new(vec![item("/x/a\nb"), item("/x/c")])];
        let text = v.export();
        assert_eq!(text.lines().count(), 3, "two copies and a blank line");
        assert!(text.contains("5\t0\t-\t/x/c\n"));
    }
}
