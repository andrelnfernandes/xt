use super::*;
use crate::viewer::{LoadOptions, ViewMode};

/// View of the tagged files (Ctrl+V): the files and which one is open.
pub struct TaggedView {
    pub items: Vec<(FileRef, PathBuf)>,
    pub index: usize,
}

impl App {
    /// Autoview's own keys: F7, Esc and Enter exit; Shift+arrows scroll
    /// the content. Returns whether the key was used.
    pub(crate) fn autoview_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::F(7) | KeyCode::Esc | KeyCode::Enter => {
                self.autoview = None;
                true
            }
            KeyCode::Char('w' | 'W') => {
                if let Some(v) = &mut self.autoview {
                    v.toggle_wrap();
                }
                true
            }
            KeyCode::Char('#' | 'l' | 'L') => {
                if let Some(v) = &mut self.autoview {
                    v.toggle_line_numbers();
                }
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
    pub(crate) fn sync_autoview(&mut self) {
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
        let (wrap, line_numbers) = (current.wrap, current.line_numbers);
        let size = self.file(f).size;
        let mut v = self
            .load_viewer_with(path.clone(), size, self.autoview_load_options())
            .unwrap_or_else(|e| {
                Viewer::from_bytes(path, 0, format!("Cannot view file: {e}").into_bytes())
            });
        (v.page, v.width) = (page, width);
        v.wrap = wrap;
        v.line_numbers = line_numbers;
        self.autoview = Some(v);
    }

    pub(crate) fn open_viewer(&mut self) {
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
    pub(crate) fn load_viewer(&self, path: PathBuf, size: u64) -> std::io::Result<Viewer> {
        let mut load = LoadOptions::view(viewer::MAX_BYTES);
        load.decompress = self.options.viewer_decompress;
        load.fallback = self.options.legacy_charset.fallback();
        self.load_viewer_with(path, size, load)
    }

    fn autoview_load_options(&self) -> LoadOptions {
        let mut load = LoadOptions::autoview();
        load.decompress = self.options.viewer_decompress;
        load.fallback = self.options.legacy_charset.fallback();
        load
    }

    pub(crate) fn load_viewer_with(
        &self,
        path: PathBuf,
        size: u64,
        load: LoadOptions,
    ) -> std::io::Result<Viewer> {
        let mut v = if let Some(remote) = &self.remote {
            let data = remote.read(&path, load.limits.output as u64)?;
            Viewer::from_file(path, size, data, load)
        } else {
            match &self.archive {
                Some(root) => {
                    let entry = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                    let mut data = Vec::new();
                    archive::Reader::open(root)?.extract(
                        &entry,
                        &mut data,
                        load.limits.output as u64,
                    )?;
                    Viewer::from_file(path, size, data, load)
                }
                None => Viewer::open(path, load)?,
            }
        };
        v.wrap = self.options.viewer_wrap;
        v.line_numbers = self.options.viewer_line_numbers;
        Ok(v)
    }

    /// Reads a file's contents for the search: from disk, or from inside the
    /// open ZIP/TAR.
    pub(crate) fn content_opener(&self) -> search::Opener {
        if let Some(remote) = self.remote.clone() {
            return Box::new(move |path: &Path| {
                let file = remote.open(path)?;
                Ok(Box::new(std::io::BufReader::new(file)) as Box<dyn BufRead>)
            });
        }
        match self.archive.clone() {
            // Streamed, never whole in memory.
            Some(root) => Box::new(move |path: &Path| {
                let entry = path.strip_prefix(&root).unwrap_or(path);
                let data = archive::extract_streamed(&root, entry)?;
                Ok(Box::new(std::io::BufReader::new(data)) as Box<dyn BufRead>)
            }),
            None => Box::new(search::open_file),
        }
    }

    /// The list's tagged files, with their paths.
    pub(crate) fn tagged_items(&self) -> Vec<(FileRef, PathBuf)> {
        self.files
            .iter()
            .copied()
            .filter(|&f| self.file(f).tagged)
            .map(|f| (f, self.file_path(f)))
            .collect()
    }

    /// Ctrl+S: searches the tagged files, bit by bit, untagging those without
    /// the term.
    pub(crate) fn start_search(&mut self, spec: SearchSpec) -> Result<(), String> {
        let job = SearchJob::new(spec.clone(), self.tagged_items(), self.content_opener())?;
        self.search = Some(spec);
        self.search_job = Some(job);
        self.tick(Duration::from_millis(30));
        Ok(())
    }

    /// Ctrl+V: opens the first tagged file; N/P go through the others.
    pub(crate) fn view_tagged(&mut self) {
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
    pub(crate) fn show_tagged(&mut self, index: usize) -> bool {
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

    pub(crate) fn viewer_action(&mut self, action: ViewerAction) {
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
            ViewerAction::CopyPath => {
                if let Some(v) = &mut self.viewer {
                    let text = v.path.to_string_lossy().into_owned();
                    let _ = crate::clipboard::set_clipboard(&text);
                    v.note = Some(format!("Copied to clipboard: {text}"));
                }
            }
            ViewerAction::Diff => self.toggle_diff(),
            ViewerAction::Edit => {
                let current = self.current_path();
                let Some(v) = &mut self.viewer else {
                    return;
                };
                // In a diff, the file it shows.
                let v = match &mut v.behind {
                    Some(file) => file,
                    None => v,
                };
                if !v.on_disk {
                    v.note = Some("Only files on disk can be edited".into());
                    return;
                }
                let cwd = v.path.parent().map_or(current, Path::to_path_buf);
                let path = v.path.clone();
                match self.editor_for(&path, &cwd) {
                    Ok(external) => self.external = Some(external),
                    Err(e) => {
                        if let Some(v) = &mut self.viewer {
                            let v = match &mut v.behind {
                                Some(file) => file,
                                None => v,
                            };
                            v.note = Some(e);
                        }
                    }
                }
            }
            ViewerAction::Next | ViewerAction::Prev | ViewerAction::First | ViewerAction::Last => {
                let Some((i, n)) = position else {
                    if action == ViewerAction::Next
                        && let Some(v) = &mut self.viewer
                    {
                        v.toggle_line_numbers();
                    }
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
            ViewerAction::GotoLine => {
                let Some(v) = &self.viewer else { return };
                let label = match v.mode {
                    ViewMode::Text => {
                        let total = v.line_count().max(1);
                        format!("GOTO line (1-{total}): ")
                    }
                    ViewMode::Hex => {
                        let max = v.size.saturating_sub(1);
                        format!("GOTO offset (0-{max:X}): ")
                    }
                };
                self.open_prompt(PromptKind::ViewerGoto, label, "");
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
    pub(crate) fn viewer_search(
        &mut self,
        spec: SearchSpec,
        forward: bool,
        from: Option<SearchFrom>,
    ) {
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
