//! Estado da aplicação e execução dos comandos.

use std::path::PathBuf;
use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

use crate::input::{self, Command, LineEvent, LineInput};
use crate::model::filespec::Filespec;
use crate::model::tree::{FileEntry, LogJob, NodeId, ROOT, Tree, VisibleRow};
use crate::viewer::Viewer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Files,
}

/// O que a janela de arquivos está listando.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileView {
    /// Arquivos do diretório sob o cursor da árvore.
    Dir,
    /// Arquivos do ramo a partir deste diretório (tecla `B`).
    Branch(NodeId),
    /// Arquivos de tudo o que foi logado (tecla `S`).
    Showall,
}

/// Referência a um arquivo dentro da árvore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRef {
    pub dir: NodeId,
    pub idx: usize,
}

/// Leitura em andamento e o que fazer quando ela terminar.
pub struct Job {
    pub log: LogJob,
    then: AfterLog,
}

enum AfterLog {
    Refresh,
    OpenBranch(NodeId),
}

pub struct App {
    pub tree: Tree,
    /// Cache das linhas visíveis; recalculado quando a árvore muda.
    pub rows: Vec<VisibleRow>,
    pub tree_state: ListState,
    pub focus: Focus,
    pub view: FileView,
    /// Arquivos listados na janela de arquivos, já filtrados.
    pub files: Vec<FileRef>,
    pub file_state: ListState,
    pub filespec: Filespec,
    /// Prompt do Filespec aberto.
    pub prompt: Option<LineInput>,
    /// Mensagem (de erro) mostrada até a próxima tecla.
    pub message: Option<String>,
    pub job: Option<Job>,
    pub viewer: Option<Viewer>,
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
        let mut app = App {
            tree,
            rows,
            tree_state: ListState::default().with_selected(Some(0)),
            focus: Focus::Tree,
            view: FileView::Dir,
            files: Vec::new(),
            file_state: ListState::default().with_selected(Some(0)),
            filespec: Filespec::default(),
            prompt: None,
            message: None,
            job: None,
            viewer: None,
            confirm_quit: false,
            should_quit: false,
            tree_page: 1,
            file_page: 1,
        };
        app.refresh_files();
        app
    }

    /// Diretório sob o cursor da árvore.
    pub fn current(&self) -> NodeId {
        self.rows[self.tree_state.selected().unwrap_or(0)].id
    }

    pub fn selected_file(&self) -> Option<FileRef> {
        self.files.get(self.file_state.selected()?).copied()
    }

    pub fn file(&self, f: FileRef) -> &FileEntry {
        &self.tree.node(f.dir).files[f.idx]
    }

    /// Diretório mostrado no cabeçalho: no Showall/Branch, é o do arquivo
    /// sob o cursor.
    pub fn header_dir(&self) -> NodeId {
        match (self.view, self.selected_file()) {
            (FileView::Dir, _) | (_, None) => self.current(),
            (_, Some(f)) => f.dir,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.message = None;

        if self.confirm_quit {
            match key.code {
                KeyCode::Char('y' | 'Y') | KeyCode::Enter => self.should_quit = true,
                _ => self.confirm_quit = false,
            }
            return;
        }
        if let Some(viewer) = &mut self.viewer {
            if !viewer.handle_key(key) {
                self.viewer = None;
            }
            return;
        }
        if self.job.is_some() {
            if key.code == KeyCode::Esc {
                self.job = None;
                self.refresh_rows(self.current());
                self.message = Some("Logging cancelled".into());
            }
            return;
        }
        if let Some(prompt) = &mut self.prompt {
            match prompt.handle_key(key) {
                LineEvent::Submit => {
                    match Filespec::parse(&prompt.text) {
                        Ok(spec) => {
                            self.filespec = spec;
                            self.refresh_files();
                        }
                        Err(e) => self.message = Some(e),
                    }
                    self.prompt = None;
                }
                LineEvent::Cancel => self.prompt = None,
                LineEvent::Editing => {}
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
            Command::SwitchWindow => match self.focus {
                Focus::Tree => self.focus = Focus::Files,
                Focus::Files => self.back_to_tree(),
            },
            Command::Back => self.back_to_tree(),
            Command::Filespec => self.prompt = Some(LineInput::new(self.filespec.text())),
            Command::Showall => self.set_view(FileView::Showall),
            Command::Branch => {
                let id = match self.view {
                    FileView::Dir => self.current(),
                    FileView::Branch(id) => id,
                    FileView::Showall => ROOT,
                };
                self.start_job(LogJob::new(id, false), AfterLog::OpenBranch(id));
            }
            Command::ExpandAll => {
                self.start_job(LogJob::new(self.current(), true), AfterLog::Refresh);
            }
            Command::View => self.open_viewer(),
            _ => match self.focus {
                Focus::Tree => self.apply_tree(cmd),
                Focus::Files => self.apply_files(cmd),
            },
        }
    }

    /// Avança a leitura em andamento por no máximo `budget`.
    pub fn tick(&mut self, budget: Duration) {
        let Some(job) = &mut self.job else { return };
        self.tree.run_job(&mut job.log, budget);
        if !job.log.is_done() {
            return;
        }
        let then = self.job.take().map(|j| j.then);
        self.refresh_rows(self.current());
        match then {
            Some(AfterLog::OpenBranch(id)) => self.set_view(FileView::Branch(id)),
            _ => self.refresh_files(),
        }
    }

    fn start_job(&mut self, log: LogJob, then: AfterLog) {
        self.job = Some(Job { log, then });
        // Ramos pequenos terminam na hora, sem piscar a mensagem de progresso.
        self.tick(Duration::from_millis(30));
    }

    fn apply_tree(&mut self, cmd: Command) {
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

    fn apply_files(&mut self, cmd: Command) {
        let pos = self.file_state.selected().unwrap_or(0);
        let target = move_cursor(cmd, pos, self.files.len(), self.file_page);
        self.file_state.select(Some(target));
    }

    fn set_view(&mut self, view: FileView) {
        self.view = view;
        self.focus = Focus::Files;
        self.file_state.select(Some(0));
        self.refresh_files();
    }

    /// Volta para a árvore. Saindo do Showall/Branch, o cursor da árvore
    /// vai para o diretório do arquivo que estava selecionado.
    fn back_to_tree(&mut self) {
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

    fn open_viewer(&mut self) {
        let Some(f) = self.selected_file() else {
            return;
        };
        let path = self.tree.path(f.dir).join(&self.file(f).name);
        match Viewer::open(path) {
            Ok(v) => self.viewer = Some(v),
            Err(e) => self.message = Some(format!("Cannot view file: {e}")),
        }
    }

    fn select_row(&mut self, index: usize) {
        if Some(index) != self.tree_state.selected() {
            self.tree_state.select(Some(index));
            let id = self.current();
            self.tree.scan(id);
            self.file_state.select(Some(0));
            self.refresh_files();
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

    /// Refaz a lista da janela de arquivos conforme a visão e o filespec.
    fn refresh_files(&mut self) {
        let dirs = match self.view {
            FileView::Dir => vec![self.current()],
            FileView::Branch(id) => self.tree.subtree(id),
            FileView::Showall => self.tree.subtree(ROOT),
        };
        let mut files = Vec::new();
        for dir in dirs {
            for (idx, f) in self.tree.node(dir).files.iter().enumerate() {
                if self.filespec.matches(&f.name) {
                    files.push(FileRef { dir, idx });
                }
            }
        }
        if self.view != FileView::Dir {
            // Ordenação estável: nomes iguais ficam na ordem da árvore.
            files.sort_by_cached_key(|&f| self.file(f).name.to_string_lossy().to_lowercase());
        }
        self.files = files;

        let last = self.files.len().saturating_sub(1);
        let sel = self.file_state.selected().unwrap_or(0).min(last);
        self.file_state.select(Some(sel));
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
    use ratatui::crossterm::event::KeyModifiers;
    use std::fs::{create_dir_all, write};

    /// raiz/{a.rs, b.txt}, raiz/sub/{c.rs}, raiz/sub/deep/{a.rs}
    fn sample() -> (tempfile::TempDir, App) {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path();
        create_dir_all(p.join("sub/deep")).unwrap();
        write(p.join("a.rs"), "fn main() {}\n").unwrap();
        write(p.join("b.txt"), "b").unwrap();
        write(p.join("sub/c.rs"), "c").unwrap();
        write(p.join("sub/deep/a.rs"), "deep").unwrap();
        let app = App::new(p.to_path_buf());
        (tmp, app)
    }

    fn names(app: &App) -> Vec<String> {
        app.files
            .iter()
            .map(|&f| app.file(f).name.to_string_lossy().into_owned())
            .collect()
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

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
        let (_tmp, mut app) = sample();
        app.apply(Command::Down);
        assert_eq!(app.tree_state.selected(), Some(1));
        app.apply(Command::Collapse);
        app.apply(Command::Collapse);
        assert_eq!(app.tree_state.selected(), Some(0));
    }

    #[test]
    fn filespec_prompt_filters_files() {
        let (_tmp, mut app) = sample();
        assert_eq!(names(&app), ["a.rs", "b.txt"]);

        press(&mut app, KeyCode::Char('f'));
        press(&mut app, KeyCode::Backspace);
        type_text(&mut app, "*.rs");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.filespec.text(), "*.rs");
        assert_eq!(names(&app), ["a.rs"]);

        // Padrão inválido: mantém o anterior e mostra o erro.
        press(&mut app, KeyCode::Char('f'));
        type_text(&mut app, " [");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.filespec.text(), "*.rs");
        assert!(app.message.is_some());
    }

    #[test]
    fn showall_lists_logged_files_sorted() {
        let (_tmp, mut app) = sample();
        app.apply(Command::Showall);
        assert_eq!(app.focus, Focus::Files);
        // "deep" ainda não foi logado.
        assert_eq!(names(&app), ["a.rs", "b.txt", "c.rs"]);
    }

    #[test]
    fn branch_logs_whole_branch() {
        let (_tmp, mut app) = sample();
        app.apply(Command::Branch);
        assert!(app.job.is_none());
        assert_eq!(app.view, FileView::Branch(ROOT));
        assert_eq!(names(&app), ["a.rs", "a.rs", "b.txt", "c.rs"]);
    }

    #[test]
    fn leaving_showall_selects_file_dir() {
        let (_tmp, mut app) = sample();
        app.apply(Command::Branch);
        app.apply(Command::Down); // segundo a.rs, em sub/deep
        let dir = app.selected_file().unwrap().dir;
        assert_ne!(dir, ROOT);
        assert_eq!(app.header_dir(), dir);

        press(&mut app, KeyCode::Esc);
        assert_eq!(app.focus, Focus::Tree);
        assert_eq!(app.view, FileView::Dir);
        assert_eq!(app.current(), dir);
        assert_eq!(names(&app), ["a.rs"]);
    }

    #[test]
    fn expand_all_runs_as_job() {
        let (_tmp, mut app) = sample();
        app.apply(Command::ExpandAll);
        assert!(app.job.is_none());
        assert_eq!(app.rows.len(), 3);
    }

    #[test]
    fn viewer_opens_and_closes() {
        let (_tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('v'));
        let viewer = app.viewer.as_ref().expect("viewer open");
        assert_eq!(viewer.visible_lines(1)[0], "fn main() {}");
        press(&mut app, KeyCode::Esc);
        assert!(app.viewer.is_none());
        assert_eq!(app.focus, Focus::Files);
    }
}
