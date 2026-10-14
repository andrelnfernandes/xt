//! F2 in prompts that ask for a folder (Copy, Move, Log, and the folder of
//! Ctrl+B and F5): picks it in a tree, as in ZTreeWin.
//!
//! Each window's tree is opened again, with its own cursor: browsing here
//! never moves the windows behind it.

use std::path::{Path, PathBuf};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::Command;
use crate::keymap::Keymap;
use crate::pane::{Focus, Pane};

pub struct Picker {
    /// One tree per window (Tab switches, with the screen split).
    pub panes: Vec<Pane>,
    pub active: usize,
}

/// What a key did in the picker.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Cancel,
    Chosen(PathBuf),
}

impl Picker {
    /// `windows`: for each window, its root and current directory. Starts in
    /// the active one, at `start` when it is inside its tree.
    pub fn new(windows: &[(PathBuf, PathBuf, bool)], active: usize, start: &Path) -> Self {
        let panes = windows
            .iter()
            .enumerate()
            .map(|(i, (root, current, show_hidden))| {
                let mut pane = Pane::new(root.clone());
                pane.tree.show_hidden = *show_hidden;
                if !(i == active && pane.goto(start)) {
                    pane.goto(current);
                }
                pane
            })
            .collect();
        Picker { panes, active }
    }

    pub fn pane(&self) -> &Pane {
        &self.panes[self.active]
    }

    pub fn pane_mut(&mut self) -> &mut Pane {
        &mut self.panes[self.active]
    }

    pub fn handle_key(&mut self, key: KeyEvent, keymap: &Keymap) -> Outcome {
        match key.code {
            KeyCode::Esc | KeyCode::F(2) => return Outcome::Cancel,
            KeyCode::Enter => return Outcome::Chosen(self.pane().current_path()),
            KeyCode::Tab | KeyCode::BackTab if self.panes.len() > 1 => {
                self.active = (self.active + 1) % self.panes.len();
                return Outcome::Stay;
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && (c.is_uppercase() || c.is_ascii_digit()) =>
            {
                self.pane_mut().spell_jump(c);
                return Outcome::Stay;
            }
            _ => {}
        }
        // The tree's own movement keys, as configured.
        if let Some(
            cmd @ (Command::Up
            | Command::Down
            | Command::PageUp
            | Command::PageDown
            | Command::Home
            | Command::End
            | Command::Expand
            | Command::Collapse),
        ) = keymap.lookup(key, Focus::Tree)
        {
            self.pane_mut().apply_tree(cmd);
        }
        Outcome::Stay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn browses_and_chooses() {
        let tmp = crate::testing::tempdir();
        let root = tmp.path().to_path_buf();
        fs::create_dir_all(root.join("a/deep")).unwrap();
        fs::create_dir_all(root.join("b")).unwrap();
        let km = Keymap::default();

        // Starts at `start` (inside the tree), else at the current directory.
        let mut p = Picker::new(
            &[(root.clone(), root.clone(), true)],
            0,
            &root.join("a/deep"),
        );
        assert_eq!(p.pane().current_path(), root.join("a/deep"));
        p.handle_key(key(KeyCode::Char('B')), &km);
        assert_eq!(p.pane().current_path(), root.join("b"));
        assert_eq!(
            p.handle_key(key(KeyCode::Enter), &km),
            Outcome::Chosen(root.join("b"))
        );

        let mut p = Picker::new(
            &[(root.clone(), root.join("a"), true)],
            0,
            Path::new("/nowhere"),
        );
        assert_eq!(p.pane().current_path(), root.join("a"));
        p.handle_key(key(KeyCode::Up), &km);
        assert_eq!(p.pane().current_path(), root);
        assert_eq!(p.handle_key(key(KeyCode::Esc), &km), Outcome::Cancel);
    }

    #[test]
    fn tab_switches_between_windows() {
        let (one, two) = (crate::testing::tempdir(), crate::testing::tempdir());
        let (one, two) = (one.path().to_path_buf(), two.path().to_path_buf());
        let windows = [
            (one.clone(), one.clone(), true),
            (two.clone(), two.clone(), true),
        ];
        let mut p = Picker::new(&windows, 0, &one);
        let km = Keymap::default();
        p.handle_key(key(KeyCode::Tab), &km);
        assert_eq!(
            p.handle_key(key(KeyCode::Enter), &km),
            Outcome::Chosen(two.clone())
        );
        p.handle_key(key(KeyCode::Tab), &km);
        assert_eq!(p.pane().current_path(), one);
    }
}
