//! Estado da aplicação e execução dos comandos.
//!
//! O estado de cada janela (árvore, cursores, filespec) fica em [`Pane`];
//! aqui fica o que é global: prompts, perguntas, operações em andamento e
//! o visualizador. `App` dá acesso direto à janela ativa (`app.tree`,
//! `app.files`...), via `Deref`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::{Deref, DerefMut};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode};

use crate::archive;
use crate::input::{self, Command, LineEvent, LineInput};
use crate::model::compare::{self, CompareMode};
use crate::model::filespec::Filespec;
use crate::model::tree::{LogJob, NodeId, ROOT};
use crate::ops::{self, Answer, OpKind, Operation, Source};
pub use crate::pane::{FileView, Focus, Pane};
use crate::shell::{self, External};
use crate::viewer::{self, Viewer};

/// Leitura em andamento e o que fazer quando ela terminar.
pub struct Job {
    pub log: LogJob,
    /// Janela cuja árvore está sendo lida.
    pane: usize,
    then: AfterLog,
}

enum AfterLog {
    Refresh,
    OpenBranch(NodeId),
    /// Abre o Branch e compara com a outra janela.
    CompareBranch(NodeId, CompareMode),
}

/// Qual menu o rodapé mostra: o normal ou o dos comandos com Ctrl/Alt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MenuMode {
    #[default]
    Normal,
    Ctrl,
    Alt,
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
    Log,
    Execute,
}

impl PromptKind {
    /// Prompts do mesmo tipo dividem o histórico (↑/↓).
    fn history_key(&self) -> Option<&'static str> {
        Some(match self {
            PromptKind::Filespec => "filespec",
            PromptKind::Transfer { .. } => "destination",
            PromptKind::Log => "log",
            PromptKind::Execute => "execute",
            PromptKind::MakeDir(_) => "mkdir",
            PromptKind::Chmod(_) => "chmod",
            PromptKind::RenameFile(_) | PromptKind::RenameDir(_) => return None,
        })
    }
}

/// Entradas guardadas por tipo de prompt.
const HISTORY_SIZE: usize = 100;

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

/// Pergunta no rodapé, respondida com uma tecla: a primeira letra de uma
/// das opções. Qualquer outra tecla desiste.
pub struct Confirm {
    pub question: String,
    pub options: &'static [&'static str],
    kind: ConfirmKind,
}

const YES_NO: &[&str] = &["Yes", "No"];
const COMPARE_OPTIONS: &[&str] = &[
    "Unique",
    "Newer",
    "Older",
    "Identical",
    "Different",
    "All new (unique + newer)",
    "Branch on/off",
];

enum ConfirmKind {
    Quit,
    DeleteFiles(Vec<Source>),
    DeleteDir(NodeId),
    /// `branch`: compara o ramo inteiro, não só o diretório.
    Compare {
        branch: bool,
    },
}

pub struct App {
    /// Uma janela, ou duas com a tela dividida (F8).
    pub panes: Vec<Pane>,
    pub active: usize,
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
    /// Diretórios que mudaram no disco e ainda não foram relidos.
    pending_changes: HashSet<PathBuf>,
    /// Histórico dos prompts, por tipo.
    history: HashMap<&'static str, Vec<String>>,
    /// Programa externo (editor, comando) a rodar pelo laço principal,
    /// que entrega o terminal a ele.
    pub external: Option<External>,
    pub should_quit: bool,
    /// Ctrl/Alt pressionado agora (só em terminais que avisam isso).
    pub held: Option<MenuMode>,
    /// Menu escolhido com F9; a próxima letra recebe esse modificador.
    pub sticky: MenuMode,
}

/// A janela ativa.
impl Deref for App {
    type Target = Pane;

    fn deref(&self) -> &Pane {
        &self.panes[self.active]
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Pane {
        &mut self.panes[self.active]
    }
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        App {
            panes: vec![Pane::new(root)],
            active: 0,
            prompt: None,
            confirm: None,
            message: None,
            job: None,
            op: None,
            viewer: None,
            last_dest: None,
            pending_changes: HashSet::new(),
            history: HashMap::new(),
            external: None,
            should_quit: false,
            held: None,
            sticky: MenuMode::Normal,
        }
    }

    pub fn is_split(&self) -> bool {
        self.panes.len() > 1
    }

    /// A outra janela, com a tela dividida.
    pub fn other(&self) -> Option<&Pane> {
        self.is_split().then(|| &self.panes[1 - self.active])
    }

    /// Diretórios a observar no disco: os atuais de cada janela (fora de
    /// arquivos compactados).
    pub fn watched_dirs(&self) -> HashSet<PathBuf> {
        self.panes
            .iter()
            .filter(|p| p.archive.is_none())
            .map(Pane::current_path)
            .collect()
    }

    /// Avisos de mudanças no disco. São aplicados assim que nada estiver
    /// em andamento (no fim de um Copy, os diretórios são relidos de
    /// qualquer jeito). Devolve se a tela mudou.
    pub fn fs_changed(&mut self, dirs: impl IntoIterator<Item = PathBuf>) -> bool {
        self.pending_changes.extend(dirs);
        if self.pending_changes.is_empty() || self.busy() || self.op.is_some() {
            return false;
        }
        let dirs: Vec<PathBuf> = self.pending_changes.drain().collect();
        self.rescan_everywhere(&dirs);
        true
    }

    /// Há trabalho em andamento que não depende do usuário.
    pub fn busy(&self) -> bool {
        self.job.is_some() || self.op.as_ref().is_some_and(|op| op.conflict.is_none())
    }

    /// Menu a mostrar no rodapé.
    pub fn menu_mode(&self) -> MenuMode {
        self.held.unwrap_or(self.sticky)
    }

    pub fn handle_key(&mut self, mut key: KeyEvent) {
        // Ctrl/Alt sozinhos só trocam o menu enquanto estão pressionados.
        if let KeyCode::Modifier(m) = key.code {
            let mode = match m {
                ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => MenuMode::Ctrl,
                ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => MenuMode::Alt,
                _ => return,
            };
            if key.kind == KeyEventKind::Release {
                if self.held == Some(mode) {
                    self.held = None;
                }
            } else {
                self.held = Some(mode);
            }
            return;
        }
        if key.kind == KeyEventKind::Release {
            return;
        }
        // Se a soltura do Ctrl/Alt se perdeu (ex.: troca de janela), a
        // próxima tecla sem ele corrige o menu.
        let still_held = match self.held {
            Some(MenuMode::Ctrl) => key.modifiers.contains(KeyModifiers::CONTROL),
            Some(MenuMode::Alt) => key.modifiers.contains(KeyModifiers::ALT),
            _ => true,
        };
        if !still_held {
            self.held = None;
        }

        self.message = None;

        if let Some(confirm) = self.confirm.take() {
            if let KeyCode::Char(c) = key.code {
                let c = c.to_ascii_lowercase();
                let chosen = confirm.options.iter().any(|o| {
                    o.chars()
                        .next()
                        .is_some_and(|k| k.to_ascii_lowercase() == c)
                });
                if chosen {
                    self.confirmed(confirm.kind, c);
                }
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
        if let Some(job) = &self.job {
            if key.code == KeyCode::Esc {
                let pane = &mut self.panes[job.pane];
                pane.refresh_rows(pane.current());
                self.job = None;
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
        if key.code == KeyCode::F(9) {
            self.sticky = match self.sticky {
                MenuMode::Normal => MenuMode::Ctrl,
                MenuMode::Ctrl => MenuMode::Alt,
                MenuMode::Alt => MenuMode::Normal,
            };
            return;
        }
        // Com o menu Ctrl/Alt aberto pelo F9, a próxima letra vale como
        // Ctrl+letra / Alt+letra. Útil onde o terminal não deixa passar a
        // combinação (Ctrl+M no Terminator, por exemplo).
        match std::mem::take(&mut self.sticky) {
            MenuMode::Normal => {}
            _ if key.code == KeyCode::Esc => return,
            MenuMode::Ctrl if matches!(key.code, KeyCode::Char(_)) => {
                key.modifiers |= KeyModifiers::CONTROL;
            }
            MenuMode::Alt if matches!(key.code, KeyCode::Char(_)) => {
                key.modifiers |= KeyModifiers::ALT;
            }
            _ => {}
        }
        if let Some(cmd) = input::map_key(key, self.focus) {
            self.apply(cmd);
        }
    }

    pub fn apply(&mut self, cmd: Command) {
        let in_files = self.focus == Focus::Files;
        let writes = matches!(
            cmd,
            Command::Move { .. }
                | Command::Delete { .. }
                | Command::Rename
                | Command::Attributes { .. }
                | Command::MakeDir
                | Command::Log
                | Command::Edit
                | Command::Execute
        );
        if writes && self.archive.is_some() {
            self.message = Some("Archives are read-only (use Copy to extract)".into());
            return;
        }
        match cmd {
            Command::Quit => self.ask(ConfirmKind::Quit, "Quit xt?".into()),
            Command::SwitchWindow => match self.focus {
                Focus::Tree => self.focus = Focus::Files,
                Focus::Files => self.back_to_tree(),
            },
            // Tab troca de janela com a tela dividida; sem ela, faz o mesmo
            // que Enter.
            Command::SwitchPane if self.is_split() => self.active = 1 - self.active,
            Command::SwitchPane => self.apply(Command::SwitchWindow),
            Command::Split => self.toggle_split(),
            Command::Compare => self.ask_compare(false),
            Command::Log => {
                let current = self.current_path().to_string_lossy().into_owned();
                self.open_prompt(PromptKind::Log, "LOG directory: ".into(), &current);
            }
            Command::Back if in_files => self.back_to_tree(),
            Command::Back => self.close_archive(),
            Command::Open => self.open_archive(),
            Command::Edit => {
                if let Some(f) = self.selected_file() {
                    self.external = Some(shell::editor(&self.file_path(f), &self.current_path()));
                }
            }
            Command::Execute => {
                let label = format!("EXECUTE in {}: ", fit_name(&self.current_path()));
                self.open_prompt(PromptKind::Execute, label, "");
            }
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
                    let files = self.files.clone();
                    self.set_tags(&files, |_| tag);
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
            Command::Invert => {
                let files = self.files.clone();
                self.set_tags(&files, |t| !t);
            }
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
        let pane = &mut self.panes[job.pane];
        pane.tree.run_job(&mut job.log, budget);
        if !job.log.is_done() {
            return;
        }
        pane.refresh_rows(pane.current());
        match job.then {
            AfterLog::OpenBranch(id) => pane.set_view(FileView::Branch(id)),
            AfterLog::CompareBranch(id, mode) => {
                pane.set_view(FileView::Branch(id));
                self.job = None;
                self.compare(mode);
                return;
            }
            AfterLog::Refresh => pane.refresh_files(),
        }
        self.job = None;
    }

    fn start_job(&mut self, log: LogJob, then: AfterLog) {
        self.job = Some(Job {
            log,
            pane: self.active,
            then,
        });
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
        self.rescan_everywhere(&op.touched_dirs());
        self.message = Some(format!("{prefix}{}", op.summary()));
    }

    /// Relê `dirs` em todas as janelas (as duas podem mostrar o mesmo lugar).
    fn rescan_everywhere<P: AsRef<Path>>(&mut self, dirs: &[P]) {
        for pane in &mut self.panes {
            pane.rescan_paths(dirs);
        }
    }

    fn ask_compare(&mut self, branch: bool) {
        let Some(other) = self.other() else {
            self.message = Some("Compare needs the split window (F8)".into());
            return;
        };
        let scope = if branch { "branch" } else { "directory" };
        let q = format!(
            "COMPARE {scope} {} with {} and tag:",
            fit_name(&self.current_path()),
            fit_name(&other.current_path())
        );
        self.ask_options(ConfirmKind::Compare { branch }, q, COMPARE_OPTIONS);
    }

    /// Marca, na lista da janela ativa, os arquivos que têm a relação
    /// pedida com o correspondente no diretório da outra janela. No Branch
    /// e no Showall, o correspondente segue o caminho relativo à raiz da
    /// lista (`src/a.rs` → `<outro>/src/a.rs`).
    fn compare(&mut self, mode: CompareMode) {
        let Some(other_base) = self.other().map(Pane::current_path) else {
            return;
        };
        let base = self.tree.path(self.view_root());
        let files = self.files.clone();
        let mut count = 0;
        for &f in &files {
            let path = self.file_path(f);
            let rel = path.strip_prefix(&base).unwrap_or(&path);
            let target = fs::symlink_metadata(other_base.join(rel)).ok();
            let entry = self.file(f);
            let tag = mode.matches(compare::relation(
                entry.size,
                entry.modified,
                target.as_ref(),
            ));
            count += usize::from(tag);
            self.set_tags(&[f], |_| tag);
        }
        self.message = Some(format!(
            "Tagged {count} of {} files: {} (compared with {})",
            files.len(),
            mode.describe(),
            other_base.display()
        ));
    }

    /// F8: divide a tela em duas janelas, ou volta a uma só. A nova janela
    /// começa na mesma raiz e no mesmo diretório da atual.
    fn toggle_split(&mut self) {
        if self.is_split() {
            let keep = self.panes.swap_remove(self.active);
            self.panes = vec![keep];
            self.active = 0;
            return;
        }
        // Dentro de um arquivo compactado, a nova janela mostra o disco.
        let base = self.outer.as_deref().unwrap_or(self);
        let mut pane = Pane::new(base.tree.path(ROOT));
        pane.goto(&base.current_path());
        self.panes.push(pane);
    }

    /// `O`: abre o ZIP/TAR sob o cursor como se fosse um diretório.
    fn open_archive(&mut self) {
        let Some(f) = self.selected_file() else {
            return;
        };
        let path = self.file_path(f);
        if self.archive.is_some() {
            self.message = Some("Archives inside archives are not supported".into());
        } else if archive::Kind::detect(&path).is_none() {
            self.message = Some("Not an archive (zip, jar, tar, tar.gz, tgz)".into());
        } else {
            match Pane::from_archive(&path) {
                Ok(pane) => {
                    let outer = std::mem::replace(self.deref_mut(), pane);
                    self.outer = Some(Box::new(outer));
                }
                Err(e) => self.message = Some(format!("Cannot open archive: {e}")),
            }
        }
    }

    /// Esc na árvore de um arquivo compactado: volta para onde estava.
    fn close_archive(&mut self) {
        if let Some(outer) = self.outer.take() {
            *self.deref_mut() = *outer;
            // O disco pode ter mudado enquanto o arquivo estava aberto.
            let current = self.current();
            self.tree.rescan(current);
            self.after_change();
        }
    }

    /// Log: abre `path` na janela ativa. Se ele está dentro da árvore atual,
    /// só leva o cursor até lá; senão, a janela passa a ter `path` como raiz.
    fn log(&mut self, text: &str) -> Result<(), String> {
        let path = ops::resolve_path(text, &self.current_path());
        let path = fs::canonicalize(&path).map_err(|e| format!("Cannot log {text}: {e}"))?;
        if !path.is_dir() {
            return Err(format!("{} is not a directory", path.display()));
        }
        if !self.goto(&path) {
            *self.deref_mut() = Pane::new(path);
        }
        Ok(())
    }

    fn open_prompt(&mut self, kind: PromptKind, label: String, text: &str) {
        let history = kind
            .history_key()
            .and_then(|k| self.history.get(k).cloned())
            .unwrap_or_default();
        self.prompt = Some(Prompt {
            label,
            input: LineInput::new(text).with_history(history),
            kind,
        });
    }

    fn remember(&mut self, kind: &PromptKind, text: &str) {
        let Some(key) = kind.history_key() else {
            return;
        };
        if text.trim().is_empty() {
            return;
        }
        let list = self.history.entry(key).or_default();
        list.retain(|t| t != text);
        list.push(text.to_string());
        if list.len() > HISTORY_SIZE {
            list.remove(0);
        }
    }

    /// Volta de um programa externo: o disco pode ter mudado.
    pub fn after_external(&mut self, result: std::io::Result<std::process::ExitStatus>) {
        let dirs: Vec<PathBuf> = self.watched_dirs().into_iter().collect();
        self.rescan_everywhere(&dirs);
        self.message = match result {
            Ok(status) if status.success() => None,
            Ok(status) => Some(format!("Command finished with {status}")),
            Err(e) => Some(format!("Cannot run command: {e}")),
        };
    }

    fn ask(&mut self, kind: ConfirmKind, question: String) {
        self.ask_options(kind, question, YES_NO);
    }

    fn ask_options(
        &mut self,
        kind: ConfirmKind,
        question: String,
        options: &'static [&'static str],
    ) {
        self.confirm = Some(Confirm {
            question,
            options,
            kind,
        });
    }

    fn submit(&mut self, kind: PromptKind, text: String) {
        self.remember(&kind, &text);
        let result = match kind {
            PromptKind::Filespec => Filespec::parse(&text).map(|spec| {
                self.filespec = spec;
                self.refresh_files();
            }),
            PromptKind::Log => self.log(&text),
            PromptKind::Execute => {
                if !text.trim().is_empty() {
                    let file = self.selected_file().filter(|_| self.focus == Focus::Files);
                    let file = file.map(|f| self.file_path(f));
                    let tagged: Vec<PathBuf> =
                        self.sources(true).into_iter().map(|s| s.path).collect();
                    let dir = self.current_path();
                    let line = shell::expand(&text, file.as_deref(), &tagged, &dir);
                    self.external = Some(shell::command(line, &dir));
                }
                Ok(())
            }
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
                let archive = self.archive.clone();
                Operation::transfer(kind, sources, &dest, paths_from)
                    .and_then(|op| match &archive {
                        Some(path) => op.reading_archive(path),
                        None => Ok(op),
                    })
                    .map(|op| self.start_op(op))
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

    fn confirmed(&mut self, kind: ConfirmKind, choice: char) {
        let yes = choice == 'y';
        match kind {
            ConfirmKind::Compare { branch } => match CompareMode::from_key(choice) {
                // Sem modo: é o "Branch on/off"; pergunta de novo.
                None => self.ask_compare(!branch),
                Some(mode) if branch => {
                    let id = self.current();
                    self.start_job(LogJob::new(id, false), AfterLog::CompareBranch(id, mode));
                }
                Some(mode) => self.compare(mode),
            },
            _ if !yes => {}
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
                        self.rescan_everywhere(&[&path]);
                        self.message = Some(format!("Deleted {}", path.display()));
                    }
                    Err(e) => self.message = Some(format!("Cannot delete directory: {e}")),
                }
            }
        }
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
        // Com a tela dividida, o destino natural é a outra janela.
        let default = match self.other() {
            Some(other) => other.current_path().to_string_lossy().into_owned(),
            None => self
                .last_dest
                .clone()
                .unwrap_or_else(|| self.current_path().to_string_lossy().into_owned()),
        };
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
            self.rescan_everywhere(&[dir]);
        }
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
        if let Some(dir) = path.parent() {
            self.rescan_everywhere(&[dir]);
        }
        Ok(())
    }

    fn make_dir(&mut self, id: NodeId, name: &str) -> Result<(), String> {
        ops::check_name(name)?;
        let path = self.tree.path(id).join(name);
        fs::create_dir(&path).map_err(|e| format!("Cannot create directory: {e}"))?;
        self.tree.rescan(id);
        self.tree.expand(id);
        self.refresh_rows(id);
        self.rescan_everywhere(&[&path]);
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
        self.rescan_everywhere(&dirs);
        let mut msg = format!("Permissions set to {mode:03o} on {changed} files");
        if let Some(e) = errors.first() {
            msg.push_str(&format!(". {} errors, first: {e}", errors.len()));
        }
        self.message = Some(msg);
        Ok(())
    }

    fn open_viewer(&mut self) {
        let Some(f) = self.selected_file() else {
            return;
        };
        let path = self.file_path(f);
        let viewer = match &self.archive {
            Some(root) => {
                let entry = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                let size = self.file(f).size;
                archive::Reader::open(root).and_then(|mut r| {
                    let mut data = Vec::new();
                    r.extract(&entry, &mut data, viewer::MAX_BYTES)?;
                    Ok(Viewer::from_bytes(path, size, data))
                })
            }
            None => Viewer::open(path),
        };
        match viewer {
            Ok(v) => self.viewer = Some(v),
            Err(e) => self.message = Some(format!("Cannot view file: {e}")),
        }
    }
}

/// Último componente de um caminho, para mensagens curtas.
fn fit_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
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

    fn modifier(app: &mut App, m: ModifierKeyCode, kind: KeyEventKind) {
        let mut key = KeyEvent::new(KeyCode::Modifier(m), KeyModifiers::NONE);
        key.kind = kind;
        app.handle_key(key);
    }

    #[test]
    fn holding_ctrl_or_alt_switches_menu() {
        let (_tmp, mut app) = sample();
        assert_eq!(app.menu_mode(), MenuMode::Normal);
        modifier(&mut app, ModifierKeyCode::LeftControl, KeyEventKind::Press);
        assert_eq!(app.menu_mode(), MenuMode::Ctrl);
        modifier(&mut app, ModifierKeyCode::LeftControl, KeyEventKind::Repeat);
        assert_eq!(app.menu_mode(), MenuMode::Ctrl);
        modifier(
            &mut app,
            ModifierKeyCode::LeftControl,
            KeyEventKind::Release,
        );
        assert_eq!(app.menu_mode(), MenuMode::Normal);

        modifier(&mut app, ModifierKeyCode::RightAlt, KeyEventKind::Press);
        assert_eq!(app.menu_mode(), MenuMode::Alt);
        // Soltura perdida: a próxima tecla sem Alt volta ao normal.
        press(&mut app, KeyCode::Down);
        assert_eq!(app.menu_mode(), MenuMode::Normal);
    }

    #[test]
    fn release_events_do_nothing() {
        let (_tmp, mut app) = sample();
        let mut key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        key.kind = KeyEventKind::Release;
        app.handle_key(key);
        assert_eq!(app.tree_state.selected(), Some(0));
        key.kind = KeyEventKind::Repeat;
        app.handle_key(key);
        assert_eq!(app.tree_state.selected(), Some(1), "held arrows repeat");
    }

    #[test]
    fn f9_cycles_menus_and_applies_modifier_once() {
        let (_tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::F(9));
        assert_eq!(app.menu_mode(), MenuMode::Ctrl);
        press(&mut app, KeyCode::F(9));
        assert_eq!(app.menu_mode(), MenuMode::Alt);
        press(&mut app, KeyCode::F(9));
        assert_eq!(app.menu_mode(), MenuMode::Normal);

        // F9 + T = Ctrl+T (marca a lista toda); depois volta ao normal.
        press(&mut app, KeyCode::F(9));
        press(&mut app, KeyCode::Char('t'));
        assert_eq!(tagged(&app).len(), 2);
        assert_eq!(app.menu_mode(), MenuMode::Normal);

        // F9 + M = Ctrl+M: mover marcados sem caminhos.
        press(&mut app, KeyCode::F(9));
        press(&mut app, KeyCode::Char('m'));
        let label = &app.prompt.as_ref().unwrap().label;
        assert!(
            label.starts_with("MOVE 2 tagged files (no paths)"),
            "{label}"
        );
        press(&mut app, KeyCode::Esc);

        // F9 F9 + C = Alt+C: copiar com caminhos.
        press(&mut app, KeyCode::F(9));
        press(&mut app, KeyCode::F(9));
        press(&mut app, KeyCode::Char('c'));
        assert!(app.prompt.as_ref().unwrap().label.contains("with paths"));
        press(&mut app, KeyCode::Esc);

        // Esc desiste do menu sem fazer nada.
        press(&mut app, KeyCode::F(9));
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.menu_mode(), MenuMode::Normal);
        assert_eq!(app.focus, Focus::Files);
    }

    #[test]
    fn f8_splits_and_tab_switches_panes() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Down); // sub
        press(&mut app, KeyCode::F(8));
        assert!(app.is_split());
        assert_eq!(app.active, 0);
        // A nova janela começa no mesmo diretório.
        assert_eq!(app.panes[1].current_path(), tmp.path().join("sub"));

        press(&mut app, KeyCode::Tab);
        assert_eq!(app.active, 1);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.current_path(), tmp.path());
        assert_eq!(app.panes[0].current_path(), tmp.path().join("sub"));

        // F8 de novo fica só com a janela ativa.
        press(&mut app, KeyCode::F(8));
        assert!(!app.is_split());
        assert_eq!(app.current_path(), tmp.path());
        // Sem divisão, Tab volta a alternar árvore/arquivos.
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.focus, Focus::Files);
    }

    #[test]
    fn copy_defaults_to_other_pane_and_refreshes_both() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::F(8));
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Down); // janela 2 em sub/
        press(&mut app, KeyCode::Tab);

        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Down); // b.txt
        press(&mut app, KeyCode::Char('c'));
        let prompt = app.prompt.as_ref().unwrap();
        assert_eq!(prompt.input.text, tmp.path().join("sub").to_str().unwrap());
        press(&mut app, KeyCode::Enter);
        while app.busy() {
            app.tick(Duration::from_millis(50));
        }
        assert!(tmp.path().join("sub/b.txt").exists());
        // A outra janela já mostra o arquivo novo.
        let other = &app.panes[1];
        let names: Vec<_> = other
            .files
            .iter()
            .map(|&f| other.file(f).name.clone())
            .collect();
        assert_eq!(names, ["b.txt", "c.rs"]);
    }

    #[test]
    fn log_moves_cursor_or_opens_new_root() {
        let (tmp, mut app) = sample();
        // Dentro da árvore atual: só leva o cursor.
        press(&mut app, KeyCode::Char('l'));
        answer_prompt(&mut app, "sub/deep");
        assert_eq!(app.current_path(), tmp.path().join("sub/deep"));
        assert_eq!(app.tree.path(ROOT), tmp.path());

        // Fora dela: a janela ganha uma nova raiz.
        let other = tempfile::tempdir().unwrap();
        press(&mut app, KeyCode::Char('l'));
        answer_prompt(&mut app, other.path().to_str().unwrap());
        let canonical = fs::canonicalize(other.path()).unwrap();
        assert_eq!(app.tree.path(ROOT), canonical);

        press(&mut app, KeyCode::Char('l'));
        answer_prompt(&mut app, "/caminho/que/nao/existe");
        assert!(app.message.as_deref().unwrap().starts_with("Cannot log"));
    }

    /// Duas janelas: a ativa em `left/`, a outra em `right/`.
    fn compare_setup() -> (tempfile::TempDir, App) {
        use std::time::{Duration, SystemTime};
        let tmp = tempfile::tempdir().unwrap();
        let (l, r) = (tmp.path().join("left"), tmp.path().join("right"));
        create_dir_all(l.join("sub")).unwrap();
        create_dir_all(&r).unwrap();
        let at = |path: PathBuf, content: &str, secs: u64| {
            write(&path, content).unwrap();
            let t = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
            fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(t)
                .unwrap();
        };
        at(l.join("same.txt"), "x", 5_000);
        at(r.join("same.txt"), "x", 5_000);
        at(l.join("newer.txt"), "new", 9_000);
        at(r.join("newer.txt"), "old", 1_000);
        at(l.join("older.txt"), "old", 1_000);
        at(r.join("older.txt"), "new", 9_000);
        at(l.join("only-left.txt"), "", 5_000);
        at(l.join("sub/deep.txt"), "", 5_000);

        let mut app = App::new(l);
        app.apply(Command::Split);
        app.active = 1;
        app.log(r.to_str().unwrap()).unwrap();
        app.active = 0;
        (tmp, app)
    }

    fn compare_with(app: &mut App, key: char) -> Vec<String> {
        press(app, KeyCode::Char('c'));
        assert!(app.confirm.is_some(), "compare question");
        press(app, KeyCode::Char(key));
        tagged(app)
    }

    #[test]
    fn compare_tags_by_relation() {
        let (_tmp, mut app) = compare_setup();
        assert_eq!(compare_with(&mut app, 'u'), ["only-left.txt"]);
        assert_eq!(compare_with(&mut app, 'n'), ["newer.txt"]);
        assert_eq!(compare_with(&mut app, 'o'), ["older.txt"]);
        assert_eq!(compare_with(&mut app, 'i'), ["same.txt"]);
        assert_eq!(compare_with(&mut app, 'd'), ["newer.txt", "older.txt"]);
        assert_eq!(compare_with(&mut app, 'a'), ["newer.txt", "only-left.txt"]);
        assert!(
            app.message
                .as_deref()
                .unwrap()
                .starts_with("Tagged 2 of 4 files")
        );
    }

    #[test]
    fn compare_branch_follows_relative_paths() {
        let (_tmp, mut app) = compare_setup();
        press(&mut app, KeyCode::Char('c'));
        press(&mut app, KeyCode::Char('b'));
        let q = &app.confirm.as_ref().expect("asks again").question;
        assert!(q.starts_with("COMPARE branch left"), "{q}");
        press(&mut app, KeyCode::Char('u'));
        while app.busy() {
            app.tick(Duration::from_millis(50));
        }
        // Abre o Branch; sub/deep.txt não existe em right/sub/: é único.
        assert_eq!(app.view, FileView::Branch(ROOT));
        assert_eq!(tagged(&app), ["deep.txt", "only-left.txt"]);
    }

    #[test]
    fn compare_needs_split_and_esc_cancels() {
        let (_tmp, mut app) = sample();
        press(&mut app, KeyCode::Char('c'));
        assert!(app.confirm.is_none());
        assert_eq!(
            app.message.as_deref(),
            Some("Compare needs the split window (F8)")
        );

        let (_tmp, mut app) = compare_setup();
        press(&mut app, KeyCode::Char('c'));
        press(&mut app, KeyCode::Esc);
        assert!(app.confirm.is_none());
        assert!(tagged(&app).is_empty());
    }

    /// Raiz com um p.zip (src/a.rs, README) ao lado de a.rs e b.txt.
    fn archive_setup() -> (tempfile::TempDir, App) {
        let (tmp, _) = sample();
        crate::archive::tests::make_zip(&tmp.path().join("p.zip"));
        let mut app = App::new(tmp.path().to_path_buf());
        press(&mut app, KeyCode::Enter);
        let i = names(&app).iter().position(|n| n == "p.zip").unwrap();
        app.file_state.select(Some(i));
        (tmp, app)
    }

    #[test]
    fn opens_browses_and_closes_archives() {
        let (tmp, mut app) = archive_setup();
        press(&mut app, KeyCode::Char('o'));
        assert_eq!(
            app.archive.as_deref(),
            Some(tmp.path().join("p.zip").as_path())
        );
        assert_eq!(app.focus, Focus::Tree);
        assert_eq!(names(&app), ["README"]);

        // Ver uma entrada sem extrair nada para o disco.
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(app.viewer.as_ref().unwrap().visible_lines(1)[0], "leia-me");
        press(&mut app, KeyCode::Esc);

        // Escrever é proibido dentro do arquivo.
        press(&mut app, KeyCode::Char('d'));
        assert!(app.confirm.is_none());
        assert!(app.message.as_deref().unwrap().contains("read-only"));

        // Esc na árvore volta exatamente para onde estava.
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Esc);
        assert!(app.archive.is_none());
        assert_eq!(app.current_path(), tmp.path());
    }

    #[test]
    fn copies_out_of_archives() {
        let (tmp, mut app) = archive_setup();
        press(&mut app, KeyCode::Char('o'));
        app.apply(Command::Branch);
        ctrl(&mut app, 't');
        alt(&mut app, 'c');
        let out = tmp.path().join("out");
        fs::create_dir(&out).unwrap();
        answer_prompt(&mut app, out.to_str().unwrap());
        assert!(app.message.as_deref().unwrap().starts_with("Copied 2 of 2"));
        assert_eq!(
            fs::read_to_string(out.join("src/a.rs")).unwrap(),
            "fn a() {}"
        );
        assert!(out.join("README").exists());
    }

    #[test]
    fn open_rejects_non_archives() {
        let (_tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('o'));
        assert!(app.archive.is_none());
        assert!(
            app.message
                .as_deref()
                .unwrap()
                .starts_with("Not an archive")
        );
    }

    #[test]
    fn fs_changes_refresh_now_or_after_operations() {
        let (tmp, mut app) = sample();
        assert_eq!(
            app.watched_dirs(),
            HashSet::from([tmp.path().to_path_buf()])
        );
        write(tmp.path().join("new.txt"), "").unwrap();
        assert!(app.fs_changed([tmp.path().to_path_buf()]));
        assert_eq!(names(&app), ["a.rs", "b.txt", "new.txt"]);

        // Com uma operação parada num conflito, espera.
        write(tmp.path().join("sub/b.txt"), "old").unwrap();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char('c'));
        answer_prompt(&mut app, "sub");
        assert!(app.op.is_some());
        write(tmp.path().join("other.txt"), "").unwrap();
        assert!(!app.fs_changed([tmp.path().to_path_buf()]));
        assert_eq!(names(&app).len(), 3);

        press(&mut app, KeyCode::Char('n'));
        assert!(app.op.is_none());
        assert!(app.fs_changed([]), "pending change applied");
        assert_eq!(names(&app).len(), 4);
    }

    #[test]
    fn edit_and_execute_request_external_programs() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('e'));
        let ext = app.external.take().expect("editor requested");
        assert_eq!(
            ext.args.last().unwrap(),
            tmp.path().join("a.rs").as_os_str()
        );

        press(&mut app, KeyCode::Char('t'));
        press(&mut app, KeyCode::Char('t'));
        press(&mut app, KeyCode::Char('x'));
        type_text(&mut app, "wc -c %t");
        press(&mut app, KeyCode::Enter);
        let ext = app.external.take().expect("command requested");
        let a = tmp.path().join("a.rs");
        let b = tmp.path().join("b.txt");
        let expected = format!("wc -c '{}' '{}'", a.display(), b.display());
        assert_eq!(ext.args[1], expected.as_str());
        assert_eq!(ext.cwd, tmp.path());
        assert!(ext.wait);

        // A volta relê o disco e mostra falhas.
        write(tmp.path().join("made-by-command"), "").unwrap();
        let status = std::process::Command::new("false").status();
        app.after_external(status);
        assert!(names(&app).contains(&"made-by-command".to_string()));
        assert!(
            app.message
                .as_deref()
                .unwrap()
                .starts_with("Command finished with")
        );
    }

    #[test]
    fn prompts_remember_history_by_kind() {
        let (_tmp, mut app) = sample();
        for spec in ["*.rs", "*.txt"] {
            press(&mut app, KeyCode::Char('f'));
            answer_prompt(&mut app, spec);
        }
        press(&mut app, KeyCode::Char('f'));
        ctrl(&mut app, 'u');
        press(&mut app, KeyCode::Up);
        assert_eq!(app.prompt.as_ref().unwrap().input.text, "*.txt");
        press(&mut app, KeyCode::Up);
        assert_eq!(app.prompt.as_ref().unwrap().input.text, "*.rs");
        press(&mut app, KeyCode::Esc);

        // Outro tipo de prompt tem histórico próprio.
        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Up);
        assert_eq!(app.prompt.as_ref().unwrap().input.text, "");
    }
}
