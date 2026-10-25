//! Passwords and questions from `ssh`, asked by Xylem.
//!
//! `ssh` runs with `SSH_ASKPASS` pointing at `xy` itself, so when it needs a
//! password, a key's passphrase or a "yes" for an unknown host, it runs
//! `xy --askpass "<question>"`. That process ([`run`]) leaves the question in
//! a folder of the connection (inside Xylem's private run folder, mode 700)
//! and waits; the running Xylem ([`Askpass::question`]) asks the user in the
//! footer and hands the answer back ([`Askpass::answer`]), which `run` prints
//! for `ssh`. On Unix the answer goes through a named pipe, so a password
//! never touches the disk.

use std::fs;
use std::io;
#[cfg(unix)]
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The environment variable with the connection's folder.
pub const DIR_VAR: &str = "XY_ASKPASS_DIR";

/// Sent instead of an answer when the user gives up.
const CANCEL: &str = "\u{0}cancel";

/// How long `xy --askpass` waits for the user.
const WAIT: Duration = Duration::from_secs(600);

/// The connection's side: where questions show up and answers go.
pub struct Askpass {
    dir: PathBuf,
}

impl Askpass {
    /// A new folder for one connection, inside the private `run` folder.
    pub fn new(run: &Path, n: usize) -> io::Result<Self> {
        let dir = run.join(format!("askpass-{n}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir(&dir)?;
        Ok(Askpass { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The question `ssh` is waiting on, if any (and not answered yet).
    pub fn question(&self) -> Option<String> {
        let text = fs::read_to_string(self.dir.join("question")).ok()?;
        Some(text)
    }

    /// Answers the pending question (`None`: the user cancelled).
    pub fn answer(&self, answer: Option<&str>) -> io::Result<()> {
        let _ = fs::remove_file(self.dir.join("question"));
        let text = answer.unwrap_or(CANCEL);
        send(&self.dir.join("answer"), text)
    }
}

impl Drop for Askpass {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[cfg(unix)]
fn send(path: &Path, text: &str) -> io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    // Non-blocking: if `xy --askpass` is gone, this fails instead of hanging.
    let mut pipe = fs::OpenOptions::new()
        .write(true)
        .custom_flags(nix::fcntl::OFlag::O_NONBLOCK.bits())
        .open(path)?;
    pipe.write_all(text.as_bytes())
}

#[cfg(not(unix))]
fn send(path: &Path, text: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(tmp, path)
}

/// Where `run` waits for the answer: on Unix, a named pipe it creates and
/// keeps open; elsewhere, a file that appears.
#[cfg(unix)]
struct Inbox(fs::File);

#[cfg(unix)]
impl Inbox {
    fn open(path: &Path) -> io::Result<Self> {
        use std::os::unix::fs::OpenOptionsExt;
        let _ = fs::remove_file(path);
        nix::unistd::mkfifo(
            path,
            nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
        )
        .map_err(io::Error::from)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(nix::fcntl::OFlag::O_NONBLOCK.bits())
            .open(path)?;
        Ok(Inbox(file))
    }

    fn poll(&mut self, _path: &Path, got: &mut Vec<u8>) -> bool {
        let mut buf = [0u8; 4096];
        loop {
            match self.0.read(&mut buf) {
                Ok(0) => return !got.is_empty(),
                Ok(n) => got.extend_from_slice(&buf[..n]),
                Err(_) => return !got.is_empty(),
            }
        }
    }
}

#[cfg(not(unix))]
struct Inbox;

#[cfg(not(unix))]
impl Inbox {
    fn open(path: &Path) -> io::Result<Self> {
        let _ = fs::remove_file(path);
        Ok(Inbox)
    }

    fn poll(&mut self, path: &Path, got: &mut Vec<u8>) -> bool {
        match fs::read(path) {
            Ok(data) => {
                let _ = fs::remove_file(path);
                *got = data;
                true
            }
            Err(_) => false,
        }
    }
}

/// `xy --askpass "<question>"`: asks the running Xylem and prints the answer
/// for `ssh`. Returns the exit status (1: cancelled or no Xylem to ask).
pub fn run(question: &str) -> i32 {
    let Some(dir) = std::env::var_os(DIR_VAR).map(PathBuf::from) else {
        eprintln!("xy --askpass: only ssh started by Xylem uses this");
        return 1;
    };
    let answer = dir.join("answer");
    let Ok(mut inbox) = Inbox::open(&answer) else {
        return 1;
    };
    let tmp = dir.join("question.tmp");
    if fs::write(&tmp, question).is_err() || fs::rename(&tmp, dir.join("question")).is_err() {
        return 1;
    }
    let deadline = Instant::now() + WAIT;
    let mut got = Vec::new();
    while Instant::now() < deadline {
        if inbox.poll(&answer, &mut got) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = fs::remove_file(&answer);
    let text = String::from_utf8_lossy(&got);
    if got.is_empty() || text == CANCEL {
        return 1;
    }
    println!("{text}");
    0
}

/// Whether the question asks for a secret (shown masked) or for a "yes".
pub fn is_secret(question: &str) -> bool {
    let q = question.to_lowercase();
    !(q.contains("(yes/no") || q.contains("continue connecting"))
}

/// The question in one line for the footer. ssh's question about an
/// unknown host takes four lines; it keeps the host and the fingerprint,
/// which is what to check.
pub fn one_line(question: &str) -> String {
    let lines: Vec<&str> = question
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if is_secret(question) {
        return format!("{} ", lines.last().copied().unwrap_or("Password:"));
    }
    let host = question
        .split("of host '")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .unwrap_or("this host");
    let key = lines
        .iter()
        .find_map(|l| l.split_once(" key fingerprint is "))
        .map(|(kind, fp)| format!(" ({kind} {})", fp.trim_end_matches('.')))
        .unwrap_or_default();
    format!("Unknown host {host}{key}. Connect and remember it? Type yes or no: ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_goes_to_xylem_and_the_answer_comes_back() {
        let tmp = crate::testing::tempdir();
        let askpass = Askpass::new(tmp.path(), 1).unwrap();
        let dir = askpass.dir().to_path_buf();
        // The `xy --askpass` side, as ssh would run it.
        let child = std::thread::spawn(move || {
            // What `run` does, without needing the environment variable.
            let answer = dir.join("answer");
            let mut inbox = Inbox::open(&answer).unwrap();
            fs::write(dir.join("question"), "andre@host's password: ").unwrap();
            let mut got = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(10);
            while !inbox.poll(&answer, &mut got) {
                assert!(Instant::now() < deadline, "no answer");
                std::thread::sleep(Duration::from_millis(10));
            }
            String::from_utf8(got).unwrap()
        });
        let deadline = Instant::now() + Duration::from_secs(10);
        let question = loop {
            if let Some(q) = askpass.question() {
                break q;
            }
            assert!(Instant::now() < deadline, "no question");
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(question, "andre@host's password: ");
        assert!(is_secret(&question));
        askpass.answer(Some("s3cret")).unwrap();
        assert_eq!(child.join().unwrap(), "s3cret");
        assert!(askpass.question().is_none(), "answered questions go away");
        let dir = askpass.dir().to_path_buf();
        drop(askpass);
        assert!(!dir.exists(), "the folder goes with the connection");
    }

    #[test]
    fn questions_fit_in_one_line() {
        let host_key = "The authenticity of host '10.0.0.5 (10.0.0.5)' can't be established.\n\
                        ED25519 key fingerprint is SHA256:AbC123.\n\
                        This key is not known by any other names.\n\
                        Are you sure you want to continue connecting (yes/no/[fingerprint])? ";
        assert_eq!(
            one_line(host_key),
            "Unknown host 10.0.0.5 (10.0.0.5) (ED25519 SHA256:AbC123). Connect and remember it? Type yes or no: "
        );
        assert_eq!(
            one_line("andre@host's password: "),
            "andre@host's password: "
        );
    }

    #[test]
    fn host_key_questions_are_not_secret() {
        assert!(!is_secret(
            "Are you sure you want to continue connecting (yes/no/[fingerprint])? "
        ));
        assert!(is_secret(
            "Enter passphrase for key '/home/a/.ssh/id_ed25519': "
        ));
    }
}
