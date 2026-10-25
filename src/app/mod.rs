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
use crate::config::{self, Choice, Config, CopyPaths, DateFormat, Options, PruneConfirm, Theme};
use crate::editors::Editors;
use crate::git;
use crate::highlight;
use crate::info;
use crate::input::{Command, LineEvent, LineInput};
use crate::keymap::Keymap;
use crate::model::compare::{self, CompareMode};
use crate::model::filespec::{self, DateOrder, Filespec};
use crate::model::rename_mask::{CaseMode, RenameMask};
use crate::model::sort::{Sort, SortKey};
use crate::model::tree::{LogJob, NodeId, ROOT};
use crate::ops::{self, Answer, OpKind, Operation, Source};
pub use crate::pane::{FileRef, FileView, Focus, Kbyte, KbyteShow, Pane};
use crate::picker::{Outcome as PickerOutcome, Picker};
use crate::platform;
use crate::search::{self, Pattern, SearchJob, SearchSpec};
use crate::settings::{Outcome, Settings};
use crate::shell::{self, External};
use crate::state::{self, Bookmark, Bookmarks, History};
use crate::term::Terminal;
use crate::viewer::{self, Action as ViewerAction, SearchFrom, Viewer};

mod actions;
mod commands;
mod context_help;
mod git_cmd;
mod prompts;
mod remote_cmd;
mod terminal;
mod usage_cmd;
mod view;

#[cfg(test)]
mod tests;

pub(crate) use actions::*;
pub(crate) use prompts::*;

pub use prompts::{Confirm, Prompt};
pub use view::TaggedView;

/// Directory bookmarks manager state (F11).
#[derive(Debug, Clone, Default)]
pub struct BookmarksView {
    pub selected: usize,
}

/// Ongoing read and what to do when it finishes.
pub struct Job {
    pub log: LogJob,
    /// Window whose tree is being read.
    pane: usize,
    then: AfterLog,
}

pub(crate) enum AfterLog {
    Refresh,
    OpenBranch(NodeId),
    /// Opens the Branch and compares it with the other window.
    CompareBranch(NodeId, CompareMode),
    /// Asks for confirmation of the Prune of this directory.
    Prune(NodeId),
    /// Opens the Usage view of this branch.
    Usage(NodeId),
}

/// Which menu the footer shows: the normal one or the Ctrl/Alt commands one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MenuMode {
    #[default]
    Normal,
    Ctrl,
    Alt,
    /// Ctrl+G: the next letter is a Git command.
    Git,
}

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
    /// Files the ongoing copy brings to the temporary folder for `O`, to
    /// open when it ends.
    pub(crate) open_after: Option<Vec<PathBuf>>,
    pub viewer: Option<Viewer>,
    /// Last destination typed in Copy/Move.
    last_dest: Option<String>,
    /// Directories that changed on disk and have not been re-read yet.
    pending_changes: HashSet<PathBuf>,
    /// Prompt history, per kind.
    pub history: History,
    /// Directory bookmarks, mapped by key (e.g. "0"-"9", "a"-"z").
    pub bookmarks: Bookmarks,
    /// Active bookmarks manager view (F11), if open.
    pub bookmarks_view: Option<BookmarksView>,
    /// F1 / Alt+H: the help, over whatever is open (which it leaves as it is).
    pub help: Option<crate::help::Help>,
    /// A remote connection being made, for the window at this index.
    pub connecting: Option<(usize, crate::remote::Connecting)>,
    /// Connections made so far (each gets a folder of its own).
    connections: usize,
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
    /// Files or directories trashed during the session, for Oops restoration.
    pub last_trashed: Vec<PathBuf>,
    pub keymap: Keymap,
    /// The `[editors]` table: which editor E uses for each file.
    pub editors: Editors,
    pub should_quit: bool,
    /// Alt+Q: the directory for the shell to go to on exit.
    pub quit_dir: Option<PathBuf>,
    /// Ctrl/Alt currently pressed (only on terminals that report it).
    pub held: Option<MenuMode>,
    /// Menu chosen with F9; the next letter receives that modifier.
    pub sticky: MenuMode,
    /// Git status of the files on screen (when `git` is on).
    pub git_status: git::StatusCache,
    /// F2 in a prompt that asks for a folder.
    pub picker: Option<Picker>,
    /// Alt+X: the shell of the built-in terminal, alive even while hidden.
    pub terminal: Option<Terminal>,
    /// The terminal is on screen and receives the keys.
    pub terminal_focus: bool,
    /// Alt+Z: the terminal takes the whole screen, instead of the bottom.
    pub terminal_full: bool,
    /// The tagged files last put in the shell's `$XY_TAGGED`.
    terminal_tagged: Vec<PathBuf>,
    /// Last directory of the terminal's shell seen, for the tree to follow it.
    terminal_cwd: Option<PathBuf>,
    /// The machine of the terminal on screen (`None`: this one), and the
    /// terminals of the other machines, still running while hidden.
    terminal_host: Option<String>,
    parked_terminals: Vec<(
        Option<String>,
        Terminal,
        Option<std::sync::Arc<crate::remote::Remote>>,
    )>,
    /// The remote machine of the terminal on screen: keeps its connection
    /// (which the terminal's ssh shares) and maps its paths.
    terminal_remote: Option<std::sync::Arc<crate::remote::Remote>>,
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
            open_after: None,
            viewer: None,
            last_dest: None,
            pending_changes: HashSet::new(),
            history: History::new(),
            bookmarks: Bookmarks::new(),
            bookmarks_view: None,
            help: None,
            connecting: None,
            connections: 0,
            history_file: None,
            external: None,
            options: Options::default(),
            config: Config::default(),
            config_file: None,
            config_error: None,
            settings: None,
            pruning: None,
            last_trashed: Vec::new(),
            keymap: Keymap::default(),
            editors: Editors::default(),
            should_quit: false,
            quit_dir: None,
            held: None,
            sticky: MenuMode::Normal,
            git_status: git::StatusCache::default(),
            picker: None,
            terminal: None,
            terminal_focus: false,
            terminal_full: false,
            terminal_tagged: Vec::new(),
            terminal_cwd: None,
            terminal_host: None,
            parked_terminals: Vec::new(),
            terminal_remote: None,
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
        let editors = Editors::from_config(&config.editors)?;
        self.keymap = keymap;
        self.editors = editors;
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
    pub(crate) fn save_settings(&mut self, config: Config) {
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
    pub(crate) fn date_order(&self) -> DateOrder {
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
            .filter(|p| p.archive.is_none() && p.remote.is_none())
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
            AfterLog::Usage(id) => pane.usage = Some(crate::usage::UsageView::new(id)),
        }
        self.job = None;
    }

    pub(crate) fn start_job(&mut self, log: LogJob, then: AfterLog) {
        self.job = Some(Job {
            log,
            pane: self.active,
            then,
        });
        // Small branches finish immediately, without flashing the progress message.
        self.tick(Duration::from_millis(30));
    }

    pub(crate) fn start_op(&mut self, op: Operation) {
        self.op = Some(op);
        self.tick(Duration::from_millis(30));
    }

    /// End (or cancellation) of an operation: re-reads the affected directories.
    pub(crate) fn finish_op(&mut self, prefix: &str) {
        let Some(op) = self.op.take() else { return };
        if op.to_trash && !op.trashed.is_empty() {
            self.last_trashed = op.trashed.clone();
        }
        self.rescan_everywhere(&op.touched_dirs());
        // After a Prune, the cursor goes to the parent directory.
        if let Some(root) = self.pruning.take()
            && let Some(parent) = root.parent()
        {
            self.goto(parent);
        }
        self.message = Some(format!("{prefix}{}", op.summary()));
        if let Some(copies) = self.open_after.take() {
            let ready = copies.into_iter().filter(|c| c.exists()).collect();
            self.open_paths(ready, &op.errors);
        }
    }

    /// Re-reads `dirs` in all windows (both may show the same place).
    pub(crate) fn rescan_everywhere<P: AsRef<Path>>(&mut self, dirs: &[P]) {
        for pane in &mut self.panes {
            pane.rescan_paths(dirs);
        }
        self.git_status.invalidate();
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
        // From a remote machine, the new window shows this one, where Xylem
        // started: local on one side, remote on the other.
        if self.remote.is_some() {
            let here = std::env::current_dir()
                .ok()
                .or_else(platform::home_dir)
                .unwrap_or_else(|| PathBuf::from("/"));
            let mut pane = self.new_pane(here);
            pane.sort = self.sort;
            pane.display = self.display;
            self.panes.push(pane);
            return;
        }
        // Inside an archive, the new window shows the disk.
        let base = self.outer.as_deref().unwrap_or(self);
        let mut pane = self.new_pane(base.tree.path(ROOT));
        pane.sort = self.sort;
        pane.display = self.display;
        pane.goto(&base.current_path());
        self.panes.push(pane);
    }

    /// A new window (F8, Log) rooted at `root`, with the options that
    /// apply to every window (show hidden files).
    pub(crate) fn new_pane(&self, root: PathBuf) -> Pane {
        let mut pane = Pane::new(root);
        if pane.tree.show_hidden != self.options.show_hidden {
            pane.tree.show_hidden = self.options.show_hidden;
            pane.after_change();
        }
        pane
    }
}
