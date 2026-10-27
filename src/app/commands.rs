use super::*;

impl App {
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
        if let Some(ed) = &mut self.editor_pty {
            ed.term.send_key(key);
            return;
        }
        if let Some(help) = &mut self.help {
            if !help.handle_key(key) {
                self.help = None;
            }
            return;
        }
        if self.terminal_focus {
            self.terminal_key(key);
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

        // Help about whatever is in front: the prompt or question stays
        // open underneath, and Esc in the help goes back to it.
        if self.is_help_key(key) {
            self.open_help();
            return;
        }

        if self.connecting.is_some() && self.prompt.is_none() && key.code == KeyCode::Esc {
            self.cancel_connection();
            return;
        }

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
        if self.bookmarks_view.is_some() {
            self.bookmarks_key(key);
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
                self.open_after = None;
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
        if self.picker.is_some() {
            self.picker_key(key);
            return;
        }
        if key.code == KeyCode::F(2) && self.prompt.as_ref().is_some_and(Prompt::picks_folder) {
            self.open_picker();
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
            if prompt.is_bookmark_jump() {
                match key.code {
                    KeyCode::Esc => {
                        self.prompt = None;
                    }
                    KeyCode::Char('\'') | KeyCode::Enter => {
                        self.prompt = None;
                        self.open_bookmarks();
                    }
                    KeyCode::Insert | KeyCode::Char('+') => {
                        self.prompt = None;
                        self.start_bookmark_set();
                    }
                    KeyCode::Char(c) if c.is_ascii_alphanumeric() => {
                        self.prompt = None;
                        let key_str = c.to_ascii_lowercase().to_string();
                        if let Some(bm) = self.bookmarks.get(&key_str).cloned() {
                            if let Err(e) = self.jump_to_path(&bm.path) {
                                self.message = Some(e);
                            }
                        } else if c == 'm' || c == 'M' || c == 'a' || c == 'A' {
                            self.start_bookmark_set();
                        } else {
                            self.message = Some(format!("Bookmark '{c}' not found"));
                        }
                    }
                    _ => {}
                }
                return;
            }
            if prompt.is_bookmark_set() {
                match key.code {
                    KeyCode::Esc => {
                        self.prompt = None;
                    }
                    KeyCode::Char(c) if c.is_ascii_alphanumeric() => {
                        self.prompt = None;
                        let key_str = c.to_ascii_lowercase().to_string();
                        let path = self.current_path();
                        self.set_bookmark(key_str.clone(), path.clone());
                        self.message =
                            Some(format!("Bookmark '{key_str}' set to {}", path.display()));
                    }
                    _ => {}
                }
                return;
            }
            if matches!(key.code, KeyCode::F(2))
                && let PromptKind::RenameTagged { case_mode, sources } = &mut prompt.kind
            {
                *case_mode = case_mode.next();
                let count = sources.len();
                let case_suffix = match *case_mode {
                    CaseMode::None => String::new(),
                    other => format!(" [{}]", other.label()),
                };
                prompt.label = format!("RENAME {count} tagged files (mask{case_suffix}): ");
                return;
            }
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
            // F6: with/without paths; F7: from where the paths start.
            if matches!(key.code, KeyCode::F(6) | KeyCode::F(7))
                && let PromptKind::Transfer {
                    kind,
                    sources,
                    tagged: true,
                    paths,
                    mode,
                    ..
                } = &mut prompt.kind
            {
                if key.code == KeyCode::F(6) {
                    *paths = !*paths;
                } else if *paths {
                    *mode = mode.next();
                } else {
                    *paths = true;
                }
                prompt.label = transfer_label(*kind, sources, true, *paths, *mode);
                return;
            }
            match prompt.input.handle_key(key) {
                LineEvent::Submit => {
                    let prompt = self.prompt.take().unwrap();
                    self.submit(prompt.kind, prompt.input.text);
                }
                LineEvent::Cancel => {
                    if let Some(Prompt {
                        kind: PromptKind::Askpass { .. },
                        ..
                    }) = self.prompt.take()
                    {
                        self.answer_askpass(None);
                    }
                }
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
                MenuMode::Alt | MenuMode::Git => MenuMode::Normal,
            };
            return;
        }
        // With the Ctrl/Alt menu opened by F9, the next letter counts as
        // Ctrl+letter / Alt+letter. Useful where the terminal does not let the
        // combination through (Ctrl+M in Terminator, for example).
        match std::mem::take(&mut self.sticky) {
            MenuMode::Normal => {}
            _ if key.code == KeyCode::Esc => return,
            MenuMode::Git => {
                self.git_action(key);
                return;
            }
            MenuMode::Ctrl if matches!(key.code, KeyCode::Char(_)) => {
                key.modifiers |= KeyModifiers::CONTROL;
            }
            MenuMode::Alt if matches!(key.code, KeyCode::Char(_)) => {
                key.modifiers |= KeyModifiers::ALT;
            }
            _ => {}
        }
        if self.usage.is_some() {
            self.usage_key(key);
            return;
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

    pub fn apply(&mut self, cmd: Command) {
        let in_files = self.focus == Focus::Files;
        let writes = matches!(
            cmd,
            Command::Move { .. }
                | Command::Delete { .. }
                | Command::Oops
                | Command::Rename { .. }
                | Command::Attributes { .. }
                | Command::MakeDir
                | Command::Log
                | Command::Edit
                | Command::Execute
                | Command::Terminal
                | Command::TerminalFull
                | Command::TerminalTagged
                | Command::Prune
                | Command::NewDate { .. }
                | Command::CreateArchive { .. }
        );
        if writes && self.archive.is_some() {
            self.message = Some("Archives are read-only (use Copy to extract)".into());
            return;
        }
        // A remote machine: browsing, viewing, searching and copying here for
        // now; Log and the terminal work too.
        let remote_writes = matches!(
            cmd,
            Command::Move { .. }
                | Command::Delete { .. }
                | Command::Oops
                | Command::Rename { .. }
                | Command::Attributes { .. }
                | Command::MakeDir
                | Command::Edit
                | Command::Execute
                | Command::TerminalTagged
                | Command::Prune
                | Command::NewDate { .. }
                | Command::CreateArchive { .. }
                | Command::Batch
        );
        if remote_writes && self.remote.is_some() {
            self.message =
                Some("Read-only on a remote machine for now: Copy brings files here".into());
            return;
        }
        match cmd {
            Command::Quit => match self.terminal_program() {
                Some(prog) => {
                    let q = format!("The terminal is running {prog}. Quit Xylem anyway?");
                    self.ask(ConfirmKind::Quit, q);
                }
                None if self.options.confirm_quit => {
                    self.ask(ConfirmKind::Quit, "Quit Xylem?".into())
                }
                None => self.should_quit = true,
            },
            Command::QuitChangeDir => {
                // Inside a ZIP/TAR, the real directory is where it is located.
                let base = self.outer.as_deref().unwrap_or(self);
                let dir = base.current_path();
                let running = self.terminal_program();
                if self.options.confirm_quit || running.is_some() {
                    let mut q = format!("Quit Xylem and change to {}?", dir.display());
                    if let Some(prog) = running {
                        q = format!("The terminal is running {prog}. {q}");
                    }
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
                self.show_text("Xylem info", text);
            }
            Command::Avail => self.show_text("Xylem avail", info::avail()),
            Command::Stats => {
                let text = self.stats_text();
                self.show_text("Xylem statistics", text);
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
            Command::Usage => self.open_usage(),
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
            Command::Help => self.open_help(),
            Command::Edit => {
                if let Some(f) = self.selected_file() {
                    let file = self.file_path(f);
                    let cwd = self.current_path();
                    match self.editor_for(&file, &cwd) {
                        Ok(external) => self.external = Some(external),
                        Err(e) => self.message = Some(e),
                    }
                }
            }
            Command::Terminal => self.show_terminal(None),
            Command::TerminalFull => self.show_terminal(Some(true)),
            Command::TerminalTagged => {
                self.show_terminal(None);
                if self.terminal_focus {
                    self.type_tagged();
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
            Command::Git => self.open_git_menu(),
            Command::Relog => {
                self.git_status.invalidate();
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
            Command::Delete {
                tagged: false,
                permanent,
            } if !in_files => self.ask_delete_dir(permanent),
            Command::Delete { tagged, permanent } => {
                let to_trash = if permanent {
                    false
                } else {
                    self.options.delete_to_trash
                };
                if let Some(sources) = self.sources_or_warn(tagged) {
                    self.ask_delete_files(sources, to_trash);
                }
            }
            Command::Oops => self.ask_oops(),
            Command::Attributes { tagged } => self.prompt_chmod(tagged),
            Command::Rename { tagged: true } => {
                if let Some(sources) = self.sources_or_warn(true) {
                    let count = sources.len();
                    let case_mode = CaseMode::None;
                    let label = format!("RENAME {count} tagged files (mask): ");
                    self.open_prompt(
                        PromptKind::RenameTagged { sources, case_mode },
                        label,
                        "*.*",
                    );
                }
            }
            Command::Rename { tagged: false } if in_files => {
                if let Some(f) = self.selected_file() {
                    let name = self.file(f).name.to_string_lossy().into_owned();
                    let tagged_count = self.sources(true).len();
                    let hint = if tagged_count > 0 {
                        format!(" (Ctrl+R / Alt+R to rename {tagged_count} tagged files)")
                    } else {
                        String::new()
                    };
                    let label = format!("RENAME {name}{hint} to: ");
                    self.open_prompt(PromptKind::RenameFile(self.file_path(f)), label, &name);
                }
            }
            Command::Rename { tagged: false } => {
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
            Command::Clipboard { tagged } => self.clip(tagged),
            Command::Bookmarks => self.open_bookmarks(),
            Command::BookmarkJump => self.start_bookmark_jump(),
            Command::BookmarkSet => self.start_bookmark_set(),
            _ => match self.focus {
                Focus::Tree => self.apply_tree(cmd),
                Focus::Files => self.apply_files(cmd),
            },
        }
    }
}

impl App {
    /// The editor E opens `file` with:
    /// 1. a matching filename pattern in `[editors]`,
    /// 2. the usual text editor (`editor` option / `$VISUAL` / `$EDITOR` / `vi`) for text files,
    /// 3. for binary files (when `edit_binary` is true): `binary_editor`, `[editors].binary`,
    ///    or an auto-detected terminal hex editor (`hexedit`, `heh`, `bvi`, `dhex`).
    pub(crate) fn editor_for(&self, file: &Path, cwd: &Path) -> Result<shell::External, String> {
        use crate::editors;
        let name = file.file_name().unwrap_or(file.as_os_str());
        if let Some(choice) = self.editors.match_pattern(name) {
            return match choice {
                editors::Choice::Command(command) => Ok(shell::editor_command(command, file, cwd)),
                editors::Choice::Usual => Ok(shell::editor(file, cwd, &self.options.editor)),
                #[cfg(test)]
                editors::Choice::None => unreachable!(),
            };
        }
        if !editors::is_binary(file) {
            return Ok(shell::editor(file, cwd, &self.options.editor));
        }
        if !self.options.edit_binary {
            return Err(
                "Binary file: editing binary files is disabled in F12 (O opens it in its own program)"
                    .into(),
            );
        }
        let cmd = if !self.options.binary_editor.trim().is_empty() {
            Some(self.options.binary_editor.trim())
        } else if let Some(c) = self.editors.binary_command() {
            Some(c)
        } else {
            shell::detect_binary_editor()
        };
        match cmd {
            Some(command) => Ok(shell::editor_command(command, file, cwd)),
            None => Err(
                "Binary file: no hex editor found (install hexedit, heh or bvi, or set one in F12). O opens it in its own program"
                    .into(),
            ),
        }
    }
}
