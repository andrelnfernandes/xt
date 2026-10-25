use super::*;

impl App {
    /// F2: picks the prompt's folder in a tree, starting at the one typed.
    pub(crate) fn open_picker(&mut self) {
        let Some(prompt) = &self.prompt else {
            return;
        };
        let base = self.outer.as_deref().unwrap_or(self).current_path();
        let typed = ops::resolve_path(&prompt.input.text, &base);
        let start = match prompt.kind.folder() {
            Some(true) => typed.parent().map_or(base, Path::to_path_buf),
            _ => typed,
        };
        // Archives are read-only: their window offers the disk behind them.
        let windows: Vec<(PathBuf, PathBuf, bool)> = self
            .panes
            .iter()
            .map(|p| {
                let p = p.outer.as_deref().unwrap_or(p);
                (p.tree.path(ROOT), p.current_path(), p.tree.show_hidden)
            })
            .collect();
        self.picker = Some(Picker::new(&windows, self.active, &start));
    }

    pub(crate) fn picker_key(&mut self, key: KeyEvent) {
        let Some(picker) = &mut self.picker else {
            return;
        };
        match picker.handle_key(key, &self.keymap) {
            PickerOutcome::Stay => {}
            PickerOutcome::Cancel => self.picker = None,
            PickerOutcome::Chosen(dir) => {
                self.picker = None;
                let Some(prompt) = &mut self.prompt else {
                    return;
                };
                // Ctrl+B and F5 keep the typed file name.
                let path = match prompt.kind.folder() {
                    Some(true) => match Path::new(&prompt.input.text).file_name() {
                        Some(name) => dir.join(name),
                        None => dir,
                    },
                    _ => dir,
                };
                prompt.input.set_text(path.to_string_lossy().into_owned());
            }
        }
    }

    /// Tags, in the active window's list, the files that have the requested
    /// relation to their counterpart in the other window's directory. In Branch
    /// and Showall, the counterpart follows the path relative to the list root
    /// (`src/a.rs` → `<other>/src/a.rs`).
    pub(crate) fn compare(&mut self, mode: CompareMode) {
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

    /// `O`: opens the ZIP/TAR under the cursor as if it were a directory.
    pub(crate) fn open_archive(&mut self) {
        let Some(f) = self.selected_file() else {
            return;
        };
        let path = self.file_path(f);
        if self.remote.is_some() {
            self.message = Some("Archives on a remote machine: copy them here first".into());
        } else if self.archive.is_some() {
            self.message = Some("Archives inside archives are not supported".into());
        } else if archive::Kind::detect(&path).is_none() {
            self.message =
                Some("Not an archive (zip, jar, 7z, tar, tar.gz, tar.xz, tar.bz2, tar.zst)".into());
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
    pub(crate) fn start_treespec(&mut self) {
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
    pub(crate) fn treespec_key(&mut self, key: KeyEvent, origin: NodeId) {
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
    pub(crate) fn show_text(&mut self, title: &str, text: String) {
        let data = text.into_bytes();
        self.tagged_view = None;
        self.viewer = Some(Viewer::from_bytes(title.into(), data.len() as u64, data));
    }

    /// Alt+I: the file under the cursor or, in the tree, the directory (with what
    /// is logged in it and in its branch).
    pub(crate) fn info_text(&self) -> String {
        let in_files = self.focus == Focus::Files;
        if let Some(remote) = &self.remote {
            let path = match self.selected_file().filter(|_| in_files) {
                Some(f) => self.file_path(f),
                None => self.current_path(),
            };
            return remote_info(remote, &path);
        }
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
    pub(crate) fn stats_text(&self) -> String {
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
        text.push_str(&info::disk(&self.current_path()));
        text.push('\n');
        text.push_str(&info::system());
        text
    }

    /// Writes the Batch file: one line per tagged file. A script (`.sh`, or
    /// `.bat`/`.cmd` on Windows) gets its header and execute permission.
    pub(crate) fn write_batch(&mut self, file: &Path, mask: &str) -> Result<(), String> {
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

    /// Applies the typed date. Without a time, each file keeps its own.
    pub(crate) fn new_date(&mut self, paths: &[PathBuf], text: &str) -> Result<(), String> {
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
    /// Inside an archive or on a remote machine, copies them first to the
    /// temporary folder (deleted when Xylem exits), in steps like any copy
    /// (Esc stops it), and opens them at the end.
    pub(crate) fn open_with(&mut self, tagged: bool) {
        let Some(sources) = self.sources_or_warn(tagged) else {
            return;
        };
        if self.remote.is_none() && self.archive.is_none() {
            let paths = sources.into_iter().map(|s| s.path).collect();
            return self.open_paths(paths, &[]);
        }
        let prepared = (|| {
            let dir = platform::run_dir().map_err(|e| e.to_string())?;
            let copies: Vec<PathBuf> = sources
                .iter()
                .map(|s| dir.join(s.path.file_name().unwrap_or_default()))
                .collect();
            // A fresh copy, without asking to overwrite the one opened before.
            for copy in &copies {
                let _ = fs::remove_file(copy);
            }
            let op = Operation::transfer(OpKind::Copy, sources, &dir, None)?;
            let op = match (&self.remote, &self.archive) {
                (Some(remote), _) => op.reading_remote(remote.clone())?,
                (None, Some(root)) => op.reading_archive(root)?,
                (None, None) => op,
            };
            Ok::<_, String>((op, copies))
        })();
        match prepared {
            Ok((op, copies)) => {
                self.open_after = Some(copies);
                self.start_op(op);
            }
            Err(e) => self.message = Some(format!("Cannot open: {e}")),
        }
    }

    /// Opens `paths` in the default program; `errors` come from before (the
    /// copy that brought them here).
    pub(crate) fn open_paths(&mut self, paths: Vec<PathBuf>, errors: &[String]) {
        let mut errors = errors.to_vec();
        let mut opened = 0;
        for path in paths {
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
    pub(crate) fn close_archive(&mut self) {
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
    pub(crate) fn log(&mut self, text: &str) -> Result<(), String> {
        if let Some(address) = crate::remote::Address::parse(text) {
            return self.connect(address);
        }
        let path = ops::resolve_path(text, &self.current_path());
        let path = dunce::canonicalize(&path).map_err(|e| format!("Cannot log {text}: {e}"))?;
        if !path.is_dir() {
            return Err(format!("{} is not a directory", path.display()));
        }
        if !self.goto(&path) {
            let (sort, display) = (self.sort, self.display);
            *self.deref_mut() = self.new_pane(path);
            self.display = display;
            self.set_sort(sort);
        }
        Ok(())
    }

    /// Jumps directly to a directory path: if it is inside the current tree, moves
    /// the cursor there and expands as needed; otherwise opens it as a new root.
    pub(crate) fn jump_to_path(&mut self, path: &Path) -> Result<(), String> {
        let path = dunce::canonicalize(path)
            .map_err(|e| format!("Cannot jump to {}: {e}", path.display()))?;
        if !path.is_dir() {
            return Err(format!("{} is not a directory", path.display()));
        }
        if !self.goto(&path) {
            let (sort, display) = (self.sort, self.display);
            *self.deref_mut() = self.new_pane(path);
            self.display = display;
            self.set_sort(sort);
        }
        Ok(())
    }

    /// Sets or updates a bookmark for `key` pointing to `path`.
    pub(crate) fn set_bookmark(&mut self, key: String, path: PathBuf) {
        self.bookmarks.insert(key, Bookmark::new(path));
        if let Some(file) = &self.history_file {
            let _ = state::save_bookmarks(file, &self.bookmarks);
        }
    }

    /// Removes a bookmark by `key`. Returns true if it existed.
    pub(crate) fn remove_bookmark(&mut self, key: &str) -> bool {
        let removed = self.bookmarks.remove(key).is_some();
        if removed && let Some(file) = &self.history_file {
            let _ = state::save_bookmarks(file, &self.bookmarks);
        }
        removed
    }

    /// Opens the prompt to jump to a bookmark (').
    pub(crate) fn start_bookmark_jump(&mut self) {
        if self.bookmarks.is_empty() {
            self.start_bookmark_set();
            return;
        }
        self.open_prompt(
            PromptKind::BookmarkJump,
            "JUMP (0-9, a-z; ' list; + bookmark here; Esc cancel): ".into(),
            "",
        );
    }

    /// Opens the prompt to bookmark the current directory (Ctrl+B / Ctrl+F11).
    pub(crate) fn start_bookmark_set(&mut self) {
        self.open_prompt(
            PromptKind::BookmarkSet,
            "SET BOOKMARK for current directory (press 0-9 or a-z, Esc to cancel): ".into(),
            "",
        );
    }

    /// Opens the bookmarks manager screen (Alt+B / F11).
    pub(crate) fn open_bookmarks(&mut self) {
        self.bookmarks_view = Some(BookmarksView { selected: 0 });
    }

    /// Handles keyboard events when the bookmarks manager (Alt+B / F11) is open.
    pub(crate) fn bookmarks_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::ALT) && matches!(key.code, KeyCode::Char('b' | 'B'))
        {
            self.bookmarks_view = None;
            return;
        }
        let count = self.bookmarks.len();
        match key.code {
            KeyCode::Esc | KeyCode::F(11) => {
                self.bookmarks_view = None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(view) = &mut self.bookmarks_view {
                    view.selected = view.selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(view) = &mut self.bookmarks_view
                    && count > 0
                {
                    view.selected = (view.selected + 1).min(count - 1);
                }
            }
            KeyCode::Home => {
                if let Some(view) = &mut self.bookmarks_view {
                    view.selected = 0;
                }
            }
            KeyCode::End => {
                if let Some(view) = &mut self.bookmarks_view {
                    view.selected = count.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                if let Some(view) = self.bookmarks_view.take()
                    && let Some((_, bm)) = self.bookmarks.iter().nth(view.selected)
                {
                    let path = bm.path.clone();
                    if let Err(e) = self.jump_to_path(&path) {
                        self.message = Some(e);
                    }
                }
            }
            KeyCode::Delete => {
                let to_remove = self
                    .bookmarks_view
                    .as_ref()
                    .and_then(|v| self.bookmarks.keys().nth(v.selected).cloned());
                if let Some(k) = to_remove {
                    self.remove_bookmark(&k);
                    let new_count = self.bookmarks.len();
                    if let Some(view) = &mut self.bookmarks_view {
                        if new_count == 0 {
                            view.selected = 0;
                        } else {
                            view.selected = view.selected.min(new_count - 1);
                        }
                    }
                    self.message = Some(format!("Bookmark '{k}' removed"));
                }
            }
            KeyCode::Insert | KeyCode::Char('+') => {
                self.bookmarks_view = None;
                self.start_bookmark_set();
            }
            KeyCode::Char(c) if c.is_ascii_alphanumeric() => {
                let key_str = c.to_ascii_lowercase().to_string();
                if let Some(bm) = self.bookmarks.get(&key_str).cloned() {
                    self.bookmarks_view = None;
                    if let Err(e) = self.jump_to_path(&bm.path) {
                        self.message = Some(e);
                    }
                } else if c == 'd' || c == 'D' {
                    let to_remove = self
                        .bookmarks_view
                        .as_ref()
                        .and_then(|v| self.bookmarks.keys().nth(v.selected).cloned());
                    if let Some(k) = to_remove {
                        self.remove_bookmark(&k);
                        let new_count = self.bookmarks.len();
                        if let Some(view) = &mut self.bookmarks_view {
                            if new_count == 0 {
                                view.selected = 0;
                            } else {
                                view.selected = view.selected.min(new_count - 1);
                            }
                        }
                        self.message = Some(format!("Bookmark '{k}' removed"));
                    }
                } else if c == 'a' || c == 'A' || c == 'm' || c == 'M' {
                    self.bookmarks_view = None;
                    self.start_bookmark_set();
                }
            }
            _ => {}
        }
    }

    /// `sources`, warning when there is nothing to operate on.
    pub(crate) fn sources_or_warn(&mut self, tagged: bool) -> Option<Vec<Source>> {
        let sources = self.sources(tagged);
        if sources.is_empty() {
            if tagged {
                self.message = Some("No tagged files".into());
            }
            return None;
        }
        Some(sources)
    }

    pub(crate) fn rename_tagged(
        &mut self,
        sources: Vec<Source>,
        case_mode: CaseMode,
        pattern: &str,
    ) {
        let pattern = pattern.trim();
        if pattern.is_empty() {
            return;
        }
        let mask = RenameMask::new(pattern, case_mode);
        let mut pairs = Vec::new();
        let mut seen_dsts = std::collections::HashSet::new();

        for (i, s) in sources.iter().enumerate() {
            let index = i + 1;
            let Some(file_name) = s.path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let new_name = mask.apply(file_name, index);
            if let Err(e) = ops::check_name(&new_name) {
                self.message = Some(format!("Invalid name '{new_name}': {e}"));
                return;
            }
            let dst = s.path.with_file_name(&new_name);
            if dst != s.path {
                if !seen_dsts.insert(dst.clone()) {
                    self.message = Some(format!("Duplicate target name generated: {new_name}"));
                    return;
                }
                pairs.push((s.path.clone(), dst, s.size));
            }
        }

        if pairs.is_empty() {
            self.message = Some("No files renamed (names unchanged)".into());
            return;
        }

        let op = Operation::rename_batch(pairs);
        self.start_op(op);
    }

    pub(crate) fn rename_file(&mut self, path: &Path, name: &str) -> Result<(), String> {
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

    pub(crate) fn rename_dir(&mut self, id: NodeId, name: &str) -> Result<(), String> {
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

    pub(crate) fn make_dir(&mut self, id: NodeId, name: &str) -> Result<(), String> {
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
    pub(crate) fn chmod(&mut self, paths: &[PathBuf], text: &str) -> Result<(), String> {
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

    /// Copies path(s) to the system clipboard (OSC 52 and native clipboard).
    pub(crate) fn clip(&mut self, tagged: bool) {
        if tagged {
            let sources = self.sources(true);
            if sources.is_empty() {
                self.message = Some("No tagged files to copy".into());
                return;
            }
            let text = sources
                .iter()
                .map(|s| s.path.to_string_lossy())
                .collect::<Vec<_>>()
                .join("\n");
            let _ = crate::clipboard::set_clipboard(&text);
            let count = sources.len();
            let plural = if count == 1 { "path" } else { "paths" };
            self.message = Some(format!("Copied {count} {plural} to clipboard"));
        } else {
            let path = if self.focus == Focus::Files
                && let Some(f) = self.selected_file()
            {
                self.file_path(f)
            } else {
                self.current_path()
            };
            let text = path.to_string_lossy().into_owned();
            let _ = crate::clipboard::set_clipboard(&text);
            self.message = Some(format!("Copied to clipboard: {text}"));
        }
    }
}

/// Alt+I on a remote machine: what SFTP tells about `path`.
fn remote_info(remote: &crate::remote::Remote, path: &Path) -> String {
    use crate::model::tree::FileKind;
    let mut text = format!(
        "INFO  {}

",
        path.display()
    );
    match remote.lstat(path) {
        Ok(e) => {
            let kind = match e.kind {
                FileKind::Symlink => "symbolic link",
                FileKind::File => "file",
                FileKind::Other => "directory or special file",
            };
            text.push_str(&format!(
                "  {:<14}{kind}
",
                "Type"
            ));
            if e.kind == FileKind::Symlink
                && let Ok(target) = remote.read_link(path)
            {
                text.push_str(&format!(
                    "  {:<14}{target}
",
                    "Points to"
                ));
            }
            text.push_str(&format!(
                "  {:<14}{} bytes
",
                "Size",
                info::group(e.size)
            ));
            text.push_str(&format!(
                "  {:<14}{:04o}
",
                "Permissions",
                e.mode & 0o7777
            ));
            if let Some(owner) = remote.owner(path) {
                text.push_str(&format!(
                    "  {:<14}{owner} (user:group ids)
",
                    "Owner"
                ));
            }
            if let Some(t) = e.modified {
                let t: chrono::DateTime<chrono::Local> = t.into();
                text.push_str(&format!(
                    "  {:<14}{}
",
                    "Modified",
                    t.format("%Y-%m-%d %H:%M:%S %z")
                ));
            }
        }
        Err(e) => text.push_str(&format!(
            "  {e}
"
        )),
    }
    text.push_str(&format!(
        "
  On {} (SFTP)
",
        remote.address.destination()
    ));
    text
}

/// Last component of a path, for short messages.
pub(crate) fn fit_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// "file.txt" or "3 tagged files".
/// "a.rs", or "3 files".
pub(crate) fn describe_paths(paths: &[PathBuf]) -> String {
    match paths {
        [one] => one
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        many => format!("{} files", many.len()),
    }
}

pub(crate) fn describe(sources: &[Source]) -> String {
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
