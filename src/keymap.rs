//! Key map: the defaults, the overrides from the config file and the
//! help text (F1). Everything is derived from the tables below.

use std::collections::HashMap;
use std::fmt;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::{Keys, Options};
use crate::input::Command;
use crate::pane::Focus;

/// Every command that can be bound to a key: its name in the config
/// file, the command and its description in the help.
const COMMANDS: &[(&str, Command, &str)] = &[
    ("up", Command::Up, "move up"),
    ("down", Command::Down, "move down"),
    ("page_up", Command::PageUp, "page up"),
    ("page_down", Command::PageDown, "page down"),
    ("home", Command::Home, "first line"),
    ("end", Command::End, "last line"),
    ("expand", Command::Expand, "expand directory"),
    (
        "collapse",
        Command::Collapse,
        "collapse directory (or go to parent)",
    ),
    (
        "expand_all",
        Command::ExpandAll,
        "log and expand the whole branch",
    ),
    (
        "switch_window",
        Command::SwitchWindow,
        "switch between tree and file window",
    ),
    ("switch_pane", Command::SwitchPane, "switch split pane"),
    (
        "split",
        Command::Split,
        "split the screen in two panes (or back to one)",
    ),
    ("back", Command::Back, "back to tree / close archive"),
    ("filespec", Command::Filespec, "filespec: filter files"),
    ("showall", Command::Showall, "show all logged files"),
    ("branch", Command::Branch, "show all files in the branch"),
    ("view", Command::View, "view file (text/hex)"),
    (
        "relog",
        Command::Relog,
        "read the directory from disk again",
    ),
    (
        "tag",
        Command::Tag,
        "tag file (in the tree: all files here)",
    ),
    (
        "untag",
        Command::Untag,
        "untag file (in the tree: all files here)",
    ),
    (
        "tag_all",
        Command::TagAll,
        "tag all files in the list (tree: all logged)",
    ),
    (
        "untag_all",
        Command::UntagAll,
        "untag all files in the list (tree: all logged)",
    ),
    ("invert", Command::Invert, "invert tags"),
    (
        "copy",
        Command::Copy {
            tagged: false,
            paths: false,
        },
        "copy file",
    ),
    (
        "copy_tagged",
        Command::Copy {
            tagged: true,
            paths: false,
        },
        "copy tagged files",
    ),
    (
        "copy_tagged_paths",
        Command::Copy {
            tagged: true,
            paths: true,
        },
        "copy tagged files with paths",
    ),
    (
        "move",
        Command::Move {
            tagged: false,
            paths: false,
        },
        "move file",
    ),
    (
        "move_tagged",
        Command::Move {
            tagged: true,
            paths: false,
        },
        "move tagged files",
    ),
    (
        "move_tagged_paths",
        Command::Move {
            tagged: true,
            paths: true,
        },
        "move tagged files with paths",
    ),
    (
        "delete",
        Command::Delete { tagged: false },
        "delete file (in the tree: empty directory)",
    ),
    (
        "delete_tagged",
        Command::Delete { tagged: true },
        "delete tagged files",
    ),
    (
        "attributes",
        Command::Attributes { tagged: false },
        "change permissions",
    ),
    (
        "attributes_tagged",
        Command::Attributes { tagged: true },
        "change permissions of tagged files",
    ),
    ("rename", Command::Rename, "rename file or directory"),
    ("make_dir", Command::MakeDir, "make a subdirectory"),
    (
        "prune",
        Command::Prune,
        "delete directory with everything in it",
    ),
    ("log", Command::Log, "open another directory in the pane"),
    (
        "compare",
        Command::Compare,
        "compare with the other pane and tag",
    ),
    (
        "open",
        Command::OpenWith { tagged: false },
        "open file with its default program",
    ),
    (
        "open_tagged",
        Command::OpenWith { tagged: true },
        "open tagged files with their default programs",
    ),
    (
        "open_archive",
        Command::OpenArchive,
        "open a zip/tar archive as a directory",
    ),
    (
        "compare_branch",
        Command::CompareBranch,
        "compare the branch with the other pane",
    ),
    ("parent", Command::Parent, "go to the parent directory"),
    ("edit", Command::Edit, "edit file"),
    ("execute", Command::Execute, "execute a shell command"),
    (
        "git",
        Command::Git,
        "Git menu: status, log, diff, blame, add, unstage, commit",
    ),
    (
        "sort",
        Command::Sort,
        "sort files (name, ext, date, size; order)",
    ),
    (
        "settings",
        Command::Settings,
        "configuration: options and theme",
    ),
    (
        "search_tagged",
        Command::SearchTagged,
        "search text in tagged files (untags the others)",
    ),
    (
        "view_tagged",
        Command::ViewTagged,
        "view tagged files (N/P: next/previous, Space: next hit)",
    ),
    (
        "treespec",
        Command::Treespec,
        "go to a directory by typing its path (Tab, / complete)",
    ),
    (
        "kbyte",
        Command::Kbyte,
        "show size or file count of each directory in the tree",
    ),
    (
        "autoview",
        Command::Autoview,
        "file list with the selected file's contents beside it",
    ),
    (
        "new_date",
        Command::NewDate { tagged: false },
        "change the modification date (in the tree: of the directory)",
    ),
    (
        "new_date_tagged",
        Command::NewDate { tagged: true },
        "change the modification date of the tagged files",
    ),
    (
        "batch",
        Command::Batch,
        "write a script or list with one line per tagged file",
    ),
    (
        "archive",
        Command::CreateArchive { tagged: false },
        "create a zip/tar archive with the file",
    ),
    (
        "archive_tagged",
        Command::CreateArchive { tagged: true },
        "create a zip/tar archive with the tagged files",
    ),
    ("info", Command::Info, "details of the file or directory"),
    (
        "avail",
        Command::Avail,
        "mounted file systems and free space",
    ),
    ("stats", Command::Stats, "statistics of the pane"),
    ("help", Command::Help, "this help"),
    (
        "file_display",
        Command::FileDisplay,
        "file window: names in columns, with size, details, long names",
    ),
    (
        "tag_branch",
        Command::TagBranch,
        "tag the logged files of the branch",
    ),
    (
        "untag_branch",
        Command::UntagBranch,
        "untag the logged files of the branch",
    ),
    (
        "invert_branch",
        Command::InvertBranch,
        "invert the tags of the branch",
    ),
    ("left", Command::Left, "previous file (one column: page up)"),
    ("right", Command::Right, "next file (one column: page down)"),
    ("quit", Command::Quit, "quit"),
    (
        "quit_cd",
        Command::QuitChangeDir,
        "quit leaving the shell in the current directory (see README)",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Any,
    Tree,
    Files,
}

use Scope::{Any, Files, Tree};

/// The default keys. Ctrl+letter acts on the tagged files; so does Alt+letter,
/// and in Copy/Move it recreates the paths.
const DEFAULT_KEYS: &[(&str, Scope, &str)] = &[
    ("up", Any, "up"),
    ("down", Any, "down"),
    ("pageup", Any, "page_up"),
    ("pagedown", Any, "page_down"),
    ("home", Any, "home"),
    ("end", Any, "end"),
    ("enter", Any, "switch_window"),
    ("tab", Any, "switch_pane"),
    ("f1", Any, "help"),
    ("f3", Any, "relog"),
    ("f2", Any, "relog"),
    ("space", Any, "down"),
    ("backspace", Any, "parent"),
    ("f8", Any, "split"),
    ("f7", Any, "autoview"),
    ("n", Any, "new_date"),
    ("ctrl+n", Files, "new_date_tagged"),
    ("ctrl+b", Files, "batch"),
    ("ctrl+g", Any, "git"),
    ("alt+i", Any, "info"),
    ("a", Tree, "avail"),
    ("?", Any, "stats"),
    ("f5", Files, "archive"),
    ("ctrl+f5", Files, "archive_tagged"),
    ("f10", Any, "quit"),
    ("f12", Any, "settings"),
    ("alt+f10", Any, "settings"),
    ("b", Any, "branch"),
    ("d", Any, "delete"),
    ("f", Any, "filespec"),
    ("i", Any, "invert"),
    ("q", Any, "quit"),
    ("alt+q", Any, "quit_cd"),
    ("r", Any, "rename"),
    ("s", Any, "showall"),
    ("t", Any, "tag"),
    ("u", Any, "untag"),
    ("x", Any, "execute"),
    ("ctrl+a", Any, "attributes_tagged"),
    ("ctrl+c", Any, "copy_tagged"),
    ("ctrl+d", Any, "delete_tagged"),
    ("ctrl+m", Any, "move_tagged"),
    ("ctrl+t", Any, "tag_all"),
    ("ctrl+u", Any, "untag_all"),
    ("alt+a", Any, "attributes_tagged"),
    ("alt+d", Any, "delete_tagged"),
    ("alt+t", Any, "tag_all"),
    ("alt+u", Any, "untag_all"),
    ("alt+s", Any, "sort"),
    ("right", Tree, "expand"),
    ("plus", Tree, "expand"),
    ("left", Tree, "collapse"),
    ("minus", Tree, "collapse"),
    ("*", Tree, "expand_all"),
    ("esc", Tree, "back"),
    ("c", Tree, "compare"),
    ("l", Tree, "log"),
    ("m", Tree, "make_dir"),
    ("alt+p", Tree, "prune"),
    ("ctrl+f7", Tree, "tag_branch"),
    ("ctrl+f8", Tree, "untag_branch"),
    ("ctrl+f9", Tree, "invert_branch"),
    ("alt+k", Any, "kbyte"),
    ("esc", Files, "back"),
    ("left", Files, "left"),
    ("right", Files, "right"),
    ("alt+f", Any, "file_display"),
    ("alt+c", Files, "copy_tagged_paths"),
    ("alt+m", Files, "move_tagged_paths"),
    ("alt+f5", Files, "open_archive"),
    ("ctrl+o", Files, "open_tagged"),
    ("ctrl+s", Files, "search_tagged"),
    ("\\", Any, "treespec"),
    // Not in ZTreeWin (there "/" is the same as "?"), but "/" is how
    // searching starts in less, vim and most Linux tools.
    ("/", Any, "treespec"),
    ("ctrl+v", Files, "view_tagged"),
    ("alt+c", Tree, "compare_branch"),
    ("a", Files, "attributes"),
    ("c", Files, "copy"),
    ("e", Files, "edit"),
    ("m", Files, "move"),
    ("o", Files, "open"),
    ("v", Files, "view"),
];

/// A key with modifiers, already normalized: lowercase letters, and only
/// Ctrl and Alt count (Shift is already part of the letter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeySpec {
    code: KeyCode,
    mods: KeyModifiers,
}

impl KeySpec {
    pub fn from_event(key: KeyEvent) -> Self {
        let code = match key.code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            other => other,
        };
        KeySpec {
            code,
            mods: key.modifiers & (KeyModifiers::CONTROL | KeyModifiers::ALT),
        }
    }

    /// `"x"`, `"F3"`, `"ctrl+r"`, `"alt+shift+x"` (Shift is ignored),
    /// `"plus"` for `+`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let lower = text.trim().to_lowercase();
        let (mods_part, key) = match lower.as_str() {
            "+" => ("", "+"),
            s if s.ends_with("++") => (&s[..s.len() - 2], "+"),
            s => s.rsplit_once('+').unwrap_or(("", s)),
        };
        let mut mods = KeyModifiers::NONE;
        for m in mods_part.split('+').filter(|m| !m.is_empty()) {
            match m {
                "ctrl" | "control" => mods |= KeyModifiers::CONTROL,
                "alt" | "meta" => mods |= KeyModifiers::ALT,
                "shift" => {}
                other => return Err(format!("unknown modifier '{other}' in '{text}'")),
            }
        }
        let code = match key {
            "enter" | "return" => KeyCode::Enter,
            "esc" | "escape" => KeyCode::Esc,
            "tab" => KeyCode::Tab,
            "backtab" => KeyCode::BackTab,
            "backspace" => KeyCode::Backspace,
            "delete" | "del" => KeyCode::Delete,
            "insert" | "ins" => KeyCode::Insert,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "pgup" => KeyCode::PageUp,
            "pagedown" | "pgdn" => KeyCode::PageDown,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "space" => KeyCode::Char(' '),
            "plus" => KeyCode::Char('+'),
            "minus" => KeyCode::Char('-'),
            k if k.len() > 1 && k.starts_with('f') => match k[1..].parse() {
                Ok(n @ 1..=24) => KeyCode::F(n),
                _ => return Err(format!("unknown key '{text}'")),
            },
            k => {
                let mut chars = k.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => KeyCode::Char(c),
                    _ => return Err(format!("unknown key '{text}'")),
                }
            }
        };
        Ok(KeySpec { code, mods })
    }
}

impl fmt::Display for KeySpec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.mods.contains(KeyModifiers::CONTROL) {
            f.write_str("Ctrl+")?;
        }
        if self.mods.contains(KeyModifiers::ALT) {
            f.write_str("Alt+")?;
        }
        match self.code {
            KeyCode::Char(' ') => f.write_str("Space"),
            KeyCode::Char(c) => write!(f, "{}", c.to_ascii_uppercase()),
            KeyCode::F(n) => write!(f, "F{n}"),
            KeyCode::Enter => f.write_str("Enter"),
            KeyCode::Esc => f.write_str("Esc"),
            KeyCode::Tab => f.write_str("Tab"),
            KeyCode::PageUp => f.write_str("PgUp"),
            KeyCode::PageDown => f.write_str("PgDn"),
            KeyCode::Up => f.write_str("↑"),
            KeyCode::Down => f.write_str("↓"),
            KeyCode::Left => f.write_str("←"),
            KeyCode::Right => f.write_str("→"),
            other => write!(f, "{other:?}"),
        }
    }
}

fn command_by_name(name: &str) -> Result<Option<Command>, String> {
    if name == "none" {
        return Ok(None);
    }
    COMMANDS
        .iter()
        .find(|(n, ..)| *n == name)
        .map(|(_, c, _)| Some(*c))
        .ok_or_else(|| format!("unknown command '{name}'"))
}

/// Key → command, per window. `None` unbinds the key.
#[derive(Debug, Clone)]
pub struct Keymap {
    maps: HashMap<Scope, HashMap<KeySpec, Option<Command>>>,
    /// Git commands are on (`git` in the configuration).
    git: bool,
}

impl Default for Keymap {
    fn default() -> Self {
        let mut keymap = Keymap {
            maps: HashMap::new(),
            git: false,
        };
        for (key, scope, name) in DEFAULT_KEYS {
            let spec = KeySpec::parse(key).expect("valid default key");
            let cmd = command_by_name(name).expect("valid default command");
            keymap.maps.entry(*scope).or_default().insert(spec, cmd);
        }
        keymap.drop_git();
        keymap
    }
}

impl Keymap {
    /// The defaults with the config file's overrides applied.
    pub fn from_config(keys: &Keys, options: &Options) -> Result<Self, String> {
        let mut keymap = Keymap::default();
        if options.git {
            keymap.git = true;
            for (key, scope, name) in DEFAULT_KEYS.iter().filter(|(.., n)| *n == "git") {
                let spec = KeySpec::parse(key).expect("valid default key");
                let cmd = command_by_name(name).expect("valid default command");
                keymap.maps.entry(*scope).or_default().insert(spec, cmd);
            }
        }
        if options.enter_views {
            let enter = KeySpec::parse("enter").expect("valid key");
            keymap
                .maps
                .entry(Files)
                .or_default()
                .insert(enter, Some(Command::View));
        }
        for (scope, table) in [(Any, &keys.any), (Tree, &keys.tree), (Files, &keys.files)] {
            let mut entries: Vec<_> = table.iter().collect();
            entries.sort();
            for (key, name) in entries {
                let spec = KeySpec::parse(key)?;
                let cmd = command_by_name(name)?;
                if scope == Any {
                    // An "any window" override also applies where the
                    // default was specific to one window.
                    for s in [Tree, Files] {
                        keymap.maps.entry(s).or_default().remove(&spec);
                    }
                }
                keymap.maps.entry(scope).or_default().insert(spec, cmd);
            }
        }
        if !keymap.git {
            keymap.drop_git();
        }
        Ok(keymap)
    }

    /// Git is off: no key opens its menu (and the help does not list it).
    fn drop_git(&mut self) {
        for map in self.maps.values_mut() {
            map.retain(|_, cmd| *cmd != Some(Command::Git));
        }
    }

    pub fn lookup(&self, key: KeyEvent, focus: Focus) -> Option<Command> {
        let spec = KeySpec::from_event(key);
        let scope = match focus {
            Focus::Tree => Tree,
            Focus::Files => Files,
        };
        for s in [scope, Any] {
            if let Some(cmd) = self.maps.get(&s).and_then(|m| m.get(&spec)) {
                return *cmd;
            }
        }
        None
    }

    /// Help text (F1): the keys currently in effect, per window.
    pub fn help_text(&self) -> String {
        let mut out = String::from(
            "xt - keys in use (F1 shows this; Esc closes)\n\
             Change them in ~/.config/xt/config.toml, see xt --default-config.\n",
        );
        for (scope, title) in [
            (Tree, "TREE WINDOW"),
            (Files, "FILE WINDOW"),
            (Any, "BOTH WINDOWS"),
        ] {
            out.push_str(&format!("\n{title}\n"));
            let map = self.maps.get(&scope);
            for (_, cmd, desc) in COMMANDS {
                let mut keys: Vec<String> = map
                    .into_iter()
                    .flatten()
                    .filter(|(_, c)| **c == Some(*cmd))
                    .map(|(k, _)| k.to_string())
                    .collect();
                if keys.is_empty() {
                    continue;
                }
                keys.sort_by_key(|k| (k.len(), k.clone()));
                out.push_str(&format!("  {:<24}{desc}\n", keys.join(", ")));
            }
        }
        out.push_str(
            "\nFIXED KEYS\n  \
             F9                      Ctrl/Alt menus; the next letter gets the modifier\n  \
             F6                      in Copy/Move of tagged files: with/without paths\n  \
             F7                      in Alt+C/Alt+M: paths from the disk root / tree root / current / below it\n  \
             F2                      in prompts that ask for a folder: pick it in a tree\n  \
             ↑, ↓                    in prompts: history\n  \
             F2, F4                  in search prompts: match case, text/hex\n\n\
             VIEWER (V, Ctrl+V)\n  \
             F / B                   find from the start / backward from the end\n  \
             /, S / \\                find from this page forward / backward\n  \
             Space, + / -            next hit (in Ctrl+V, continues in the next files)\n  \
             N / P, Alt+Home/End     next / previous / first / last tagged file\n  \
             U                       untag the file being viewed\n  \
             H                       hex / text\n  \
             E                       edit the file in the configured editor\n\n\
             NAVIGATION\n  \
             Shift+letter, 0-9       next directory / file starting with it\n",
        );
        if self.git {
            out.push_str(
                "\nGIT MENU (Ctrl+G; on the current file, the tagged ones, or the directory in the tree)\n  \
                 S / L / D / B           status / log / diff against HEAD / blame, in the viewer\n  \
                 A / U                   add / unstage\n  \
                 C                       commit (git's own editor)\n  \
                 D (in the viewer)       diff of a changed file since the last commit / back\n  \
                 File list               two-letter status before the name, as in git status --short\n",
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(code: KeyCode, mods: KeyModifiers, focus: Focus) -> Option<Command> {
        Keymap::default().lookup(KeyEvent::new(code, mods), focus)
    }

    #[test]
    fn maps_letters_by_window() {
        let none = KeyModifiers::NONE;
        assert_eq!(
            map(KeyCode::Char('m'), none, Focus::Tree),
            Some(Command::MakeDir)
        );
        assert_eq!(
            map(KeyCode::Char('M'), KeyModifiers::SHIFT, Focus::Files),
            Some(Command::Move {
                tagged: false,
                paths: false
            })
        );
        assert_eq!(
            map(KeyCode::Char('c'), none, Focus::Tree),
            Some(Command::Compare)
        );
        assert_eq!(map(KeyCode::Char('z'), none, Focus::Tree), None);
        assert_eq!(map(KeyCode::Up, KeyModifiers::CONTROL, Focus::Tree), None);
    }

    #[test]
    fn ctrl_and_alt_mean_tagged() {
        for (mods, paths) in [(KeyModifiers::CONTROL, false), (KeyModifiers::ALT, true)] {
            assert_eq!(
                map(KeyCode::Char('c'), mods, Focus::Files),
                Some(Command::Copy {
                    tagged: true,
                    paths
                })
            );
            assert_eq!(
                map(KeyCode::Char('m'), mods, Focus::Files),
                Some(Command::Move {
                    tagged: true,
                    paths
                })
            );
            assert_eq!(
                map(KeyCode::Char('t'), mods, Focus::Tree),
                Some(Command::TagAll)
            );
        }
    }

    #[test]
    fn keys_follow_ztreewin() {
        let none = KeyModifiers::NONE;
        assert_eq!(map(KeyCode::F(3), none, Focus::Tree), Some(Command::Relog));
        assert_eq!(
            map(KeyCode::Char('c'), KeyModifiers::ALT, Focus::Tree),
            Some(Command::CompareBranch)
        );
        assert_eq!(
            map(KeyCode::Char('m'), KeyModifiers::ALT, Focus::Tree),
            None,
            "Mirror"
        );
        assert_eq!(
            map(KeyCode::F(5), KeyModifiers::ALT, Focus::Files),
            Some(Command::OpenArchive)
        );
        assert_eq!(
            map(
                KeyCode::F(5),
                KeyModifiers::ALT | KeyModifiers::SHIFT,
                Focus::Files
            ),
            Some(Command::OpenArchive)
        );
        assert_eq!(
            map(KeyCode::Char('o'), none, Focus::Files),
            Some(Command::OpenWith { tagged: false })
        );
        assert_eq!(
            map(KeyCode::Char('o'), KeyModifiers::CONTROL, Focus::Files),
            Some(Command::OpenWith { tagged: true })
        );
        assert_eq!(map(KeyCode::Left, none, Focus::Files), Some(Command::Left));
        assert_eq!(
            map(KeyCode::Right, none, Focus::Files),
            Some(Command::Right)
        );
        assert_eq!(
            map(KeyCode::Backspace, none, Focus::Tree),
            Some(Command::Parent)
        );
        assert_eq!(
            map(KeyCode::Char(' '), none, Focus::Files),
            Some(Command::Down)
        );
    }

    #[test]
    fn parses_and_prints_keys() {
        let k = |s| KeySpec::parse(s).unwrap();
        assert_eq!(k("ctrl+r").to_string(), "Ctrl+R");
        assert_eq!(k("Alt+Shift+X").to_string(), "Alt+X");
        assert_eq!(k("F12").to_string(), "F12");
        assert_eq!(k("+"), k("plus"));
        assert_eq!(k("ctrl++").to_string(), "Ctrl++");
        assert_eq!(k("pgdn"), k("pagedown"));
        assert!(KeySpec::parse("hyper+x").is_err());
        assert!(KeySpec::parse("f99").is_err());
        assert!(KeySpec::parse("xy").is_err());
    }

    #[test]
    fn config_overrides_defaults() {
        let keys = Keys {
            any: [("f3", "view"), ("c", "edit")]
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .into(),
            tree: HashMap::new(),
            files: [("q", "none"), ("enter", "view")]
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .into(),
        };
        let km = Keymap::from_config(&keys, &Options::default()).unwrap();
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        assert_eq!(
            km.lookup(key(KeyCode::F(3)), Focus::Files),
            Some(Command::View)
        );
        // "c" in any window beats the defaults of both windows.
        assert_eq!(
            km.lookup(key(KeyCode::Char('c')), Focus::Tree),
            Some(Command::Edit)
        );
        assert_eq!(
            km.lookup(key(KeyCode::Char('c')), Focus::Files),
            Some(Command::Edit)
        );
        // "none" unbinds; the rest stays default.
        assert_eq!(km.lookup(key(KeyCode::Char('q')), Focus::Files), None);
        assert_eq!(
            km.lookup(key(KeyCode::Char('q')), Focus::Tree),
            Some(Command::Quit)
        );
        assert_eq!(
            km.lookup(key(KeyCode::Enter), Focus::Files),
            Some(Command::View)
        );
        assert_eq!(
            km.lookup(key(KeyCode::Enter), Focus::Tree),
            Some(Command::SwitchWindow)
        );
    }

    #[test]
    fn enter_views_option() {
        let options = Options {
            enter_views: true,
            ..Options::default()
        };
        let km = Keymap::from_config(&Keys::default(), &options).unwrap();
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(km.lookup(enter, Focus::Files), Some(Command::View));
        assert_eq!(km.lookup(enter, Focus::Tree), Some(Command::SwitchWindow));
    }

    #[test]
    fn rejects_bad_config() {
        let bad = |k: &str, v: &str| Keys {
            any: HashMap::from([(k.to_string(), v.to_string())]),
            ..Keys::default()
        };
        assert!(
            Keymap::from_config(&bad("f3", "explode"), &Options::default())
                .unwrap_err()
                .contains("unknown command")
        );
        assert!(Keymap::from_config(&bad("hyper+x", "view"), &Options::default()).is_err());
    }

    #[test]
    fn help_lists_keys_in_use() {
        let keys = Keys {
            files: HashMap::from([("f3".to_string(), "view".to_string())]),
            ..Keys::default()
        };
        let help = Keymap::from_config(&keys, &Options::default())
            .unwrap()
            .help_text();
        assert!(help.contains("FILE WINDOW"));
        let view_line = help.lines().find(|l| l.contains("view file")).unwrap();
        assert!(
            view_line.contains("V") && view_line.contains("F3"),
            "{view_line}"
        );
        assert!(
            help.lines()
                .any(|l| l.starts_with("  Alt+C ") && l.contains("with paths"))
        );
    }

    #[test]
    fn git_key_only_when_enabled() {
        let ctrl_g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
        let off = Keymap::default();
        assert_eq!(off.lookup(ctrl_g, Focus::Tree), None);
        assert!(!off.help_text().contains("GIT MENU"));

        let options = Options {
            git: true,
            ..Options::default()
        };
        let on = Keymap::from_config(&Keys::default(), &options).unwrap();
        assert_eq!(on.lookup(ctrl_g, Focus::Files), Some(Command::Git));
        assert!(on.help_text().contains("GIT MENU"));

        // Bound to another key in the file, but off: still nothing.
        let mut keys = Keys::default();
        keys.any.insert("alt+g".into(), "git".into());
        let km = Keymap::from_config(&keys, &Options::default()).unwrap();
        let alt_g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::ALT);
        assert_eq!(km.lookup(alt_g, Focus::Tree), None);
    }

    #[test]
    fn every_command_has_a_default_key() {
        let options = Options {
            git: true,
            ..Options::default()
        };
        let km = Keymap::from_config(&Keys::default(), &options).unwrap();
        for (name, cmd, _) in COMMANDS {
            let bound = km
                .maps
                .values()
                .any(|m| m.values().any(|c| *c == Some(*cmd)));
            assert!(bound, "{name} has no default key");
        }
    }
}
