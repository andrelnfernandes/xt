//! The Usage view (Alt+W): opening it and its keys.

use super::*;
use crate::usage::Item;

impl App {
    /// Logs the branch of the current directory, without going into other
    /// disks mounted inside it, and opens the view on it.
    pub(crate) fn open_usage(&mut self) {
        let id = self.current();
        let device = self.tree.device(id);
        self.start_job(LogJob::one_disk(id, device), AfterLog::Usage(id));
    }

    pub(crate) fn usage_key(&mut self, key: KeyEvent) {
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        if key.code == KeyCode::Esc || alt && matches!(key.code, KeyCode::Char('w' | 'W')) {
            self.usage = None;
            return;
        }
        let pane = &mut self.panes[self.active];
        let Some(view) = pane.usage.as_mut() else {
            return;
        };
        let tree = &pane.tree;
        let len = view.rows(tree).len();
        let page = view.page.max(1) as isize;
        match key.code {
            KeyCode::Up => view.move_by(-1, len),
            KeyCode::Down => view.move_by(1, len),
            KeyCode::PageUp => view.move_by(-page, len),
            KeyCode::PageDown => view.move_by(page, len),
            KeyCode::Home => view.move_by(-(len as isize), len),
            KeyCode::End => view.move_by(len as isize, len),
            KeyCode::Enter | KeyCode::Right => {
                let Some(row) = view.selected(tree) else {
                    return;
                };
                let Item::Dir(id) = row.item else { return };
                if row.mark == '>' {
                    // Another disk: an analysis of its own.
                    self.usage = None;
                    let device = self.tree.device(id);
                    self.start_job(LogJob::one_disk(id, device), AfterLog::Usage(id));
                } else {
                    view.enter(id);
                }
            }
            KeyCode::Backspace | KeyCode::Left => {
                if !view.back(tree) {
                    self.message = Some("This is where the analysis started".into());
                }
            }
            KeyCode::Char('\\') => {
                let dir = view.dir;
                self.usage = None;
                self.tree.reveal(dir);
                self.refresh_rows(dir);
                self.select_node(dir);
                self.focus = Focus::Tree;
            }
            KeyCode::Char('a' | 'A') => view.apparent = !view.apparent,
            KeyCode::Char('c' | 'C') => view.counts = !view.counts,
            KeyCode::Char('s' | 'S') => {
                view.order = view.order.next();
                view.list.select(Some(0));
                self.message = Some(format!("Sorted by {}", view.order.label()));
            }
            KeyCode::F(3) | KeyCode::F(2) => {
                let dir = view.dir;
                self.tree.rescan(dir);
                let device = self.tree.device(dir);
                self.start_job(LogJob::one_disk(dir, device), AfterLog::Refresh);
            }
            _ => {
                // What makes sense with the view open: windows, the
                // terminal, settings and quitting.
                if let Some(cmd) = self.keymap.lookup(key, self.focus)
                    && matches!(
                        cmd,
                        Command::Split
                            | Command::SwitchWindow
                            | Command::SwitchPane
                            | Command::Terminal
                            | Command::TerminalFull
                            | Command::Settings
                            | Command::Quit
                            | Command::QuitChangeDir
                    )
                {
                    self.apply(cmd);
                }
            }
        }
    }
}
