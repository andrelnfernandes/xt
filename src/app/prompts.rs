use super::*;
use crate::viewer::ViewMode;

/// Text field open in the footer.
pub struct Prompt {
    pub label: String,
    pub input: LineInput,
    pub(crate) kind: PromptKind,
}

pub(crate) enum PromptKind {
    Filespec,
    Transfer {
        kind: OpKind,
        sources: Vec<Source>,
        tagged: bool,
        /// Recreate each file's path at the destination, from the base
        /// of `mode` (Alt+C / Alt+M; F6 toggles, F7 changes the mode).
        paths: bool,
        mode: CopyPaths,
        bases: PathBases,
    },
    RenameFile(PathBuf),
    RenameDir(NodeId),
    RenameTagged {
        sources: Vec<Source>,
        case_mode: CaseMode,
    },
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
    /// Quick jump to a directory bookmark (').
    BookmarkJump,
    /// Set a directory bookmark for the current directory (Ctrl+F11).
    BookmarkSet,
    /// Goto line in the viewer (g / :).
    ViewerGoto,
    /// A question from ssh while connecting: a password (`secret`, shown
    /// masked) or a yes/no.
    Askpass {
        secret: bool,
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
            PromptKind::RenameTagged { .. } => "rename_mask",
            PromptKind::SearchTagged { .. } | PromptKind::ViewerSearch { .. } => "search",
            PromptKind::ViewerGoto => "goto",
            PromptKind::Chmod(_) => "chmod",
            PromptKind::RenameFile(_)
            | PromptKind::RenameDir(_)
            | PromptKind::Prune { .. }
            | PromptKind::BookmarkJump
            | PromptKind::BookmarkSet
            | PromptKind::Askpass { .. } => {
                return None;
            }
        })
    }
}

impl PromptKind {
    pub(crate) fn search_spec_mut(&mut self) -> Option<&mut SearchSpec> {
        match self {
            PromptKind::SearchTagged { spec } | PromptKind::ViewerSearch { spec, .. } => Some(spec),
            _ => None,
        }
    }

    pub(crate) fn search_label(&self) -> Option<String> {
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

/// Entries kept per prompt kind.
const HISTORY_SIZE: usize = 100;

impl PromptKind {
    /// Asks for a folder (F2 picks it in a tree): `Some(true)` when the
    /// text is a file and only its folder is picked.
    pub(crate) fn folder(&self) -> Option<bool> {
        match self {
            PromptKind::Transfer { .. } | PromptKind::Log => Some(false),
            PromptKind::BatchFile | PromptKind::CreateArchive(_) => Some(true),
            _ => None,
        }
    }
}

impl Prompt {
    /// F2 opens the folder picker.
    pub fn picks_folder(&self) -> bool {
        self.kind.folder().is_some()
    }

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

    pub fn is_bookmark_jump(&self) -> bool {
        matches!(self.kind, PromptKind::BookmarkJump)
    }

    pub fn is_bookmark_set(&self) -> bool {
        matches!(self.kind, PromptKind::BookmarkSet)
    }

    /// A password: shown masked.
    pub fn is_secret(&self) -> bool {
        matches!(self.kind, PromptKind::Askpass { secret: true })
    }

    pub fn is_rename_tagged(&self) -> bool {
        matches!(self.kind, PromptKind::RenameTagged { .. })
    }

    pub fn is_goto(&self) -> bool {
        matches!(self.kind, PromptKind::ViewerGoto)
    }
}

/// Where Alt+C / Alt+M recreate the paths from, in each mode.
pub(crate) struct PathBases {
    full: PathBuf,
    tree: PathBuf,
    current: PathBuf,
    relative: PathBuf,
}

impl PathBases {
    /// For a list whose root is `relative`, in a tree rooted at `tree`.
    /// Inside an archive, paths never go above it.
    fn new(relative: PathBuf, tree: PathBuf, archive: bool) -> Self {
        let mut current = relative
            .parent()
            .map_or(relative.clone(), Path::to_path_buf);
        let mut full = relative
            .ancestors()
            .last()
            .map_or(relative.clone(), Path::to_path_buf);
        if archive {
            full = tree.clone();
            if !current.starts_with(&tree) {
                current = tree.clone();
            }
        }
        PathBases {
            full,
            tree,
            current,
            relative,
        }
    }

    fn get(&self, mode: CopyPaths) -> &Path {
        match mode {
            CopyPaths::Full => &self.full,
            CopyPaths::Tree => &self.tree,
            CopyPaths::Current => &self.current,
            CopyPaths::Relative => &self.relative,
        }
    }
}

pub(crate) fn transfer_label(
    kind: OpKind,
    sources: &[Source],
    tagged: bool,
    paths: bool,
    mode: CopyPaths,
) -> String {
    let verb = if kind == OpKind::Copy { "COPY" } else { "MOVE" };
    let mode = match (tagged, paths) {
        (false, _) => String::new(),
        (true, false) => " (no paths)".into(),
        (true, true) => format!(" (with paths: {})", mode.as_str()),
    };
    format!("{verb} {}{mode} to: ", describe(sources))
}

/// Footer question, answered with a key: the first letter of one of
/// the options. Any other key gives up.
pub struct Confirm {
    pub question: String,
    pub options: &'static [&'static str],
    pub(crate) kind: ConfirmKind,
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

pub(crate) enum ConfirmKind {
    Quit,
    /// Alt+Q: quit and leave this directory for the shell.
    QuitChangeDir(PathBuf),
    DeleteFiles {
        sources: Vec<Source>,
        to_trash: bool,
    },
    DeleteDir {
        id: NodeId,
        to_trash: bool,
    },
    RestoreTrash(Vec<PathBuf>),
    /// Copy/Move to a folder that does not exist yet: create it first?
    CreateDest {
        kind: OpKind,
        sources: Vec<Source>,
        dest: PathBuf,
        paths_from: Option<PathBuf>,
    },
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
    "Percent",
    "None",
    "All",
    "Tagged",
    "Branch",
    "Directory",
];

const SORT_OPTIONS: &[&str] = &["Name", "Ext", "Date", "Size", "Order", "Path first"];

impl App {
    /// Builds the Prune list and asks for the typed confirmation. Other mount
    /// points inside the branch are left out (so a mounted USB drive
    /// inside it is not deleted by mistake).
    pub(crate) fn prepare_prune(&mut self, id: NodeId) {
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

    pub(crate) fn ask_kbyte(&mut self, k: Kbyte) {
        let files = if k.tagged_only {
            "tagged files"
        } else {
            "all files"
        };
        let scope = if k.branch { "branch" } else { "directory" };
        let q = format!("KBYTE: show in the tree ({files}, per {scope}):");
        self.ask_options(ConfirmKind::Kbyte(k), q, KBYTE_OPTIONS);
    }

    pub(crate) fn ask_sort(&mut self, sort: Sort) {
        let order = if sort.descending {
            "descending"
        } else {
            "ascending"
        };
        let path = if sort.path_first { ", path first" } else { "" };
        let q = format!("SORT files ({order}{path}) by:");
        self.ask_options(ConfirmKind::Sort(sort), q, SORT_OPTIONS);
    }

    pub(crate) fn ask_compare(&mut self, branch: bool) {
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

    /// F5 / Ctrl+F5: asks for the archive name. Inside an archive, paths
    /// are relative to the list root (as in Alt+C).
    pub(crate) fn prompt_create_archive(&mut self, tagged: bool) {
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

    /// N: asks for the new date, prefilled with the current one (of the file or directory).
    pub(crate) fn prompt_new_date(&mut self, tagged: bool) {
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

    pub(crate) fn open_prompt(&mut self, kind: PromptKind, label: String, text: &str) {
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

    pub(crate) fn remember(&mut self, kind: &PromptKind, text: &str) {
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

    pub(crate) fn ask(&mut self, kind: ConfirmKind, question: String) {
        self.ask_options(kind, question, YES_NO);
    }

    pub(crate) fn ask_options(
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

    pub(crate) fn submit(&mut self, kind: PromptKind, text: String) {
        self.remember(&kind, &text);
        let result = match kind {
            PromptKind::Filespec => Filespec::parse_with(&text, self.date_order()).map(|spec| {
                self.filespec = spec;
                self.refresh_files();
            }),
            PromptKind::Log => self.log(&text),
            PromptKind::Askpass { .. } => {
                self.answer_askpass(Some(&text));
                Ok(())
            }
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
            PromptKind::ViewerGoto => {
                let clean = text.trim().trim_start_matches(':').trim();
                if let Some(v) = &mut self.viewer {
                    if v.mode == ViewMode::Text {
                        match clean.parse::<usize>() {
                            Ok(line) if line > 0 => v.goto_line(line),
                            _ => v.note = Some(format!("Invalid line number: {text}")),
                        }
                    } else {
                        let offset = usize::from_str_radix(
                            clean.trim_start_matches("0x").trim_start_matches("0X"),
                            16,
                        )
                        .or_else(|_| clean.parse::<usize>());
                        match offset {
                            Ok(off) => v.goto_line(off),
                            _ => v.note = Some(format!("Invalid offset: {text}")),
                        }
                    }
                }
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
                mode,
                bases,
                ..
            } => {
                if crate::remote::Address::parse(&text).is_some() {
                    Err("Copying to a remote machine comes in a later version".into())
                } else {
                    let dest = ops::resolve_path(&text, &self.tree.path(self.current()));
                    // Saves the absolute path: a relative one would change meaning
                    // next time, from another directory.
                    self.last_dest = Some(dest.to_string_lossy().into_owned());
                    let paths_from = paths.then(|| bases.get(mode).to_path_buf());
                    if needs_new_dir(&dest, sources.len(), paths_from.is_some()) {
                        let question = format!("Create the missing folder {}?", dest.display());
                        let kind = ConfirmKind::CreateDest {
                            kind,
                            sources,
                            dest,
                            paths_from,
                        };
                        self.ask(kind, question);
                        Ok(())
                    } else {
                        self.start_transfer(kind, sources, &dest, paths_from.as_deref())
                    }
                }
            }
            PromptKind::RenameFile(path) => self.rename_file(&path, &text),
            PromptKind::RenameDir(id) => self.rename_dir(id, &text),
            PromptKind::RenameTagged { sources, case_mode } => {
                self.rename_tagged(sources, case_mode, &text);
                Ok(())
            }
            PromptKind::MakeDir(id) => self.make_dir(id, &text),
            PromptKind::Chmod(paths) => self.chmod(&paths, &text),
            PromptKind::BookmarkJump => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    self.open_bookmarks();
                    Ok(())
                } else {
                    let key = trimmed
                        .chars()
                        .next()
                        .unwrap()
                        .to_ascii_lowercase()
                        .to_string();
                    if let Some(bm) = self.bookmarks.get(&key).cloned() {
                        self.jump_to_path(&bm.path)
                    } else {
                        Err(format!("Bookmark '{key}' not found"))
                    }
                }
            }
            PromptKind::BookmarkSet => {
                let trimmed = text.trim();
                if let Some(c) = trimmed.chars().next().filter(|c| c.is_ascii_alphanumeric()) {
                    let key = c.to_ascii_lowercase().to_string();
                    let path = self.current_path();
                    self.set_bookmark(key.clone(), path.clone());
                    self.message = Some(format!("Bookmark '{key}' set to {}", path.display()));
                    Ok(())
                } else {
                    Err("Bookmark key must be 0-9 or a-z".into())
                }
            }
        };
        if let Err(e) = result {
            self.message = Some(e);
        }
    }

    /// Starts a Copy/Move to `dest` (reading from the open archive, if any).
    fn start_transfer(
        &mut self,
        kind: OpKind,
        sources: Vec<Source>,
        dest: &Path,
        paths_from: Option<&Path>,
    ) -> Result<(), String> {
        let into_remote = self
            .panes
            .iter()
            .filter_map(|p| p.remote.as_ref())
            .any(|r| dest.starts_with(&r.root));
        if into_remote {
            return Err("Copying to a remote machine comes in a later version".into());
        }
        let (archive, remote) = (self.archive.clone(), self.remote.clone());
        Operation::transfer(kind, sources, dest, paths_from)
            .and_then(|op| match (&archive, remote) {
                (_, Some(remote)) => op.reading_remote(remote),
                (Some(path), None) => op.reading_archive(path),
                (None, None) => Ok(op),
            })
            .map(|op| self.start_op(op))
    }

    pub(crate) fn confirmed(&mut self, kind: ConfirmKind, choice: char) {
        let yes = choice == 'y';
        match kind {
            ConfirmKind::CreateDest {
                kind,
                sources,
                dest,
                paths_from,
            } => {
                if !yes {
                    return;
                }
                let started = fs::create_dir_all(&dest)
                    .map_err(|e| format!("Cannot create {}: {e}", dest.display()))
                    .and_then(|()| {
                        self.start_transfer(kind, sources, &dest, paths_from.as_deref())
                    });
                if let Err(e) = started {
                    self.message = Some(e);
                }
            }
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
                    'p' => k.show = KbyteShow::Percent,
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
            _ if !yes
                && !matches!(
                    kind,
                    ConfirmKind::DeleteFiles { .. } | ConfirmKind::DeleteDir { .. }
                ) => {}
            ConfirmKind::Quit => self.should_quit = true,
            ConfirmKind::QuitChangeDir(dir) => {
                self.quit_dir = Some(dir);
                self.should_quit = true;
            }
            ConfirmKind::DeleteFiles { sources, to_trash } => {
                if choice == 't' && !to_trash {
                    return self.ask_delete_files(sources, true);
                }
                if choice == 'p' && to_trash {
                    return self.ask_delete_files(sources, false);
                }
                if yes {
                    self.start_op(Operation::delete(sources, to_trash));
                }
            }
            ConfirmKind::DeleteDir { id, to_trash } => {
                if choice == 't' && !to_trash {
                    return self.ask_delete_dir_with(id, true);
                }
                if choice == 'p' && to_trash {
                    return self.ask_delete_dir_with(id, false);
                }
                if yes {
                    let path = self.tree.path(id);
                    let parent = self.tree.node(id).parent.unwrap_or(ROOT);
                    let removed = if to_trash {
                        platform::trash_file(&path)
                    } else {
                        fs::remove_dir(&path)
                    };
                    match removed {
                        Ok(()) => {
                            if to_trash {
                                self.last_trashed = vec![path.clone()];
                            }
                            self.tree.rescan(parent);
                            self.refresh_rows(parent);
                            self.refresh_files();
                            self.rescan_everywhere(&[&path]);
                            let action = if to_trash { "Moved" } else { "Deleted" };
                            let suffix = if to_trash { " to trash" } else { "" };
                            self.message = Some(format!("{action} {}{suffix}", path.display()));
                        }
                        Err(e) => {
                            let action = if to_trash {
                                "move directory to trash"
                            } else {
                                "delete directory"
                            };
                            self.message = Some(format!("Cannot {action}: {e}"));
                        }
                    }
                }
            }
            ConfirmKind::RestoreTrash(paths) => {
                if yes {
                    match platform::restore_trash_paths(&paths) {
                        Ok(_) => {
                            let dirs: Vec<_> = paths.iter().filter_map(|p| p.parent()).collect();
                            self.rescan_everywhere(&dirs);
                            self.message =
                                Some(format!("Restored {} from trash", describe_paths(&paths)));
                            self.last_trashed.clear();
                        }
                        Err(e) => {
                            self.message = Some(format!("Cannot restore from trash: {e}"));
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn prompt_transfer(&mut self, kind: OpKind, tagged: bool, paths: bool) {
        let Some(sources) = self.sources_or_warn(tagged) else {
            return;
        };
        let mode = self.options.copy_paths;
        let label = transfer_label(kind, &sources, tagged, paths, mode);
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
            mode,
            bases: PathBases::new(
                self.tree.path(self.view_root()),
                self.tree.path(ROOT),
                self.archive.is_some() || self.remote.is_some(),
            ),
        };
        self.open_prompt(prompt, label, &default);
    }

    pub(crate) fn prompt_chmod(&mut self, tagged: bool) {
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

    pub(crate) fn ask_delete_files(&mut self, sources: Vec<Source>, to_trash: bool) {
        let label = describe(&sources);
        let q = if to_trash {
            format!("MOVE {label} to trash?")
        } else {
            format!("DELETE {label}? This cannot be undone.")
        };
        let options: &'static [&'static str] = if to_trash {
            &["Yes", "No", "Permanent"]
        } else {
            &["Yes", "No", "Trash"]
        };
        self.ask_options(ConfirmKind::DeleteFiles { sources, to_trash }, q, options);
    }

    pub(crate) fn ask_delete_dir(&mut self, permanent: bool) {
        let to_trash = if permanent {
            false
        } else {
            self.options.delete_to_trash
        };
        self.ask_delete_dir_with(self.current(), to_trash);
    }

    pub(crate) fn ask_delete_dir_with(&mut self, id: NodeId, to_trash: bool) {
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
        let name = node.name.to_string_lossy();
        let q = if to_trash {
            format!("MOVE directory {name} to trash?")
        } else {
            format!("DELETE directory {name}?")
        };
        let options: &'static [&'static str] = if to_trash {
            &["Yes", "No", "Permanent"]
        } else {
            &["Yes", "No", "Trash"]
        };
        self.ask_options(ConfirmKind::DeleteDir { id, to_trash }, q, options);
    }

    pub(crate) fn ask_oops(&mut self) {
        if !self.last_trashed.is_empty() {
            let label = describe_paths(&self.last_trashed);
            let q = format!("RESTORE {label} from trash?");
            let paths = self.last_trashed.clone();
            self.ask(ConfirmKind::RestoreTrash(paths), q);
            return;
        }
        match platform::latest_trash_item() {
            Ok(Some(item)) => {
                let name = item.name.clone();
                let q = format!("RESTORE {name} from trash?");
                self.ask(ConfirmKind::RestoreTrash(vec![item.original_path]), q);
            }
            Ok(None) => {
                self.message = Some("Trash is empty (nothing to restore)".into());
            }
            Err(e) => {
                self.message = Some(format!("Cannot check trash: {e}"));
            }
        }
    }
}

/// A Copy/Move destination that does not exist and is not a new name for a
/// single file (in an existing folder): it would have to be created first.
fn needs_new_dir(dest: &Path, sources: usize, paths: bool) -> bool {
    let new_name = !paths && sources == 1 && dest.parent().is_some_and(Path::is_dir);
    fs::symlink_metadata(dest).is_err() && !new_name
}
