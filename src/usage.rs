//! The Usage view (Alt+W): where the space of a branch goes. The folders
//! and files of a directory by the space they take, with their share of it,
//! going down into the folders to what fills the disk. The numbers come
//! from the tree already in memory ([`Tree::usage`]), recomputed only when
//! it changes.

use std::collections::HashMap;
use std::time::SystemTime;

use ratatui::widgets::ListState;

use crate::model::tree::{FileKind, NodeId, Tree, Usage, UsageMap};

/// How the rows are sorted (`S` cycles them).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Order {
    #[default]
    Size,
    Name,
    Files,
    Date,
}

impl Order {
    pub fn next(self) -> Order {
        match self {
            Order::Size => Order::Name,
            Order::Name => Order::Files,
            Order::Files => Order::Date,
            Order::Date => Order::Size,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Order::Size => "size",
            Order::Name => "name",
            Order::Files => "files",
            Order::Date => "date",
        }
    }
}

/// What a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Dir(NodeId),
    /// A file of the directory shown: its index there.
    File(usize),
}

/// One row of the view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub item: Item,
    pub name: String,
    /// Space it takes, as chosen with `A` (on the disk or apparent).
    pub size: u64,
    pub disk: u64,
    pub apparent: u64,
    pub files: u64,
    pub modified: Option<SystemTime>,
    /// One letter, as in other disk usage tools: `>` another disk, `!`
    /// unreadable, `.` something below unreadable, `e` empty, `@` a link or
    /// special file, `H` another hard link of a file already counted.
    pub mark: char,
}

pub struct UsageView {
    /// Where the analysis started: everything below it was logged.
    pub root: NodeId,
    /// The directory listed.
    pub dir: NodeId,
    pub list: ListState,
    /// Sizes as the files' sizes, not the space on the disk (`A`).
    pub apparent: bool,
    /// The column with the number of files (`C`).
    pub counts: bool,
    pub order: Order,
    /// Height of the list as of the last draw (for PgUp/PgDn).
    pub page: usize,
    /// The row under the cursor as of the last draw, for the panel on the
    /// right.
    pub shown: Option<Row>,
    devices: HashMap<NodeId, Option<u64>>,
    /// The tree version the map was computed for.
    cached: Option<(u64, UsageMap)>,
}

impl UsageView {
    pub fn new(root: NodeId) -> Self {
        let mut list = ListState::default();
        list.select(Some(0));
        UsageView {
            root,
            dir: root,
            list,
            apparent: false,
            counts: false,
            order: Order::default(),
            page: 10,
            shown: None,
            devices: HashMap::new(),
            cached: None,
        }
    }

    /// The sums, computed again only when the tree changed.
    pub fn map(&mut self, tree: &Tree) -> &UsageMap {
        let version = tree.version();
        if self.cached.as_ref().is_none_or(|(v, _)| *v != version) {
            let map = tree.usage(self.root, &mut self.devices);
            self.cached = Some((version, map));
        }
        &self.cached.as_ref().expect("just computed").1
    }

    /// What the directory listed takes.
    pub fn total(&mut self, tree: &Tree) -> Usage {
        let dir = self.dir;
        self.map(tree).dirs.get(&dir).copied().unwrap_or_default()
    }

    /// Rows of the directory listed: its folders and files mixed, sorted.
    pub fn rows(&mut self, tree: &Tree) -> Vec<Row> {
        let (dir, apparent, order) = (self.dir, self.apparent, self.order);
        let map = self.map(tree);
        let node = tree.node(dir);
        let mut rows = Vec::with_capacity(node.children.len() + node.files.len());
        for &c in &node.children {
            let child = tree.node(c);
            let u = map.dirs.get(&c).copied().unwrap_or_default();
            let mark = if u.other_disk {
                '>'
            } else if child.error.is_some() {
                '!'
            } else if u.incomplete {
                '.'
            } else if child.children.is_empty() && child.files.is_empty() {
                'e'
            } else {
                ' '
            };
            rows.push(Row {
                item: Item::Dir(c),
                name: child.name.to_string_lossy().into_owned(),
                size: if apparent { u.apparent } else { u.disk },
                disk: u.disk,
                apparent: u.apparent,
                files: u.files,
                modified: None,
                mark,
            });
        }
        for (i, f) in node.files.iter().enumerate() {
            let repeat = map.repeats.contains(&(dir, i));
            let (disk, size) = if repeat { (0, 0) } else { (f.disk, f.size) };
            let mark = if repeat {
                'H'
            } else if f.kind != FileKind::File {
                '@'
            } else {
                ' '
            };
            rows.push(Row {
                item: Item::File(i),
                name: f.name.to_string_lossy().into_owned(),
                size: if apparent { size } else { disk },
                disk,
                apparent: size,
                files: 1,
                modified: f.modified,
                mark,
            });
        }
        let by_name = |a: &Row, b: &Row| a.name.to_lowercase().cmp(&b.name.to_lowercase());
        match order {
            // Other disks last: their size is not part of this one.
            Order::Size => rows.sort_by(|a, b| {
                (a.mark == '>')
                    .cmp(&(b.mark == '>'))
                    .then(b.size.cmp(&a.size))
                    .then_with(|| by_name(a, b))
            }),
            Order::Name => rows.sort_by(by_name),
            Order::Files => rows.sort_by(|a, b| b.files.cmp(&a.files).then_with(|| by_name(a, b))),
            Order::Date => {
                rows.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| by_name(a, b)))
            }
        }
        rows
    }

    /// Goes into the folder `id`, with the cursor on the first row.
    pub fn enter(&mut self, id: NodeId) {
        self.dir = id;
        self.list.select(Some(0));
    }

    /// Goes back to the folder above, with the cursor on the folder it came
    /// from. `false` at the root of the analysis.
    pub fn back(&mut self, tree: &Tree) -> bool {
        if self.dir == self.root {
            return false;
        }
        let from = self.dir;
        let Some(parent) = tree.node(from).parent else {
            return false;
        };
        self.dir = parent;
        let at = self
            .rows(tree)
            .iter()
            .position(|r| r.item == Item::Dir(from))
            .unwrap_or(0);
        self.list.select(Some(at));
        true
    }

    /// The row under the cursor.
    pub fn selected(&mut self, tree: &Tree) -> Option<Row> {
        let i = self.list.selected().unwrap_or(0);
        self.rows(tree).into_iter().nth(i)
    }

    /// Moves the cursor by `delta` rows (PgUp/PgDn: a page), within `len`.
    pub fn move_by(&mut self, delta: isize, len: usize) {
        let at = self.list.selected().unwrap_or(0) as isize + delta;
        let last = len.saturating_sub(1) as isize;
        self.list.select(Some(at.clamp(0, last.max(0)) as usize));
    }
}

/// "512 B", "1.2 KiB", "33.0 GiB": short, for the columns.
pub fn size(n: u64) -> String {
    const UNITS: [&str; 5] = ["KiB", "MiB", "GiB", "TiB", "PiB"];
    if n < 1024 {
        return format!("{n} B");
    }
    let mut v = n as f64 / 1024.0;
    let mut unit = 0;
    while v >= 1023.95 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    format!("{v:.1} {}", UNITS[unit])
}

/// `part` as a share of `whole`: "41%", "5.8%", "<0.1%", or "-" when there
/// is nothing to share.
pub fn percent(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "-".into();
    }
    let pct = part as f64 * 100.0 / whole as f64;
    if pct > 0.0 && pct < 0.05 {
        "<0.1%".into()
    } else if pct == 0.0 || pct >= 9.95 {
        format!("{pct:.0}%")
    } else {
        format!("{pct:.1}%")
    }
}

/// The filled part of a bar of `width` cells for `part` against `largest`
/// (which fills it all), in eighths of a cell; the rest is the track, drawn
/// by the caller.
pub fn bar(part: u64, largest: u64, width: usize) -> String {
    const PARTS: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
    let eighths = if largest == 0 {
        0
    } else {
        ((part as u128 * (width * 8) as u128 + largest as u128 / 2) / largest as u128) as usize
    }
    .min(width * 8);
    let mut out = "█".repeat(eighths / 8);
    out.push_str(PARTS[eighths % 8]);
    out
}

/// How much of the whole a share is, for its color: up to 10%, up to 40%,
/// more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Light,
    Medium,
    Heavy,
}

pub fn weight(part: u64, whole: u64) -> Weight {
    let pct = if whole == 0 {
        0.0
    } else {
        part as f64 * 100.0 / whole as f64
    };
    if pct > 40.0 {
        Weight::Heavy
    } else if pct > 10.0 {
        Weight::Medium
    } else {
        Weight::Light
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn logged(dir: &std::path::Path) -> Tree {
        let mut tree = Tree::new(dir.to_path_buf());
        tree.expand_all(crate::model::tree::ROOT);
        tree
    }

    #[test]
    fn shares_and_bars() {
        assert_eq!(percent(1, 4), "25%");
        assert_eq!(percent(58, 1000), "5.8%");
        assert_eq!(percent(1, 100_000), "<0.1%");
        assert_eq!(percent(0, 10), "0%");
        assert_eq!(percent(3, 0), "-");
        assert_eq!(bar(10, 10, 4), "████");
        assert_eq!(bar(5, 10, 4), "██");
        assert_eq!(bar(1, 16, 2), "▏");
        assert_eq!(bar(0, 16, 2), "");
        assert_eq!(weight(5, 100), Weight::Light);
        assert_eq!(weight(30, 100), Weight::Medium);
        assert_eq!(weight(41, 100), Weight::Heavy);
        assert_eq!(size(512), "512 B");
        assert_eq!(size(1536), "1.5 KiB");
        assert_eq!(size(35_325_000_000), "32.9 GiB");
    }

    #[test]
    fn folders_and_files_mixed_by_size() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::create_dir_all(p.join("small")).unwrap();
        fs::create_dir_all(p.join("empty")).unwrap();
        fs::write(p.join("small/a"), vec![1u8; 1000]).unwrap();
        fs::write(p.join("big.tar"), vec![1u8; 50_000]).unwrap();
        fs::write(p.join(".hidden"), vec![1u8; 3000]).unwrap();
        let mut tree = logged(p);
        tree.show_hidden = false;
        let mut view = UsageView::new(crate::model::tree::ROOT);
        view.apparent = true;
        let rows = view.rows(&tree);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        // The file first: the largest. Hidden files count too.
        assert_eq!(names, ["big.tar", ".hidden", "small", "empty"]);
        assert_eq!(rows[3].mark, 'e');
        assert_eq!(view.total(&tree).apparent, 54_000);
        assert_eq!(view.total(&tree).files, 3);
        view.order = Order::Name;
        assert_eq!(view.rows(&tree)[0].name, ".hidden");
    }

    #[test]
    fn going_down_and_back() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::create_dir_all(p.join("a/b")).unwrap();
        fs::create_dir_all(p.join("z")).unwrap();
        fs::write(p.join("a/b/f"), vec![1u8; 9000]).unwrap();
        let tree = logged(p);
        let mut view = UsageView::new(crate::model::tree::ROOT);
        let Item::Dir(a) = view.rows(&tree)[0].item else {
            panic!("a first")
        };
        view.enter(a);
        assert_eq!(view.rows(&tree)[0].name, "b");
        assert!(view.back(&tree));
        assert_eq!(view.selected(&tree).unwrap().name, "a");
        assert!(!view.back(&tree), "the root of the analysis");
    }

    #[cfg(unix)]
    #[test]
    fn hard_links_count_once() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::create_dir_all(p.join("one")).unwrap();
        fs::create_dir_all(p.join("two")).unwrap();
        fs::write(p.join("one/f"), vec![1u8; 20_000]).unwrap();
        fs::hard_link(p.join("one/f"), p.join("two/f")).unwrap();
        let tree = logged(p);
        let mut view = UsageView::new(crate::model::tree::ROOT);
        view.apparent = true;
        assert_eq!(view.total(&tree).apparent, 20_000);
        let rows = view.rows(&tree);
        let one = rows.iter().find(|r| r.name == "one").unwrap();
        let two = rows.iter().find(|r| r.name == "two").unwrap();
        assert_eq!((one.apparent, two.apparent), (20_000, 0));
        let Item::Dir(two_id) = two.item else {
            panic!()
        };
        view.enter(two_id);
        assert_eq!(view.rows(&tree)[0].mark, 'H');
    }

    #[test]
    fn other_disks_are_left_out() {
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::create_dir_all(p.join("mnt")).unwrap();
        fs::write(p.join("mnt/f"), vec![1u8; 5000]).unwrap();
        fs::write(p.join("here"), vec![1u8; 100]).unwrap();
        let tree = logged(p);
        let mut view = UsageView::new(crate::model::tree::ROOT);
        view.apparent = true;
        // "mnt" on another device, as a mount point would be.
        let mnt = tree.node(crate::model::tree::ROOT).children[0];
        view.devices.insert(crate::model::tree::ROOT, Some(1));
        view.devices.insert(mnt, Some(2));
        let rows = view.rows(&tree);
        assert_eq!(rows.last().unwrap().mark, '>');
        assert_eq!(view.total(&tree).apparent, 100);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_folders_are_flagged() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = crate::testing::tempdir();
        let p = tmp.path();
        fs::create_dir_all(p.join("a/locked")).unwrap();
        fs::set_permissions(p.join("a/locked"), fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::read_dir(p.join("a/locked")).is_ok(); // running as root
        let tree = logged(p);
        fs::set_permissions(p.join("a/locked"), fs::Permissions::from_mode(0o755)).unwrap();
        if readable {
            return;
        }
        let mut view = UsageView::new(crate::model::tree::ROOT);
        assert_eq!(view.rows(&tree)[0].mark, '.');
        let Item::Dir(a) = view.rows(&tree)[0].item else {
            panic!()
        };
        view.enter(a);
        assert_eq!(view.rows(&tree)[0].mark, '!');
    }
}
