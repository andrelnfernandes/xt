//! End-to-end tests: the real `xy` in a pseudo-terminal, driven by keys and
//! checked on the screen it draws and on the files it changes. Each run has
//! a home of its own, so the user's configuration is never touched.
#![cfg(target_os = "linux")]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

const ROWS: u16 = 30;
const COLS: u16 = 110;

struct Xy {
    keys: Arc<Mutex<Box<dyn Write + Send>>>,
    screen: Arc<Mutex<vt100::Parser>>,
    child: Box<dyn Child + Send + Sync>,
    _master: Box<dyn MasterPty + Send>,
}

impl Xy {
    /// Starts `xy` with `args`, with `home` as its home.
    fn start(home: &Path, args: &[&str]) -> Xy {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: ROWS,
                cols: COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_xy"));
        cmd.args(args);
        cmd.env_clear();
        for (name, value) in [
            ("HOME", home.to_str().unwrap()),
            ("PATH", "/usr/bin:/bin"),
            ("TERM", "xterm-256color"),
            ("SHELL", "/bin/sh"),
            ("LANG", "C.UTF-8"),
        ] {
            cmd.env(name, value);
        }
        let child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);
        let keys = Arc::new(Mutex::new(pair.master.take_writer().unwrap()));
        let screen = Arc::new(Mutex::new(vt100::Parser::new(ROWS, COLS, 0)));
        let mut output = pair.master.try_clone_reader().unwrap();
        let (screen2, keys2) = (screen.clone(), keys.clone());
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = output.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let data = &buf[..n];
                let mut parser = screen2.lock().unwrap();
                parser.process(data);
                // Answers what a terminal would: the cursor position and
                // the device attributes (which tell crossterm, quickly,
                // that the kitty keyboard protocol is not there).
                let mut answer = Vec::new();
                if contains(data, b"\x1b[6n") {
                    let (row, col) = parser.screen().cursor_position();
                    answer.extend(format!("\x1b[{};{}R", row + 1, col + 1).bytes());
                }
                if contains(data, b"\x1b[c") {
                    answer.extend(b"\x1b[?62;22c");
                }
                drop(parser);
                if !answer.is_empty() {
                    let _ = keys2.lock().unwrap().write_all(&answer);
                }
            }
        });
        Xy {
            keys,
            screen,
            child,
            _master: pair.master,
        }
    }

    /// Types `keys` (raw bytes: "\r" is Enter, "\x14" Ctrl+T, "\x1bq" Alt+Q).
    fn press(&self, keys: &str) {
        let mut w = self.keys.lock().unwrap();
        w.write_all(keys.as_bytes()).unwrap();
        w.flush().unwrap();
        drop(w);
        // A lone Esc must not merge with the next key into an Alt+key.
        thread::sleep(Duration::from_millis(if keys == "\x1b" { 150 } else { 60 }));
    }

    fn contents(&self) -> String {
        self.screen.lock().unwrap().screen().contents()
    }

    /// Waits (up to 10 s) for `text` on the screen.
    fn wait_for(&self, text: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.contents().contains(text) {
            assert!(
                Instant::now() < deadline,
                "{text:?} did not appear on the screen:\n{}",
                self.contents()
            );
            thread::sleep(Duration::from_millis(30));
        }
    }

    /// Waits for `xy` to exit and returns whether it succeeded.
    fn wait_exit(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.success();
            }
            assert!(
                Instant::now() < deadline,
                "xy did not exit:\n{}",
                self.contents()
            );
            thread::sleep(Duration::from_millis(30));
        }
    }

    fn quit(&mut self) {
        self.press("q");
        self.wait_for("Quit Xylem?");
        self.press("y");
        assert!(self.wait_exit(), "xy failed");
    }
}

impl Drop for Xy {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// A home and, inside it, a project with `a.rs`, `b.txt` and `sub/`.
fn sample() -> (tempfile::TempDir, PathBuf) {
    let home = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let project = home.path().join("project");
    std::fs::create_dir_all(project.join("sub")).unwrap();
    std::fs::write(project.join("a.rs"), "fn main() {}\n").unwrap();
    std::fs::write(project.join("b.txt"), "hello\n").unwrap();
    (home, project)
}

/// Waits (up to 10 s) for `check` to hold, for effects on disk.
fn eventually(what: &str, check: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !check() {
        assert!(Instant::now() < deadline, "{what}");
        thread::sleep(Duration::from_millis(30));
    }
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn opens_tags_copies_and_quits() {
    let (home, project) = sample();
    let mut xy = Xy::start(home.path(), &[project.to_str().unwrap()]);
    xy.wait_for("a.rs");
    xy.wait_for("b.txt");
    xy.press("\r"); // file window
    xy.press("\x14"); // Ctrl+T: tag all
    xy.press("\x03"); // Ctrl+C: copy the tagged files
    xy.wait_for("COPY");
    xy.press("\x15"); // Ctrl+U: clear the prompt
    xy.press(&format!("{}\r", project.join("sub").display()));
    xy.wait_for("Copied 2 of 2");
    assert_eq!(names_in(&project.join("sub")), ["a.rs", "b.txt"]);
    xy.quit();
}

#[test]
fn filespec_filters_the_list() {
    let (home, project) = sample();
    let mut xy = Xy::start(home.path(), &[project.to_str().unwrap()]);
    xy.wait_for("b.txt");
    xy.press("f");
    xy.wait_for("FILESPEC");
    xy.press("\x15");
    xy.press("*.rs\r");
    xy.wait_for("Files: *.rs");
    let screen = xy.contents();
    assert!(
        screen.contains("a.rs") && !screen.contains("b.txt"),
        "{screen}"
    );
    xy.quit();
}

#[test]
fn batch_rename_with_a_mask() {
    let (home, project) = sample();
    let mut xy = Xy::start(home.path(), &[project.to_str().unwrap()]);
    xy.wait_for("b.txt");
    xy.press("\r");
    xy.press("\x14"); // tag all
    xy.press("\x12"); // Ctrl+R: rename the tagged files
    xy.wait_for("RENAME");
    xy.press("\x15");
    xy.press("file_<01>.*\r");
    eventually("the files were not renamed", || {
        names_in(&project) == ["file_01.rs", "file_02.txt", "sub"]
    });
    xy.quit();
}

#[test]
fn alt_q_leaves_the_directory_for_the_shell() {
    let (home, project) = sample();
    let cwd_file = home.path().join("cwd");
    let mut xy = Xy::start(
        home.path(),
        &[
            "--cwd-file",
            cwd_file.to_str().unwrap(),
            project.to_str().unwrap(),
        ],
    );
    xy.wait_for("sub");
    xy.press("\x1b[B"); // ↓ to sub
    xy.press("\x1bq"); // Alt+Q
    xy.wait_for("Quit Xylem and change to");
    xy.press("y");
    assert!(xy.wait_exit());
    let dir = std::fs::read_to_string(&cwd_file).unwrap();
    assert_eq!(Path::new(&dir), project.join("sub"));
}

#[test]
fn moves_the_configuration_of_xt() {
    let (home, project) = sample();
    let old = home.path().join(".config/xt");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("config.toml"), "[options]\nconfirm_quit = false\n").unwrap();
    let mut xy = Xy::start(home.path(), &[project.to_str().unwrap()]);
    xy.wait_for("xt is now Xylem");
    let new = home.path().join(".config/xylem/config.toml");
    assert!(new.exists() && !old.exists());
    // The moved configuration is in effect: no confirmation to quit.
    xy.press("q");
    assert!(xy.wait_exit());
}
