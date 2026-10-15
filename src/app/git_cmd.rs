use super::*;

impl App {
    /// Reads the Git status of the repositories on screen, if it changed.
    /// Called before drawing.
    pub fn refresh_git(&mut self) {
        if !self.options.git {
            return;
        }
        let dirs: Vec<PathBuf> = self
            .panes
            .iter()
            .filter(|p| p.archive.is_none())
            .map(|p| p.current_path())
            .collect();
        self.git_status.refresh(dirs);
    }

    /// Ctrl+G: opens the Git menu, if the current directory is in a
    /// repository.
    pub(crate) fn open_git_menu(&mut self) {
        if !self.options.git {
            return;
        }
        if self.archive.is_some() || git::repo_root(&self.current_path()).is_none() {
            self.message = Some("Not a Git repository".into());
            return;
        }
        self.sticky = MenuMode::Git;
    }

    /// The file can be compared with its last committed version (Git on,
    /// and the file changed, staged or not; untracked files have none).
    pub fn can_diff(&self, file: &Path) -> bool {
        self.options.git
            && self
                .git_status
                .marker(file)
                .is_some_and(|xy| xy != *b"??" && xy != *b"!!")
    }

    /// D in the viewer: from the file to its `git diff HEAD`, and back.
    pub(crate) fn toggle_diff(&mut self) {
        let Some(v) = self.viewer.take() else {
            return;
        };
        if let Some(file) = v.behind {
            self.viewer = Some(*file);
            return;
        }
        let mut v = v;
        if !v.on_disk || !self.can_diff(&v.path) {
            if self.options.git {
                v.note = Some("No changes since the last commit".into());
            }
            self.viewer = Some(v);
            return;
        }
        let root = v.path.parent().and_then(git::repo_root);
        match root.map(|root| git::report(git::Action::Diff, &root, std::slice::from_ref(&v.path)))
        {
            Some(Ok(r)) => {
                let name = v.path.file_name().unwrap_or_default().to_string_lossy();
                let title = PathBuf::from(format!("git diff {name}"));
                let size = r.text.len() as u64;
                let mut diff =
                    Viewer::from_bytes(title, size, r.text).colored_as(Path::new(r.colors_of));
                diff.behind = Some(Box::new(v));
                self.viewer = Some(diff);
            }
            Some(Err(e)) => {
                v.note = Some(format!("Git: {e}"));
                self.viewer = Some(v);
            }
            None => self.viewer = Some(v),
        }
    }

    /// A letter of the Git menu: on the tagged files of the list, else the
    /// current file, or the current directory in the tree.
    pub(crate) fn git_action(&mut self, key: KeyEvent) {
        let KeyCode::Char(c) = key.code else {
            return;
        };
        let Some(action) = git::Action::from_key(c) else {
            self.message = Some("Git: Status, Log, Diff, Blame, Add, Unstage or Commit".into());
            return;
        };
        let dir = self.current_path();
        let Some(root) = git::repo_root(&dir) else {
            return;
        };
        let mut paths: Vec<PathBuf> = self.sources(true).into_iter().map(|s| s.path).collect();
        if paths.is_empty() {
            paths = match self.focus {
                Focus::Files => self.sources(false).into_iter().map(|s| s.path).collect(),
                Focus::Tree => vec![dir],
            };
        }
        match action {
            // The whole repository: what matters is the general picture.
            git::Action::Status => paths.clear(),
            git::Action::Blame if self.focus == Focus::Tree || paths.len() != 1 => {
                self.message = Some("Blame works on one file".into());
                return;
            }
            _ if paths.is_empty() => {
                self.message = Some("No file".into());
                return;
            }
            _ => {}
        }
        match action {
            git::Action::Commit => {
                self.external = Some(shell::command("git commit".into(), &root));
            }
            a if a.shows_output() => match git::report(a, &root, &paths) {
                Ok(r) => {
                    let size = r.text.len() as u64;
                    self.tagged_view = None;
                    let v = Viewer::from_bytes(r.title, size, r.text);
                    self.viewer = Some(v.colored_as(Path::new(r.colors_of)));
                }
                Err(e) => self.message = Some(format!("Git: {e}")),
            },
            a => {
                self.message = Some(match git::change(a, &root, &paths) {
                    Ok(Ok(())) => {
                        let verb = if a == git::Action::Add {
                            "Added"
                        } else {
                            "Unstaged"
                        };
                        format!("{verb} {}", describe_paths(&paths))
                    }
                    Ok(Err(msg)) => format!("Git: {msg}"),
                    Err(e) => format!("Git: {e}"),
                });
                self.git_status.invalidate();
            }
        }
    }
}
