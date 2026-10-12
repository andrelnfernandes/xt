//! Mapa de teclas: os padrões, as trocas do arquivo de configuração e o
//! texto da ajuda (F1). Tudo sai das tabelas abaixo.

use std::collections::HashMap;
use std::fmt;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::Keys;
use crate::input::Command;
use crate::pane::Focus;

/// Todos os comandos que podem ir para uma tecla: nome no arquivo de
/// configuração, comando e descrição na ajuda.
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
    ("log", Command::Log, "open another directory in the pane"),
    (
        "compare",
        Command::Compare,
        "compare with the other pane and tag",
    ),
    ("open", Command::Open, "open a zip/tar archive"),
    ("edit", Command::Edit, "edit file"),
    ("execute", Command::Execute, "execute a shell command"),
    (
        "sort",
        Command::Sort,
        "sort files (name, ext, date, size; order)",
    ),
    ("help", Command::Help, "this help"),
    ("quit", Command::Quit, "quit"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Any,
    Tree,
    Files,
}

use Scope::{Any, Files, Tree};

/// As teclas padrão. Ctrl+letra age nos arquivos marcados; Alt+letra
/// também, e no Copy/Move recria os caminhos.
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
    ("f2", Any, "relog"),
    ("f8", Any, "split"),
    ("f10", Any, "quit"),
    ("b", Any, "branch"),
    ("d", Any, "delete"),
    ("f", Any, "filespec"),
    ("i", Any, "invert"),
    ("q", Any, "quit"),
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
    ("alt+c", Any, "copy_tagged_paths"),
    ("alt+d", Any, "delete_tagged"),
    ("alt+m", Any, "move_tagged_paths"),
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
    ("esc", Files, "back"),
    ("left", Files, "back"),
    ("a", Files, "attributes"),
    ("c", Files, "copy"),
    ("e", Files, "edit"),
    ("m", Files, "move"),
    ("o", Files, "open"),
    ("v", Files, "view"),
];

/// Uma tecla com modificadores, já normalizada: letras em minúsculas e só
/// Ctrl e Alt contam (Shift já está na própria letra).
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

    /// `"x"`, `"F3"`, `"ctrl+r"`, `"alt+shift+x"` (Shift é ignorado),
    /// `"plus"` para `+`.
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

/// Tecla → comando, por janela. `None` desliga a tecla.
#[derive(Debug, Clone)]
pub struct Keymap {
    maps: HashMap<Scope, HashMap<KeySpec, Option<Command>>>,
}

impl Default for Keymap {
    fn default() -> Self {
        let mut keymap = Keymap {
            maps: HashMap::new(),
        };
        for (key, scope, name) in DEFAULT_KEYS {
            let spec = KeySpec::parse(key).expect("valid default key");
            let cmd = command_by_name(name).expect("valid default command");
            keymap.maps.entry(*scope).or_default().insert(spec, cmd);
        }
        keymap
    }
}

impl Keymap {
    /// Os padrões com as trocas do arquivo de configuração.
    pub fn from_config(keys: &Keys) -> Result<Self, String> {
        let mut keymap = Keymap::default();
        for (scope, table) in [(Any, &keys.any), (Tree, &keys.tree), (Files, &keys.files)] {
            let mut entries: Vec<_> = table.iter().collect();
            entries.sort();
            for (key, name) in entries {
                let spec = KeySpec::parse(key)?;
                let cmd = command_by_name(name)?;
                if scope == Any {
                    // Uma troca "em qualquer janela" vale também onde o
                    // padrão era específico de uma janela.
                    for s in [Tree, Files] {
                        keymap.maps.entry(s).or_default().remove(&spec);
                    }
                }
                keymap.maps.entry(scope).or_default().insert(spec, cmd);
            }
        }
        Ok(keymap)
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

    /// Texto da ajuda (F1): as teclas em vigor, por janela.
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
             ↑, ↓                    in prompts: history\n",
        );
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
                map(KeyCode::Char('m'), mods, Focus::Tree),
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
        let km = Keymap::from_config(&keys).unwrap();
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        assert_eq!(
            km.lookup(key(KeyCode::F(3)), Focus::Files),
            Some(Command::View)
        );
        // "c" em qualquer janela vence os padrões das duas janelas.
        assert_eq!(
            km.lookup(key(KeyCode::Char('c')), Focus::Tree),
            Some(Command::Edit)
        );
        assert_eq!(
            km.lookup(key(KeyCode::Char('c')), Focus::Files),
            Some(Command::Edit)
        );
        // "none" desliga; o resto continua padrão.
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
    fn rejects_bad_config() {
        let bad = |k: &str, v: &str| Keys {
            any: HashMap::from([(k.to_string(), v.to_string())]),
            ..Keys::default()
        };
        assert!(
            Keymap::from_config(&bad("f3", "explode"))
                .unwrap_err()
                .contains("unknown command")
        );
        assert!(Keymap::from_config(&bad("hyper+x", "view")).is_err());
    }

    #[test]
    fn help_lists_keys_in_use() {
        let keys = Keys {
            files: HashMap::from([("f3".to_string(), "view".to_string())]),
            ..Keys::default()
        };
        let help = Keymap::from_config(&keys).unwrap().help_text();
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
    fn every_command_has_a_default_key() {
        let km = Keymap::default();
        for (name, cmd, _) in COMMANDS {
            let bound = km
                .maps
                .values()
                .any(|m| m.values().any(|c| *c == Some(*cmd)));
            assert!(bound, "{name} has no default key");
        }
    }
}
