//! Estado da aplicação e execução dos comandos.

use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

use crate::input::{self, Command};
use crate::model::tree::{NodeId, Tree, VisibleRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Files,
}

pub struct App {
    pub tree: Tree,
    /// Cache das linhas visíveis; recalculado quando a árvore muda.
    pub rows: Vec<VisibleRow>,
    pub tree_state: ListState,
    pub file_state: ListState,
    pub focus: Focus,
    /// Mostrando a pergunta "Quit xt? (Y/N)".
    pub confirm_quit: bool,
    pub should_quit: bool,
    /// Altura útil das listas no último desenho (para PgUp/PgDn).
    pub tree_page: usize,
    pub file_page: usize,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        let tree = Tree::new(root);
        let rows = tree.visible_rows();
        App {
            tree,
            rows,
            tree_state: ListState::default().with_selected(Some(0)),
            file_state: ListState::default().with_selected(Some(0)),
            focus: Focus::Tree,
            confirm_quit: false,
            should_quit: false,
            tree_page: 1,
            file_page: 1,
        }
    }

    /// Diretório sob o cursor da árvore.
    pub fn current(&self) -> NodeId {
        self.rows[self.tree_state.selected().unwrap_or(0)].id
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.confirm_quit {
            match key.code {
                KeyCode::Char('y' | 'Y') | KeyCode::Enter => self.should_quit = true,
                _ => self.confirm_quit = false,
            }
            return;
        }
        if let Some(cmd) = input::map_key(key, self.focus) {
            self.apply(cmd);
        }
    }

    pub fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Quit => self.confirm_quit = true,
            Command::SwitchWindow => {
                self.focus = match self.focus {
                    Focus::Tree => Focus::Files,
                    Focus::Files => Focus::Tree,
                }
            }
            Command::Back => self.focus = Focus::Tree,
            _ => match self.focus {
                Focus::Tree => self.apply_tree(cmd),
                Focus::Files => self.apply_files(cmd),
            },
        }
    }

    fn apply_tree(&mut self, cmd: Command) {
        let cur = self.current();
        let pos = self.tree_state.selected().unwrap_or(0);
        let target = match cmd {
            Command::Expand => {
                self.tree.expand(cur);
                self.refresh_rows(cur);
                return;
            }
            Command::ExpandAll => {
                self.tree.expand_all(cur);
                self.refresh_rows(cur);
                return;
            }
            Command::Collapse => {
                let node = self.tree.node(cur);
                if node.expanded && self.tree.has_children(cur) {
                    self.tree.collapse(cur);
                    self.refresh_rows(cur);
                } else if let Some(parent) = node.parent {
                    self.select_node(parent);
                }
                return;
            }
            _ => move_cursor(cmd, pos, self.rows.len(), self.tree_page),
        };
        self.select_row(target);
    }

    fn apply_files(&mut self, cmd: Command) {
        let len = self.tree.node(self.current()).files.len();
        let pos = self.file_state.selected().unwrap_or(0);
        self.file_state
            .select(Some(move_cursor(cmd, pos, len, self.file_page)));
    }

    fn select_row(&mut self, index: usize) {
        if Some(index) != self.tree_state.selected() {
            self.tree_state.select(Some(index));
            self.file_state = ListState::default().with_selected(Some(0));
            let id = self.current();
            self.tree.scan(id);
        }
    }

    fn select_node(&mut self, id: NodeId) {
        if let Some(i) = self.rows.iter().position(|r| r.id == id) {
            self.select_row(i);
        }
    }

    /// Recalcula as linhas visíveis mantendo o cursor em `keep`.
    fn refresh_rows(&mut self, keep: NodeId) {
        self.rows = self.tree.visible_rows();
        let i = self.rows.iter().position(|r| r.id == keep).unwrap_or(0);
        self.tree_state.select(Some(i));
    }
}

/// Nova posição de um cursor numa lista de `len` itens.
fn move_cursor(cmd: Command, pos: usize, len: usize, page: usize) -> usize {
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
    fn left_on_leaf_goes_to_parent() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("a")).unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.apply(Command::Down);
        assert_eq!(app.tree_state.selected(), Some(1));
        app.apply(Command::Collapse);
        assert_eq!(app.tree_state.selected(), Some(0));
    }
}
