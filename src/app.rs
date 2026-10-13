//! Application state and command execution.
//!
//! The state of each window (tree, cursors, filespec) lives in [`Pane`];
//! what is global lives here: prompts, questions, ongoing operations and
//! the viewer. `App` gives direct access to the active window (`app.tree`,
//! `app.files`...) via `Deref`.

use std::collections::HashSet;
use std::fs;
use std::io::BufRead;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode};

use crate::archive::{self, ArchiveItem, ArchiveJob};
use crate::config::{self, Config, DateFormat, Options, PruneConfirm, Theme};
use crate::highlight;
use crate::info;
use crate::input::{Command, LineEvent, LineInput};
use crate::keymap::Keymap;
use crate::model::compare::{self, CompareMode};
use crate::model::filespec::{self, DateOrder, Filespec};
use crate::model::sort::{Sort, SortKey};
use crate::model::tree::{LogJob, NodeId, ROOT};
use crate::ops::{self, Answer, OpKind, Operation, Source};
pub use crate::pane::{FileRef, FileView, Focus, Kbyte, KbyteShow, Pane};
use crate::platform;
use crate::search::{self, Pattern, SearchJob, SearchSpec};
use crate::settings::{Outcome, Settings};
use crate::shell::{self, External};
use crate::state::{self, History};
use crate::viewer::{self, Action as ViewerAction, SearchFrom, Viewer};

/// Ongoing read and what to do when it finishes.
pub struct Job {
    pub log: LogJob,
    /// Window whose tree is being read.
    pane: usize,
    then: AfterLog,
}

enum AfterLog {
    Refresh,
    OpenBranch(NodeId),
    /// Opens the Branch and compares it with the other window.
    CompareBranch(NodeId, CompareMode),
    /// Asks for confirmation of the Prune of this directory.
    Prune(NodeId),
}

/// Which menu the footer shows: the normal one or the Ctrl/Alt commands one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MenuMode {
    #[default]
    Normal,
    Ctrl,
    Alt,
}

/// Text field open in the footer.
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
        /// Recreate the subdirectories from `base` at the destination.
        paths: bool,
        base: PathBuf,
    },
    RenameFile(PathBuf),
    RenameDir(NodeId),
    MakeDir(NodeId),
    Chmod(Vec<PathBuf>),
    Log,
    Execute,
    /// Ctrl+S: term to search for in the tagged files (F2 and F4 change the options).
    SearchTagged {
        spec: SearchSpec,
    },
    /// Search inside the viewer.
    ViewerSearch {
        spec: SearchSpec,
        forward: bool,
        from: SearchFrom,
    },
    /// Treespec: the tree cursor follows what is typed; Esc goes back
    /// to `origin`.
    Treespec {
        origin: NodeId,
    },
    /// New date: date and time for these paths.
    NewDate(Vec<PathBuf>),
    /// Batch: first the file to create, then the line mask.
    BatchFile,
    BatchMask {
        file: PathBuf,
    },
    /// F5: name of the archive to create from these files.
    CreateArchive(Vec<ArchiveItem>),
    /// Confirmation of a Prune: the typed text must be `word`.
    Prune {
        root: PathBuf,
        word: String,
        files: Vec<Source>,
        dirs: Vec<PathBuf>,
    },
}

impl PromptKind {
    /// Prompts of the same kind share history (↑/↓).
    fn history_key(&self) -> Option<&'static str> {
        Some(match self {
            PromptKind::Filespec => "filespec",
            PromptKind::Transfer { .. } => "destination",
            PromptKind::Log => "log",
            PromptKind::Treespec { .. } => "treespec",
            PromptKind::NewDate(_) => "date",
            PromptKind::BatchFile => "batch_file",
            PromptKind::BatchMask { .. } => "batch_mask",
            PromptKind::CreateArchive(_) => "archive",
            PromptKind::Execute => "execute",
            PromptKind::MakeDir(_) => "mkdir",
            PromptKind::SearchTagged { .. } | PromptKind::ViewerSearch { .. } => "search",
            PromptKind::Chmod(_) => "chmod",
            PromptKind::RenameFile(_) | PromptKind::RenameDir(_) | PromptKind::Prune { .. } => {
                return None;
            }
        })
    }
}

impl PromptKind {
    fn search_spec_mut(&mut self) -> Option<&mut SearchSpec> {
        match self {
            PromptKind::SearchTagged { spec } | PromptKind::ViewerSearch { spec, .. } => Some(spec),
            _ => None,
        }
    }

    fn search_label(&self) -> Option<String> {
        Some(match self {
            PromptKind::SearchTagged { spec } => {
                format!("SEARCH tagged files ({}): ", spec.mode_label())
            }
            PromptKind::ViewerSearch { spec, forward, .. } => {
                let dir = if *forward { "FIND" } else { "FIND backward" };
                format!("{dir} ({}): ", spec.mode_label())
            }
            _ => return None,
        })
    }
}

/// View of the tagged files (Ctrl+V): the files and which one is open.
pub struct TaggedView {
    pub items: Vec<(FileRef, PathBuf)>,
    pub index: usize,
}

/// Autoview reads only the start of each file, to stay fast.
const AUTOVIEW_BYTES: u64 = 1024 * 1024;

/// Entries kept per prompt kind.
const HISTORY_SIZE: usize = 100;

impl Prompt {
    /// The Copy/Move prompt for tagged files accepts F6 (with/without paths).
    pub fn toggles_paths(&self) -> bool {
        matches!(self.kind, PromptKind::Transfer { tagged: true, .. })
    }

    /// Search prompts accept F2 (case) and F4 (text/hex).
    pub fn is_search(&self) -> bool {
        matches!(
            self.kind,
            PromptKind::SearchTagged { .. } | PromptKind::ViewerSearch { .. }
        )
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

/// Footer question, answered with a key: the first letter of one of
/// the options. Any other key gives up.
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
    /// Alt+Q: quit and leave this directory for the shell.
    QuitChangeDir(PathBuf),
    DeleteFiles(Vec<Source>),
    DeleteDir(NodeId),
    /// `branch`: compares the whole branch, not just the directory.
    Compare {
        branch: bool,
    },
    /// Sort order being chosen (Order and Path first change this copy).
    Sort(Sort),
    /// Alt+K being chosen (All/Tagged and Branch/Directory change the copy).
    Kbyte(Kbyte),
}

const KBYTE_OPTIONS: &[&str] = &[
    "Size",
    "Count",
    "None",
    "All",
    "Tagged",
    "Branch",
    "Directory",
];

const SORT_OPTIONS: &[&str] = &["Name", "Ext", "Date", "Size", "Order", "Path first"];

pub struct App {
    /// One window, or two with the screen split (F8).
    pub panes: Vec<Pane>,
    pub active: usize,
    pub prompt: Option<Prompt>,
    pub confirm: Option<Confirm>,
    /// Message shown until the next key.
    pub message: Option<String>,
    pub job: Option<Job>,
    /// Ongoing Ctrl+S.
    pub search_job: Option<SearchJob<FileRef>>,
    /// Last search term (Ctrl+S and the viewer share it, so that
    /// Space in Ctrl+V continues the search done with Ctrl+S).
    pub search: Option<SearchSpec>,
    pub tagged_view: Option<TaggedView>,
    /// F7: the contents of the selected file next to the list.
    pub autoview: Option<Viewer>,
    /// F5: archive being created.
    pub archive_job: Option<ArchiveJob>,
    /// Ongoing Copy/Move/Delete.
    pub op: Option<Operation>,
    pub viewer: Option<Viewer>,
    /// Last destination typed in Copy/Move.
    last_dest: Option<String>,
    /// Directories that changed on disk and have not been re-read yet.
    pending_changes: HashSet<PathBuf>,
    /// Prompt history, per kind.
    pub history: History,
    /// Where to save the history on each new entry (no file in tests).
    pub history_file: Option<PathBuf>,
    /// External program (editor, command) to be run by the main loop,
    /// which hands the terminal over to it.
    pub external: Option<External>,
    /// Options from the configuration file.
    pub options: Options,
    /// The configuration in effect (the config screen starts from it).
    pub config: Config,
    /// Where to save the configuration (no file in tests).
    pub config_file: Option<PathBuf>,
    /// Error reading the file at startup.
    pub config_error: Option<String>,
    pub settings: Option<Settings>,
    /// Directory being deleted by an ongoing Prune.
    pruning: Option<PathBuf>,
    pub keymap: Keymap,
    pub should_quit: bool,
    /// Alt+Q: the directory for the shell to go to on exit.
    pub quit_dir: Option<PathBuf>,
    /// Ctrl/Alt currently pressed (only on terminals that report it).
    pub held: Option<MenuMode>,
    /// Menu chosen with F9; the next letter receives that modifier.
    pub sticky: MenuMode,
}

/// The active window.
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
            search_job: None,
            search: None,
            tagged_view: None,
            autoview: None,
            archive_job: None,
            op: None,
            viewer: None,
            last_dest: None,
            pending_changes: HashSet::new(),
            history: History::new(),
            history_file: None,
            external: None,
            options: Options::default(),
            config: Config::default(),
            config_file: None,
            config_error: None,
            settings: None,
            pruning: None,
            keymap: Keymap::default(),
            should_quit: false,
            quit_dir: None,
            held: None,
            sticky: MenuMode::Normal,
        }
    }

    pub fn is_split(&self) -> bool {
        self.panes.len() > 1
    }

    /// The other window, when the screen is split.
    pub fn other(&self) -> Option<&Pane> {
        self.is_split().then(|| &self.panes[1 - self.active])
    }

    /// Applies the configuration: options, keys, formats and theme. Used at
    /// startup and when the config screen saves.
    pub fn apply_config(&mut self, config: &Config) -> Result<(), String> {
        let keymap = Keymap::from_config(&config.keys, &config.options)?;
        let theme = Theme::from_config(config)?;
        self.keymap = keymap;
        highlight::set_xt_theme(&config.theme, theme.background);
        highlight::set_enabled(config.options.syntax_highlight);
        config::set_theme(theme);
        config::set_formats(config.options.formats());
        self.options = config.options.clone();
        self.config = config.clone();
        for pane in &mut self.panes {
            let hidden_changed = pane.tree.show_hidden != config.options.show_hidden;
            pane.tree.show_hidden = config.options.show_hidden;
            if hidden_changed {
                pane.after_change();
            }
        }
        Ok(())
    }

    /// F3 on the config screen: saves (keeping the file's comments)
    /// and applies immediately.
    fn save_settings(&mut self, config: Config) {
        if let Some(path) = &self.config_file
            && let Err(e) = config.save(path)
        {
            if let Some(s) = &mut self.settings {
                s.message = Some(format!("Cannot save: {e}"));
            }
            return;
        }
        match self.apply_config(&config) {
            Ok(()) => {
                self.settings = None;
                self.message = Some(match &self.config_file {
                    Some(path) => format!("Configuration saved to {}", path.display()),
                    None => "Configuration applied".into(),
                });
            }
            Err(e) => {
                if let Some(s) = &mut self.settings {
                    s.message = Some(format!("Cannot apply: {e}"));
                }
            }
        }
    }

    /// Order of dates typed in the Filespec: that of the configured format.
    fn date_order(&self) -> DateOrder {
        match self.options.date_format {
            DateFormat::Iso => DateOrder::Ymd,
            DateFormat::Dmy => DateOrder::Dmy,
            DateFormat::Mdy => DateOrder::Mdy,
        }
    }

    /// Directories to watch on disk: the current ones of each window (excluding
    /// archives).
    pub fn watched_dirs(&self) -> HashSet<PathBuf> {
        self.panes
            .iter()
            .filter(|p| p.archive.is_none())
            .map(Pane::current_path)
            .collect()
    }

    /// Disk change notices. They are applied as soon as nothing is
    /// in progress (at the end of a Copy, the directories are re-read
    /// anyway). Returns whether the screen changed.
    pub fn fs_changed(&mut self, dirs: impl IntoIterator<Item = PathBuf>) -> bool {
        self.pending_changes.extend(dirs);
        if self.pending_changes.is_empty() || self.busy() || self.op.is_some() {
            return false;
        }
        let dirs: Vec<PathBuf> = self.pending_changes.drain().collect();
        self.rescan_everywhere(&dirs);
        true
    }

    /// There is ongoing work that does not depend on the user.
    pub fn busy(&self) -> bool {
        self.job.is_some()
            || self.search_job.is_some()
            || self.archive_job.is_some()
            || self.op.as_ref().is_some_and(|op| op.conflict.is_none())
    }

    /// Menu to show in the footer.
    pub fn menu_mode(&self) -> MenuMode {
        self.held.unwrap_or(self.sticky)
    }

    pub fn handle_key(&mut self, mut key: KeyEvent) {
        // Ctrl/Alt alone only switch the menu while they are held down.
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
        // If the Ctrl/Alt release was lost (e.g. window switch), the
        // next key without it fixes the menu.
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
        if let Some(settings) = &mut self.settings {
            match settings.handle_key(key) {
                Outcome::Stay => {}
                Outcome::Close => self.settings = None,
                Outcome::Save(config) => self.save_settings(*config),
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
        if self.archive_job.is_some() {
            if key.code == KeyCode::Esc
                && let Some(job) = self.archive_job.take()
            {
                let dest = job.dest.clone();
                job.cancel();
                self.message = Some(format!("Cancelled: {} was not created", dest.display()));
            }
            return;
        }
        if let Some(job) = &self.search_job {
            if key.code == KeyCode::Esc {
                self.message = Some(format!(
                    "Search cancelled after {} of {} files",
                    job.done(),
                    job.total()
                ));
                self.search_job = None;
            }
            return;
        }
        if let Some(Prompt {
            kind: PromptKind::Treespec { origin },
            ..
        }) = &self.prompt
        {
            let origin = *origin;
            self.treespec_key(key, origin);
            return;
        }
        if let Some(prompt) = &mut self.prompt {
            if matches!(key.code, KeyCode::F(2) | KeyCode::F(4))
                && let Some(spec) = prompt.kind.search_spec_mut()
            {
                if key.code == KeyCode::F(2) {
                    spec.case_sensitive = !spec.case_sensitive;
                } else {
                    spec.hex = !spec.hex;
                }
                prompt.label = prompt.kind.search_label().unwrap_or_default();
                return;
            }
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
        if let Some(viewer) = &mut self.viewer {
            let action = viewer.handle_key(key);
            self.viewer_action(action);
            return;
        }
        if self.autoview.is_some() && self.autoview_key(key) {
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
        // With the Ctrl/Alt menu opened by F9, the next letter counts as
        // Ctrl+letter / Alt+letter. Useful where the terminal does not let the
        // combination through (Ctrl+M in Terminator, for example).
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
        // Shift+letter (and digits): jumps to the next name starting with
        // it, as in ZTreeWin.
        if let KeyCode::Char(c) = key.code
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && (c.is_uppercase() || c.is_ascii_digit())
        {
            self.spell_jump(c);
            return;
        }
        if let Some(cmd) = self.keymap.lookup(key, self.focus) {
            self.apply(cmd);
        }
        self.sync_autoview();
    }

    /// Autoview's own keys: F7, Esc and Enter exit; Shift+arrows scroll
    /// the content. Returns whether the key was used.
    fn autoview_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::F(7) | KeyCode::Esc | KeyCode::Enter => {
                self.autoview = None;
                true
            }
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                if let Some(v) = &mut self.autoview {
                    v.scroll(key.code);
                }
                true
            }
            _ => false,
        }
    }

    /// Keeps the Autoview showing the selected file.
    fn sync_autoview(&mut self) {
        let Some(current) = &self.autoview else {
            return;
        };
        let Some(f) = self.selected_file() else {
            return;
        };
        let path = self.file_path(f);
        if current.path == path {
            return;
        }
        let (page, width) = (current.page, current.width);
        let size = self.file(f).size;
        let mut v = self
            .load_viewer_limited(path.clone(), size, AUTOVIEW_BYTES)
            .unwrap_or_else(|e| {
                Viewer::from_bytes(path, 0, format!("Cannot view file: {e}").into_bytes())
            });
        (v.page, v.width) = (page, width);
        self.autoview = Some(v);
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
                | Command::Prune
                | Command::NewDate { .. }
                | Command::CreateArchive { .. }
        );
        if writes && self.archive.is_some() {
            self.message = Some("Archives are read-only (use Copy to extract)".into());
            return;
        }
        match cmd {
            Command::Quit if !self.options.confirm_quit => self.should_quit = true,
            Command::Quit => self.ask(ConfirmKind::Quit, "Quit xt?".into()),
            Command::QuitChangeDir => {
                // Inside a ZIP/TAR, the real directory is where it is located.
                let base = self.outer.as_deref().unwrap_or(self);
                let dir = base.current_path();
                if self.options.confirm_quit {
                    let q = format!("Quit xt and change to {}?", dir.display());
                    self.ask(ConfirmKind::QuitChangeDir(dir), q);
                } else {
                    self.quit_dir = Some(dir);
                    self.should_quit = true;
                }
            }
            Command::SwitchWindow => match self.focus {
                Focus::Tree => self.focus = Focus::Files,
                Focus::Files => self.back_to_tree(),
            },
            // Tab switches window when the screen is split; otherwise it does the same
            // as Enter.
            Command::SwitchPane if self.is_split() => self.active = 1 - self.active,
            Command::SwitchPane => self.apply(Command::SwitchWindow),
            Command::Split => self.toggle_split(),
            Command::Compare => self.ask_compare(false),
            Command::Prune => {
                let id = self.current();
                if id == ROOT {
                    self.message = Some("Cannot prune the root directory".into());
                } else {
                    // Log the whole branch first, to know everything that will be deleted.
                    self.start_job(LogJob::new(id, false), AfterLog::Prune(id));
                }
            }
            Command::Sort => {
                let sort = self.sort;
                self.ask_sort(sort);
            }
            Command::Log => {
                let current = self.current_path().to_string_lossy().into_owned();
                self.open_prompt(PromptKind::Log, "LOG directory: ".into(), &current);
            }
            Command::Back if in_files => self.back_to_tree(),
            Command::Back => self.close_archive(),
            Command::OpenArchive => self.open_archive(),
            Command::SearchTagged => {
                if self.sources_or_warn(true).is_some() {
                    let spec = self.search.clone().unwrap_or_else(|| SearchSpec::new(""));
                    let text = spec.text.clone();
                    let kind = PromptKind::SearchTagged { spec };
                    let label = kind.search_label().unwrap_or_default();
                    self.open_prompt(kind, label, &text);
                }
            }
            Command::ViewTagged => self.view_tagged(),
            Command::Treespec => self.start_treespec(),
            Command::FileDisplay => self.cycle_display(),
            Command::NewDate { tagged } => self.prompt_new_date(tagged),
            Command::CreateArchive { tagged } => self.prompt_create_archive(tagged),
            Command::Info => {
                let text = self.info_text();
                self.show_text("xt info", text);
            }
            Command::Avail => self.show_text("xt avail", info::avail()),
            Command::Stats => {
                let text = self.stats_text();
                self.show_text("xt statistics", text);
            }
            Command::Batch => {
                if self.sources_or_warn(true).is_some() {
                    let base = self.outer.as_deref().unwrap_or(self);
                    let default = base.current_path().join(shell::BATCH_FILE);
                    let default = default.to_string_lossy().into_owned();
                    let label = "BATCH: write to file: ".to_string();
                    self.open_prompt(PromptKind::BatchFile, label, &default);
                }
            }
            Command::Autoview => {
                // Opens on the files window, with an empty viewer that
                // `sync_autoview` replaces with the selected file.
                self.focus = Focus::Files;
                self.autoview = Some(Viewer::from_bytes(PathBuf::new(), 0, Vec::new()));
            }
            Command::TagBranch | Command::UntagBranch | Command::InvertBranch => {
                let refs = self.branch_matching_files(self.current());
                match cmd {
                    Command::TagBranch => self.set_tags(&refs, |_| true),
                    Command::UntagBranch => self.set_tags(&refs, |_| false),
                    _ => self.set_tags(&refs, |t| !t),
                }
                let tagged = refs.iter().filter(|&&f| self.file(f).tagged).count();
                self.message = Some(format!(
                    "{tagged} of {} logged files in the branch are tagged",
                    refs.len()
                ));
            }
            Command::Kbyte => {
                let k = self.kbyte;
                self.ask_kbyte(k);
            }
            Command::OpenWith { tagged } => self.open_with(tagged),
            Command::CompareBranch => self.ask_compare(true),
            Command::Parent => {
                let cur = self.current();
                if self.view == FileView::Dir
                    && let Some(parent) = self.tree.node(cur).parent
                {
                    // On the files window, stays there, showing the parent.
                    self.select_node(parent);
                }
            }
            Command::Settings => {
                self.settings = Some(Settings::new(
                    self.config.clone(),
                    self.config_error.clone(),
                ));
            }
            Command::Help => {
                let text = self.keymap.help_text().into_bytes();
                let size = text.len() as u64;
                self.viewer = Some(Viewer::from_bytes("xt help".into(), size, text));
            }
            Command::Edit => {
                if let Some(f) = self.selected_file() {
                    let file = self.file_path(f);
                    let cwd = self.current_path();
                    self.external = Some(shell::editor(&file, &cwd, &self.options.editor));
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

    /// Advances the ongoing work by at most `budget`.
    pub fn tick(&mut self, budget: Duration) {
        if let Some(job) = &mut self.archive_job {
            job.step(budget);
            if job.is_done() {
                let job = self.archive_job.take().unwrap();
                let (count, errors) = (job.total() - job.errors.len(), job.errors.clone());
                self.message = Some(match job.finish() {
                    Ok(dest) => {
                        if let Some(dir) = dest.parent() {
                            self.rescan_everywhere(&[dir]);
                        }
                        let mut m = format!("Archived {count} files into {}", dest.display());
                        if let Some(e) = errors.first() {
                            m.push_str(&format!(". {} errors, first: {e}", errors.len()));
                        }
                        m
                    }
                    Err(e) => format!("Cannot create archive: {e}"),
                });
            }
            return;
        }
        if let Some(job) = &mut self.search_job {
            let pane = &mut self.panes[self.active];
            for (f, found) in job.step(budget) {
                if !found {
                    pane.tree.file_mut(f.dir, f.idx).tagged = false;
                }
            }
            if job.is_done() {
                self.message = Some(job.summary());
                self.search_job = None;
            }
            return;
        }
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
            AfterLog::Prune(id) => {
                pane.refresh_files();
                self.job = None;
                self.prepare_prune(id);
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
        // Small branches finish immediately, without flashing the progress message.
        self.tick(Duration::from_millis(30));
    }

    fn start_op(&mut self, op: Operation) {
        self.op = Some(op);
        self.tick(Duration::from_millis(30));
    }

    /// Builds the Prune list and asks for the typed confirmation. Other mount
    /// points inside the branch are left out (so a mounted USB drive
    /// inside it is not deleted by mistake).
    fn prepare_prune(&mut self, id: NodeId) {
        let root = self.tree.path(id);
        let root_dev = match fs::symlink_metadata(&root) {
            Ok(m) => platform::device(&root, &m),
            Err(e) => {
                self.message = Some(format!("Cannot prune {}: {e}", root.display()));
                return;
            }
        };
        let (mut files, mut dirs) = (Vec::new(), Vec::new());
        let (mut mounts, mut unreadable) = (0, 0);
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            let path = self.tree.path(n);
            match fs::symlink_metadata(&path) {
                Ok(m) if platform::device(&path, &m) == root_dev => {}
                Ok(_) => {
                    mounts += 1;
                    continue;
                }
                Err(_) => continue,
            }
            let node = self.tree.node(n);
            unreadable += usize::from(node.error.is_some());
            for f in &node.files {
                files.push(Source {
                    path: path.join(&f.name),
                    size: f.size,
                });
            }
            stack.extend(node.children.iter().copied());
            dirs.push(path);
        }

        let name = fit_name(&root);
        let word = match self.options.prune_confirmation {
            PruneConfirm::Prune => "PRUNE".to_string(),
            PruneConfirm::Name => name.clone(),
            PruneConfirm::Y => "Y".to_string(),
        };
        let bytes: u64 = files.iter().map(|f| f.size).sum();
        let mut warnings = String::new();
        if mounts > 0 {
            warnings.push_str(&format!(", skipping {mounts} mount point(s)"));
        }
        if unreadable > 0 {
            warnings.push_str(&format!(", {unreadable} unreadable dir(s)"));
        }
        let label = format!(
            "PRUNE {name}: {} files, {} dirs, {} bytes{warnings}. Type {word} to confirm: ",
            files.len(),
            dirs.len(),
            bytes
        );
        let kind = PromptKind::Prune {
            root,
            word,
            files,
            dirs,
        };
        self.open_prompt(kind, label, "");
    }

    /// End (or cancellation) of an operation: re-reads the affected directories.
    fn finish_op(&mut self, prefix: &str) {
        let Some(op) = self.op.take() else { return };
        self.rescan_everywhere(&op.touched_dirs());
        // After a Prune, the cursor goes to the parent directory.
        if let Some(root) = self.pruning.take()
            && let Some(parent) = root.parent()
        {
            self.goto(parent);
        }
        self.message = Some(format!("{prefix}{}", op.summary()));
    }

    /// Re-reads `dirs` in all windows (both may show the same place).
    fn rescan_everywhere<P: AsRef<Path>>(&mut self, dirs: &[P]) {
        for pane in &mut self.panes {
            pane.rescan_paths(dirs);
        }
    }

    fn ask_kbyte(&mut self, k: Kbyte) {
        let files = if k.tagged_only {
            "tagged files"
        } else {
            "all files"
        };
        let scope = if k.branch { "branch" } else { "directory" };
        let q = format!("KBYTE: show in the tree ({files}, per {scope}):");
        self.ask_options(ConfirmKind::Kbyte(k), q, KBYTE_OPTIONS);
    }

    fn ask_sort(&mut self, sort: Sort) {
        let order = if sort.descending {
            "descending"
        } else {
            "ascending"
        };
        let path = if sort.path_first { ", path first" } else { "" };
        let q = format!("SORT files ({order}{path}) by:");
        self.ask_options(ConfirmKind::Sort(sort), q, SORT_OPTIONS);
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

    /// Tags, in the active window's list, the files that have the requested
    /// relation to their counterpart in the other window's directory. In Branch
    /// and Showall, the counterpart follows the path relative to the list root
    /// (`src/a.rs` → `<other>/src/a.rs`).
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
            let tolerance = compare::Tolerance {
                two_seconds: self.options.compare_ignore_2s,
                one_hour: self.options.compare_ignore_1h,
            };
            let tag = mode.matches(compare::relation(
                entry.size,
                entry.modified,
                target.as_ref(),
                tolerance,
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

    /// F8: splits the screen into two windows, or goes back to one. The new window
    /// starts at the same root and directory as the current one.
    fn toggle_split(&mut self) {
        if self.is_split() {
            let keep = self.panes.swap_remove(self.active);
            self.panes = vec![keep];
            self.active = 0;
            return;
        }
        // Inside an archive, the new window shows the disk.
        let base = self.outer.as_deref().unwrap_or(self);
        let mut pane = Pane::new(base.tree.path(ROOT));
        pane.sort = self.sort;
        pane.display = self.display;
        pane.goto(&base.current_path());
        self.panes.push(pane);
    }

    /// `O`: opens the ZIP/TAR under the cursor as if it were a directory.
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

    /// `\`: opens the Treespec prompt with the current path. In Branch and
    /// Showall, goes straight to the selected file's directory.
    fn start_treespec(&mut self) {
        if self.view != FileView::Dir {
            if let Some(f) = self.selected_file() {
                let name = self.file(f).name.clone();
                self.view = FileView::Dir;
                self.show_node(f.dir);
                self.refresh_files();
                if let Some(i) = self.files.iter().position(|&r| self.file(r).name == name) {
                    self.file_state.select(Some(i));
                }
            }
            return;
        }
        let mut text = self.current_path().to_string_lossy().into_owned();
        if !text.ends_with(std::path::is_separator) {
            text.push(std::path::MAIN_SEPARATOR);
        }
        let origin = self.current();
        self.open_prompt(PromptKind::Treespec { origin }, "TREESPEC: ".into(), &text);
    }

    /// Keys in the Treespec prompt: `/` or `\` complete and go down one
    /// level, Tab/Shift+Tab complete and then switch siblings, Esc goes back.
    fn treespec_key(&mut self, key: KeyEvent, origin: NodeId) {
        let Some(prompt) = &mut self.prompt else {
            return;
        };
        let text = prompt.input.text.clone();
        let new_text = match key.code {
            KeyCode::Char('/' | '\\') => {
                let mut t = self.treespec_complete(&text).unwrap_or(text.clone());
                if !t.ends_with(std::path::is_separator) {
                    t.push(std::path::MAIN_SEPARATOR);
                }
                Some(t)
            }
            KeyCode::Tab | KeyCode::BackTab => {
                let forward = key.code == KeyCode::Tab;
                match self.treespec_complete(&text) {
                    Some(t) if t != text => Some(t),
                    _ => self.treespec_sibling(&text, forward),
                }
            }
            _ => None,
        };
        let Some(prompt) = &mut self.prompt else {
            return;
        };
        let text = match new_text {
            Some(t) => {
                prompt.input = LineInput::new(&t).with_history(prompt.input.history());
                t
            }
            None => match prompt.input.handle_key(key) {
                LineEvent::Submit => {
                    let prompt = self.prompt.take().unwrap();
                    self.submit(prompt.kind, prompt.input.text);
                    return;
                }
                LineEvent::Cancel => {
                    self.prompt = None;
                    self.show_node(origin);
                    return;
                }
                LineEvent::Editing => prompt.input.text.clone(),
            },
        };
        if let Some(id) = self.treespec_target(&text) {
            self.show_node(id);
        }
    }

    /// Shows a text in the viewer (Info, Avail, statistics).
    fn show_text(&mut self, title: &str, text: String) {
        let data = text.into_bytes();
        self.tagged_view = None;
        self.viewer = Some(Viewer::from_bytes(title.into(), data.len() as u64, data));
    }

    /// Alt+I: the file under the cursor or, in the tree, the directory (with what
    /// is logged in it and in its branch).
    fn info_text(&self) -> String {
        let in_files = self.focus == Focus::Files;
        match self.selected_file().filter(|_| in_files) {
            Some(f) if self.archive.is_none() => info::path_info(&self.file_path(f)),
            Some(f) => {
                let e = self.file(f);
                format!(
                    "INFO  {} (inside an archive)\n\n  Size          {} bytes\n  Permissions   {:04o}\n",
                    self.file_path(f).display(),
                    info::group(e.size),
                    e.mode
                )
            }
            None => {
                let id = self.current();
                let mut text = if self.archive.is_some() {
                    format!(
                        "INFO  {} (inside an archive)\n",
                        self.current_path().display()
                    )
                } else {
                    info::path_info(&self.current_path())
                };
                let dir = self.tree.dir_totals(false, false)[id];
                let branch = self.tree.dir_totals(true, false)[id];
                let subdirs = self.tree.visible_subtree(id).len() - 1;
                text.push_str(&format!(
                    "\nLOGGED\n  {:<14}{} files, {} bytes\n  {:<14}{} files, {} bytes, {} subdirectories\n",
                    "Directory",
                    info::group(dir.0),
                    info::group(dir.1),
                    "Branch",
                    info::group(branch.0),
                    info::group(branch.1),
                    info::group(subdirs as u64)
                ));
                text
            }
        }
    }

    /// ?: statistics of the active window.
    fn stats_text(&self) -> String {
        let g = info::group;
        let logged = self.tree.logged_totals();
        let listed: u64 = self.files.iter().map(|&f| self.file(f).size).sum();
        let (tagged, tagged_bytes) = self
            .files
            .iter()
            .map(|&f| self.file(f))
            .filter(|f| f.tagged)
            .fold((0u64, 0u64), |(n, b), f| (n + 1, b + f.size));
        let view = match self.view {
            FileView::Dir => "directory",
            FileView::Branch(_) => "branch",
            FileView::Showall => "showall",
        };
        let mut text = format!("STATISTICS  {}\n\n", self.tree.path(ROOT).display());
        let mut row = |k: &str, v: String| text.push_str(&format!("  {k:<14}{v}\n"));
        row(
            "Logged",
            format!(
                "{} dirs, {} files, {} bytes",
                g(logged.dirs),
                g(logged.files),
                g(logged.bytes)
            ),
        );
        row("Filespec", self.filespec.text().to_string());
        row("Sort", self.sort.label());
        row(
            "Listing",
            format!(
                "{view}: {} files, {} bytes",
                g(self.files.len() as u64),
                g(listed)
            ),
        );
        row(
            "Tagged",
            format!("{} files, {} bytes", g(tagged), g(tagged_bytes)),
        );
        if let Some(disk) = platform::disk_space(&self.current_path()) {
            row(
                "Disk",
                format!("{} bytes free of {}", g(disk.available), g(disk.total)),
            );
        }
        text
    }

    /// F5 / Ctrl+F5: asks for the archive name. Inside an archive, paths
    /// are relative to the list root (as in Alt+C).
    fn prompt_create_archive(&mut self, tagged: bool) {
        let Some(sources) = self.sources_or_warn(tagged) else {
            return;
        };
        let base = self.tree.path(self.view_root());
        let items: Vec<ArchiveItem> = sources
            .iter()
            .map(|s| ArchiveItem {
                name: archive::entry_name(
                    s.path
                        .strip_prefix(&base)
                        .unwrap_or(Path::new(s.path.file_name().unwrap_or_default())),
                ),
                path: s.path.clone(),
                size: s.size,
            })
            .collect();
        let stem = match sources.as_slice() {
            [one] => fit_name(&one.path),
            _ => fit_name(&base),
        };
        let default = self.current_path().join(format!("{stem}.tar.gz"));
        let label = format!("ARCHIVE {} to (.zip, .tar, .tar.gz): ", describe(&sources));
        self.open_prompt(
            PromptKind::CreateArchive(items),
            label,
            &default.to_string_lossy(),
        );
    }

    /// Writes the Batch file: one line per tagged file. A script (`.sh`, or
    /// `.bat`/`.cmd` on Windows) gets its header and execute permission.
    fn write_batch(&mut self, file: &Path, mask: &str) -> Result<(), String> {
        if mask.trim().is_empty() {
            return Err("Batch cancelled: empty line mask".into());
        }
        let sources = self.sources(true);
        let header = shell::batch_header(file);
        let script = header.is_some();
        let mut data = header.unwrap_or_default().to_vec();
        for s in &sources {
            data.extend(shell::batch_line(mask, &s.path, s.size));
            data.extend_from_slice(shell::NEWLINE);
        }
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file)
            .map_err(|e| format!("Cannot create {}: {e}", file.display()))?;
        std::io::Write::write_all(&mut out, &data).map_err(|e| e.to_string())?;
        if script {
            let _ = platform::set_file_mode(&out, 0o755);
        }
        if let Some(dir) = file.parent() {
            self.rescan_everywhere(&[dir]);
        }
        self.message = Some(format!(
            "Wrote {} lines to {}",
            sources.len(),
            file.display()
        ));
        Ok(())
    }

    /// N: asks for the new date, prefilled with the current one (of the file or directory).
    fn prompt_new_date(&mut self, tagged: bool) {
        let paths: Vec<PathBuf> = if self.focus == Focus::Tree && !tagged {
            vec![self.current_path()]
        } else {
            match self.sources_or_warn(tagged) {
                Some(s) => s.into_iter().map(|s| s.path).collect(),
                None => return,
            }
        };
        let modified = fs::metadata(&paths[0]).and_then(|m| m.modified()).ok();
        let pattern = self.options.date_format.pattern();
        let current = modified
            .map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format(pattern)
                    .to_string()
            })
            .unwrap_or_default();
        let what = match paths.as_slice() {
            [one] => fit_name(one),
            many => format!("{} tagged files", many.len()),
        };
        let label = format!("NEW DATE for {what} (date [HH:MM], now, today): ");
        self.open_prompt(PromptKind::NewDate(paths), label, &current);
    }

    /// Applies the typed date. Without a time, each file keeps its own.
    fn new_date(&mut self, paths: &[PathBuf], text: &str) -> Result<(), String> {
        let (date, time) = filespec::parse_date_time(text, self.date_order())
            .ok_or_else(|| format!("Invalid date: {text}"))?;
        let mut errors = Vec::new();
        let mut changed = 0;
        for path in paths {
            let result = (|| -> Result<(), String> {
                let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
                if meta.is_symlink() {
                    return Err("symbolic link skipped (the date would change its target)".into());
                }
                let old = chrono::DateTime::<chrono::Local>::from(
                    meta.modified().map_err(|e| e.to_string())?,
                );
                let when = date
                    .and_time(time.unwrap_or(old.time()))
                    .and_local_timezone(chrono::Local)
                    .earliest()
                    .ok_or("time does not exist (daylight saving change)")?;
                platform::set_modified(path, when.into()).map_err(|e| e.to_string())
            })();
            match result {
                Ok(()) => changed += 1,
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        let dirs: Vec<&Path> = paths.iter().filter_map(|p| p.parent()).collect();
        self.rescan_everywhere(&dirs);
        let mut msg = format!("New date set on {changed} of {}", paths.len());
        if let Some(e) = errors.first() {
            msg.push_str(&format!(". Error: {e}"));
        }
        self.message = Some(msg);
        Ok(())
    }

    /// `O`: opens the file (or the tagged files) in the system's default program.
    /// Inside an archive, extracts first to a temporary folder
    /// (deleted when xt exits).
    fn open_with(&mut self, tagged: bool) {
        let Some(sources) = self.sources_or_warn(tagged) else {
            return;
        };
        let mut errors = Vec::new();
        let mut opened = 0;
        for source in sources {
            let path = match &self.archive {
                Some(root) => match extract_for_opening(root, &source.path) {
                    Ok(p) => p,
                    Err(e) => {
                        errors.push(format!("{}: {e}", source.path.display()));
                        continue;
                    }
                },
                None => source.path,
            };
            match shell::open_detached(&path, &self.options.opener) {
                Ok(()) => opened += 1,
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        self.message = Some(match errors.first() {
            None => format!("Opened {opened} file(s)"),
            Some(e) => format!("Opened {opened} file(s). Error: {e}"),
        });
    }

    /// Esc in an archive's tree: goes back to where it was.
    fn close_archive(&mut self) {
        if let Some(outer) = self.outer.take() {
            *self.deref_mut() = *outer;
            // The disk may have changed while the archive was open.
            let current = self.current();
            self.tree.rescan(current);
            self.after_change();
        }
    }

    /// Log: opens `path` in the active window. If it is inside the current tree,
    /// just moves the cursor there; otherwise, the window gets `path` as its root.
    fn log(&mut self, text: &str) -> Result<(), String> {
        let path = ops::resolve_path(text, &self.current_path());
        let path = dunce::canonicalize(&path).map_err(|e| format!("Cannot log {text}: {e}"))?;
        if !path.is_dir() {
            return Err(format!("{} is not a directory", path.display()));
        }
        if !self.goto(&path) {
            let (sort, display) = (self.sort, self.display);
            *self.deref_mut() = Pane::new(path);
            self.display = display;
            self.set_sort(sort);
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
        // Leading "@": do not save (e.g. command with a password).
        if self.options.history_skip_at && text.trim_start().starts_with('@') {
            return;
        }
        let list = self.history.entry(key.to_string()).or_default();
        list.retain(|t| t != text);
        list.push(text.to_string());
        if list.len() > HISTORY_SIZE {
            list.remove(0);
        }
        if let Some(path) = &self.history_file
            && let Err(e) = state::save_history(path, &self.history)
        {
            self.message = Some(format!("Cannot save history to {}: {e}", path.display()));
        }
    }

    /// Back from an external program: the disk may have changed.
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
            PromptKind::Filespec => Filespec::parse_with(&text, self.date_order()).map(|spec| {
                self.filespec = spec;
                self.refresh_files();
            }),
            PromptKind::Log => self.log(&text),
            PromptKind::NewDate(paths) => self.new_date(&paths, &text),
            PromptKind::BatchFile => {
                let base = self.outer.as_deref().unwrap_or(self).current_path();
                let file = ops::resolve_path(&text, &base);
                if fs::symlink_metadata(&file).is_ok() {
                    Err(format!("{} already exists", file.display()))
                } else {
                    let label = format!(
                        "BATCH line per tagged file (%f %d %n %b %e %s; uppercase = unquoted) into {}: ",
                        fit_name(&file)
                    );
                    self.open_prompt(PromptKind::BatchMask { file }, label, "");
                    Ok(())
                }
            }
            PromptKind::BatchMask { file } => self.write_batch(&file, &text),
            PromptKind::CreateArchive(items) => {
                let dest = ops::resolve_path(&text, &self.current_path());
                if fs::symlink_metadata(&dest).is_ok() {
                    Err(format!("{} already exists", dest.display()))
                } else {
                    // The archive itself is not included in it.
                    let items = items.into_iter().filter(|i| i.path != dest).collect();
                    ArchiveJob::new(&dest, items)
                        .map(|job| {
                            self.archive_job = Some(job);
                            self.tick(Duration::from_millis(30));
                        })
                        .map_err(|e| format!("Cannot create archive: {e}"))
                }
            }
            // Navigation already happened while typing; outside the tree,
            // it becomes a Log.
            PromptKind::Treespec { .. } => match self.treespec_target(&text) {
                Some(_) => Ok(()),
                None => self.log(&text),
            },
            PromptKind::SearchTagged { mut spec } => {
                spec.text = text;
                self.start_search(spec)
            }
            PromptKind::ViewerSearch {
                mut spec,
                forward,
                from,
            } => {
                spec.text = text;
                self.viewer_search(spec, forward, Some(from));
                Ok(())
            }
            PromptKind::Prune {
                root,
                word,
                files,
                dirs,
            } => {
                if text.trim().eq_ignore_ascii_case(&word) {
                    self.pruning = Some(root);
                    self.start_op(Operation::prune(files, dirs));
                    Ok(())
                } else {
                    Err("Prune cancelled: confirmation did not match".into())
                }
            }
            PromptKind::Execute => {
                // The "@" (do not save in history) is not passed to the shell.
                let text = text.trim_start().strip_prefix('@').unwrap_or(&text);
                if !text.trim().is_empty() {
                    let file = self.selected_file().filter(|_| self.focus == Focus::Files);
                    let file = file.map(|f| self.file_path(f));
                    let tagged: Vec<PathBuf> =
                        self.sources(true).into_iter().map(|s| s.path).collect();
                    let dir = self.current_path();
                    let line = shell::expand(text, file.as_deref(), &tagged, &dir);
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
                // Saves the absolute path: a relative one would change meaning
                // next time, from another directory.
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
                // Not a compare mode: it was "Branch on/off", so ask again.
                None => self.ask_compare(!branch),
                Some(mode) if branch => {
                    let id = self.current();
                    self.start_job(LogJob::new(id, false), AfterLog::CompareBranch(id, mode));
                }
                Some(mode) => self.compare(mode),
            },
            ConfirmKind::Kbyte(mut k) => {
                match choice {
                    's' => k.show = KbyteShow::Size,
                    'c' => k.show = KbyteShow::Count,
                    'n' => k.show = KbyteShow::None,
                    other => {
                        match other {
                            'a' => k.tagged_only = false,
                            't' => k.tagged_only = true,
                            'b' => k.branch = true,
                            _ => k.branch = false,
                        }
                        return self.ask_kbyte(k);
                    }
                }
                self.kbyte = k;
            }
            ConfirmKind::Sort(mut sort) => {
                sort.key = match choice {
                    'o' => {
                        sort.descending = !sort.descending;
                        return self.ask_sort(sort);
                    }
                    'p' => {
                        sort.path_first = !sort.path_first;
                        return self.ask_sort(sort);
                    }
                    'n' => SortKey::Name,
                    'e' => SortKey::Ext,
                    'd' => SortKey::Date,
                    _ => SortKey::Size,
                };
                self.set_sort(sort);
            }
            _ if !yes => {}
            ConfirmKind::Quit => self.should_quit = true,
            ConfirmKind::QuitChangeDir(dir) => {
                self.quit_dir = Some(dir);
                self.should_quit = true;
            }
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

    /// `sources`, warning when there is nothing to operate on.
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
        // With the screen split, the natural destination is the other window.
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
            .map(|m| platform::mode_text(platform::mode(&m)))
            .unwrap_or_default();
        let label = platform::mode_prompt(&describe(&sources));
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
        // Keep the cursor on the renamed file.
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

    /// Permissions (Linux) or attributes (Windows, where `+R -A` changes
    /// each file's own attributes).
    fn chmod(&mut self, paths: &[PathBuf], text: &str) -> Result<(), String> {
        platform::parse_mode(text, 0)?;
        let mut errors = Vec::new();
        let mut changed = 0;
        for path in paths {
            let meta = match fs::symlink_metadata(path) {
                // Symbolic links have no permissions of their own on Linux.
                Ok(m) if m.is_symlink() => continue,
                Ok(m) => m,
                Err(e) => {
                    errors.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };
            let mode = platform::parse_mode(text, platform::mode(&meta))?;
            match platform::set_mode(path, mode) {
                Ok(()) => changed += 1,
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        let mut dirs: Vec<&Path> = paths.iter().filter_map(|p| p.parent()).collect();
        dirs.dedup();
        self.rescan_everywhere(&dirs);
        let what = platform::describe_mode(0).0;
        let mut msg = format!("{what} set to {} on {changed} files", text.trim());
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
        self.tagged_view = None;
        match self.load_viewer(self.file_path(f), self.file(f).size) {
            Ok(v) => self.viewer = Some(v),
            Err(e) => self.message = Some(format!("Cannot view file: {e}")),
        }
    }

    /// Opens `path` in the viewer (from inside the ZIP/TAR, if applicable).
    fn load_viewer(&self, path: PathBuf, size: u64) -> std::io::Result<Viewer> {
        self.load_viewer_limited(path, size, viewer::MAX_BYTES)
    }

    fn load_viewer_limited(&self, path: PathBuf, size: u64, limit: u64) -> std::io::Result<Viewer> {
        match &self.archive {
            Some(root) => {
                let entry = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                let mut data = Vec::new();
                archive::Reader::open(root)?.extract(&entry, &mut data, limit)?;
                Ok(Viewer::from_bytes(path, size, data))
            }
            None => Viewer::open_limited(path, limit),
        }
    }

    /// Reads a file's contents for the search: from disk, or from inside the
    /// open ZIP/TAR.
    fn content_opener(&self) -> search::Opener {
        match self.archive.clone() {
            Some(root) => Box::new(move |path: &Path| {
                let entry = path.strip_prefix(&root).unwrap_or(path).to_path_buf();
                let mut data = Vec::new();
                archive::Reader::open(&root)?.extract(&entry, &mut data, u64::MAX)?;
                Ok(Box::new(std::io::Cursor::new(data)) as Box<dyn BufRead>)
            }),
            None => Box::new(search::open_file),
        }
    }

    /// The list's tagged files, with their paths.
    fn tagged_items(&self) -> Vec<(FileRef, PathBuf)> {
        self.files
            .iter()
            .copied()
            .filter(|&f| self.file(f).tagged)
            .map(|f| (f, self.file_path(f)))
            .collect()
    }

    /// Ctrl+S: searches the tagged files, bit by bit, untagging those without
    /// the term.
    fn start_search(&mut self, spec: SearchSpec) -> Result<(), String> {
        let job = SearchJob::new(spec.clone(), self.tagged_items(), self.content_opener())?;
        self.search = Some(spec);
        self.search_job = Some(job);
        self.tick(Duration::from_millis(30));
        Ok(())
    }

    /// Ctrl+V: opens the first tagged file; N/P go through the others.
    fn view_tagged(&mut self) {
        let items = self.tagged_items();
        if items.is_empty() {
            self.message = Some("No tagged files".into());
            return;
        }
        self.tagged_view = Some(TaggedView { items, index: 0 });
        if !self.show_tagged(0) {
            self.tagged_view = None;
        }
    }

    /// Shows tagged file `index` in the viewer, keeping the screen size.
    fn show_tagged(&mut self, index: usize) -> bool {
        let Some((f, path)) = self
            .tagged_view
            .as_ref()
            .and_then(|tv| tv.items.get(index).cloned())
        else {
            return false;
        };
        let size = self.tree.node(f.dir).files.get(f.idx).map_or(0, |e| e.size);
        match self.load_viewer(path, size) {
            Ok(mut v) => {
                if let Some(old) = &self.viewer {
                    (v.page, v.width) = (old.page, old.width);
                }
                self.viewer = Some(v);
                if let Some(tv) = &mut self.tagged_view {
                    tv.index = index;
                }
                true
            }
            Err(e) => {
                let msg = format!("Cannot view file: {e}");
                match &mut self.viewer {
                    Some(v) => v.note = Some(msg),
                    None => self.message = Some(msg),
                }
                false
            }
        }
    }

    fn viewer_action(&mut self, action: ViewerAction) {
        let position = self
            .tagged_view
            .as_ref()
            .map(|tv| (tv.index, tv.items.len()));
        match action {
            ViewerAction::None => {}
            ViewerAction::Close => {
                self.viewer = None;
                self.tagged_view = None;
            }
            ViewerAction::Next | ViewerAction::Prev | ViewerAction::First | ViewerAction::Last => {
                let Some((i, n)) = position else {
                    return;
                };
                let target = match action {
                    ViewerAction::Next => (i + 1).min(n - 1),
                    ViewerAction::Prev => i.saturating_sub(1),
                    ViewerAction::First => 0,
                    _ => n - 1,
                };
                if target != i {
                    self.show_tagged(target);
                } else if let Some(v) = &mut self.viewer {
                    let edge = if target == 0 { "First" } else { "Last" };
                    v.note = Some(format!("{edge} tagged file"));
                }
            }
            ViewerAction::Untag => {
                let Some(f) = self
                    .tagged_view
                    .as_ref()
                    .and_then(|tv| tv.items.get(tv.index))
                    .map(|(f, _)| *f)
                else {
                    return;
                };
                self.tree.file_mut(f.dir, f.idx).tagged = false;
                if let Some(v) = &mut self.viewer {
                    v.note = Some("Untagged".into());
                }
            }
            ViewerAction::Search { forward, from } => {
                let spec = self.search.clone().unwrap_or_else(|| SearchSpec::new(""));
                let text = spec.text.clone();
                let kind = PromptKind::ViewerSearch {
                    spec,
                    forward,
                    from,
                };
                let label = kind.search_label().unwrap_or_default();
                self.open_prompt(kind, label, &text);
            }
            ViewerAction::Continue { forward } => {
                let dir = forward.unwrap_or_else(|| self.viewer.as_ref().is_none_or(|v| v.forward));
                match self.search.clone() {
                    Some(spec) => self.viewer_search(spec, dir, None),
                    None => self.viewer_action(ViewerAction::Search {
                        forward: dir,
                        from: SearchFrom::Page,
                    }),
                }
            }
        }
    }

    /// Searches the open file; in Ctrl+V, if there is no more in it,
    /// moves on to the next tagged files (or the previous ones, going backwards).
    fn viewer_search(&mut self, spec: SearchSpec, forward: bool, from: Option<SearchFrom>) {
        let pattern = match Pattern::new(&spec) {
            Ok(p) => p,
            Err(e) => {
                if let Some(v) = &mut self.viewer {
                    v.note = Some(e);
                }
                return;
            }
        };
        let text = spec.text.clone();
        self.search = Some(spec);
        let Some(viewer) = &mut self.viewer else {
            return;
        };
        let found = match from {
            Some(from) => viewer.search(&pattern, forward, from),
            None => viewer.search_next(&pattern, forward),
        };
        if found {
            return;
        }
        if let Some(tv) = &self.tagged_view {
            let candidates: Vec<(usize, PathBuf)> = if forward {
                (tv.index + 1..tv.items.len())
                    .map(|i| (i, tv.items[i].1.clone()))
                    .collect()
            } else {
                (0..tv.index)
                    .rev()
                    .map(|i| (i, tv.items[i].1.clone()))
                    .collect()
            };
            let open = self.content_opener();
            for (i, path) in candidates {
                let has = open(&path)
                    .and_then(|r| pattern.matches_reader(r))
                    .unwrap_or(false);
                if has && self.show_tagged(i) {
                    if let Some(v) = &mut self.viewer {
                        v.search(&pattern, forward, SearchFrom::Edge);
                    }
                    return;
                }
            }
        }
        if let Some(v) = &mut self.viewer {
            v.note = Some(format!("\"{text}\" not found"));
        }
    }
}

/// xt's temporary folder for opening files from inside ZIP/TAR.
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir().join(format!("xt-{}", std::process::id()))
}

/// Extracts an entry of `archive` (virtual path `entry`) to the
/// temporary folder, so an external program can open it.
fn extract_for_opening(archive: &Path, entry: &Path) -> std::io::Result<PathBuf> {
    let rel = entry.strip_prefix(archive).unwrap_or(entry);
    let dir = temp_dir();
    fs::create_dir_all(&dir)?;
    let out = dir.join(rel.file_name().unwrap_or_default());
    let mut file = fs::File::create(&out)?;
    archive::Reader::open(archive)?.extract(rel, &mut file, u64::MAX)?;
    Ok(out)
}

/// Last component of a path, for short messages.
fn fit_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// "file.txt" or "3 tagged files".
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
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    /// root/{a.rs, b.txt}, root/sub/{c.rs}, root/sub/deep/{a.rs}
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

        // Invalid pattern: keeps the previous one and shows the error.
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
        // "deep" has not been logged yet.
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
        app.apply(Command::Down); // second a.rs, in sub/deep
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

    /// Answers an open prompt, replacing the suggested text.
    fn answer_prompt(app: &mut App, text: &str) {
        assert!(app.prompt.is_some(), "prompt should be open");
        ctrl(app, 'u');
        type_text(app, text);
        press(app, KeyCode::Enter);
        // Finishes the operation, if any.
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

        // In the tree, T tags the whole current directory.
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
        // The tree already shows the new files in sub/.
        let sub = app.tree.find(&dest).unwrap();
        assert_eq!(app.tree.node(sub).files.len(), 3);
        // Tags remain at the source.
        assert_eq!(tagged(&app).len(), 2);
    }

    #[test]
    fn move_with_conflict_prompt() {
        let (tmp, mut app) = sample();
        write(tmp.path().join("sub/b.txt"), "old").unwrap();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char('m'));
        answer_prompt(&mut app, "sub"); // relative to the current directory
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

        // Cursor on "new" (order: new, sub).
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

        // A directory with contents is not deleted.
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char('d'));
        assert!(app.confirm.is_none());
        assert_eq!(app.message.as_deref(), Some("Directory is not empty"));
    }

    #[cfg(unix)]
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

    /// Tags the two a.rs of the root's Branch (root and sub/deep).
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
        // The destination shows up in the tree and, when expanded, the created folders.
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

        // Two a.rs going to the same place: the second is a conflict.
        assert!(app.op.as_ref().unwrap().conflict.is_some());
        press(&mut app, KeyCode::Char('n'));
        assert!(dest.join("a.rs").exists());
        assert!(!dest.join("sub").exists());
    }

    #[test]
    fn alt_move_with_paths_from_branch_root() {
        let (tmp, mut app) = sample();
        // Branch from sub/: paths relative to sub.
        press(&mut app, KeyCode::Down);
        app.apply(Command::Branch);
        ctrl(&mut app, 't');
        let dest = tmp.path().join("out");
        std::fs::create_dir(&dest).unwrap();
        alt(&mut app, 'm');
        // Relative to the tree cursor's directory (sub), not the file's.
        answer_prompt(&mut app, "../out");
        // The next prompt, from another directory, suggests the already-resolved destination.
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
        // Lost release: the next key without Alt goes back to normal.
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

        // F9 + T = Ctrl+T (tags the whole list); then back to normal.
        press(&mut app, KeyCode::F(9));
        press(&mut app, KeyCode::Char('t'));
        assert_eq!(tagged(&app).len(), 2);
        assert_eq!(app.menu_mode(), MenuMode::Normal);

        // F9 + M = Ctrl+M: move tagged files without paths.
        press(&mut app, KeyCode::F(9));
        press(&mut app, KeyCode::Char('m'));
        let label = &app.prompt.as_ref().unwrap().label;
        assert!(
            label.starts_with("MOVE 2 tagged files (no paths)"),
            "{label}"
        );
        press(&mut app, KeyCode::Esc);

        // F9 F9 + C = Alt+C: copy with paths.
        press(&mut app, KeyCode::F(9));
        press(&mut app, KeyCode::F(9));
        press(&mut app, KeyCode::Char('c'));
        assert!(app.prompt.as_ref().unwrap().label.contains("with paths"));
        press(&mut app, KeyCode::Esc);

        // Esc gives up on the menu without doing anything.
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
        // The new window starts in the same directory.
        assert_eq!(app.panes[1].current_path(), tmp.path().join("sub"));

        press(&mut app, KeyCode::Tab);
        assert_eq!(app.active, 1);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.current_path(), tmp.path());
        assert_eq!(app.panes[0].current_path(), tmp.path().join("sub"));

        // F8 again leaves only the active window.
        press(&mut app, KeyCode::F(8));
        assert!(!app.is_split());
        assert_eq!(app.current_path(), tmp.path());
        // Without a split, Tab goes back to toggling tree/files.
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.focus, Focus::Files);
    }

    #[test]
    fn copy_defaults_to_other_pane_and_refreshes_both() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::F(8));
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Down); // window 2 in sub/
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
        // The other window already shows the new file.
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
        let (tmp, _) = sample();
        // As in `main`: the root is canonical (on Windows, the temporary
        // directory comes with a short 8.3 name).
        let root = dunce::canonicalize(tmp.path()).unwrap();
        let mut app = App::new(root.clone());
        // Inside the current tree: just moves the cursor.
        press(&mut app, KeyCode::Char('l'));
        answer_prompt(&mut app, "sub/deep");
        assert_eq!(app.current_path(), root.join("sub/deep"));
        assert_eq!(app.tree.path(ROOT), root);

        // Outside it: the window gets a new root.
        let other = tempfile::tempdir().unwrap();
        press(&mut app, KeyCode::Char('l'));
        answer_prompt(&mut app, other.path().to_str().unwrap());
        let canonical = dunce::canonicalize(other.path()).unwrap();
        assert_eq!(app.tree.path(ROOT), canonical);

        press(&mut app, KeyCode::Char('l'));
        answer_prompt(&mut app, "/caminho/que/nao/existe");
        assert!(app.message.as_deref().unwrap().starts_with("Cannot log"));
    }

    /// Two windows: the active one in `left/`, the other in `right/`.
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
        // Opens the Branch; sub/deep.txt does not exist in right/sub/: it is unique.
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

    /// Root with a p.zip (src/a.rs, README) next to a.rs and b.txt.
    fn archive_setup() -> (tempfile::TempDir, App) {
        let (tmp, _) = sample();
        crate::archive::tests::make_zip(&tmp.path().join("p.zip"));
        let mut app = App::new(tmp.path().to_path_buf());
        press(&mut app, KeyCode::Enter);
        let i = names(&app).iter().position(|n| n == "p.zip").unwrap();
        app.file_state.select(Some(i));
        (tmp, app)
    }

    fn alt_f5(app: &mut App) {
        app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::ALT));
    }

    #[test]
    fn opens_browses_and_closes_archives() {
        let (tmp, mut app) = archive_setup();
        alt_f5(&mut app);
        assert_eq!(
            app.archive.as_deref(),
            Some(tmp.path().join("p.zip").as_path())
        );
        assert_eq!(app.focus, Focus::Tree);
        assert_eq!(names(&app), ["README"]);

        // View an entry without extracting anything to disk.
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(app.viewer.as_ref().unwrap().visible_lines(1)[0], "leia-me");
        press(&mut app, KeyCode::Esc);

        // Writing is forbidden inside the archive.
        press(&mut app, KeyCode::Char('d'));
        assert!(app.confirm.is_none());
        assert!(app.message.as_deref().unwrap().contains("read-only"));

        // Esc in the tree goes back exactly to where it was.
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Esc);
        assert!(app.archive.is_none());
        assert_eq!(app.current_path(), tmp.path());
    }

    #[test]
    fn copies_out_of_archives() {
        let (tmp, mut app) = archive_setup();
        alt_f5(&mut app);
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
        alt_f5(&mut app);
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

        // With an operation stopped on a conflict, waits.
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

    #[cfg(unix)]
    #[test]
    fn edit_and_execute_request_external_programs() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('e'));
        let ext = app.external.take().expect("editor requested");
        let quoted = format!(" '{}'", tmp.path().join("a.rs").display());
        assert!(ext.line.to_string_lossy().ends_with(&quoted));

        press(&mut app, KeyCode::Char('t'));
        press(&mut app, KeyCode::Char('t'));
        press(&mut app, KeyCode::Char('x'));
        type_text(&mut app, "wc -c %t");
        press(&mut app, KeyCode::Enter);
        let ext = app.external.take().expect("command requested");
        let a = tmp.path().join("a.rs");
        let b = tmp.path().join("b.txt");
        let expected = format!("wc -c '{}' '{}'", a.display(), b.display());
        assert_eq!(ext.line, expected.as_str());
        assert_eq!(ext.cwd, tmp.path());
        assert!(ext.wait);

        // Going back re-reads the disk and shows failures.
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

        // Another kind of prompt has its own history.
        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Up);
        assert_eq!(app.prompt.as_ref().unwrap().input.text, "");
    }

    #[test]
    fn quit_without_confirmation_when_configured() {
        let (_tmp, mut app) = sample();
        app.options.confirm_quit = false;
        press(&mut app, KeyCode::Char('q'));
        assert!(app.should_quit);
        assert!(app.confirm.is_none());
    }

    #[test]
    fn f1_shows_help_in_viewer() {
        let (_tmp, mut app) = sample();
        press(&mut app, KeyCode::F(1));
        let viewer = app.viewer.as_ref().expect("help open");
        assert!(viewer.visible_lines(1)[0].starts_with("xt - keys in use"));
        press(&mut app, KeyCode::Esc);
        assert!(app.viewer.is_none());
    }

    #[test]
    fn history_is_saved_after_each_entry() {
        let (tmp, mut app) = sample();
        let file = tmp.path().join("state/history.toml");
        app.history_file = Some(file.clone());
        press(&mut app, KeyCode::Char('f'));
        answer_prompt(&mut app, "*.rs");
        assert_eq!(state::load_history(&file)["filespec"], ["*.rs"]);

        // A new run starts with the saved history.
        let mut again = App::new(tmp.path().to_path_buf());
        again.history = state::load_history(&file);
        press(&mut again, KeyCode::Char('f'));
        press(&mut again, KeyCode::Up);
        assert_eq!(again.prompt.as_ref().unwrap().input.text, "*.rs");
    }

    #[test]
    fn alt_s_sorts_keeping_the_cursor() {
        let (tmp, mut app) = sample();
        write(tmp.path().join("big.bin"), vec![0u8; 100]).unwrap();
        app.apply(Command::Relog);
        press(&mut app, KeyCode::Enter);
        assert_eq!(names(&app), ["a.rs", "b.txt", "big.bin"]);
        press(&mut app, KeyCode::Down); // b.txt
        let selected = |app: &App| app.file(app.selected_file().unwrap()).name.clone();
        let before = selected(&app);

        alt(&mut app, 's');
        press(&mut app, KeyCode::Char('o')); // descending: asks again
        assert!(
            app.confirm
                .as_ref()
                .unwrap()
                .question
                .contains("descending")
        );
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.sort.label(), "Size↓");
        assert_eq!(names(&app), ["big.bin", "a.rs", "b.txt"]);
        assert_eq!(selected(&app), before, "cursor stays on the same file");

        // The F8 new window inherits the sort order.
        press(&mut app, KeyCode::F(8));
        assert_eq!(app.panes[1].sort, app.panes[0].sort);
    }

    #[test]
    fn prune_needs_typed_confirmation() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Down); // sub (with sub/deep/a.rs)
        alt(&mut app, 'p');
        let label = app.prompt.as_ref().expect("asks to type").label.clone();
        assert!(label.starts_with("PRUNE sub: 2 files, 2 dirs"), "{label}");

        // Wrong text: nothing is deleted.
        answer_prompt(&mut app, "yes");
        assert!(tmp.path().join("sub/deep/a.rs").exists());
        assert!(
            app.message
                .as_deref()
                .unwrap()
                .starts_with("Prune cancelled")
        );

        alt(&mut app, 'p');
        answer_prompt(&mut app, "prune");
        assert!(!tmp.path().join("sub").exists());
        assert!(app.message.as_deref().unwrap().starts_with("Pruned 4 of 4"));
        assert_eq!(app.current(), ROOT, "cursor goes to the parent");
        assert!(
            tmp.path().join("a.rs").exists(),
            "outside the branch is untouched"
        );
    }

    #[test]
    fn prune_refuses_root_and_can_use_the_name() {
        let (tmp, mut app) = sample();
        alt(&mut app, 'p');
        assert!(app.prompt.is_none());
        assert_eq!(
            app.message.as_deref(),
            Some("Cannot prune the root directory")
        );

        app.options.prune_confirmation = PruneConfirm::Name;
        press(&mut app, KeyCode::Down);
        alt(&mut app, 'p');
        assert!(
            app.prompt
                .as_ref()
                .unwrap()
                .label
                .contains("Type sub to confirm")
        );
        answer_prompt(&mut app, "SUB");
        assert!(!tmp.path().join("sub").exists());
    }

    #[test]
    fn hidden_files_option() {
        let (tmp, mut app) = sample();
        write(tmp.path().join(".env"), "").unwrap();
        create_dir_all(tmp.path().join(".git")).unwrap();
        app.apply(Command::Relog);
        assert!(names(&app).contains(&".env".to_string()));

        let config = Config {
            options: Options {
                show_hidden: false,
                ..Options::default()
            },
            ..Config::default()
        };
        app.apply_config(&config).unwrap();
        assert_eq!(names(&app), ["a.rs", "b.txt"]);
        assert!(!app.rows.iter().any(|r| app.tree.node(r.id).name == ".git"));
    }

    #[cfg(unix)]
    #[test]
    fn at_prefix_skips_history_and_is_stripped() {
        let (_tmp, mut app) = sample();
        app.options.history_skip_at = true;
        press(&mut app, KeyCode::Char('x'));
        type_text(&mut app, "@echo secret");
        press(&mut app, KeyCode::Enter);
        let ext = app.external.take().unwrap();
        assert_eq!(ext.line, "echo secret");
        assert!(!app.history.contains_key("execute"));
    }

    #[test]
    fn f12_settings_save_and_apply() {
        let (tmp, mut app) = sample();
        let file = tmp.path().join("cfg/config.toml");
        app.config_file = Some(file.clone());
        write(tmp.path().join(".hidden"), "").unwrap();
        app.apply(Command::Relog);

        press(&mut app, KeyCode::F(12));
        assert!(app.settings.is_some());
        press(&mut app, KeyCode::PageDown);
        press(&mut app, KeyCode::Char('a')); // hide hidden files
        press(&mut app, KeyCode::F(3));
        assert!(app.settings.is_none());
        assert!(!app.options.show_hidden);
        assert!(
            !names(&app).contains(&".hidden".to_string()),
            "applied at once"
        );
        let (saved, err) = Config::load_from(&file);
        assert!(err.is_none());
        assert!(!saved.options.show_hidden);
    }

    #[test]
    fn settings_esc_discards() {
        let (_tmp, mut app) = sample();
        press(&mut app, KeyCode::F(12));
        press(&mut app, KeyCode::Char('b'));
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Esc);
        assert!(app.settings.is_none());
        assert_eq!(app.options, Options::default());
    }

    /// A fake "default program" that copies the received file to `out/`.
    fn fake_opener(tmp: &Path) -> (String, PathBuf) {
        let out = tmp.join("opened");
        fs::create_dir_all(&out).unwrap();
        (format!("cp -t {}", out.display()), out)
    }

    fn wait_for(path: &Path) -> bool {
        (0..100).any(|_| {
            std::thread::sleep(Duration::from_millis(20));
            path.exists()
        })
    }

    #[test]
    fn o_opens_with_the_default_program() {
        let (tmp, mut app) = sample();
        let (opener, out) = fake_opener(tmp.path());
        app.options.opener = opener;
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('o'));
        assert_eq!(app.message.as_deref(), Some("Opened 1 file(s)"));
        assert!(wait_for(&out.join("a.rs")));

        ctrl(&mut app, 't');
        ctrl(&mut app, 'o');
        assert_eq!(app.message.as_deref(), Some("Opened 2 file(s)"));
        assert!(wait_for(&out.join("b.txt")));
    }

    #[test]
    fn o_inside_archive_extracts_first() {
        let (tmp, mut app) = archive_setup();
        let (opener, out) = fake_opener(tmp.path());
        app.options.opener = opener;
        alt_f5(&mut app);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('o'));
        assert!(wait_for(&out.join("README")));
        assert_eq!(fs::read_to_string(out.join("README")).unwrap(), "leia-me");
    }

    #[test]
    fn alt_c_in_tree_compares_the_branch() {
        let (_tmp, mut app) = compare_setup();
        alt(&mut app, 'c');
        let q = &app.confirm.as_ref().expect("compare question").question;
        assert!(q.starts_with("COMPARE branch"), "{q}");
    }

    #[test]
    fn backspace_goes_to_parent_in_both_windows() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.current(), ROOT);

        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.current_path(), tmp.path().join("sub"));
        press(&mut app, KeyCode::Backspace);
        assert_eq!(app.current(), ROOT);
        assert_eq!(app.focus, Focus::Files);
        assert_eq!(names(&app), ["a.rs", "b.txt"]);
    }

    /// Three files; "senha" appears in two of them, twice in the first.
    fn search_setup() -> (tempfile::TempDir, App) {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path().join("1.txt"),
            "linha\na SENHA aqui\noutra\nsenha de novo\n",
        )
        .unwrap();
        write(tmp.path().join("2.txt"), "nada a ver\n").unwrap();
        write(tmp.path().join("3.txt"), "\n\n\nsó a senha no fim\n").unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        press(&mut app, KeyCode::Enter);
        ctrl(&mut app, 't');
        (tmp, app)
    }

    fn viewer_space(app: &mut App) -> (String, Option<usize>) {
        press(app, KeyCode::Char(' '));
        let v = app.viewer.as_ref().unwrap();
        let name = v.path.file_name().unwrap().to_string_lossy().into_owned();
        (name, v.hit.map(|h| h.line))
    }

    #[test]
    fn ctrl_s_then_ctrl_v_then_space_walks_the_hits() {
        let (_tmp, mut app) = search_setup();
        ctrl(&mut app, 's');
        let label = app.prompt.as_ref().unwrap().label.clone();
        assert_eq!(label, "SEARCH tagged files (text, ignore case): ");
        type_text(&mut app, "senha");
        press(&mut app, KeyCode::Enter);
        while app.busy() {
            app.tick(Duration::from_millis(50));
        }
        assert_eq!(tagged(&app), ["1.txt", "3.txt"], "2.txt was untagged");
        assert!(
            app.message
                .as_deref()
                .unwrap()
                .starts_with("2 of 3 tagged files contain")
        );

        // Ctrl+V and Space: uses the Ctrl+S term and goes across the files.
        ctrl(&mut app, 'v');
        assert_eq!(app.tagged_view.as_ref().unwrap().items.len(), 2);
        assert_eq!(viewer_space(&mut app), ("1.txt".into(), Some(1)));
        assert_eq!(viewer_space(&mut app), ("1.txt".into(), Some(3)));
        assert_eq!(viewer_space(&mut app), ("3.txt".into(), Some(3)));
        let (_, hit) = viewer_space(&mut app);
        assert_eq!(hit, Some(3), "no more hits: stays");
        assert!(
            app.viewer
                .as_ref()
                .unwrap()
                .note
                .as_deref()
                .unwrap()
                .contains("not found")
        );

        // Backwards goes back to the previous file.
        press(&mut app, KeyCode::Char('-'));
        let v = app.viewer.as_ref().unwrap();
        assert_eq!(v.path.file_name().unwrap(), "1.txt");
        assert_eq!(v.hit.map(|h| h.line), Some(3));
    }

    #[test]
    fn ctrl_v_next_prev_and_untag() {
        let (_tmp, mut app) = search_setup();
        ctrl(&mut app, 'v');
        let current = |app: &App| {
            let v = app.viewer.as_ref().unwrap();
            v.path.file_name().unwrap().to_string_lossy().into_owned()
        };
        assert_eq!(current(&app), "1.txt");
        press(&mut app, KeyCode::Char('n'));
        assert_eq!(current(&app), "2.txt");
        press(&mut app, KeyCode::Char('u'));
        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::ALT));
        assert_eq!(current(&app), "3.txt");
        press(&mut app, KeyCode::Char('n'));
        assert_eq!(
            app.viewer.as_ref().unwrap().note.as_deref(),
            Some("Last tagged file")
        );
        press(&mut app, KeyCode::Esc);
        assert!(app.viewer.is_none() && app.tagged_view.is_none());
        assert_eq!(tagged(&app), ["1.txt", "3.txt"]);
    }

    #[test]
    fn viewer_find_prompt_and_options() {
        let (_tmp, mut app) = search_setup();
        press(&mut app, KeyCode::Char('v')); // only 1.txt
        press(&mut app, KeyCode::Char('f'));
        assert!(
            app.prompt
                .as_ref()
                .unwrap()
                .label
                .starts_with("FIND (text, ignore case)")
        );
        press(&mut app, KeyCode::F(2));
        assert!(app.prompt.as_ref().unwrap().label.contains("match case"));
        type_text(&mut app, "SENHA");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.viewer.as_ref().unwrap().hit.map(|h| h.line), Some(1));
        // Case-sensitive, the "senha" on line 3 does not count.
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(app.viewer.as_ref().unwrap().hit.map(|h| h.line), Some(1));

        // The term and options come back on the next Ctrl+S.
        press(&mut app, KeyCode::Esc);
        ctrl(&mut app, 's');
        let prompt = app.prompt.as_ref().unwrap();
        assert_eq!(prompt.input.text, "SENHA");
        assert!(prompt.label.contains("match case"));
    }

    #[test]
    fn search_can_be_cancelled_and_needs_tags() {
        let (_tmp, mut app) = search_setup();
        ctrl(&mut app, 'u');
        ctrl(&mut app, 's');
        assert!(app.prompt.is_none());
        assert_eq!(app.message.as_deref(), Some("No tagged files"));
        ctrl(&mut app, 'v');
        assert!(app.viewer.is_none());
    }

    #[test]
    fn searches_inside_archives() {
        let (_tmp, mut app) = archive_setup();
        alt_f5(&mut app);
        app.apply(Command::Branch);
        ctrl(&mut app, 't');
        ctrl(&mut app, 's');
        type_text(&mut app, "fn a");
        press(&mut app, KeyCode::Enter);
        while app.busy() {
            app.tick(Duration::from_millis(50));
        }
        assert_eq!(tagged(&app), ["a.rs"]);
    }

    fn treespec_setup() -> (tempfile::TempDir, App, String) {
        let tmp = tempfile::tempdir().unwrap();
        for d in ["Docs/rust/src", "Docs/notes", "downloads"] {
            create_dir_all(tmp.path().join(d)).unwrap();
        }
        write(tmp.path().join("Docs/rust/src/main.rs"), "").unwrap();
        let app = App::new(tmp.path().to_path_buf());
        let root = tmp.path().to_string_lossy().into_owned();
        (tmp, app, root)
    }

    fn current_name(app: &App) -> String {
        app.tree
            .node(app.current())
            .name
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn treespec_navigates_while_typing() {
        let (_tmp, mut app, root) = treespec_setup();
        press(&mut app, KeyCode::Char('\\'));
        assert_eq!(app.prompt.as_ref().unwrap().input.text, format!("{root}/"));
        type_text(&mut app, "do");
        assert_eq!(current_name(&app), "Docs", "cursor follows the typing");
        type_text(&mut app, "/");
        assert_eq!(
            app.prompt.as_ref().unwrap().input.text,
            format!("{root}/Docs/")
        );
        type_text(&mut app, "r\\s");
        assert_eq!(
            app.prompt.as_ref().unwrap().input.text,
            format!("{root}/Docs/rust/s")
        );
        assert_eq!(current_name(&app), "src");
        press(&mut app, KeyCode::Enter);
        assert!(app.prompt.is_none());
        assert_eq!(current_name(&app), "src");
        assert_eq!(names(&app), ["main.rs"]);
    }

    #[test]
    fn treespec_tab_cycles_and_esc_returns() {
        let (_tmp, mut app, root) = treespec_setup();
        press(&mut app, KeyCode::Char('\\'));
        type_text(&mut app, "do");
        press(&mut app, KeyCode::Tab);
        assert_eq!(
            app.prompt.as_ref().unwrap().input.text,
            format!("{root}/Docs")
        );
        press(&mut app, KeyCode::Tab);
        assert_eq!(current_name(&app), "downloads");
        press(&mut app, KeyCode::BackTab);
        assert_eq!(current_name(&app), "Docs");
        press(&mut app, KeyCode::Esc);
        assert!(app.prompt.is_none());
        assert_eq!(app.current(), ROOT, "back to where it started");
    }

    #[test]
    fn treespec_outside_the_tree_logs() {
        let (_tmp, mut app, _root) = treespec_setup();
        let other = tempfile::tempdir().unwrap();
        press(&mut app, KeyCode::Char('\\'));
        ctrl(&mut app, 'u');
        type_text(&mut app, other.path().to_str().unwrap());
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.tree.path(ROOT),
            dunce::canonicalize(other.path()).unwrap()
        );
    }

    #[test]
    fn treespec_in_branch_goes_to_the_file() {
        let (_tmp, mut app, _root) = treespec_setup();
        app.apply(Command::Branch);
        assert_eq!(names(&app), ["main.rs"]);
        press(&mut app, KeyCode::Char('\\'));
        assert!(app.prompt.is_none());
        assert_eq!(app.view, FileView::Dir);
        assert_eq!(current_name(&app), "src");
        assert_eq!(app.file(app.selected_file().unwrap()).name, "main.rs");
    }

    #[test]
    fn shift_letter_jumps_instead_of_running_commands() {
        let (_tmp, mut app, _root) = treespec_setup();
        app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
        assert_eq!(current_name(&app), "Docs");
        app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
        assert_eq!(current_name(&app), "downloads");
        // Lowercase is still a command (d = Delete).
        press(&mut app, KeyCode::Char('d'));
        assert!(app.confirm.is_some());
    }

    #[test]
    fn alt_k_chooses_what_to_show() {
        let (_tmp, mut app) = sample();
        alt(&mut app, 'k');
        press(&mut app, KeyCode::Char('b'));
        let q = &app.confirm.as_ref().expect("asks again").question;
        assert!(q.contains("per branch"), "{q}");
        press(&mut app, KeyCode::Char('t'));
        press(&mut app, KeyCode::Char('c'));
        assert!(app.confirm.is_none());
        assert_eq!(
            app.kbyte,
            Kbyte {
                show: KbyteShow::Count,
                tagged_only: true,
                branch: true
            }
        );
        alt(&mut app, 'k');
        press(&mut app, KeyCode::Char('n'));
        assert_eq!(app.kbyte.show, KbyteShow::None);
    }

    #[test]
    fn alt_q_quits_remembering_the_directory() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Down);
        alt(&mut app, 'q');
        assert!(
            app.confirm
                .as_ref()
                .unwrap()
                .question
                .starts_with("Quit xt and change to")
        );
        press(&mut app, KeyCode::Char('y'));
        assert!(app.should_quit);
        assert_eq!(
            app.quit_dir.as_deref(),
            Some(tmp.path().join("sub").as_path())
        );

        // Normal Q does not change the directory.
        let (_tmp, mut app) = sample();
        press(&mut app, KeyCode::Char('q'));
        press(&mut app, KeyCode::Char('y'));
        assert!(app.should_quit && app.quit_dir.is_none());
    }

    #[test]
    fn filespec_with_size_and_date_in_configured_order() {
        let (tmp, mut app) = sample();
        write(tmp.path().join("big.bin"), vec![0u8; 5000]).unwrap();
        app.apply(Command::Relog);
        press(&mut app, KeyCode::Char('f'));
        answer_prompt(&mut app, ">s4K");
        assert_eq!(names(&app), ["big.bin"]);

        app.options.date_format = DateFormat::Dmy;
        let today = chrono::Local::now().format("%d/%m/%Y").to_string();
        press(&mut app, KeyCode::Char('f'));
        answer_prompt(&mut app, &format!("*.rs,={today}"));
        assert_eq!(names(&app), ["a.rs"]);
        press(&mut app, KeyCode::Char('f'));
        answer_prompt(&mut app, "<TODAY");
        assert!(names(&app).is_empty());
    }

    #[test]
    fn ctrl_f7_f8_f9_tag_the_branch() {
        let (_tmp, mut app) = sample();
        app.apply(Command::ExpandAll);
        let fkey =
            |app: &mut App, n| app.handle_key(KeyEvent::new(KeyCode::F(n), KeyModifiers::CONTROL));
        let all_tagged = |app: &App| -> usize {
            app.logged_matching_files()
                .iter()
                .filter(|&&f| app.file(f).tagged)
                .count()
        };
        press(&mut app, KeyCode::Down); // sub
        fkey(&mut app, 7);
        assert_eq!(all_tagged(&app), 2, "sub/c.rs and sub/deep/a.rs");
        assert!(app.message.as_deref().unwrap().starts_with("2 of 2"));
        press(&mut app, KeyCode::Up);
        fkey(&mut app, 9);
        assert_eq!(
            all_tagged(&app),
            2,
            "root's 2 files now tagged, sub's untagged"
        );
        fkey(&mut app, 8);
        assert_eq!(all_tagged(&app), 0);
    }

    #[test]
    fn f7_autoview_follows_the_cursor() {
        let (_tmp, mut app) = sample();
        press(&mut app, KeyCode::F(7));
        assert_eq!(app.focus, Focus::Files);
        let shown = |app: &App| app.autoview.as_ref().unwrap().visible_lines(1)[0].clone();
        assert_eq!(shown(&app), "fn main() {}");
        press(&mut app, KeyCode::Down);
        assert_eq!(shown(&app), "b");
        // The files window commands remain valid.
        press(&mut app, KeyCode::Char('t'));
        assert_eq!(tagged(&app), ["b.txt"]);
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT));
        assert!(app.autoview.is_some());
        press(&mut app, KeyCode::F(7));
        assert!(app.autoview.is_none());
        assert_eq!(app.focus, Focus::Files);
    }

    #[test]
    fn n_sets_new_dates() {
        use chrono::{Datelike, Timelike};
        let (tmp, mut app) = sample();
        app.options.date_format = DateFormat::Dmy;
        let mtime = |p: &str| {
            chrono::DateTime::<chrono::Local>::from(
                fs::metadata(tmp.path().join(p))
                    .unwrap()
                    .modified()
                    .unwrap(),
            )
        };
        let before = mtime("a.rs");

        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('n'));
        let prompt = app.prompt.as_ref().unwrap();
        assert!(prompt.label.starts_with("NEW DATE for a.rs"));
        assert_eq!(
            prompt.input.text,
            before.format("%d/%m/%Y %H:%M").to_string()
        );
        answer_prompt(&mut app, "01/02/2020 13:45");
        let after = mtime("a.rs");
        assert_eq!((after.year(), after.month(), after.day()), (2020, 2, 1));
        assert_eq!((after.hour(), after.minute()), (13, 45));

        // Tagged files, date only: each keeps its time.
        ctrl(&mut app, 't');
        let b_time = mtime("b.txt").time();
        ctrl(&mut app, 'n');
        answer_prompt(&mut app, "2019-03-04");
        assert_eq!(mtime("b.txt").date_naive().to_string(), "2019-03-04");
        assert_eq!(mtime("b.txt").time().minute(), b_time.minute());
        assert!(
            app.message
                .as_deref()
                .unwrap()
                .starts_with("New date set on 2 of 2")
        );

        press(&mut app, KeyCode::Char('n'));
        answer_prompt(&mut app, "ontem");
        assert!(app.message.as_deref().unwrap().starts_with("Invalid date"));

        // In the tree, the directory's date.
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Char('n'));
        answer_prompt(&mut app, "2018-01-01");
        assert_eq!(mtime("sub").date_naive().to_string(), "2018-01-01");
    }

    #[cfg(unix)]
    #[test]
    fn ctrl_b_writes_a_batch_file() {
        let (tmp, mut app) = sample();
        press(&mut app, KeyCode::Enter);
        ctrl(&mut app, 't');
        ctrl(&mut app, 'b');
        assert_eq!(
            app.prompt.as_ref().unwrap().input.text,
            tmp.path().join("xt-batch.sh").to_str().unwrap()
        );
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, "wc -c %f");
        press(&mut app, KeyCode::Enter);
        let script = tmp.path().join("xt-batch.sh");
        let text = fs::read_to_string(&script).unwrap();
        let a = tmp.path().join("a.rs");
        assert!(text.starts_with("#!/bin/sh\n"));
        assert!(text.contains(&format!("wc -c '{}'\n", a.display())));
        assert_eq!(
            fs::metadata(&script).unwrap().permissions().mode() & 0o111,
            0o111
        );
        assert!(
            names(&app).contains(&"xt-batch.sh".to_string()),
            "list refreshed"
        );

        // Data list, and does not overwrite.
        ctrl(&mut app, 'b');
        answer_prompt(&mut app, "lista.csv");
        type_text(&mut app, "%N;%S");
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            fs::read_to_string(tmp.path().join("lista.csv")).unwrap(),
            "a.rs;13\nb.txt;1\n"
        );
        ctrl(&mut app, 'b');
        answer_prompt(&mut app, "lista.csv");
        assert!(app.message.as_deref().unwrap().ends_with("already exists"));
    }

    #[test]
    fn f5_creates_archives_with_relative_paths() {
        let (tmp, mut app) = sample();
        app.apply(Command::Branch);
        ctrl(&mut app, 't');
        app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::CONTROL));
        let prompt = app.prompt.as_ref().unwrap();
        let root_name = tmp
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(prompt.input.text.ends_with(&format!("{root_name}.tar.gz")));
        answer_prompt(&mut app, "pack.zip");
        assert!(
            app.message
                .as_deref()
                .unwrap()
                .starts_with("Archived 4 files"),
            "{:?}",
            app.message
        );

        let mut r = archive::Reader::open(&tmp.path().join("pack.zip")).unwrap();
        let mut names: Vec<String> = r
            .list()
            .unwrap()
            .iter()
            .map(|e| archive::entry_name(&e.path))
            .collect();
        names.sort();
        assert_eq!(names, ["a.rs", "b.txt", "sub/c.rs", "sub/deep/a.rs"]);

        // Does not overwrite.
        app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::CONTROL));
        answer_prompt(&mut app, "pack.zip");
        assert!(app.message.as_deref().unwrap().ends_with("already exists"));
    }

    #[test]
    fn info_avail_and_stats_screens() {
        let (_tmp, mut app) = sample();
        let first_line = |app: &App| app.viewer.as_ref().unwrap().visible_lines(1)[0].clone();

        alt(&mut app, 'i');
        assert!(first_line(&app).starts_with("INFO "));
        let text = app.viewer.as_ref().unwrap().visible_lines(40).join("\n");
        assert!(
            text.contains("directory") && text.contains("LOGGED"),
            "{text}"
        );
        press(&mut app, KeyCode::Esc);

        press(&mut app, KeyCode::Enter);
        alt(&mut app, 'i');
        let text = app.viewer.as_ref().unwrap().visible_lines(40).join("\n");
        assert!(text.contains("a.rs") && text.contains("13 bytes"), "{text}");
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Esc);

        press(&mut app, KeyCode::Char('a'));
        assert!(first_line(&app).starts_with("AVAIL"));
        press(&mut app, KeyCode::Esc);

        press(&mut app, KeyCode::Char('?'));
        let text = app.viewer.as_ref().unwrap().visible_lines(20).join("\n");
        assert!(text.contains("Logged") && text.contains("Tagged"), "{text}");
    }

    #[test]
    fn slash_also_opens_treespec() {
        let (_tmp, mut app, root) = treespec_setup();
        press(&mut app, KeyCode::Char('/'));
        assert_eq!(app.prompt.as_ref().unwrap().input.text, format!("{root}/"));
        type_text(&mut app, "dow");
        assert_eq!(current_name(&app), "downloads");
        press(&mut app, KeyCode::Esc);
        // "?" still shows the statistics ("/" in the viewer still searches text).
        press(&mut app, KeyCode::Char('?'));
        assert!(app.viewer.is_some(), "? still shows statistics");
    }
}
