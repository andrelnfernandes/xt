//! Which editor E opens a file with: the `[editors]` table of the config.
//!
//! Each key is a list of name patterns (as in Filespec: `*.png *.jpg`), each
//! value a command line, with `%f` for the file (or the file at the end).
//! The first key that matches wins. The `binary` key is for the other files
//! whose content is not text; without it, E refuses them, since a text
//! editor would only mangle them. The rest go to the usual editor.

use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};

/// The key for binary files that no pattern took.
pub const BINARY: &str = "binary";

/// The `[editors]` table as written, in the file's order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table(pub Vec<(String, String)>);

impl<'de> Deserialize<'de> for Table {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct InOrder;
        impl<'de> Visitor<'de> for InOrder {
            type Value = Table;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a table of patterns and commands")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Table, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry::<String, String>()? {
                    out.push(entry);
                }
                Ok(Table(out))
            }
        }
        d.deserialize_map(InOrder)
    }
}

/// What E does with a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice<'a> {
    /// This command line (`%f` or the file at the end).
    Command(&'a str),
    /// The usual editor (the `editor` option, `$VISUAL`...).
    Usual,
    /// Nothing: a binary file and no `binary` key.
    #[cfg(test)]
    None,
}

/// The table, ready to match.
#[derive(Debug, Clone, Default)]
pub struct Editors {
    rules: Vec<(GlobSet, String)>,
    binary: Option<String>,
}

impl Editors {
    pub fn from_config(table: &Table) -> Result<Self, String> {
        let mut editors = Editors::default();
        for (key, command) in &table.0 {
            if key == BINARY {
                editors.binary = Some(command.clone());
                continue;
            }
            let mut set = GlobSetBuilder::new();
            for pattern in key.split_whitespace() {
                let glob = GlobBuilder::new(pattern)
                    .case_insensitive(true)
                    .literal_separator(true)
                    .build()
                    .map_err(|e| format!("[editors]: invalid pattern '{pattern}': {}", e.kind()))?;
                set.add(glob);
            }
            let set = set.build().map_err(|e| format!("[editors]: {e}"))?;
            editors.rules.push((set, command.clone()));
        }
        Ok(editors)
    }

    /// Matches `name` against the filename patterns in `[editors]` (ignoring `binary`).
    pub fn match_pattern(&self, name: &OsStr) -> Option<Choice<'_>> {
        let (_, command) = self.rules.iter().find(|(set, _)| set.is_match(name))?;
        Some(match command.trim() {
            "" => Choice::Usual,
            command => Choice::Command(command),
        })
    }

    /// The `binary` command configured in `[editors]`, if non-empty.
    pub fn binary_command(&self) -> Option<&str> {
        self.binary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    /// The editor for a file named `name`; `binary` is only asked when no
    /// pattern matches. An empty command means the usual editor.
    #[cfg(test)]
    pub fn choose(&self, name: &OsStr, binary: impl FnOnce() -> bool) -> Choice<'_> {
        let command = match self.rules.iter().find(|(set, _)| set.is_match(name)) {
            Some((_, command)) => command,
            None if binary() => match &self.binary {
                Some(command) => command,
                None => return Choice::None,
            },
            None => return Choice::Usual,
        };
        match command.trim() {
            "" => Choice::Usual,
            command => Choice::Command(command),
        }
    }
}

/// Whether the file on disk is binary, by the same rule as the viewer's
/// (which then shows it in hex): a null byte near the start.
pub fn is_binary(path: &Path) -> bool {
    let mut head = Vec::new();
    File::open(path)
        .and_then(|f| {
            f.take(crate::viewer::BINARY_PROBE as u64)
                .read_to_end(&mut head)
        })
        .is_ok_and(|_| crate::viewer::looks_binary(&head))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editors(text: &str) -> Editors {
        let table: Table = toml::from_str(text).unwrap();
        Editors::from_config(&table).unwrap()
    }

    #[test]
    fn the_first_pattern_that_matches_wins() {
        let e = editors(
            "'*.PNG *.jpg' = 'gimp %f'\n'*.jpg' = 'other'\n'*.svg' = ''\nbinary = 'hexedit'\n",
        );
        let text = || false;
        assert_eq!(e.choose("a.png".as_ref(), text), Choice::Command("gimp %f"));
        assert_eq!(e.choose("B.JPG".as_ref(), text), Choice::Command("gimp %f"));
        assert_eq!(e.choose("c.svg".as_ref(), || true), Choice::Usual);
        assert_eq!(e.choose("d.rs".as_ref(), text), Choice::Usual);
        assert_eq!(
            e.choose("e.bin".as_ref(), || true),
            Choice::Command("hexedit")
        );
    }

    #[test]
    fn binary_files_without_an_editor_are_refused() {
        let e = Editors::default();
        assert_eq!(e.choose("a.jpg".as_ref(), || true), Choice::None);
        assert_eq!(e.choose("a.txt".as_ref(), || false), Choice::Usual);
    }

    #[test]
    fn binary_is_only_checked_when_no_pattern_matches() {
        let e = editors("'*.png' = 'gimp'\n");
        let never = || panic!("read the file for nothing");
        assert_eq!(e.choose("a.png".as_ref(), never), Choice::Command("gimp"));
    }

    #[test]
    fn bad_patterns_are_errors() {
        let table: Table = toml::from_str("'a[' = 'x'\n").unwrap();
        let err = Editors::from_config(&table).unwrap_err();
        assert!(err.contains("a["), "{err}");
    }

    #[test]
    fn content_decides_what_is_binary() {
        let tmp = crate::testing::tempdir();
        let (text, data) = (tmp.path().join("t.txt"), tmp.path().join("d.txt"));
        std::fs::write(&text, "olá, ação\n").unwrap();
        std::fs::write(&data, b"\xff\xd8\xff\xe0\0\x10JFIF").unwrap();
        assert!(!is_binary(&text));
        assert!(is_binary(&data));
        assert!(!is_binary(&tmp.path().join("missing")));
    }
}
