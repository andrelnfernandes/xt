//! Xylem — text-mode file manager inspired by XTree/ZTreeWin.
//!
//! Usage: `xy [directory]` (default: current directory); `xy --help` for more.

mod app;
mod archive;
mod clipboard;
mod config;
mod editors;
mod filetype;
mod git;
mod help;
mod highlight;
mod info;
mod input;
mod keymap;
mod model;
mod ops;
mod pane;
mod picker;
mod platform;
mod remote;
mod search;
mod settings;
mod shell;
mod state;
mod term;
#[cfg(test)]
mod testing;
mod ui;
mod viewer;
mod watch;

use std::io::{Write, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::crossterm::{cursor, execute, terminal};

use app::App;

const USAGE: &str = "\
Xylem - text-mode file manager inspired by XTree/ZTreeWin

Usage: xy [--cwd-file FILE] [DIRECTORY | user@host:[PATH] | ssh://[user@]host[:port][/PATH]]

Options:
  --cwd-file FILE    on Alt+Q (quit and change directory), write the current
                     directory to FILE, for a shell function to cd into it
  --default-config   print a commented configuration file
  --man              show the manual page; redirected, print its source
                     (for `xy --man > ~/.local/share/man/man1/xy.1`)
  -h, --help         show this help
  -V, --version      show the version";

/// In a terminal, the formatted page through `man` (or groff and a pager);
/// redirected, or without a formatter (Windows), the roff source.
fn show_man_page(page: &str) {
    use std::io::IsTerminal;
    use std::process::{Command, Stdio};

    /// Runs `cmd` with `input` on its stdin; whether it finished well.
    fn feed(cmd: &mut Command, input: &[u8]) -> bool {
        let Ok(mut child) = cmd.stdin(Stdio::piped()).spawn() else {
            return false;
        };
        // A pager quit before the end closes the pipe: not a failure.
        let _ = child.stdin.take().map(|mut stdin| stdin.write_all(input));
        child.wait().is_ok_and(|status| status.success())
    }

    if stdout().is_terminal() {
        // man-db reads the page from stdin with `-l -`.
        if feed(
            Command::new("man").args(["-l", "-"]).stderr(Stdio::null()),
            page.as_bytes(),
        ) {
            return;
        }
        let formatted = Command::new("groff")
            .args(["-man", "-Tutf8"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .and_then(|mut child| {
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(page.as_bytes())?;
                }
                child.wait_with_output()
            });
        if let Ok(output) = formatted
            && output.status.success()
        {
            let pager = std::env::var("PAGER")
                .ok()
                .filter(|pager| !pager.trim().is_empty())
                .unwrap_or_else(|| "less -R".into());
            let mut words = pager.split_whitespace();
            let shown = words
                .next()
                .is_some_and(|program| feed(Command::new(program).args(words), &output.stdout));
            if !shown {
                let _ = stdout().write_all(&output.stdout);
            }
            return;
        }
    }
    print!("{page}");
}

fn main() -> anyhow::Result<()> {
    // Run by ssh as SSH_ASKPASS for a connection Xylem started: the question
    // is the only argument, and the answer goes to stdout.
    if std::env::var_os(remote::askpass::DIR_VAR).is_some() {
        let question = std::env::args().nth(1).unwrap_or_default();
        std::process::exit(remote::askpass::run(&question));
    }
    let mut root = None;
    let mut remote_address = None;
    let mut cwd_file: Option<PathBuf> = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--clipboard-daemon") => {
                let _ = clipboard::run_daemon();
                return Ok(());
            }
            Some("--cwd-file") => match args.next() {
                Some(file) => cwd_file = Some(PathBuf::from(file)),
                None => bail!("--cwd-file needs a file name\n\n{USAGE}"),
            },
            Some("-h" | "--help") => {
                let config =
                    config::Config::path().map_or_else(|| "-".into(), |p| p.display().to_string());
                println!(
                    "{USAGE}\n\nConfiguration: {config}\nInside Xylem, F1 (or Alt+H) opens the help about what you are doing."
                );
                return Ok(());
            }
            Some("-V" | "--version") => {
                println!("xy {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Some("--default-config") => {
                print!("{}", config::DEFAULT_CONFIG);
                return Ok(());
            }
            Some("--man") => {
                show_man_page(&help::man_page());
                return Ok(());
            }
            Some(s) if s.starts_with('-') => bail!("unknown option {s}\n\n{USAGE}"),
            _ if root.is_some() || remote_address.is_some() => {
                bail!("only one directory can be given\n\n{USAGE}")
            }
            Some(text) if remote::Address::parse(text).is_some() => {
                remote_address = remote::Address::parse(text);
            }
            _ => root = Some(PathBuf::from(arg)),
        }
    }
    let root = match root {
        Some(path) => path,
        None => std::env::current_dir().context("cannot read current directory")?,
    };
    let root =
        dunce::canonicalize(&root).with_context(|| format!("cannot open {}", root.display()))?;
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }

    let migrated = platform::migrate_from_xt();
    let (config, config_error) = config::Config::load();
    let mut app = App::new(root);
    app.set_sort(config.options.initial_sort());
    app.display = config.options.file_display;
    // `load` already validated; with an error in the file, `config` is the defaults.
    let _ = app.apply_config(&config);
    app.config_file = config::Config::path();
    app.config_error = config_error.clone();
    app.message = config_error.or_else(|| {
        let (from, to) = migrated.first()?;
        Some(format!(
            "xt is now Xylem: moved {} to {}",
            from.display(),
            to.display()
        ))
    });
    if let Some(address) = remote_address
        && let Err(e) = app.connect(address)
    {
        app.message = Some(e);
    }
    app.history_file = state::history_path();
    if let Some(path) = &app.history_file {
        let state = state::load_state(path);
        app.history = state.history;
        app.bookmarks = state.bookmarks;
    }
    // `init` also installs a panic hook that restores the terminal.
    let mut terminal = ratatui::init();
    let mut keyboard = Keyboard::detect();
    let result = run(&mut terminal, &mut app, &mut keyboard);
    keyboard.restore();
    ratatui::restore();
    // Files extracted from ZIP/TAR to open with O, the terminal's script.
    if let Some(base) = platform::cache_dir() {
        let _ = std::fs::remove_dir_all(platform::run_dir_path(&base));
    }
    // Alt+Q: leaves the directory for the shell function to cd into.
    if let Some(dir) = &app.quit_dir {
        match &cwd_file {
            Some(file) => std::fs::write(file, dir.as_os_str().as_encoded_bytes())
                .with_context(|| format!("cannot write {}", file.display()))?,
            None => eprintln!(
                "xy: Alt+Q changes the shell's directory only through a shell function \
                 using --cwd-file (see the README)"
            ),
        }
    }
    result
}

/// Kitty keyboard protocol, when the terminal supports it (kitty,
/// WezTerm, foot, Ghostty, Alacritty...).
///
/// - Simple mode: Ctrl+M arrives separately from Enter and Esc is no longer
///   ambiguous. Used while a prompt is open, so typing
///   (accents, dead keys) keeps working as usual.
/// - Full mode: the terminal also reports when each key is pressed
///   and released, including bare Ctrl and Alt; this is what allows switching the menu
///   while Ctrl/Alt is held, as in ZTree.
///
/// Without support (e.g. VTE terminals like Terminator), none of this is enabled.
struct Keyboard {
    supported: bool,
    full: bool,
}

impl Keyboard {
    const SIMPLE: KeyboardEnhancementFlags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES;
    const FULL: KeyboardEnhancementFlags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        .union(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        .union(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
        .union(KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS);

    fn detect() -> Self {
        let supported = terminal::supports_keyboard_enhancement().unwrap_or(false)
            && execute!(stdout(), PushKeyboardEnhancementFlags(Self::SIMPLE)).is_ok();
        if supported {
            // If the program panics, turn the protocol off before ratatui's
            // hook restores the terminal; otherwise the shell would keep
            // receiving strange key codes. Popping once too often is
            // harmless.
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
                let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
                previous(info);
            }));
        }
        Keyboard {
            supported,
            full: false,
        }
    }

    /// Enables full mode outside prompts and disables it inside them.
    fn sync(&mut self, app: &mut App) -> std::io::Result<()> {
        // In the terminal, typing must reach the shell as usual (accents, dead keys).
        let want_full = self.supported && app.prompt.is_none() && !app.terminal_focus;
        if want_full == self.full {
            return Ok(());
        }
        if want_full {
            execute!(stdout(), PushKeyboardEnhancementFlags(Self::FULL))?;
        } else {
            execute!(stdout(), PopKeyboardEnhancementFlags)?;
        }
        self.full = want_full;
        // In simple mode the release of Ctrl/Alt is not reported.
        app.held = None;
        Ok(())
    }

    fn restore(&mut self) {
        if self.full {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
        if self.supported {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
        self.full = false;
    }

    /// Returns to simple mode after a `restore` (e.g. on coming back from the editor).
    fn resume(&mut self) {
        if self.supported {
            let _ = execute!(stdout(), PushKeyboardEnhancementFlags(Self::SIMPLE));
        }
    }
}

/// Hands the terminal to an external program (editor, command) and takes it
/// back when it finishes.
fn run_external(
    terminal: &mut DefaultTerminal,
    keyboard: &mut Keyboard,
    ext: &shell::External,
) -> std::io::Result<std::process::ExitStatus> {
    keyboard.restore();
    terminal::disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, cursor::Show)?;

    let status = ext.command().status();
    if ext.wait {
        print!("\n[Xylem] Press Enter to return to Xylem...");
        let _ = stdout().flush();
        let _ = std::io::stdin().read_line(&mut String::new());
    }

    execute!(stdout(), EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    keyboard.resume();
    terminal.clear()?;
    status
}

fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    keyboard: &mut Keyboard,
) -> anyhow::Result<()> {
    // Without inotify (limit exhausted, unsupported system), there is just no
    // automatic refresh.
    let mut watcher = watch::Watcher::new().ok();
    let mut redraw = true;
    let mut last_fs_check = Instant::now();
    while !app.should_quit {
        keyboard.sync(app)?;
        if let Some(w) = &mut watcher {
            let dirs = if app.options.auto_refresh {
                app.watched_dirs()
            } else {
                Default::default()
            };
            w.watch_only(dirs);
        }
        app.refresh_git();
        // A connection being made: ssh's questions and the result.
        app.poll_connection();
        redraw |= app.terminal_tick();
        if redraw {
            terminal.draw(|frame| ui::draw(frame, app))?;
        }
        redraw = true;
        // With work in progress (log, copy...), don't block waiting for a key:
        // process a chunk and redraw the progress.
        if app.busy() && !event::poll(Duration::ZERO)? {
            app.tick(Duration::from_millis(50));
            continue;
        }
        // Idle, wait for a key for up to half a second; in the meantime,
        // apply the changes seen on disk (batched, so a large copy
        // done by another program doesn't trigger a reread per file).
        // With the terminal on screen, wake up often to show what the
        // shell writes, but redraw only when it writes something.
        let wait = if app.terminal_focus {
            15
        } else if app.connecting.is_some() {
            100
        } else {
            500
        };
        if !event::poll(Duration::from_millis(wait))? {
            redraw = !app.terminal_focus;
            if last_fs_check.elapsed() >= Duration::from_millis(500) {
                last_fs_check = Instant::now();
                if let Some(w) = &watcher {
                    app.fs_changed(w.changed_dirs());
                }
                redraw = true;
            }
            continue;
        }
        // Press, repeat and release: the App decides which to use.
        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }
        if let Some(ext) = app.external.take() {
            let result = run_external(terminal, keyboard, &ext);
            app.after_external(result);
        }
    }
    Ok(())
}
