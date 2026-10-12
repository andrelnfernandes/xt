//! Estado da aplicação e execução dos comandos.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

use crate::input::{self, Command, LineEvent, LineInput};
use crate::model::filespec::Filespec;
use crate::model::tree::{FileEntry, LogJob, NodeId, ROOT, Tree, VisibleRow};
use crate::ops::{self, Answer, OpKind, Operation, Source};
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

/// Campo de texto aberto no rodapé.
pub struct Prompt {
    pub label: String,
    pub input: LineInput,
    kind: PromptKind,
}

enum PromptKind {
    Filespec,
    Transfer {
        kind: OpKind,
        sources: Vec<Source>,
        tagged: bool,
        /// Recriar os subdiretórios a partir de `base` no destino.
        paths: bool,
        base: PathBuf,
    },
    RenameFile(PathBuf),
    RenameDir(NodeId),
    MakeDir(NodeId),
    Chmod(Vec<PathBuf>),
}

impl Prompt {
    /// O prompt de Copy/Move de marcados aceita F6 (com/sem caminhos).
    pub fn toggles_paths(&self) -> bool {
        matches!(self.kind, PromptKind::Transfer { tagged: true, .. })
    }
}

fn transfer_label(kind: OpKind, sources: &[Source], tagged: bool, paths: bool) -> String {
    let verb = if kind == OpKind::Copy { "COPY" } else { "MOVE" };
    let mode = match (tagged, paths) {
        (false, _) => "",
        (true, false) => " (no paths)",
        (true, true) => " (with paths)",
    };
    format!("{verb} {}{mode} to: ", describe(sources))
}

/// Pergunta Sim/Não no rodapé.
pub struct Confirm {
    pub question: String,
    kind: ConfirmKind,
}

enum ConfirmKind {
    Quit,
    DeleteFiles(Vec<Source>),
    DeleteDir(NodeId),
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
    pub prompt: Option<Prompt>,
    pub confirm: Option<Confirm>,
    /// Mensagem mostrada até a próxima tecla.
    pub message: Option<String>,
    pub job: Option<Job>,
    /// Copy/Move/Delete em andamento.
    pub op: Option<Operation>,
    pub viewer: Option<Viewer>,
    /// Último destino digitado no Copy/Move.
    last_dest: Option<String>,
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
            confirm: None,
            message: None,
            job: None,
            op: None,
            viewer: None,
            last_dest: None,
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

    fn file_path(&self, f: FileRef) -> PathBuf {
        self.tree.path(f.dir).join(&self.file(f).name)
    }

    /// Diretório mostrado no cabeçalho: no Showall/Branch, é o do arquivo
    /// sob o cursor.
    pub fn header_dir(&self) -> NodeId {
        match (self.view, self.selected_file()) {
            (FileView::Dir, _) | (_, None) => self.current(),
            (_, Some(f)) => f.dir,
        }
    }

    /// Diretório-raiz da lista atual: o diretório, o Branch ou a árvore.
    fn view_root(&self) -> NodeId {
        match self.view {
            FileView::Dir => self.current(),
            FileView::Branch(id) => id,
            FileView::Showall => ROOT,
        }
    }

    /// Há trabalho em andamento que não depende do usuário.
    pub fn busy(&self) -> bool {
        self.job.is_some() || self.op.as_ref().is_some_and(|op| op.conflict.is_none())
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        self.message = None;

        if let Some(confirm) = self.confirm.take() {
            if matches!(key.code, KeyCode::Char('y' | 'Y')) {
                self.confirmed(confirm.kind);
            }
            return;
        }
        if let Some(viewer) = &mut self.viewer {
            if !viewer.handle_key(key) {
                self.viewer = None;
            }
            return;
        }
        if let Some(op) = &mut self.op {
            if op.conflict.is_some() {
                let answer = match key.code {
                    KeyCode::Char('y' | 'Y') => Some(Answer::Yes),
                    KeyCode::Char('n' | 'N') => Some(Answer::No),
                    KeyCode::Char('a' | 'A') => Some(Answer::All),
                    KeyCode::Char('s' | 'S') => Some(Answer::SkipAll),
                    _ => None,
                };
                if let Some(answer) = answer {
                    op.answer(answer);
                    self.tick(Duration::from_millis(30));
                    return;
                }
            }
            if key.code == KeyCode::Esc {
                op.cancel();
                self.finish_op("Cancelled. ");
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
            if key.code == KeyCode::F(6)
                && let PromptKind::Transfer {
                    kind,
                    sources,
                    tagged: true,
                    paths,
                    ..
                } = &mut prompt.kind
            {
                *paths = !*paths;
                prompt.label = transfer_label(*kind, sources, true, *paths);
                return;
            }
            match prompt.input.handle_key(key) {
                LineEvent::Submit => {
                    let prompt = self.prompt.take().unwrap();
                    self.submit(prompt.kind, prompt.input.text);
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
        let in_files = self.focus == Focus::Files;
        match cmd {
            Command::Quit => self.ask(ConfirmKind::Quit, "Quit xt?".into()),
            Command::SwitchWindow => match self.focus {
                Focus::Tree => self.focus = Focus::Files,
                Focus::Files => self.back_to_tree(),
            },
            Command::Back => self.back_to_tree(),
            Command::Filespec => {
                let text = self.filespec.text().to_string();
                self.open_prompt(PromptKind::Filespec, "FILESPEC: ".into(), &text);
            }
            Command::Showall => self.set_view(FileView::Showall),
            Command::Branch => {
                let id = self.view_root();
                self.start_job(LogJob::new(id, false), AfterLog::OpenBranch(id));
            }
            Command::ExpandAll => {
                self.start_job(LogJob::new(self.current(), true), AfterLog::Refresh);
            }
            Command::View => self.open_viewer(),
            Command::Relog => {
                let id = self.header_dir();
                self.tree.rescan(id);
                self.after_change();
                self.message = Some(format!("Relogged {}", self.tree.path(id).display()));
            }
            Command::Tag | Command::Untag => {
                let tag = cmd == Command::Tag;
                if in_files {
                    if let Some(f) = self.selected_file() {
                        self.set_tags(&[f], |_| tag);
                        self.apply_files(Command::Down);
                    }
                } else {
                    self.set_tags(&self.files.clone(), |_| tag);
                }
            }
            Command::TagAll | Command::UntagAll => {
                let tag = cmd == Command::TagAll;
                let refs = if in_files {
                    self.files.clone()
                } else {
                    self.logged_matching_files()
                };
                self.set_tags(&refs, |_| tag);
            }
            Command::Invert => self.set_tags(&self.files.clone(), |t| !t),
            Command::Copy { tagged, paths } => self.prompt_transfer(OpKind::Copy, tagged, paths),
            Command::Move { tagged, paths } => self.prompt_transfer(OpKind::Move, tagged, paths),
            Command::Delete { tagged: false } if !in_files => self.ask_delete_dir(),
            Command::Delete { tagged } => {
                if let Some(sources) = self.sources_or_warn(tagged) {
                    let q = format!("DELETE {}? This cannot be undone.", describe(&sources));
                    self.ask(ConfirmKind::DeleteFiles(sources), q);
                }
            }
            Command::Attributes { tagged } => self.prompt_chmod(tagged),
            Command::Rename if in_files => {
                if let Some(f) = self.selected_file() {
                    let name = self.file(f).name.to_string_lossy().into_owned();
                    let label = format!("RENAME {name} to: ");
                    self.open_prompt(PromptKind::RenameFile(self.file_path(f)), label, &name);
                }
            }
            Command::Rename => {
                let id = self.current();
                if id == ROOT {
                    self.message = Some("Cannot rename the root directory".into());
                } else {
                    let name = self.tree.node(id).name.to_string_lossy().into_owned();
                    let label = format!("RENAME directory {name} to: ");
                    self.open_prompt(PromptKind::RenameDir(id), label, &name);
                }
            }
            Command::MakeDir => {
                let id = self.current();
                let label = format!("MAKE DIRECTORY in {}: ", self.tree.path(id).display());
                self.open_prompt(PromptKind::MakeDir(id), label, "");
            }
            _ => match self.focus {
                Focus::Tree => self.apply_tree(cmd),
                Focus::Files => self.apply_files(cmd),
            },
        }
    }

    /// Avança o trabalho em andamento por no máximo `budget`.
    pub fn tick(&mut self, budget: Duration) {
        if let Some(op) = &mut self.op {
            op.step(budget);
            if op.is_done() {
                self.finish_op("");
            }
            return;
        }
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

    fn start_op(&mut self, op: Operation) {
        self.op = Some(op);
        self.tick(Duration::from_millis(30));
    }

    /// Fim (ou cancelamento) de uma operação: relê os diretórios afetados.
    fn finish_op(&mut self, prefix: &str) {
        let Some(op) = self.op.take() else { return };
        for dir in op.touched_dirs() {
            self.rescan_path(&dir);
        }
        self.after_change();
        self.message = Some(format!("{prefix}{}", op.summary()));
    }

    fn open_prompt(&mut self, kind: PromptKind, label: String, text: &str) {
        self.prompt = Some(Prompt {
            label,
            input: LineInput::new(text),
            kind,
        });
    }

    fn ask(&mut self, kind: ConfirmKind, question: String) {
        self.confirm = Some(Confirm { question, kind });
    }

    fn submit(&mut self, kind: PromptKind, text: String) {
        let result = match kind {
            PromptKind::Filespec => Filespec::parse(&text).map(|spec| {
                self.filespec = spec;
                self.refresh_files();
            }),
            PromptKind::Transfer {
                kind,
                sources,
                paths,
                base,
                ..
            } => {
                let dest = ops::resolve_path(&text, &self.tree.path(self.current()));
                // Guarda o caminho absoluto: um relativo mudaria de sentido
                // na próxima vez, a partir de outro diretório.
                self.last_dest = Some(dest.to_string_lossy().into_owned());
                let paths_from = paths.then_some(base.as_path());
                Operation::transfer(kind, sources, &dest, paths_from).map(|op| self.start_op(op))
            }
            PromptKind::RenameFile(path) => self.rename_file(&path, &text),
            PromptKind::RenameDir(id) => self.rename_dir(id, &text),
            PromptKind::MakeDir(id) => self.make_dir(id, &text),
            PromptKind::Chmod(paths) => self.chmod(&paths, &text),
        };
        if let Err(e) = result {
            self.message = Some(e);
        }
    }

    fn confirmed(&mut self, kind: ConfirmKind) {
        match kind {
            ConfirmKind::Quit => self.should_quit = true,
            ConfirmKind::DeleteFiles(sources) => self.start_op(Operation::delete(sources)),
            ConfirmKind::DeleteDir(id) => {
                let path = self.tree.path(id);
                let parent = self.tree.node(id).parent.unwrap_or(ROOT);
                match fs::remove_dir(&path) {
                    Ok(()) => {
                        self.tree.rescan(parent);
                        self.refresh_rows(parent);
                        self.refresh_files();
                        self.message = Some(format!("Deleted {}", path.display()));
                    }
                    Err(e) => self.message = Some(format!("Cannot delete directory: {e}")),
                }
            }
        }
    }

    /// Arquivos sobre os quais uma operação age: os marcados da lista
    /// atual ou o arquivo sob o cursor.
    fn sources(&self, tagged: bool) -> Vec<Source> {
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

    /// `sources`, avisando quando não há nada para operar.
    fn sources_or_warn(&mut self, tagged: bool) -> Option<Vec<Source>> {
        let sources = self.sources(tagged);
        if sources.is_empty() {
            if tagged {
                self.message = Some("No tagged files".into());
            }
            return None;
        }
        Some(sources)
    }

    fn prompt_transfer(&mut self, kind: OpKind, tagged: bool, paths: bool) {
        let Some(sources) = self.sources_or_warn(tagged) else {
            return;
        };
        let label = transfer_label(kind, &sources, tagged, paths);
        let default = self.last_dest.clone().unwrap_or_else(|| {
            self.tree
                .path(self.current())
                .to_string_lossy()
                .into_owned()
        });
        let prompt = PromptKind::Transfer {
            kind,
            sources,
            tagged,
            paths,
            base: self.tree.path(self.view_root()),
        };
        self.open_prompt(prompt, label, &default);
    }

    fn prompt_chmod(&mut self, tagged: bool) {
        let Some(sources) = self.sources_or_warn(tagged) else {
            return;
        };
        let mode = fs::metadata(&sources[0].path)
            .map(|m| format!("{:03o}", m.permissions().mode() & 0o7777))
            .unwrap_or_default();
        let label = format!("PERMISSIONS for {} (octal): ", describe(&sources));
        let paths = sources.into_iter().map(|s| s.path).collect();
        self.open_prompt(PromptKind::Chmod(paths), label, &mode);
    }

    fn ask_delete_dir(&mut self) {
        let id = self.current();
        if id == ROOT {
            self.message = Some("Cannot delete the root directory".into());
            return;
        }
        self.tree.scan(id);
        self.tree.rescan(id);
        let node = self.tree.node(id);
        if !node.files.is_empty() || !node.children.is_empty() {
            self.message = Some("Directory is not empty".into());
            self.after_change();
            return;
        }
        let q = format!("DELETE directory {}?", node.name.to_string_lossy());
        self.ask(ConfirmKind::DeleteDir(id), q);
    }

    fn rename_file(&mut self, path: &Path, name: &str) -> Result<(), String> {
        ops::check_name(name)?;
        let new = path.with_file_name(name);
        if new == path {
            return Ok(());
        }
        if fs::symlink_metadata(&new).is_ok() {
            return Err(format!("{name} already exists"));
        }
        fs::rename(path, &new).map_err(|e| format!("Cannot rename: {e}"))?;
        if let Some(dir) = path.parent() {
            self.rescan_path(dir);
        }
        self.after_change();
        // Mantém o cursor no arquivo renomeado.
        if let Some(i) = self.files.iter().position(|&f| self.file_path(f) == new) {
            self.file_state.select(Some(i));
        }
        Ok(())
    }

    fn rename_dir(&mut self, id: NodeId, name: &str) -> Result<(), String> {
        ops::check_name(name)?;
        let path = self.tree.path(id);
        let new = path.with_file_name(name);
        if new == path {
            return Ok(());
        }
        if fs::symlink_metadata(&new).is_ok() {
            return Err(format!("{name} already exists"));
        }
        fs::rename(&path, &new).map_err(|e| format!("Cannot rename: {e}"))?;
        let parent = self.tree.node(id).parent.unwrap_or(ROOT);
        self.tree.rescan(parent);
        let keep = self.tree.find(&new).unwrap_or(parent);
        self.tree.scan(keep);
        self.refresh_rows(keep);
        self.refresh_files();
        Ok(())
    }

    fn make_dir(&mut self, id: NodeId, name: &str) -> Result<(), String> {
        ops::check_name(name)?;
        let path = self.tree.path(id).join(name);
        fs::create_dir(&path).map_err(|e| format!("Cannot create directory: {e}"))?;
        self.tree.rescan(id);
        self.tree.expand(id);
        self.refresh_rows(id);
        self.message = Some(format!("Created {}", path.display()));
        Ok(())
    }

    fn chmod(&mut self, paths: &[PathBuf], text: &str) -> Result<(), String> {
        let mode = ops::parse_mode(text)?;
        let mut errors = Vec::new();
        let mut changed = 0;
        for path in paths {
            // Links simbólicos não têm permissões próprias no Linux.
            if fs::symlink_metadata(path).is_ok_and(|m| m.is_symlink()) {
                continue;
            }
            match fs::set_permissions(path, fs::Permissions::from_mode(mode)) {
                Ok(()) => changed += 1,
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        let mut dirs: Vec<&Path> = paths.iter().filter_map(|p| p.parent()).collect();
        dirs.dedup();
        for dir in dirs {
            self.rescan_path(dir);
        }
        self.after_change();
        let mut msg = format!("Permissions set to {mode:03o} on {changed} files");
        if let Some(e) = errors.first() {
            msg.push_str(&format!(". {} errors, first: {e}", errors.len()));
        }
        self.message = Some(msg);
        Ok(())
    }

    fn set_tags(&mut self, refs: &[FileRef], f: impl Fn(bool) -> bool) {
        for &r in refs {
            let entry = self.tree.file_mut(r.dir, r.idx);
            entry.tagged = f(entry.tagged);
        }
    }

    /// Todos os arquivos logados que passam no filespec.
    fn logged_matching_files(&self) -> Vec<FileRef> {
        let mut refs = Vec::new();
        for dir in self.tree.subtree(ROOT) {
            for (idx, f) in self.tree.node(dir).files.iter().enumerate() {
                if self.filespec.matches(&f.name) {
                    refs.push(FileRef { dir, idx });
                }
            }
        }
        refs
    }

    /// Relê `dir`; se ele ainda não está na árvore (ex.: acabou de ser
    /// criado), relê o ancestral mais próximo que está.
    fn rescan_path(&mut self, dir: &Path) {
        if let Some(id) = dir.ancestors().find_map(|d| self.tree.find(d)) {
            self.tree.rescan(id);
        }
    }

    /// Atualiza a árvore e a lista de arquivos depois de mudanças no disco.
    fn after_change(&mut self) {
        self.refresh_rows(self.current());
        self.refresh_files();
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
        match Viewer::open(self.file_path(f)) {
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

/// "arquivo.txt" ou "3 tagged files".
fn describe(sources: &[Source]) -> String {
    match sources {
        [one] => one
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        many => format!("{} tagged files", many.len()),
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

    fn ctrl(app: &mut App, c: char) {
        app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
    }

    /// Responde a um prompt aberto, trocando o texto sugerido.
    fn answer_prompt(app: &mut App, text: &str) {
        assert!(app.prompt.is_some(), "prompt should be open");
        ctrl(app, 'u');
        type_text(app, text);
        press(app, KeyCode::Enter);
        // Termina a operação, se houver.
        while app.busy() {
            app.tick(Duration::from_millis(50));
        }
    }

    fn tagged(app: &App) -> Vec<String> {
        app.files
            .iter()
            .filter(|&&f| app.file(f).tagged)
            .map(|&f| app.file(f).name.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn tag_untag_and_invert() {
        let (_tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('t'));
        assert_eq!(tagged(&app), ["a.rs"]);
        assert_eq!(app.file_state.selected(), Some(1), "cursor moves down");
        press(&mut app, KeyCode::Char('i'));
        assert_eq!(tagged(&app), ["b.txt"]);
        ctrl(&mut app, 't');
        assert_eq!(tagged(&app).len(), 2);
        ctrl(&mut app, 'u');
        assert!(tagged(&app).is_empty());

        // Na árvore, T marca o diretório atual inteiro.
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('t'));
        assert_eq!(tagged(&app).len(), 2);
    }

    #[test]
    fn ctrl_t_in_tree_tags_all_logged_matching_files() {
        let (_tmp, mut app) = sample();
        app.filespec = Filespec::parse("*.rs").unwrap();
        ctrl(&mut app, 't');
        app.apply(Command::Showall);
        app.filespec = Filespec::default();
        app.refresh_files();
        assert_eq!(tagged(&app), ["a.rs", "c.rs"]);
    }

    #[test]
    fn copy_tagged_files_to_directory() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        ctrl(&mut app, 't');
        ctrl(&mut app, 'c');
        let dest = tmp.path().join("sub");
        answer_prompt(&mut app, dest.to_str().unwrap());

        assert!(dest.join("a.rs").exists() && dest.join("b.txt").exists());
        assert!(app.message.as_deref().unwrap().starts_with("Copied 2 of 2"));
        // A árvore já mostra os arquivos novos em sub/.
        let sub = app.tree.find(&dest).unwrap();
        assert_eq!(app.tree.node(sub).files.len(), 3);
        // Tags continuam na origem.
        assert_eq!(tagged(&app).len(), 2);
    }

    #[test]
    fn move_with_conflict_prompt() {
        let (tmp, mut app) = sample();
        write(tmp.path().join("sub/b.txt"), "old").unwrap();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char('m'));
        answer_prompt(&mut app, "sub"); // relativo ao diretório atual
        assert!(app.op.as_ref().unwrap().conflict.is_some());

        press(&mut app, KeyCode::Char('y'));
        assert!(app.op.is_none());
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("sub/b.txt")).unwrap(),
            "b"
        );
        assert_eq!(names(&app), ["a.rs"]);
    }

    #[test]
    fn delete_asks_for_confirmation() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('d'));
        assert!(app.confirm.is_some());
        press(&mut app, KeyCode::Char('n'));
        assert!(tmp.path().join("a.rs").exists());

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));
        while app.busy() {
            app.tick(Duration::from_millis(50));
        }
        assert!(!tmp.path().join("a.rs").exists());
        assert_eq!(names(&app), ["b.txt"]);
    }

    #[test]
    fn tagged_ops_without_tags_warn() {
        let (_tmp, mut app) = sample();
        ctrl(&mut app, 'd');
        assert!(app.confirm.is_none());
        assert_eq!(app.message.as_deref(), Some("No tagged files"));
    }

    #[test]
    fn rename_file_keeps_cursor_on_it() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('r'));
        answer_prompt(&mut app, "z.rs");
        assert!(tmp.path().join("z.rs").exists());
        assert_eq!(names(&app), ["b.txt", "z.rs"]);
        assert_eq!(app.file_state.selected(), Some(1));

        press(&mut app, KeyCode::Char('r'));
        answer_prompt(&mut app, "b.txt");
        assert_eq!(app.message.as_deref(), Some("b.txt already exists"));
    }

    #[test]
    fn make_rename_and_delete_directory() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Char('m'));
        answer_prompt(&mut app, "new");
        assert!(tmp.path().join("new").is_dir());
        assert!(app.rows.iter().any(|r| app.tree.node(r.id).name == "new"));

        // Cursor no "new" (ordem: new, sub).
        press(&mut app, KeyCode::Down);
        assert_eq!(app.tree.node(app.current()).name, "new");
        press(&mut app, KeyCode::Char('r'));
        answer_prompt(&mut app, "renamed");
        assert!(tmp.path().join("renamed").is_dir());
        assert_eq!(app.tree.node(app.current()).name, "renamed");

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));
        assert!(!tmp.path().join("renamed").exists());
        assert_eq!(app.current(), ROOT);

        // Diretório com conteúdo não é apagado.
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char('d'));
        assert!(app.confirm.is_none());
        assert_eq!(app.message.as_deref(), Some("Directory is not empty"));
    }

    #[test]
    fn chmod_tagged_files() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        ctrl(&mut app, 't');
        ctrl(&mut app, 'a');
        answer_prompt(&mut app, "600");
        for f in ["a.rs", "b.txt"] {
            let mode = std::fs::metadata(tmp.path().join(f))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert_eq!(app.file(app.files[0]).mode & 0o777, 0o600);

        ctrl(&mut app, 'a');
        answer_prompt(&mut app, "999");
        assert!(app.message.as_deref().unwrap().contains("octal"));
    }

    #[test]
    fn relog_picks_up_external_changes() {
        let (tmp, mut app) = sample();
        write(tmp.path().join("new.txt"), "").unwrap();
        assert_eq!(names(&app).len(), 2);
        press(&mut app, KeyCode::F(2));
        assert_eq!(names(&app), ["a.rs", "b.txt", "new.txt"]);
    }

    fn alt(app: &mut App, c: char) {
        app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT));
    }

    /// Marca os dois a.rs do Branch da raiz (raiz e sub/deep).
    fn tag_both_a_rs(tmp: &tempfile::TempDir, app: &mut App) -> PathBuf {
        app.apply(Command::Branch);
        app.filespec = Filespec::parse("a.rs").unwrap();
        app.refresh_files();
        ctrl(app, 't');
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        dest
    }

    #[test]
    fn alt_copy_recreates_paths() {
        let (tmp, mut app) = sample();
        let dest = tag_both_a_rs(&tmp, &mut app);
        alt(&mut app, 'c');
        assert!(app.prompt.as_ref().unwrap().label.contains("with paths"));
        answer_prompt(&mut app, dest.to_str().unwrap());

        assert!(dest.join("a.rs").exists());
        assert_eq!(
            std::fs::read_to_string(dest.join("sub/deep/a.rs")).unwrap(),
            "deep"
        );
        // O destino aparece na árvore e, ao expandi-lo, as pastas criadas.
        let out = app.tree.find(&dest).expect("dest in tree");
        app.tree.expand(out);
        assert!(app.tree.find(&dest.join("sub")).is_some());
    }

    #[test]
    fn ctrl_copy_is_flat_and_f6_toggles() {
        let (tmp, mut app) = sample();
        let dest = tag_both_a_rs(&tmp, &mut app);
        ctrl(&mut app, 'c');
        assert!(app.prompt.as_ref().unwrap().label.contains("no paths"));
        press(&mut app, KeyCode::F(6));
        assert!(app.prompt.as_ref().unwrap().label.contains("with paths"));
        press(&mut app, KeyCode::F(6));
        answer_prompt(&mut app, dest.to_str().unwrap());

        // Dois a.rs indo para o mesmo lugar: o segundo é um conflito.
        assert!(app.op.as_ref().unwrap().conflict.is_some());
        press(&mut app, KeyCode::Char('n'));
        assert!(dest.join("a.rs").exists());
        assert!(!dest.join("sub").exists());
    }

    #[test]
    fn alt_move_with_paths_from_branch_root() {
        let (tmp, mut app) = sample();
        // Branch a partir de sub/: caminhos relativos a sub.
        press(&mut app, KeyCode::Down);
        app.apply(Command::Branch);
        ctrl(&mut app, 't');
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        alt(&mut app, 'm');
        // Relativo ao diretório do cursor da árvore (sub), não ao do arquivo.
        answer_prompt(&mut app, "../out");
        // O próximo prompt, de outro diretório, sugere o destino já resolvido.
        app.apply(Command::Back);
        app.select_node(ROOT);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('c'));
        assert_eq!(
            app.prompt.as_ref().unwrap().input.text,
            dest.to_str().unwrap()
        );

        assert!(dest.join("c.rs").exists());
        assert!(dest.join("deep/a.rs").exists());
        assert!(!tmp.path().join("sub/deep/a.rs").exists());
    }
}
