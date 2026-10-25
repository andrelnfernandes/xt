use super::*;

impl App {
    /// Back from an external program: the disk may have changed.
    /// Alt+X (or Alt+Z, in full screen): shows the terminal, starting the
    /// shell the first time. If the shell is waiting at the prompt, it goes
    /// to the tree's directory and `$XY_TAGGED` gets the tagged files.
    pub(crate) fn show_terminal(&mut self, full: Option<bool>) {
        let dir = self.current_path();
        let tagged: Vec<PathBuf> = self.sources(true).into_iter().map(|s| s.path).collect();
        let remote = self.remote.clone();
        self.switch_terminal(remote.as_ref().map(|r| r.address.destination()));
        if self.terminal.is_none() {
            let spawned = match &remote {
                // ssh -t to the window's machine, starting in its directory;
                // ssh itself runs here, in the home directory.
                Some(r) => {
                    let (program, args) = r.terminal_command(&dir);
                    let home = platform::home_dir().unwrap_or_else(std::env::temp_dir);
                    Terminal::spawn_remote(&program, &args, &home)
                }
                None => {
                    let history = self
                        .options
                        .terminal_history
                        .then(platform::state_dir)
                        .flatten();
                    Terminal::spawn(&dir, history.as_deref())
                }
            };
            match spawned {
                Ok(t) => {
                    self.terminal = Some(t);
                    self.terminal_cwd = Some(dir.clone());
                    self.terminal_tagged.clear();
                    self.terminal_remote = remote.clone();
                }
                Err(e) => {
                    self.message = Some(format!("Cannot start the shell: {e}"));
                    return;
                }
            }
        }
        if remote.is_some() {
            // Nothing is typed into a remote shell: Xylem cannot tell whether
            // a program is running there.
            self.terminal_focus = true;
            if let Some(full) = full {
                self.terminal_full = full;
            }
            return;
        }
        let shell_dir = self.terminal.as_ref().and_then(Terminal::cwd);
        let shell_dir = shell_dir.or_else(|| self.terminal_cwd.clone());
        let cd = (shell_dir.as_ref() != Some(&dir)).then_some(dir.as_path());
        if let Some(t) = &mut self.terminal
            && (cd.is_some() || tagged != self.terminal_tagged)
            && t.at_prompt()
        {
            t.sync(cd, &tagged);
            self.terminal_tagged = tagged;
        }
        self.terminal_cwd = Some(dir);
        self.terminal_focus = true;
        if let Some(full) = full {
            self.terminal_full = full;
        }
    }

    /// Keys while the terminal is on screen: everything goes to the shell,
    /// except the terminal commands (Alt+X back to Xylem, Alt+Z full screen,
    /// Alt+O types the tagged files) and Shift+PgUp/PgDn (scrollback).
    pub(crate) fn terminal_key(&mut self, key: KeyEvent) {
        match self.keymap.lookup(key, self.focus) {
            Some(Command::Terminal) => return self.hide_terminal(),
            Some(Command::TerminalFull) => {
                self.terminal_full = !self.terminal_full;
                return;
            }
            Some(Command::TerminalTagged) => return self.type_tagged(),
            _ => {}
        }
        let shift = key.modifiers == KeyModifiers::SHIFT;
        let Some(t) = &mut self.terminal else {
            self.terminal_focus = false;
            return;
        };
        match key.code {
            KeyCode::PageUp if shift => t.scroll(10),
            KeyCode::PageDown if shift => t.scroll(-10),
            _ => t.send_key(key),
        }
    }

    /// Puts on screen the terminal of `host` (`None`: this machine),
    /// parking the one that was there; it keeps running.
    fn switch_terminal(&mut self, host: Option<String>) {
        if host == self.terminal_host {
            return;
        }
        if let Some(t) = self.terminal.take() {
            let parked = (self.terminal_host.take(), t, self.terminal_remote.take());
            self.parked_terminals.push(parked);
        }
        self.terminal_remote = None;
        if let Some(i) = self.parked_terminals.iter().position(|(h, ..)| *h == host) {
            let (_, t, remote) = self.parked_terminals.swap_remove(i);
            self.terminal = Some(t);
            self.terminal_remote = remote;
        }
        self.terminal_host = host;
        self.terminal_cwd = None;
        self.terminal_tagged.clear();
    }

    /// Types the tagged files at the cursor, quoted and relative to the
    /// shell's directory when inside it.
    pub(crate) fn type_tagged(&mut self) {
        if self.terminal.as_ref().is_some_and(Terminal::is_remote) {
            self.message = Some("Alt+O types local files; not in a remote terminal yet".into());
            return;
        }
        let tagged = self.sources(true);
        let Some(t) = &mut self.terminal else {
            return;
        };
        if tagged.is_empty() {
            return;
        }
        let cwd = t.cwd().unwrap_or_default();
        let paths: Vec<PathBuf> = tagged
            .into_iter()
            .map(|s| match s.path.strip_prefix(&cwd) {
                Ok(rel) if !cwd.as_os_str().is_empty() => rel.to_path_buf(),
                _ => s.path,
            })
            .collect();
        let text = shell::expand("%t ", None, &paths, Path::new(""));
        t.write(text.as_encoded_bytes());
    }

    pub(crate) fn hide_terminal(&mut self) {
        self.terminal_focus = false;
        // What the commands changed on disk.
        let dirs: Vec<PathBuf> = self.watched_dirs().into_iter().collect();
        self.rescan_everywhere(&dirs);
    }

    /// Called by the main loop: notices when the shell exits and makes the
    /// tree follow its `cd`s. Returns whether the terminal must be redrawn.
    pub fn terminal_tick(&mut self) -> bool {
        let Some(t) = &mut self.terminal else {
            return false;
        };
        if t.exited() {
            self.terminal = None;
            self.terminal_remote = None;
            self.terminal_cwd = None;
            self.terminal_tagged.clear();
            if self.terminal_focus {
                self.hide_terminal();
            }
            self.message = Some("The terminal's shell has exited".into());
            return true;
        }
        let dirty = t.take_dirty();
        if dirty {
            t.on_output();
        }
        // A remote shell: the directory it reports (OSC 7), inside the window.
        if dirty
            && t.is_remote()
            && let Some(reported) = t.reported_dir()
            && let Some(remote) = self.terminal_remote.clone()
        {
            let reported = PathBuf::from(reported);
            if self.terminal_cwd.as_ref() != Some(&reported) {
                self.terminal_cwd = Some(reported.clone());
                self.message = None;
                let inside = remote.virtual_path(&reported.to_string_lossy());
                match inside {
                    Some(path) if self.current_path() == path || self.goto(&path) => {}
                    _ => {
                        self.message = Some(format!(
                            "The shell is in {} on {}, outside the tree",
                            reported.display(),
                            remote.address.destination()
                        ));
                    }
                }
            }
            return dirty;
        }
        if dirty
            && let Some(cwd) = t.cwd()
            && self.terminal_cwd.as_ref() != Some(&cwd)
        {
            self.terminal_cwd = Some(cwd.clone());
            let cwd = dunce::canonicalize(&cwd).unwrap_or(cwd);
            self.message = None;
            if self.current_path() != cwd && !self.goto(&cwd) {
                // Outside the tree's root (or hidden): the tree stays where
                // it got, and says why.
                self.message = Some(format!(
                    "The shell is in {}, outside the tree ({} logs another directory)",
                    cwd.display(),
                    self.keymap.key_for(Command::Log).unwrap_or("L".into())
                ));
            }
        }
        dirty
    }

    pub fn after_external(&mut self, result: std::io::Result<std::process::ExitStatus>) {
        let dirs: Vec<PathBuf> = self.watched_dirs().into_iter().collect();
        self.rescan_everywhere(&dirs);
        // Back from the editor (E in the viewer): shows the new contents
        // (in a diff, the new diff).
        if let Some(v) = &mut self.viewer {
            let in_diff = v.behind.is_some();
            let file = match &mut v.behind {
                Some(file) => &mut **file,
                None => v,
            };
            if file.on_disk
                && let Err(e) = file.reload()
            {
                self.viewer = None;
                self.tagged_view = None;
                self.message = Some(format!("Cannot view file: {e}"));
                return;
            }
            if in_diff {
                let file = self.viewer.take().and_then(|v| v.behind).map(|b| *b);
                self.viewer = file;
                self.git_status.invalidate();
                self.refresh_git();
                self.toggle_diff();
            }
        }
        self.message = match result {
            Ok(status) if status.success() => None,
            Ok(status) => Some(format!("Command finished with {status}")),
            Err(e) => Some(format!("Cannot run command: {e}")),
        };
    }

    /// A program running in the built-in terminal (not just the shell at its
    /// prompt), which quitting would end, maybe with unsaved work.
    pub(crate) fn terminal_program(&mut self) -> Option<String> {
        let describe = |host: &Option<String>, t: &mut Terminal| match (t.is_remote(), host) {
            (true, Some(host)) if !t.exited() => Some(format!("a session on {host}")),
            (true, _) => None,
            (false, _) => t.foreground_program(),
        };
        let host = self.terminal_host.clone();
        if let Some(t) = &mut self.terminal
            && let Some(program) = describe(&host, t)
        {
            return Some(program);
        }
        self.parked_terminals
            .iter_mut()
            .find_map(|(host, t, _)| describe(host, t))
    }
}
