//! Sorting of the file list (Alt+S).

use std::time::SystemTime;

use super::tree::FileEntry;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Name,
    Ext,
    Date,
    Size,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sort {
    pub key: SortKey,
    pub descending: bool,
    /// In Branch/Showall: groups by directory before sorting.
    pub path_first: bool,
}

impl Sort {
    /// E.g. "Name↑", "Size↓".
    pub fn label(&self) -> String {
        let key = match self.key {
            SortKey::Name => "Name",
            SortKey::Ext => "Ext",
            SortKey::Date => "Date",
            SortKey::Size => "Size",
        };
        format!("{key}{}", if self.descending { "↓" } else { "↑" })
    }

    /// Sort key of a file. `dir_rank` is the position of its directory
    /// in the tree: it breaks ties between equal names (or comes first, with
    /// `path_first`).
    pub fn key_for(&self, file: &FileEntry, dir_rank: usize) -> SortKeyValue {
        let name = file.name.to_string_lossy().to_lowercase();
        let primary = match self.key {
            SortKey::Name => Primary::Text(String::new()),
            SortKey::Ext => Primary::Text(extension(&name).to_string()),
            SortKey::Date => Primary::Time(file.modified.unwrap_or(SystemTime::UNIX_EPOCH)),
            SortKey::Size => Primary::Size(file.size),
        };
        SortKeyValue {
            group: if self.path_first { dir_rank } else { 0 },
            primary,
            name,
            dir_rank,
        }
    }
}

/// Comparable value built once per file (sorting hundreds of thousands
/// of files can't recompute names on every comparison).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SortKeyValue {
    group: usize,
    primary: Primary,
    name: String,
    dir_rank: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Primary {
    Text(String),
    Time(SystemTime),
    Size(u64),
}

/// Extension (without the dot); hidden files like ".bashrc" have none.
fn extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(0) | None => "",
        Some(i) => &name[i + 1..],
    }
}

/// Sorts `items`, computing each one's key only once.
/// In descending order, directory groups (with `path_first`) stay
/// in tree order; only the contents of each group are reversed.
pub fn sort_by<T>(items: &mut Vec<T>, sort: &Sort, key: impl Fn(&T) -> SortKeyValue) {
    let mut keyed: Vec<(SortKeyValue, T)> = items.drain(..).map(|t| (key(&t), t)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    if sort.descending {
        let mut start = 0;
        while start < keyed.len() {
            let group = keyed[start].0.group;
            let end = keyed[start..]
                .iter()
                .position(|(k, _)| k.group != group)
                .map_or(keyed.len(), |n| start + n);
            keyed[start..end].reverse();
            start = end;
        }
    }
    items.extend(keyed.into_iter().map(|(_, t)| t));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tree::FileKind;
    use std::time::Duration;

    fn file(name: &str, size: u64, secs: u64) -> FileEntry {
        FileEntry {
            name: name.into(),
            size,
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
            mode: 0o644,
            kind: FileKind::File,
            tagged: false,
        }
    }

    fn sorted(files: &[(FileEntry, usize)], sort: Sort) -> Vec<String> {
        let mut items: Vec<&(FileEntry, usize)> = files.iter().collect();
        sort_by(&mut items, &sort, |(f, rank)| sort.key_for(f, *rank));
        items
            .iter()
            .map(|(f, _)| f.name.to_string_lossy().into_owned())
            .collect()
    }

    fn sample() -> Vec<(FileEntry, usize)> {
        vec![
            (file("b.txt", 30, 100), 0),
            (file("A.rs", 10, 300), 0),
            (file("c.md", 20, 200), 1),
            (file("a.rs", 5, 50), 1),
        ]
    }

    #[test]
    fn sorts_by_each_key() {
        let s = |key| Sort {
            key,
            ..Sort::default()
        };
        assert_eq!(
            sorted(&sample(), s(SortKey::Name)),
            ["A.rs", "a.rs", "b.txt", "c.md"]
        );
        assert_eq!(
            sorted(&sample(), s(SortKey::Ext)),
            ["c.md", "A.rs", "a.rs", "b.txt"]
        );
        assert_eq!(
            sorted(&sample(), s(SortKey::Date)),
            ["a.rs", "b.txt", "c.md", "A.rs"]
        );
        assert_eq!(
            sorted(&sample(), s(SortKey::Size)),
            ["a.rs", "A.rs", "c.md", "b.txt"]
        );
    }

    #[test]
    fn descending_and_path_first() {
        let size_desc = Sort {
            key: SortKey::Size,
            descending: true,
            path_first: false,
        };
        assert_eq!(
            sorted(&sample(), size_desc),
            ["b.txt", "c.md", "A.rs", "a.rs"]
        );

        let by_dir = Sort {
            path_first: true,
            ..Sort::default()
        };
        assert_eq!(sorted(&sample(), by_dir), ["A.rs", "b.txt", "a.rs", "c.md"]);
        let by_dir_desc = Sort {
            descending: true,
            ..by_dir
        };
        assert_eq!(
            sorted(&sample(), by_dir_desc),
            ["b.txt", "A.rs", "c.md", "a.rs"]
        );
    }

    #[test]
    fn extensions_and_labels() {
        assert_eq!(extension(".bashrc"), "");
        assert_eq!(extension("a.tar.gz"), "gz");
        assert_eq!(extension("Makefile"), "");
        assert_eq!(Sort::default().label(), "Name↑");
    }
}
