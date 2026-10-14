//! Directory tree kept in memory, XTree style.
//!
//! Nodes live in an arena (`Vec<DirNode>`) and refer to each other by index,
//! which avoids pointers and keeps the structure compact even with millions
//! of logged directories.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::platform;

pub type NodeId = usize;

pub const ROOT: NodeId = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    File,
    Symlink,
    Other,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: OsString,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub mode: u32,
    pub kind: FileKind,
    /// Tagged with `T` for group operations.
    pub tagged: bool,
}

impl FileEntry {
    /// Hidden file (dot name on Linux, H attribute on Windows).
    pub fn is_hidden(&self) -> bool {
        platform::is_hidden(&self.name, self.mode)
    }
}

#[derive(Debug)]
pub struct DirNode {
    /// Directory name; for the root it is the full path.
    pub name: OsString,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub files: Vec<FileEntry>,
    pub expanded: bool,
    /// Already read from disk ("logged").
    pub scanned: bool,
    /// Error reading the directory (e.g. permission denied).
    pub error: Option<String>,
    /// Hidden directory (dot name on Linux, H attribute on Windows).
    pub hidden: bool,
}

/// A visible row in the tree pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRow {
    pub id: NodeId,
    /// Connectors drawn before the name (`│  ├──`).
    pub prefix: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    pub dirs: u64,
    pub files: u64,
    pub bytes: u64,
}

pub struct Tree {
    nodes: Vec<DirNode>,
    /// Tree of an archive: everything is already read and nothing is reread
    /// from disk.
    is_virtual: bool,
    /// Shows hidden directories (name starting with a dot). Only affects what
    /// is displayed; everything logged is still kept here.
    pub show_hidden: bool,
}

/// Reading of a whole branch done in small steps, so the interface can
/// show progress and the user can cancel.
pub struct LogJob {
    stack: Vec<NodeId>,
    /// Besides reading, expands the nodes in the tree (`*` key).
    expand: bool,
    /// Directories processed so far.
    pub dirs: u64,
}

impl LogJob {
    pub fn new(root: NodeId, expand: bool) -> Self {
        LogJob {
            stack: vec![root],
            expand,
            dirs: 0,
        }
    }

    pub fn is_done(&self) -> bool {
        self.stack.is_empty()
    }
}

impl Tree {
    /// Creates the tree with `root` already logged and expanded.
    pub fn new(root: PathBuf) -> Self {
        let mut tree = Tree {
            nodes: vec![DirNode {
                name: root.into_os_string(),
                parent: None,
                children: Vec::new(),
                files: Vec::new(),
                expanded: false,
                scanned: false,
                error: None,
                hidden: false,
            }],
            is_virtual: false,
            show_hidden: true,
        };
        tree.expand(ROOT);
        tree
    }

    /// Virtual tree from a listing: paths relative to `root`
    /// and, for each one, the file (`Some`) or a directory (`None`).
    /// Directories that only appear on a file's path are created.
    pub fn from_listing(root: PathBuf, listing: Vec<(PathBuf, Option<FileEntry>)>) -> Self {
        let mut tree = Tree {
            nodes: vec![DirNode {
                name: root.into_os_string(),
                parent: None,
                children: Vec::new(),
                files: Vec::new(),
                expanded: true,
                scanned: true,
                error: None,
                hidden: false,
            }],
            is_virtual: true,
            show_hidden: true,
        };
        for (path, file) in listing {
            let dirs = match file {
                Some(_) => path.parent().unwrap_or(Path::new("")),
                None => path.as_path(),
            };
            let mut cur = ROOT;
            for name in dirs.iter() {
                let found = find_child(&tree, cur, name);
                cur = match found {
                    Some(c) => c,
                    None => {
                        let hidden = platform::is_hidden(name, 0);
                        let c = tree.add_node(name.to_os_string(), cur, hidden);
                        tree.nodes[c].scanned = true;
                        tree.nodes[cur].children.push(c);
                        c
                    }
                };
            }
            if let Some(file) = file {
                tree.nodes[cur].files.push(file);
            }
        }
        for node in &mut tree.nodes {
            node.files.sort_by_key(|f| sort_key(&f.name));
        }
        for i in 0..tree.nodes.len() {
            let mut children = std::mem::take(&mut tree.nodes[i].children);
            children.sort_by_key(|&c| sort_key(&tree.nodes[c].name));
            tree.nodes[i].children = children;
        }
        tree
    }

    pub fn node(&self, id: NodeId) -> &DirNode {
        &self.nodes[id]
    }

    /// Full path of the directory.
    pub fn path(&self, id: NodeId) -> PathBuf {
        let mut parts = Vec::new();
        let mut cur = Some(id);
        while let Some(n) = cur {
            parts.push(&self.nodes[n].name);
            cur = self.nodes[n].parent;
        }
        parts.iter().rev().collect()
    }

    /// Reads the directory from disk, if it has not been read yet.
    pub fn scan(&mut self, id: NodeId) {
        if self.nodes[id].scanned || self.is_virtual {
            return;
        }
        self.nodes[id].scanned = true;

        let path = self.path(id);
        let (dirs, files) = match read_dir_entries(&path) {
            Ok(r) => r,
            Err(e) => {
                self.nodes[id].error = Some(e.to_string());
                return;
            }
        };

        for (name, hidden) in dirs {
            let child = self.add_node(name, id, hidden);
            self.nodes[id].children.push(child);
        }
        self.nodes[id].files = files;
    }

    fn add_node(&mut self, name: OsString, parent: NodeId, hidden: bool) -> NodeId {
        self.nodes.push(DirNode {
            name,
            parent: Some(parent),
            children: Vec::new(),
            files: Vec::new(),
            expanded: false,
            scanned: false,
            error: None,
            hidden,
        });
        self.nodes.len() - 1
    }

    /// Rereads an already logged directory (re-log). Subdirectories that still
    /// exist keep what was already read below them; files keep their tag.
    pub fn rescan(&mut self, id: NodeId) {
        if !self.nodes[id].scanned || self.is_virtual {
            return;
        }
        let path = self.path(id);
        let (dirs, mut files) = match read_dir_entries(&path) {
            Ok(r) => r,
            Err(e) => {
                let node = &mut self.nodes[id];
                node.error = Some(e.to_string());
                node.files.clear();
                node.children.clear();
                return;
            }
        };

        let tagged: HashSet<&OsString> = self.nodes[id]
            .files
            .iter()
            .filter(|f| f.tagged)
            .map(|f| &f.name)
            .collect();
        for f in &mut files {
            f.tagged = tagged.contains(&f.name);
        }

        let mut old: HashMap<OsString, NodeId> = self.nodes[id]
            .children
            .iter()
            .map(|&c| (self.nodes[c].name.clone(), c))
            .collect();
        // Those that vanished are left loose in the arena, outside the tree.
        let children = dirs
            .into_iter()
            .map(|(name, hidden)| match old.remove(&name) {
                Some(c) => {
                    self.nodes[c].hidden = hidden;
                    c
                }
                None => self.add_node(name, id, hidden),
            })
            .collect();

        let node = &mut self.nodes[id];
        node.children = children;
        node.files = files;
        node.error = None;
    }

    /// Node of directory `path`, if it is in the tree.
    pub fn find(&self, path: &Path) -> Option<NodeId> {
        let rel = path.strip_prefix(&self.nodes[ROOT].name).ok()?;
        let mut cur = ROOT;
        for comp in rel.components() {
            let name = comp.as_os_str();
            cur = *self.nodes[cur]
                .children
                .iter()
                .find(|&&c| self.nodes[c].name == name)?;
        }
        Some(cur)
    }

    pub fn file_mut(&mut self, dir: NodeId, idx: usize) -> &mut FileEntry {
        &mut self.nodes[dir].files[idx]
    }

    /// Expands a node. The children are also read so we know whether
    /// they have subdirectories (and show the `+` indicator).
    pub fn expand(&mut self, id: NodeId) {
        self.scan(id);
        self.nodes[id].expanded = true;
        for i in 0..self.nodes[id].children.len() {
            let child = self.nodes[id].children[i];
            self.scan(child);
        }
    }

    pub fn collapse(&mut self, id: NodeId) {
        self.nodes[id].expanded = false;
    }

    /// Expands the whole branch below `id` at once.
    #[cfg(test)]
    pub fn expand_all(&mut self, id: NodeId) {
        let mut job = LogJob::new(id, true);
        self.run_job(&mut job, Duration::MAX);
    }

    /// Advances a [`LogJob`] until it finishes or exceeds `budget`.
    pub fn run_job(&mut self, job: &mut LogJob, budget: Duration) {
        let start = Instant::now();
        while let Some(n) = job.stack.pop() {
            if job.expand {
                self.expand(n);
            } else {
                self.scan(n);
            }
            job.dirs += 1;
            // Reverse order so children are visited alphabetically.
            job.stack
                .extend(self.nodes[n].children.iter().rev().copied());
            if start.elapsed() >= budget {
                break;
            }
        }
    }

    /// `id` and all its descendants already created, in pre-order.
    pub fn subtree(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            out.push(n);
            stack.extend(self.nodes[n].children.iter().rev().copied());
        }
        out
    }

    /// Expands the ancestors of `id` so it shows up in the tree.
    pub fn reveal(&mut self, id: NodeId) {
        let mut cur = self.nodes[id].parent;
        while let Some(n) = cur {
            self.nodes[n].expanded = true;
            cur = self.nodes[n].parent;
        }
    }

    pub fn has_children(&self, id: NodeId) -> bool {
        !self.nodes[id].children.is_empty()
    }

    /// Visible rows, in order, honoring collapsed nodes.
    pub fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut rows = vec![VisibleRow {
            id: ROOT,
            prefix: String::new(),
        }];
        if self.nodes[ROOT].expanded {
            self.push_children(ROOT, "", &mut rows);
        }
        rows
    }

    /// Displayed subdirectories (without the hidden ones, if applicable).
    fn visible_children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes[id]
            .children
            .iter()
            .copied()
            .filter(|&c| self.show_hidden || !self.nodes[c].hidden)
    }

    /// Like `subtree`, but without entering hidden directories (unless
    /// they are displayed). `id` itself is always included.
    pub fn visible_subtree(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            out.push(n);
            let children: Vec<NodeId> = self.visible_children(n).collect();
            stack.extend(children.into_iter().rev());
        }
        out
    }

    fn push_children(&self, id: NodeId, indent: &str, rows: &mut Vec<VisibleRow>) {
        let children: Vec<NodeId> = self.visible_children(id).collect();
        for (i, &child) in children.iter().enumerate() {
            let last = i + 1 == children.len();
            let collapsed =
                !self.nodes[child].expanded && self.visible_children(child).next().is_some();
            let connector = match (last, collapsed) {
                (false, false) => "├──",
                (false, true) => "├─+",
                (true, false) => "└──",
                (true, true) => "└─+",
            };
            rows.push(VisibleRow {
                id: child,
                prefix: format!("{indent}{connector}"),
            });
            if self.nodes[child].expanded {
                let next = format!("{indent}{}", if last { "   " } else { "│  " });
                self.push_children(child, &next, rows);
            }
        }
    }

    /// For each directory (by `NodeId`), how many files and how many bytes
    /// it has: only in it, or in the whole branch (`branch`). Counts only the
    /// tagged ones with `tagged_only`, and honors `show_hidden`. Done in a
    /// single bottom-up pass, so it works even on huge trees.
    pub fn dir_totals(&self, branch: bool, tagged_only: bool) -> Vec<(u64, u64)> {
        let mut totals = vec![(0u64, 0u64); self.nodes.len()];
        let order = self.visible_subtree(ROOT);
        for &id in order.iter().rev() {
            let node = &self.nodes[id];
            for f in &node.files {
                if (tagged_only && !f.tagged) || (!self.show_hidden && f.is_hidden()) {
                    continue;
                }
                totals[id].0 += 1;
                totals[id].1 += f.size;
            }
            if branch {
                let children: Vec<NodeId> = self.visible_children(id).collect();
                for c in children {
                    totals[id].0 += totals[c].0;
                    totals[id].1 += totals[c].1;
                }
            }
        }
        totals
    }

    /// Totals of everything logged so far.
    pub fn logged_totals(&self) -> Totals {
        let mut t = Totals::default();
        for n in self.subtree(ROOT).into_iter().map(|id| &self.nodes[id]) {
            if !n.scanned {
                continue;
            }
            t.dirs += 1;
            t.files += n.files.len() as u64;
            t.bytes += n.files.iter().map(|f| f.size).sum::<u64>();
        }
        t
    }
}

/// Subdirectory found on disk: name and whether it is hidden.
type DirName = (OsString, bool);

/// Reads a directory and returns (subdirectories, files), both sorted.
/// Symlinks are never followed: they appear as files.
fn read_dir_entries(path: &Path) -> std::io::Result<(Vec<DirName>, Vec<FileEntry>)> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name();
        let Ok(meta) = entry.metadata() else {
            // Entry that vanished or without stat permission: shown anyway.
            files.push(FileEntry {
                name,
                size: 0,
                modified: None,
                mode: 0,
                kind: FileKind::Other,
                tagged: false,
            });
            continue;
        };
        let ft = meta.file_type();
        if ft.is_dir() {
            let hidden = platform::is_hidden(&name, platform::mode(&meta));
            dirs.push((name, hidden));
            continue;
        }
        let kind = if ft.is_symlink() {
            FileKind::Symlink
        } else if ft.is_file() {
            FileKind::File
        } else {
            FileKind::Other
        };
        files.push(FileEntry {
            name,
            size: meta.len(),
            modified: meta.modified().ok(),
            mode: platform::mode(&meta),
            kind,
            tagged: false,
        });
    }

    dirs.sort_by_key(|(name, _)| sort_key(name));
    files.sort_by_key(|f| sort_key(&f.name));
    Ok((dirs, files))
}

fn find_child(tree: &Tree, parent: NodeId, name: &std::ffi::OsStr) -> Option<NodeId> {
    tree.nodes[parent]
        .children
        .iter()
        .copied()
        .find(|&c| tree.nodes[c].name == name)
}

fn sort_key(name: &OsString) -> String {
    name.to_string_lossy().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, write};

    fn sample() -> (tempfile::TempDir, Tree) {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        create_dir_all(p.join("b/deep/deeper")).unwrap();
        create_dir_all(p.join("a")).unwrap();
        write(p.join("z.txt"), "12345").unwrap();
        write(p.join("A.txt"), "1").unwrap();
        write(p.join("b/inside"), "xyz").unwrap();
        let tree = Tree::new(p.to_path_buf());
        (tmp, tree)
    }

    fn names(tree: &Tree, ids: &[NodeId]) -> Vec<String> {
        ids.iter()
            .map(|&i| tree.node(i).name.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn logs_root_sorted() {
        let (_tmp, tree) = sample();
        let root = tree.node(ROOT);
        assert_eq!(names(&tree, &root.children), ["a", "b"]);
        let files: Vec<_> = root
            .files
            .iter()
            .map(|f| f.name.to_str().unwrap())
            .collect();
        assert_eq!(files, ["A.txt", "z.txt"]);
    }

    #[test]
    fn path_is_rebuilt_from_parents() {
        let (tmp, mut tree) = sample();
        let b = tree.node(ROOT).children[1];
        tree.expand(b);
        let deep = tree.node(b).children[0];
        assert_eq!(tree.path(deep), tmp.path().join("b/deep"));
    }

    #[test]
    fn visible_rows_follow_expansion() {
        let (_tmp, mut tree) = sample();
        let prefixes: Vec<_> = tree.visible_rows().into_iter().map(|r| r.prefix).collect();
        // "b" has children and is collapsed: "+" indicator.
        assert_eq!(prefixes, ["", "├──", "└─+"]);

        tree.expand_all(ROOT);
        let prefixes: Vec<_> = tree.visible_rows().into_iter().map(|r| r.prefix).collect();
        assert_eq!(prefixes, ["", "├──", "└──", "   └──", "      └──"]);

        tree.collapse(ROOT);
        assert_eq!(tree.visible_rows().len(), 1);
    }

    #[test]
    fn totals_count_only_logged_dirs() {
        let (_tmp, mut tree) = sample();
        // root + a + b (children read when the root was expanded)
        assert_eq!(
            tree.logged_totals(),
            Totals {
                dirs: 3,
                files: 3,
                bytes: 9
            }
        );
        tree.expand_all(ROOT);
        assert_eq!(tree.logged_totals().dirs, 5);
    }

    #[test]
    fn log_job_can_run_in_slices() {
        let (_tmp, mut tree) = sample();
        let mut job = LogJob::new(ROOT, false);
        // Zero budget: processes one directory per call.
        let mut calls = 0;
        while !job.is_done() {
            tree.run_job(&mut job, Duration::ZERO);
            calls += 1;
        }
        assert_eq!(calls, 5);
        assert_eq!(job.dirs, 5);
        assert_eq!(tree.logged_totals().dirs, 5);
        // Only read, did not expand.
        assert_eq!(tree.visible_rows().len(), 3);
    }

    #[test]
    fn subtree_and_reveal() {
        let (_tmp, mut tree) = sample();
        tree.expand_all(ROOT);
        let b = tree.node(ROOT).children[1];
        assert_eq!(names(&tree, &tree.subtree(b)), ["b", "deep", "deeper"]);

        let deeper = tree.subtree(b)[2];
        tree.collapse(ROOT);
        tree.collapse(b);
        tree.reveal(deeper);
        assert!(tree.visible_rows().iter().any(|r| r.id == deeper));
    }

    #[test]
    fn rescan_keeps_subtrees_and_tags() {
        let (tmp, mut tree) = sample();
        tree.expand_all(ROOT);
        tree.file_mut(ROOT, 1).tagged = true; // z.txt
        let b = tree.node(ROOT).children[1];

        std::fs::remove_dir(tmp.path().join("a")).unwrap();
        create_dir_all(tmp.path().join("c")).unwrap();
        write(tmp.path().join("new.txt"), "").unwrap();
        tree.rescan(ROOT);

        let root = tree.node(ROOT);
        assert_eq!(names(&tree, &root.children), ["b", "c"]);
        assert_eq!(root.children[0], b, "b keeps its node");
        assert_eq!(names(&tree, &tree.subtree(b)), ["b", "deep", "deeper"]);
        let tags: Vec<_> = root
            .files
            .iter()
            .map(|f| (f.name.to_str().unwrap(), f.tagged))
            .collect();
        assert_eq!(
            tags,
            [("A.txt", false), ("new.txt", false), ("z.txt", true)]
        );
        // "a" is gone from the tree and from the totals.
        assert_eq!(tree.logged_totals().dirs, 4);
    }

    #[test]
    fn finds_nodes_by_path() {
        let (tmp, mut tree) = sample();
        tree.expand_all(ROOT);
        assert_eq!(tree.find(tmp.path()), Some(ROOT));
        let deep = tree.find(&tmp.path().join("b/deep")).unwrap();
        assert_eq!(tree.path(deep), tmp.path().join("b/deep"));
        assert_eq!(tree.find(&tmp.path().join("nope")), None);
        assert_eq!(tree.find(Path::new("/elsewhere")), None);
    }

    #[test]
    fn builds_virtual_tree_from_listing() {
        let file = |name: &str, size| FileEntry {
            name: name.into(),
            size,
            modified: None,
            mode: 0o644,
            kind: FileKind::File,
            tagged: false,
        };
        let listing = vec![
            (PathBuf::from("z.txt"), Some(file("z.txt", 1))),
            (PathBuf::from("src/deep/b.rs"), Some(file("b.rs", 2))),
            (PathBuf::from("src/a.rs"), Some(file("a.rs", 3))),
            (PathBuf::from("empty"), None),
        ];
        let mut tree = Tree::from_listing(PathBuf::from("/x/p.zip"), listing);
        assert_eq!(names(&tree, &tree.node(ROOT).children), ["empty", "src"]);
        let src = tree.find(Path::new("/x/p.zip/src")).unwrap();
        assert_eq!(tree.node(src).files[0].name, "a.rs");
        let deep = tree.find(Path::new("/x/p.zip/src/deep")).unwrap();
        assert_eq!(tree.path(deep), Path::new("/x/p.zip/src/deep"));
        assert_eq!(
            tree.logged_totals(),
            Totals {
                dirs: 4,
                files: 3,
                bytes: 6
            }
        );

        // Nothing is read from disk: neither scan nor re-log.
        tree.rescan(ROOT);
        tree.expand(src);
        assert!(tree.node(ROOT).error.is_none());
        assert_eq!(tree.node(ROOT).files.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn hides_dot_directories_when_asked() {
        let (tmp, mut tree) = sample();
        create_dir_all(tmp.path().join(".git/objects")).unwrap();
        create_dir_all(tmp.path().join("b/.cache")).unwrap();
        tree.rescan(ROOT);
        let b = tree.find(&tmp.path().join("b")).unwrap();
        tree.rescan(b);
        tree.expand_all(ROOT);
        let names_of = |t: &Tree| -> Vec<String> {
            t.visible_rows()
                .iter()
                .skip(1)
                .map(|r| t.node(r.id).name.to_string_lossy().into_owned())
                .collect()
        };
        assert!(names_of(&tree).contains(&".git".to_string()));

        tree.show_hidden = false;
        assert_eq!(names_of(&tree), ["a", "b", "deep", "deeper"]);
        assert_eq!(
            names(&tree, &tree.visible_subtree(b)),
            ["b", "deep", "deeper"]
        );
        // What is logged does not change.
        assert_eq!(
            names(&tree, &tree.subtree(b)),
            ["b", ".cache", "deep", "deeper"]
        );
    }

    #[test]
    fn directory_and_branch_totals() {
        let (_tmp, mut tree) = sample();
        tree.expand_all(ROOT);
        let b = tree.node(ROOT).children[1];
        tree.file_mut(ROOT, 1).tagged = true; // z.txt, 5 bytes
        let dir = tree.dir_totals(false, false);
        assert_eq!(dir[ROOT], (2, 6));
        assert_eq!(dir[b], (1, 3));
        let branch = tree.dir_totals(true, false);
        assert_eq!(branch[ROOT], (3, 9));
        assert_eq!(branch[b], (1, 3));
        let tagged = tree.dir_totals(true, true);
        assert_eq!(tagged[ROOT], (1, 5));
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed() {
        let tmp = crate::testing::tempdir();
        create_dir_all(tmp.path().join("real")).unwrap();
        std::os::unix::fs::symlink(tmp.path(), tmp.path().join("loop")).unwrap();
        let mut tree = Tree::new(tmp.path().to_path_buf());
        tree.expand_all(ROOT);
        assert_eq!(names(&tree, &tree.node(ROOT).children), ["real"]);
        assert_eq!(tree.node(ROOT).files[0].kind, FileKind::Symlink);
    }

    #[test]
    fn unreadable_dir_records_error() {
        let mut tree = Tree::new(PathBuf::from("/caminho/que/nao/existe"));
        tree.scan(ROOT);
        assert!(tree.node(ROOT).error.is_some());
    }
}
