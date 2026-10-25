//! Built-in terminal (Alt+X): the user's shell running in a pseudo-terminal,
//! drawn inside Xylem's screen.
//!
//! `portable-pty` runs the shell (PTY on Unix, ConPTY on Windows), `vt100`
//! interprets what it writes and `tui-term` draws the screen. A thread
//! reads the shell's output; the main loop only draws and sends keys.
//!
//! To change the shell's directory and set `$XY_TAGGED`, Xylem writes a small
//! script in a private folder (under `~/.cache/xylem`) and types ` . script` at
//! the prompt.

use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::config::TerminalTheme;

use crate::shell;

/// Lines kept above the screen (Shift+PgUp/PgDn).
const SCROLLBACK: usize = 2000;

pub struct Terminal {
    parser: Arc<Mutex<vt100::Parser>>,
    /// The shell wrote something since the last draw.
    dirty: Arc<AtomicBool>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    size: (u16, u16),
    kind: ShellKind,
    /// The script typed at the prompt by `sync`, in a folder only we can write to.
    script: Option<PathBuf>,
    /// A `sync` asked for before the shell showed its first prompt: typed
    /// earlier, the line would show up twice (echoed, then at the prompt).
    pending: Option<(Option<PathBuf>, Vec<PathBuf>)>,
    prompt_seen: bool,
}

/// How to write the script for the user's shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellKind {
    /// bash, zsh, ksh: `XY_TAGGED=('a' 'b')`.
    Arrays,
    /// fish: `set -g XY_TAGGED 'a' 'b'`, and `source` instead of `.`.
    Fish,
    /// Other POSIX shells (sh, dash): without arrays, only the `cd`.
    Posix,
    /// `cmd` (Windows): the `cd` is typed directly.
    Cmd,
}

impl ShellKind {
    fn of(shell: &str) -> Self {
        let name = Path::new(shell)
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        match name.as_str() {
            "bash" | "zsh" | "ksh" | "mksh" => ShellKind::Arrays,
            "fish" => ShellKind::Fish,
            _ if cfg!(windows) => ShellKind::Cmd,
            _ => ShellKind::Posix,
        }
    }
}

impl Terminal {
    /// Starts the user's shell (`$SHELL`, `%COMSPEC%` on Windows) in `cwd`.
    /// `history`: folder for a shell history of the terminal's own, apart
    /// from the one of the user's other terminals (`None`: the usual one).
    pub fn spawn(cwd: &Path, history: Option<&Path>) -> anyhow::Result<Self> {
        Self::spawn_shell(&default_shell(), &[], cwd, history)
    }

    fn spawn_shell(
        shell: &str,
        args: &[&str],
        cwd: &Path,
        history: Option<&Path>,
    ) -> anyhow::Result<Self> {
        let size = (24, 80);
        let pair = native_pty_system().openpty(pty_size(size))?;
        let kind = ShellKind::of(shell);
        let mut cmd = CommandBuilder::new(shell);
        cmd.args(args);
        cmd.cwd(cwd);
        // Lets scripts and prompts know they are inside Xylem.
        cmd.env("XY_TERMINAL", "1");
        if let Some(dir) = history
            && let Some((name, value)) = history_env(shell, dir)
        {
            // Without the folder, the shell would just not save the history.
            let _ = fs::create_dir_all(dir);
            cmd.env(name, value);
        }
        let child = pair.slave.spawn_command(cmd)?;
        // Without this, the reader never sees EOF when the shell exits.
        drop(pair.slave);

        let parser = Arc::new(Mutex::new(vt100::Parser::new(size.0, size.1, SCROLLBACK)));
        let dirty = Arc::new(AtomicBool::new(true));
        let mut reader = pair.master.try_clone_reader()?;
        let (p, d) = (parser.clone(), dirty.clone());
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                p.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .process(&buf[..n]);
                d.store(true, Ordering::Relaxed);
            }
            d.store(true, Ordering::Relaxed);
        });
        let writer = pair.master.take_writer()?;
        Ok(Terminal {
            parser,
            dirty,
            master: pair.master,
            writer,
            child,
            size,
            kind,
            script: None,
            pending: None,
            prompt_seen: false,
        })
    }

    pub fn parser(&self) -> MutexGuard<'_, vt100::Parser> {
        self.parser.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Whether there is new output to draw (and clears the flag).
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::Relaxed)
    }

    pub fn exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// Adjusts the PTY and the screen to the area where it is drawn.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let size = (rows.max(1), cols.max(1));
        if size == self.size {
            return;
        }
        self.size = size;
        let _ = self.master.resize(pty_size(size));
        self.parser().screen_mut().set_size(size.0, size.1);
    }

    pub fn write(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    /// Sends a key, coded as xterm does.
    pub fn send_key(&mut self, key: KeyEvent) {
        let app_cursor = self.parser().screen().application_cursor();
        if let Some(bytes) = encode_key(key, app_cursor) {
            // Typing brings the view back from the scrollback.
            self.parser().screen_mut().set_scrollback(0);
            self.write(&bytes);
        }
    }

    /// Shift+PgUp/PgDn: moves through the lines that went off the screen.
    pub fn scroll(&mut self, lines: isize) {
        let mut parser = self.parser();
        let screen = parser.screen_mut();
        let pos = screen.scrollback().saturating_add_signed(lines);
        screen.set_scrollback(pos);
    }

    /// Brings the shell in line with Xylem: goes to `cd` (if given) and sets
    /// `$XY_TAGGED` to the tagged files. Only call with the shell at the
    /// prompt (`at_prompt`), or the line goes to whatever program is running.
    pub fn sync(&mut self, cd: Option<&Path>, tagged: &[PathBuf]) {
        if !self.prompt_seen {
            self.pending = Some((cd.map(Path::to_path_buf), tagged.to_vec()));
            return;
        }
        let expand = |template: &str, file: Option<&Path>| {
            shell::expand(template, file, tagged, cd.unwrap_or(Path::new(""))).into_encoded_bytes()
        };
        if self.kind == ShellKind::Cmd {
            if cd.is_some() {
                self.write(&expand("cd /d %d\r", None));
            }
            return;
        }
        let mut script = Vec::new();
        if cd.is_some() {
            script.extend(expand("cd -- %d\n", None));
        }
        match self.kind {
            ShellKind::Arrays => script.extend(expand("XY_TAGGED=(%t)\n", None)),
            ShellKind::Fish => script.extend(expand("set -g XY_TAGGED %t\n", None)),
            ShellKind::Posix | ShellKind::Cmd => {}
        }
        if script.is_empty() {
            return;
        }
        let source = if self.kind == ShellKind::Fish {
            " source %f\r"
        } else {
            // The space keeps the line out of the history (bash's
            // ignorespace, zsh's HIST_IGNORE_SPACE).
            " . %f\r"
        };
        match self.write_script(&script) {
            Ok(path) => self.write(&expand(source, Some(&path))),
            // Without the script, at least the `cd`.
            Err(_) if cd.is_some() => self.write(&expand(" cd -- %d\r", None)),
            Err(_) => {}
        }
    }

    /// Called when the shell writes something: once the first prompt is on
    /// screen (the cursor after some text), runs the postponed `sync`.
    pub fn on_output(&mut self) {
        if self.prompt_seen {
            return;
        }
        let ready = {
            let parser = self.parser();
            let screen = parser.screen();
            screen.cursor_position().1 > 0
        };
        if ready {
            self.prompt_seen = true;
            if let Some((cd, tagged)) = self.pending.take() {
                self.sync(cd.as_deref(), &tagged);
            }
        }
    }

    fn write_script(&mut self, contents: &[u8]) -> std::io::Result<PathBuf> {
        let path = match &self.script {
            Some(path) => path.clone(),
            None => {
                let path = private_dir()?.join("sync.sh");
                self.script = Some(path.clone());
                path
            }
        };
        fs::write(&path, contents)?;
        Ok(path)
    }

    /// The shell is in the foreground, i.e. no program started from it is
    /// running (so a typed `cd` goes to the shell, not to vim).
    pub fn at_prompt(&self) -> bool {
        #[cfg(unix)]
        {
            let pid = self.child.process_id();
            pid.is_some() && self.master.process_group_leader().map(|g| g as u32) == pid
        }
        #[cfg(not(unix))]
        false
    }

    /// The program started from the shell that is in the foreground (e.g.
    /// `vim`), which quitting Xylem would end: `None` at the prompt. Its name
    /// comes from /proc; elsewhere it is just "a program".
    pub fn foreground_program(&mut self) -> Option<String> {
        #[cfg(unix)]
        {
            if self.exited() || self.at_prompt() {
                return None;
            }
            let pgid = self.master.process_group_leader()?;
            let comm = fs::read_to_string(format!("/proc/{pgid}/comm"));
            Some(comm.map_or("a program".into(), |c| c.trim().to_string()))
        }
        #[cfg(not(unix))]
        None
    }

    /// The shell's current directory, to make the tree follow it.
    pub fn cwd(&self) -> Option<PathBuf> {
        let pid = self.child.process_id()?;
        if cfg!(target_os = "linux") {
            std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
        } else {
            None
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.child.kill();
        if let Some(dir) = self.script.as_deref().and_then(Path::parent) {
            let _ = fs::remove_dir_all(dir);
            // The run's folder too, if nothing else is in it.
            if let Some(run) = dir.parent() {
                let _ = fs::remove_dir(run);
            }
        }
    }
}

/// A new folder for the script, inside Xylem's folder for this run
/// (`~/.cache/xylem/run-<pid>`, readable only by the user). One per terminal, so
/// that two terminals (in the tests) never share a script.
fn private_dir() -> std::io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let dir = crate::platform::run_dir()?.join(format!("terminal-{nanos}"));
    // Private already: `run_dir` is readable only by the user.
    fs::create_dir(&dir)?;
    Ok(dir)
}

/// Colors for the terminal: default text and background, and the 16 basic
/// ones (`None` keeps the terminal's).
struct Palette {
    text: Color,
    background: Color,
    ansi: Option<[u32; 16]>,
}

fn palette(theme: TerminalTheme, text: Color, background: Color) -> Option<Palette> {
    let (fg, bg, ansi) = match theme {
        TerminalTheme::Native => return None,
        TerminalTheme::Xylem => {
            return Some(Palette {
                text,
                background,
                ansi: None,
            });
        }
        TerminalTheme::Solarized => (
            0x839496,
            0x002b36,
            [
                0x073642, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0xd33682, 0x2aa198, 0xeee8d5,
                0x586e75, 0xcb4b16, 0x93a1a1, 0x657b83, 0x839496, 0x6c71c4, 0x93a1a1, 0xfdf6e3,
            ],
        ),
        TerminalTheme::Gruvbox => (
            0xebdbb2,
            0x282828,
            [
                0x282828, 0xcc241d, 0x98971a, 0xd79921, 0x458588, 0xb16286, 0x689d6a, 0xa89984,
                0x928374, 0xfb4934, 0xb8bb26, 0xfabd2f, 0x83a598, 0xd3869b, 0x8ec07c, 0xebdbb2,
            ],
        ),
        TerminalTheme::Nord => (
            0xd8dee9,
            0x2e3440,
            [
                0x3b4252, 0xbf616a, 0xa3be8c, 0xebcb8b, 0x81a1c1, 0xb48ead, 0x88c0d0, 0xe5e9f0,
                0x4c566a, 0xbf616a, 0xa3be8c, 0xebcb8b, 0x81a1c1, 0xb48ead, 0x8fbcbb, 0xeceff4,
            ],
        ),
        TerminalTheme::Dracula => (
            0xf8f8f2,
            0x282a36,
            [
                0x21222c, 0xff5555, 0x50fa7b, 0xf1fa8c, 0xbd93f9, 0xff79c6, 0x8be9fd, 0xf8f8f2,
                0x6272a4, 0xff6e6e, 0x69ff94, 0xffffa5, 0xd6acff, 0xff92df, 0xa4ffff, 0xffffff,
            ],
        ),
        TerminalTheme::OneDark => (
            0xabb2bf,
            0x282c34,
            [
                0x282c34, 0xe06c75, 0x98c379, 0xe5c07b, 0x61afef, 0xc678dd, 0x56b6c2, 0xabb2bf,
                0x5c6370, 0xe06c75, 0x98c379, 0xe5c07b, 0x61afef, 0xc678dd, 0x56b6c2, 0xffffff,
            ],
        ),
    };
    Some(Palette {
        text: Color::from_u32(fg),
        background: Color::from_u32(bg),
        ansi: Some(ansi),
    })
}

/// Repaints the terminal's cells (already drawn in `area`) with the colors
/// of `theme`: the default ones become the theme's text and background, and
/// the 16 basic ones its palette. 256-color and RGB stay as they are.
/// `text`/`background` are Xylem's, for the `xylem` theme.
pub fn apply_theme(
    buf: &mut Buffer,
    area: Rect,
    theme: TerminalTheme,
    text: Color,
    background: Color,
) {
    let Some(p) = palette(theme, text, background) else {
        return;
    };
    let map = |c: Color, default: Color| match c {
        Color::Reset => default,
        Color::Indexed(i) if i < 16 => p
            .ansi
            .map_or(c, |ansi| Color::from_u32(ansi[usize::from(i)])),
        other => other,
    };
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.fg = map(cell.fg, p.text);
                cell.bg = map(cell.bg, p.background);
            }
        }
    }
}

/// The variable that points the shell to a history file of its own, in
/// `dir`: `HISTFILE` (one file per shell, as the formats differ) or, in
/// fish, `fish_history` (a session name; fish keeps the file in its folder).
/// The variable is only a default: an rc file that sets `HISTFILE`
/// unconditionally wins.
fn history_env(shell: &str, dir: &Path) -> Option<(&'static str, std::ffi::OsString)> {
    let stem = Path::new(shell)
        .file_stem()
        .and_then(OsStr::to_str)?
        .to_ascii_lowercase();
    match (ShellKind::of(shell), stem.as_str()) {
        (ShellKind::Cmd, _) => None,
        (ShellKind::Fish, _) => Some(("fish_history", "xylem".into())),
        (_, name) => Some((
            "HISTFILE",
            dir.join(format!("{name}_history")).into_os_string(),
        )),
    }
}

fn pty_size((rows, cols): (u16, u16)) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn default_shell() -> String {
    let var = if cfg!(windows) { "COMSPEC" } else { "SHELL" };
    std::env::var(var)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| if cfg!(windows) { "cmd.exe" } else { "/bin/sh" }.into())
}

/// The bytes an xterm sends for `key`. `app_cursor`: the program asked for
/// the arrows in "application" mode (`ESC O A` instead of `ESC [ A`).
pub fn encode_key(key: KeyEvent, app_cursor: bool) -> Option<Vec<u8>> {
    let mods = key.modifiers;
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    let alt = mods.contains(KeyModifiers::ALT);
    // xterm's modifier parameter: 1 + Shift(1) + Alt(2) + Ctrl(4).
    let param =
        1 + u8::from(mods.contains(KeyModifiers::SHIFT)) + 2 * u8::from(alt) + 4 * u8::from(ctrl);
    let csi = |end: &str| {
        if param > 1 {
            format!("\x1b[1;{param}{end}").into_bytes()
        } else if app_cursor {
            format!("\x1bO{end}").into_bytes()
        } else {
            format!("\x1b[{end}").into_bytes()
        }
    };
    let tilde = |n: u8| {
        if param > 1 {
            format!("\x1b[{n};{param}~").into_bytes()
        } else {
            format!("\x1b[{n}~").into_bytes()
        }
    };
    let mut out = match key.code {
        KeyCode::Char(c) if ctrl => {
            let b = match c.to_ascii_lowercase() {
                c @ 'a'..='z' => c as u8 - b'a' + 1,
                ' ' | '@' | '2' => 0,
                '[' | '3' => 0x1b,
                '\\' | '4' => 0x1c,
                ']' | '5' => 0x1d,
                '^' | '6' => 0x1e,
                '_' | '-' | '/' | '7' => 0x1f,
                '?' | '8' => 0x7f,
                _ => return None,
            };
            vec![b]
        }
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace if ctrl => vec![0x08],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => csi("A"),
        KeyCode::Down => csi("B"),
        KeyCode::Right => csi("C"),
        KeyCode::Left => csi("D"),
        KeyCode::Home => csi("H"),
        KeyCode::End => csi("F"),
        KeyCode::Insert => tilde(2),
        KeyCode::Delete => tilde(3),
        KeyCode::PageUp => tilde(5),
        KeyCode::PageDown => tilde(6),
        KeyCode::F(n @ 1..=4) => {
            let end = (b'P' + n - 1) as char;
            if param > 1 {
                format!("\x1b[1;{param}{end}").into_bytes()
            } else {
                format!("\x1bO{end}").into_bytes()
            }
        }
        KeyCode::F(n @ 5..=12) => tilde([15, 17, 18, 19, 20, 21, 23, 24][usize::from(n - 5)]),
        _ => return None,
    };
    // Alt on a plain key: ESC before it (special keys carry it in `param`).
    if alt
        && matches!(
            key.code,
            KeyCode::Char(_) | KeyCode::Enter | KeyCode::Backspace
        )
    {
        out.insert(0, 0x1b);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> Vec<u8> {
        encode_key(KeyEvent::new(code, mods), false).unwrap()
    }

    #[test]
    fn encodes_like_xterm() {
        let none = KeyModifiers::NONE;
        assert_eq!(key(KeyCode::Char('ç'), none), "ç".as_bytes());
        assert_eq!(key(KeyCode::Char('c'), KeyModifiers::CONTROL), [3]);
        assert_eq!(key(KeyCode::Char('b'), KeyModifiers::ALT), b"\x1bb");
        assert_eq!(key(KeyCode::Up, none), b"\x1b[A");
        assert_eq!(key(KeyCode::Right, KeyModifiers::CONTROL), b"\x1b[1;5C");
        assert_eq!(key(KeyCode::Delete, none), b"\x1b[3~");
        assert_eq!(key(KeyCode::F(1), none), b"\x1bOP");
        assert_eq!(key(KeyCode::F(5), none), b"\x1b[15~");
        let app = encode_key(KeyEvent::new(KeyCode::Up, none), true).unwrap();
        assert_eq!(app, b"\x1bOA");
    }

    /// Waits (up to 10 s) for `text` to appear on the screen.
    #[cfg(target_os = "linux")]
    fn wait_for(term: &Terminal, text: &str) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !term.parser().screen().contents().contains(text) {
            assert!(
                std::time::Instant::now() < deadline,
                "{text:?} did not appear:\n{}",
                term.parser().screen().contents()
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[cfg(target_os = "linux")]
    fn bash(cwd: &Path) -> Terminal {
        let mut term =
            Terminal::spawn_shell("bash", &["--norc", "--noprofile"], cwd, None).unwrap();
        term.write(b"PS1='$ '\r");
        term
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn runs_a_shell_and_follows_its_directory() {
        let dir = crate::testing::tempdir();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let mut term = bash(dir.path());
        term.write(b"cd sub && echo XT-$((1+1))\r");
        wait_for(&term, "XT-2");
        assert_eq!(
            term.cwd().unwrap().canonicalize().unwrap(),
            sub.canonicalize().unwrap()
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn sync_changes_directory_and_sets_the_tagged_files() {
        let dir = crate::testing::tempdir();
        let sub = dir.path().join("it's here");
        fs::create_dir(&sub).unwrap();
        let mut term = bash(dir.path());
        // Before the first prompt, the sync waits for it.
        let tagged = [sub.join("a b.txt"), sub.join("c.txt")];
        term.sync(Some(&sub), &tagged);
        assert!(term.pending.is_some());
        // As in the app, on every output until the prompt is there. (Waiting
        // for "$ " is not enough: the typed `PS1='$ '` echoes it before bash
        // prints its first prompt.)
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while term.pending.is_some() {
            assert!(
                std::time::Instant::now() < deadline,
                "no prompt:\n{}",
                term.parser().screen().contents()
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
            term.on_output();
        }
        term.write(b"echo n=${#XY_TAGGED[@]} \"${XY_TAGGED[0]##*/}\" \"$PWD\"\r");
        wait_for(&term, &format!("n=2 a b.txt {}", sub.display()));
        assert!(term.at_prompt());
        // The script lives inside the run's private folder, and goes away
        // with the terminal.
        let script = term.script.clone().unwrap();
        let run = script.parent().and_then(Path::parent).unwrap();
        let mode = fs::metadata(run).unwrap().permissions();
        assert_eq!(
            std::os::unix::fs::PermissionsExt::mode(&mode) & 0o777,
            0o700
        );
        drop(term);
        assert!(!script.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn names_the_program_running_in_the_foreground() {
        let dir = crate::testing::tempdir();
        let mut term = bash(dir.path());
        term.write(b"echo READY\r");
        wait_for(&term, "READY");
        assert_eq!(term.foreground_program(), None);
        term.write(b"sleep 30\r");
        // Bash forks, gives the terminal to the child and only then does the
        // child exec `sleep`: for a moment the foreground program is still
        // called "bash". Wait for the name, not just for a program.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while term.foreground_program().as_deref() != Some("sleep") {
            assert!(
                std::time::Instant::now() < deadline,
                "sleep did not start: {:?}",
                term.foreground_program()
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn themes_repaint_default_and_basic_colors_only() {
        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        buf[(1, 0)]
            .set_fg(Color::Indexed(1))
            .set_bg(Color::Indexed(200));
        buf[(2, 0)].set_fg(Color::Rgb(1, 2, 3));
        let mut native = buf.clone();
        apply_theme(
            &mut native,
            area,
            TerminalTheme::Native,
            Color::White,
            Color::Blue,
        );
        assert_eq!(native, buf);

        apply_theme(
            &mut buf,
            area,
            TerminalTheme::Gruvbox,
            Color::White,
            Color::Blue,
        );
        assert_eq!(buf[(0, 0)].fg, Color::from_u32(0xebdbb2));
        assert_eq!(buf[(0, 0)].bg, Color::from_u32(0x282828));
        assert_eq!(buf[(1, 0)].fg, Color::from_u32(0xcc241d));
        assert_eq!(buf[(1, 0)].bg, Color::Indexed(200));
        assert_eq!(buf[(2, 0)].fg, Color::Rgb(1, 2, 3));

        let mut xy = Buffer::empty(area);
        xy[(1, 0)].set_fg(Color::Indexed(1));
        apply_theme(
            &mut xy,
            area,
            TerminalTheme::Xylem,
            Color::White,
            Color::Blue,
        );
        assert_eq!((xy[(0, 0)].fg, xy[(0, 0)].bg), (Color::White, Color::Blue));
        assert_eq!(xy[(1, 0)].fg, Color::Indexed(1));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn own_history_file() {
        let dir = crate::testing::tempdir();
        let history = dir.path().join("state");
        let mut term = Terminal::spawn_shell(
            "bash",
            &["--norc", "--noprofile"],
            dir.path(),
            Some(&history),
        )
        .unwrap();
        term.write(b"echo \"H=$HISTFILE\"\r");
        wait_for(
            &term,
            &format!("H={}", history.join("bash_history").display()),
        );
        assert!(history.is_dir());
        assert_eq!(
            history_env("/usr/bin/fish", &history),
            Some(("fish_history", "xylem".into()))
        );
    }

    #[test]
    fn shell_kinds() {
        assert_eq!(ShellKind::of("/usr/bin/zsh"), ShellKind::Arrays);
        assert_eq!(ShellKind::of("/usr/bin/fish"), ShellKind::Fish);
        assert_eq!(
            ShellKind::of("/bin/sh"),
            if cfg!(windows) {
                ShellKind::Cmd
            } else {
                ShellKind::Posix
            }
        );
    }
}
