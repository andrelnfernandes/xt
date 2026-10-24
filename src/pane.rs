//! A xt window: tree, file list, filespec and cursors.
//!
//! The normal screen has one; with the window split (F8) there are two, each with
//! its own tree.

use std::path::{Component, Path, PathBuf};

use ratatui::widgets::ListState;

use crate::archive::{self, EntryKind};
use crate::input::Command;
use crate::model::filespec::Filespec;
use crate::model::sort::{self, Sort};
use crate::model::tree::{FileEntry, FileKind, NodeId, ROOT, Tree, VisibleRow};
use crate::ops::Source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Files,
}

/// What the file window is listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileView {
    /// Files of the directory under the tree cursor.
    Dir,
    /// Files of the branch starting at this directory (`B` key).
    Branch(NodeId),
    /// Files of everything that was logged (`S` key).
    Showall,
}

/// Reference to a file inside the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRef {
    pub dir: NodeId,
    pub idx: usize,
}

/// File window modes (Alt+F), in ZTreeWin order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileDisplay {
    /// Several columns, names only.
    Names,
    /// Several columns: name, size and permissions.
    NamesSize,
    /// One column: name, size, date and permissions.
    #[default]
    Details,
    /// One column, names only (full length).
    LongNames,
}

impl FileDisplay {
    pub fn next(self) -> Self {
        match self {
            FileDisplay::Names => FileDisplay::NamesSize,
            FileDisplay::NamesSize => FileDisplay::Details,
            FileDisplay::Details => FileDisplay::LongNames,
            FileDisplay::LongNames => FileDisplay::Names,
        }
    }

    pub fn multi_column(self) -> bool {
        matches!(self, FileDisplay::Names | FileDisplay::NamesSize)
    }
}

/// What Alt+K shows next to the directories.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Kbyte {
    pub show: KbyteShow,
    /// Tagged files only.
    pub tagged_only: bool,
    /// Sums the whole branch, not just the directory.
    pub branch: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KbyteShow {
    #[default]
    None,
    Size,
    Count,
}

impl Kbyte {
    /// Ex.: "size of branch, tagged".
    pub fn describe(&self) -> String {
        let what = match self.show {
            KbyteShow::None => return String::new(),
            KbyteShow::Size => "K",
            KbyteShow::Count => "files",
        };
        let scope = if self.branch { "branch" } else { "dir" };
        let tagged = if self.tagged_only { ", tagged" } else { "" };
        format!("{what} per {scope}{tagged}")
    }
}

pub struct Pane {
    pub tree: Tree,
    /// Cache of the visible rows; recomputed when the tree changes.
    pub rows: Vec<VisibleRow>,
    pub tree_state: ListState,
    pub focus: Focus,
    pub view: FileView,
    /// Files listed in the file window, already filtered.
    pub files: Vec<FileRef>,
    pub file_state: ListState,
    pub filespec: Filespec,
    pub sort: Sort,
    /// Alt+K: size or file count next to each directory.
    pub kbyte: Kbyte,
    /// Alt+F: how the file window displays the list.
    pub display: FileDisplay,
    /// File window columns as of the last draw (multi-column modes),
    /// so the arrows move by row.
    pub file_cols: usize,
    /// Usable height of the lists as of the last draw (for PgUp/PgDn).
    pub tree_page: usize,
    pub file_page: usize,
    /// With a ZIP/TAR open: its path (root of the virtual tree)...
    pub archive: Option<PathBuf>,
    /// ...and the window as it was before opening it, to go back with Esc.
    pub outer: Option<Box<Pane>>,
}

impl Pane {
    pub fn new(root: PathBuf) -> Self {
        Self::with_tree(Tree::new(root))
    }

    /// Window showing the contents of an archive.
    pub fn from_archive(path: &Path) -> std::io::Result<Self> {
        let mut reader = archive::Reader::open(path)?;
        let listing = reader
            .list()?
            .into_iter()
            .map(|e| {
                let file = (e.kind != EntryKind::Dir).then(|| FileEntry {
                    name: e.path.file_name().unwrap_or_default().to_os_string(),
                    size: e.size,
                    modified: e.modified,
                    mode: crate::platform::from_unix_mode(e.mode),
                    kind: match e.kind {
                        EntryKind::File => FileKind::File,
                        EntryKind::Symlink => FileKind::Symlink,
                        _ => FileKind::Other,
                    },
                    tagged: false,
                });
                (e.path, file)
            })
            .collect();
        let mut pane = Self::with_tree(Tree::from_listing(path.to_path_buf(), listing));
        pane.archive = Some(path.to_path_buf());
        Ok(pane)
    }

    fn with_tree(tree: Tree) -> Self {
        let rows = tree.visible_rows();
        let mut pane = Pane {
            tree,
            rows,
            tree_state: ListState::default().with_selected(Some(0)),
            focus: Focus::Tree,
            view: FileView::Dir,
            files: Vec::new(),
            file_state: ListState::default().with_selected(Some(0)),
            filespec: Filespec::default(),
            sort: Sort::default(),
            kbyte: Kbyte::default(),
            display: FileDisplay::default(),
            file_cols: 1,
            tree_page: 1,
            file_page: 1,
            archive: None,
            outer: None,
        };
        pane.refresh_files();
        pane
    }

    /// Directory under the tree cursor.
    pub fn current(&self) -> NodeId {
        self.rows[self.tree_state.selected().unwrap_or(0)].id
    }

    pub fn current_path(&self) -> PathBuf {
        self.tree.path(self.current())
    }

    pub fn selected_file(&self) -> Option<FileRef> {
        self.files.get(self.file_state.selected()?).copied()
    }

    pub fn file(&self, f: FileRef) -> &FileEntry {
        &self.tree.node(f.dir).files[f.idx]
    }

    pub fn file_path(&self, f: FileRef) -> PathBuf {
        self.tree.path(f.dir).join(&self.file(f).name)
    }

    /// Directory shown in the header: in Showall/Branch, that of the file
    /// under the cursor.
    pub fn header_dir(&self) -> NodeId {
        match (self.view, self.selected_file()) {
            (FileView::Dir, _) | (_, None) => self.current(),
            (_, Some(f)) => f.dir,
        }
    }

    /// Root directory of the current list: the directory, the Branch or the tree.
    pub fn view_root(&self) -> NodeId {
        match self.view {
            FileView::Dir => self.current(),
            FileView::Branch(id) => id,
            FileView::Showall => ROOT,
        }
    }

    /// Files an operation acts on: the tagged ones in the current
    /// list or the file under the cursor.
    pub fn sources(&self, tagged: bool) -> Vec<Source> {
        let refs: Vec<FileRef> = if tagged {
            self.files
                .iter()
                .copied()
                .filter(|&f| self.file(f).tagged)
                .collect()
        } else if self.focus == Focus::Files {
            self.selected_file().into_iter().collect()
        } else {
            Vec::new()
        };
        refs.into_iter()
            .map(|f| Source {
                path: self.file_path(f),
                size: self.file(f).size,
            })
            .collect()
    }

    pub fn set_tags(&mut self, refs: &[FileRef], f: impl Fn(bool) -> bool) {
        for &r in refs {
            let entry = self.tree.file_mut(r.dir, r.idx);
            entry.tagged = f(entry.tagged);
        }
    }

    /// All logged files that match the filespec.
    pub fn logged_matching_files(&self) -> Vec<FileRef> {
        self.branch_matching_files(ROOT)
    }

    /// The logged files of the branch of `id` that match the filespec.
    pub fn branch_matching_files(&self, id: NodeId) -> Vec<FileRef> {
        let mut refs = Vec::new();
        for dir in self.tree.visible_subtree(id) {
            for (idx, f) in self.tree.node(dir).files.iter().enumerate() {
                let shown = self.tree.show_hidden || !f.is_hidden();
                if shown && self.filespec.matches_entry(f) {
                    refs.push(FileRef { dir, idx });
                }
            }
        }
        refs
    }

    /// Rereads `dir`; if it is not in the tree yet (e.g. it was just
    /// created), rereads the nearest ancestor that is.
    pub fn rescan_path(&mut self, dir: &Path) {
        if let Some(id) = dir.ancestors().find_map(|d| self.tree.find(d)) {
            self.tree.rescan(id);
        }
    }

    /// Updates the tree and the file list after changes on disk.
    pub fn after_change(&mut self) {
        self.refresh_rows(self.current());
        self.refresh_files();
    }

    /// Rereads `dirs` and updates the screen, keeping the cursor on the same file
    /// (if it still exists).
    pub fn rescan_paths<P: AsRef<Path>>(&mut self, dirs: &[P]) {
        self.keeping_cursor(|pane| {
            for dir in dirs {
                pane.rescan_path(dir.as_ref());
            }
        });
    }

    /// Changes the sort order, keeping the cursor on the same file.
    pub fn set_sort(&mut self, sort: Sort) {
        self.keeping_cursor(|pane| pane.sort = sort);
    }

    /// Runs `change` and updates the screen with the cursor on the same file as
    /// before (if it still exists).
    fn keeping_cursor(&mut self, change: impl FnOnce(&mut Self)) {
        // Saved before the change: afterwards, the references in the list may
        // no longer be valid.
        let selected = self.selected_file().and_then(|f| {
            let entry = self.tree.node(f.dir).files.get(f.idx)?;
            Some((f.dir, entry.name.clone()))
        });
        change(self);
        self.after_change();
        if let Some((dir, name)) = selected
            && let Some(i) = self
                .files
                .iter()
                .position(|&f| f.dir == dir && self.file(f).name == name)
        {
            self.file_state.select(Some(i));
        }
    }

    /// Moves the tree cursor to `path`, reading and expanding the path.
    /// Stops at the last existing directory; returns whether it reached the target.
    pub fn goto(&mut self, path: &Path) -> bool {
        match self.walk(path) {
            Some((id, reached)) => {
                self.show_node(id);
                reached
            }
            None => false,
        }
    }

    /// Finds the directory for `path` in the tree, reading what is missing from disk.
    /// Returns the deepest one that exists and whether it got all the way; `None` if
    /// the path is outside the tree.
    fn walk(&mut self, path: &Path) -> Option<(NodeId, bool)> {
        let rel = path.strip_prefix(self.tree.path(ROOT)).ok()?;
        let mut cur = ROOT;
        for comp in rel.components() {
            let Component::Normal(name) = comp else {
                continue;
            };
            self.tree.scan(cur);
            let node = self.tree.node(cur);
            match node
                .children
                .iter()
                .find(|&&c| self.tree.node(c).name == name)
            {
                Some(&child) => cur = child,
                None => return Some((cur, false)),
            }
        }
        Some((cur, true))
    }

    /// Puts the tree cursor on `id`, expanding the path to it.
    pub fn show_node(&mut self, id: NodeId) {
        self.tree.reveal(id);
        self.tree.scan(id);
        if self.current() != id {
            self.refresh_rows(id);
            self.file_state.select(Some(0));
            self.refresh_files();
        }
    }

    /// Treespec: the directory for a half-typed path, where the last
    /// piece is the start of a name (`/home/an` → `/home/andre`).
    pub fn treespec_target(&mut self, text: &str) -> Option<NodeId> {
        let (dir, partial) = split_partial(text);
        let (node, reached) = self.walk(Path::new(dir))?;
        if !reached || partial.is_empty() {
            return Some(node);
        }
        Some(self.treespec_match(node, partial).unwrap_or(node))
    }

    /// First subdirectory of `parent` whose name starts with `partial`
    /// (case-insensitive). Hidden ones only if the start is a dot.
    fn treespec_match(&self, parent: NodeId, partial: &str) -> Option<NodeId> {
        let partial = partial.to_lowercase();
        self.treespec_choices(parent, &partial)
            .into_iter()
            .find(|&c| {
                self.tree
                    .node(c)
                    .name
                    .to_string_lossy()
                    .to_lowercase()
                    .starts_with(&partial)
            })
    }

    fn treespec_choices(&self, parent: NodeId, partial: &str) -> Vec<NodeId> {
        let show_hidden = self.tree.show_hidden || partial.starts_with('.');
        self.tree
            .node(parent)
            .children
            .iter()
            .copied()
            .filter(|&c| show_hidden || !self.tree.node(c).hidden)
            .collect()
    }

    /// Completes the last piece with the name of the directory it points to.
    pub fn treespec_complete(&mut self, text: &str) -> Option<String> {
        let (dir, partial) = split_partial(text);
        if partial.is_empty() {
            return None;
        }
        let (parent, true) = self.walk(Path::new(dir))? else {
            return None;
        };
        let target = self.treespec_match(parent, partial)?;
        Some(format!(
            "{dir}{}",
            self.tree.node(target).name.to_string_lossy()
        ))
    }

    /// Replaces the last piece with the next (or previous) directory at the same
    /// level (Tab / Shift+Tab).
    pub fn treespec_sibling(&mut self, text: &str, forward: bool) -> Option<String> {
        let (dir, partial) = split_partial(text);
        let (parent, true) = self.walk(Path::new(dir))? else {
            return None;
        };
        let choices = self.treespec_choices(parent, partial);
        if choices.is_empty() {
            return None;
        }
        let pos = choices
            .iter()
            .position(|&c| self.tree.node(c).name.to_string_lossy() == partial);
        let next = match (pos, forward) {
            (None, true) => 0,
            (None, false) => choices.len() - 1,
            (Some(i), true) => (i + 1) % choices.len(),
            (Some(i), false) => (i + choices.len() - 1) % choices.len(),
        };
        Some(format!(
            "{dir}{}",
            self.tree.node(choices[next]).name.to_string_lossy()
        ))
    }

    /// Shift+letter: next directory (in the tree) or file (in the list)
    /// whose name starts with `c`, wrapping around at the end.
    pub fn spell_jump(&mut self, c: char) {
        let c = c.to_lowercase().next().unwrap_or(c);
        let starts = |name: &std::ffi::OsStr| name.to_string_lossy().to_lowercase().starts_with(c);
        match self.focus {
            Focus::Tree => {
                let n = self.rows.len();
                let cur = self.tree_state.selected().unwrap_or(0);
                if let Some(i) = (1..=n)
                    .map(|k| (cur + k) % n)
                    .find(|&i| starts(&self.tree.node(self.rows[i].id).name))
                {
                    self.select_row(i);
                }
            }
            Focus::Files => {
                let n = self.files.len();
                let cur = self.file_state.selected().unwrap_or(0);
                if let Some(i) = (1..=n)
                    .map(|k| (cur + k) % n)
                    .find(|&i| starts(&self.file(self.files[i]).name))
                {
                    self.file_state.select(Some(i));
                }
            }
        }
    }

    pub fn apply_tree(&mut self, cmd: Command) {
        let cur = self.current();
        let pos = self.tree_state.selected().unwrap_or(0);
        match cmd {
            Command::Expand => {
                self.tree.expand(cur);
                self.refresh_rows(cur);
            }
            Command::Collapse => {
                let node = self.tree.node(cur);
                if node.expanded && self.tree.has_children(cur) {
                    self.tree.collapse(cur);
                    self.refresh_rows(cur);
                } else if let Some(parent) = node.parent {
                    self.select_node(parent);
                }
            }
            _ => self.select_row(move_cursor(cmd, pos, self.rows.len(), self.tree_page)),
        }
    }

    pub fn apply_files(&mut self, cmd: Command) {
        let pos = self.file_state.selected().unwrap_or(0);
        let len = self.files.len();
        let cols = if self.display.multi_column() {
            self.file_cols.max(1)
        } else {
            1
        };
        let target = match cmd {
            // In columns, ← and → move one file and ↑ ↓ one row; in a
            // single column, ← and → page (as in ZTreeWin).
            Command::Left if cols > 1 => pos.saturating_sub(1),
            Command::Right if cols > 1 => (pos + 1).min(len.saturating_sub(1)),
            Command::Left => move_cursor(Command::PageUp, pos, len, self.file_page),
            Command::Right => move_cursor(Command::PageDown, pos, len, self.file_page),
            Command::Up if cols > 1 => pos.checked_sub(cols).unwrap_or(pos),
            Command::Down if cols > 1 && pos + cols < len => pos + cols,
            Command::Down if cols > 1 => pos,
            _ => move_cursor(cmd, pos, len, self.file_page),
        };
        self.file_state.select(Some(target));
    }

    /// Alt+F: next display mode.
    pub fn cycle_display(&mut self) {
        self.display = self.display.next();
        *self.file_state.offset_mut() = 0;
    }

    pub fn set_view(&mut self, view: FileView) {
        self.view = view;
        self.focus = Focus::Files;
        self.file_state.select(Some(0));
        self.refresh_files();
    }

    /// Back to the tree. Leaving Showall/Branch, the tree cursor
    /// goes to the directory of the file that was selected.
    pub fn back_to_tree(&mut self) {
        let target = (self.view != FileView::Dir)
            .then(|| self.selected_file())
            .flatten()
            .map(|f| f.dir);
        self.view = FileView::Dir;
        self.focus = Focus::Tree;
        match target {
            Some(dir) => {
                self.tree.reveal(dir);
                self.refresh_rows(dir);
                self.file_state.select(Some(0));
                self.refresh_files();
            }
            None => self.refresh_files(),
        }
    }

    pub fn select_row(&mut self, index: usize) {
        if Some(index) != self.tree_state.selected() {
            self.tree_state.select(Some(index));
            let id = self.current();
            self.tree.scan(id);
            self.file_state.select(Some(0));
            self.refresh_files();
        }
    }

    pub fn select_node(&mut self, id: NodeId) {
        if let Some(i) = self.rows.iter().position(|r| r.id == id) {
            self.select_row(i);
        }
    }

    /// Recomputes the visible rows, keeping the cursor on `keep`.
    pub fn refresh_rows(&mut self, keep: NodeId) {
        self.rows = self.tree.visible_rows();
        let i = self.rows.iter().position(|r| r.id == keep).unwrap_or(0);
        self.tree_state.select(Some(i));
    }

    /// "DISK Statistics", as in ZTreeWin: everything logged, what of it
    /// matches the filespec and what of it is tagged. Matching and Tagged
    /// count what Showall would list: not inside hidden directories (unless
    /// they are displayed), nor hidden files.
    pub fn disk_stats(&self) -> DiskStats {
        let mut stats = DiskStats::default();
        let add = |t: &mut Count, f: &FileEntry| {
            t.files += 1;
            t.bytes += f.size;
        };
        let scanned = |id: &NodeId| self.tree.node(*id).scanned;
        for id in self.tree.subtree(ROOT).iter().filter(|id| scanned(id)) {
            for f in &self.tree.node(*id).files {
                add(&mut stats.logged, f);
            }
        }
        for id in self
            .tree
            .visible_subtree(ROOT)
            .iter()
            .filter(|id| scanned(id))
        {
            for f in &self.tree.node(*id).files {
                if !self.tree.show_hidden && f.is_hidden() {
                    continue;
                }
                if self.filespec.matches_entry(f) {
                    add(&mut stats.matching, f);
                }
                if f.tagged {
                    add(&mut stats.tagged, f);
                }
            }
        }
        stats
    }

    /// Rebuilds the file window list according to the view and the filespec.
    pub fn refresh_files(&mut self) {
        let dirs = match self.view {
            FileView::Dir => vec![self.current()],
            FileView::Branch(id) => self.tree.visible_subtree(id),
            FileView::Showall => self.tree.visible_subtree(ROOT),
        };
        let mut files = Vec::new();
        for (rank, dir) in dirs.into_iter().enumerate() {
            for (idx, f) in self.tree.node(dir).files.iter().enumerate() {
                let shown = self.tree.show_hidden || !f.is_hidden();
                if shown && self.filespec.matches_entry(f) {
                    files.push((FileRef { dir, idx }, rank));
                }
            }
        }
        let sort = self.sort;
        sort::sort_by(&mut files, &sort, |&(f, rank)| {
            sort.key_for(self.file(f), rank)
        });
        self.files = files.into_iter().map(|(f, _)| f).collect();

        let last = self.files.len().saturating_sub(1);
        let sel = self.file_state.selected().unwrap_or(0).min(last);
        self.file_state.select(Some(sel));
    }
}

/// Files and bytes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Count {
    pub files: u64,
    pub bytes: u64,
}

/// The "DISK Statistics" panel (see `Pane::disk_stats`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiskStats {
    pub logged: Count,
    pub matching: Count,
    pub tagged: Count,
}

/// "/home/an" → ("/home/", "an"); "/home/" → ("/home/", ""). On Windows,
/// `\` also separates.
fn split_partial(text: &str) -> (&str, &str) {
    match text.rfind(std::path::is_separator) {
        Some(i) => (&text[..=i], &text[i + 1..]),
        None => ("", text),
    }
}

/// New position of a cursor in a list of `len` items.
pub fn move_cursor(cmd: Command, pos: usize, len: usize, page: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let last = len - 1;
    let page = page.max(1);
    match cmd {
        Command::Up => pos.saturating_sub(1),
        Command::Down => (pos + 1).min(last),
        Command::PageUp => pos.saturating_sub(page),
        Command::PageDown => (pos + page).min(last),
        Command::Home => 0,
        Command::End => last,
        _ => pos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::create_dir_all;

    #[test]
    fn cursor_stays_in_bounds() {
        assert_eq!(move_cursor(Command::Up, 0, 5, 3), 0);
        assert_eq!(move_cursor(Command::Down, 4, 5, 3), 4);
        assert_eq!(move_cursor(Command::PageDown, 1, 5, 3), 4);
        assert_eq!(move_cursor(Command::PageUp, 4, 5, 3), 1);
        assert_eq!(move_cursor(Command::End, 0, 5, 3), 4);
        assert_eq!(move_cursor(Command::Down, 0, 0, 3), 0);
    }

    #[test]
    fn after_change_keeps_cursor_on_same_file() {
        let tmp = crate::testing::tempdir();
        for f in ["b", "d"] {
            std::fs::write(tmp.path().join(f), "").unwrap();
        }
        let mut pane = Pane::new(tmp.path().to_path_buf());
        pane.file_state.select(Some(1)); // "d"
        std::fs::write(tmp.path().join("a"), "").unwrap();
        std::fs::write(tmp.path().join("c"), "").unwrap();
        pane.rescan_paths(&[tmp.path()]);
        let name = &pane.file(pane.selected_file().unwrap()).name;
        assert_eq!(name, "d");

        // If the file vanished, the cursor stays inside the list.
        std::fs::remove_file(tmp.path().join("d")).unwrap();
        pane.rescan_paths(&[tmp.path()]);
        assert_eq!(pane.file_state.selected(), Some(2));
    }

    #[test]
    fn disk_stats_cover_everything_logged() {
        let tmp = crate::testing::tempdir();
        std::fs::write(tmp.path().join("a.rs"), "0123456789").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "01234").unwrap();
        create_dir_all(tmp.path().join("sub")).unwrap();
        std::fs::write(tmp.path().join("sub/c.rs"), "012").unwrap();
        let mut pane = Pane::new(tmp.path().to_path_buf());
        let count = |files, bytes| Count { files, bytes };

        pane.filespec = Filespec::parse("*.rs").unwrap();
        let b = pane
            .tree
            .node(ROOT)
            .files
            .iter()
            .position(|f| f.name == "b.txt");
        pane.tree.file_mut(ROOT, b.unwrap()).tagged = true;

        let stats = pane.disk_stats();
        assert_eq!(stats.logged, count(3, 18));
        assert_eq!(stats.matching, count(2, 13), "a.rs and sub/c.rs");
        assert_eq!(stats.tagged, count(1, 5), "tagged, even if not matching");

        // A hidden directory counts as logged, but not as matching.
        create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join(".git/x.rs"), "0123").unwrap();
        crate::testing::hide(&tmp.path().join(".git"));
        pane.tree.show_hidden = false;
        pane.tree.rescan(ROOT);
        let git = pane.tree.find(&tmp.path().join(".git")).unwrap();
        pane.tree.scan(git);
        let stats = pane.disk_stats();
        assert_eq!(stats.logged, count(4, 22));
        assert_eq!(stats.matching, count(2, 13));
    }

    #[test]
    fn treespec_follows_what_is_typed() {
        let tmp = crate::testing::tempdir();
        for d in ["Docs/rust", "Docs/notes", "downloads", ".config/xt"] {
            create_dir_all(tmp.path().join(d)).unwrap();
        }
        let root = tmp.path().to_string_lossy().into_owned();
        let mut pane = Pane::new(tmp.path().to_path_buf());
        let name = |pane: &Pane, id| pane.tree.node(id).name.to_string_lossy().into_owned();

        let id = pane.treespec_target(&format!("{root}/do")).unwrap();
        assert_eq!(name(&pane, id), "Docs", "case-insensitive, first in order");
        let id = pane.treespec_target(&format!("{root}/dow")).unwrap();
        assert_eq!(name(&pane, id), "downloads");
        let id = pane.treespec_target(&format!("{root}/Docs/n")).unwrap();
        assert_eq!(name(&pane, id), "notes");
        let id = pane.treespec_target(&format!("{root}/.c")).unwrap();
        assert_eq!(name(&pane, id), ".config");
        // Nothing matches: stays in the parent directory.
        let id = pane.treespec_target(&format!("{root}/zz")).unwrap();
        assert_eq!(id, ROOT);
        assert!(pane.treespec_target("/fora/da/arvore").is_none());

        assert_eq!(
            pane.treespec_complete(&format!("{root}/do")),
            Some(format!("{root}/Docs"))
        );
        let next = pane
            .treespec_sibling(&format!("{root}/Docs"), true)
            .unwrap();
        assert_eq!(next, format!("{root}/downloads"));
        let wrap = pane.treespec_sibling(&next, true).unwrap();
        assert_eq!(wrap, format!("{root}/.config"), "hidden shown by default");
        let back = pane
            .treespec_sibling(&format!("{root}/Docs"), false)
            .unwrap();
        assert_eq!(back, format!("{root}/.config"));
    }

    #[test]
    fn shift_letter_jumps_with_wraparound() {
        let tmp = crate::testing::tempdir();
        for d in ["alpha", "beta", "bravo"] {
            create_dir_all(tmp.path().join(d)).unwrap();
        }
        let mut pane = Pane::new(tmp.path().to_path_buf());
        let current = |p: &Pane| p.tree.node(p.current()).name.to_string_lossy().into_owned();
        pane.spell_jump('B');
        assert_eq!(current(&pane), "beta");
        pane.spell_jump('B');
        assert_eq!(current(&pane), "bravo");
        pane.spell_jump('B');
        assert_eq!(current(&pane), "beta", "wraps around");
        pane.spell_jump('Z');
        assert_eq!(current(&pane), "beta", "no match: stays");
    }

    #[test]
    fn arrows_move_by_column_in_multi_column_modes() {
        let tmp = crate::testing::tempdir();
        for i in 0..10 {
            std::fs::write(tmp.path().join(format!("f{i}")), "").unwrap();
        }
        let mut pane = Pane::new(tmp.path().to_path_buf());
        pane.display = FileDisplay::Names;
        pane.file_cols = 4;
        pane.file_page = 8;
        let sel = |p: &Pane| p.file_state.selected().unwrap();
        pane.apply_files(Command::Right);
        assert_eq!(sel(&pane), 1);
        pane.apply_files(Command::Down);
        assert_eq!(sel(&pane), 5);
        pane.apply_files(Command::Down);
        assert_eq!(sel(&pane), 9);
        pane.apply_files(Command::Down);
        assert_eq!(sel(&pane), 9, "no row below");
        pane.apply_files(Command::Up);
        assert_eq!(sel(&pane), 5);
        pane.apply_files(Command::Left);
        assert_eq!(sel(&pane), 4);

        // In a single column, ← and → page.
        pane.display = FileDisplay::Details;
        pane.file_page = 3;
        pane.apply_files(Command::Right);
        assert_eq!(sel(&pane), 7);
        pane.cycle_display();
        assert_eq!(pane.display, FileDisplay::LongNames);
        pane.cycle_display();
        assert_eq!(pane.display, FileDisplay::Names);
    }

    #[test]
    fn goto_opens_the_path() {
        let tmp = crate::testing::tempdir();
        create_dir_all(tmp.path().join("a/b/c")).unwrap();
        let mut pane = Pane::new(tmp.path().to_path_buf());

        assert!(pane.goto(&tmp.path().join("a/b/c")));
        assert_eq!(pane.current_path(), tmp.path().join("a/b/c"));
        assert!(pane.rows.iter().any(|r| r.id == pane.current()));

        // Path that does not exist: stops at the last directory that exists.
        assert!(!pane.goto(&tmp.path().join("a/x/y")));
        assert_eq!(pane.current_path(), tmp.path().join("a"));
        // Outside the tree: no movement.
        assert!(!pane.goto(Path::new("/elsewhere")));
        assert_eq!(pane.current_path(), tmp.path().join("a"));
    }
}
