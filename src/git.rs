//! Git (Ctrl+G, when `git = true` in the configuration): a menu of
//! commands and the status of each file in the file list. Not in XTree or
//! ZTreeWin, so it is off by default.
//!
//! Everything runs the `git` program: it follows the user's own
//! configuration (aliases, hooks, editor) and adds no dependencies.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// The menu (Ctrl+G), in display order; the first letter chooses.
pub const MENU: &[&str] = &["Status", "Log", "Diff", "Blame", "Add", "Unstage", "Commit"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Status,
    Log,
    Diff,
    Blame,
    Add,
    Unstage,
    Commit,
}

impl Action {
    pub fn from_key(c: char) -> Option<Self> {
        Some(match c.to_ascii_lowercase() {
            's' => Action::Status,
            'l' => Action::Log,
            'd' => Action::Diff,
            'b' => Action::Blame,
            'a' => Action::Add,
            'u' => Action::Unstage,
            'c' => Action::Commit,
            _ => return None,
        })
    }

    /// Shows the output in the viewer (the others change the repository).
    pub fn shows_output(self) -> bool {
        matches!(
            self,
            Action::Status | Action::Log | Action::Diff | Action::Blame
        )
    }
}

/// The repository that contains `dir`: the nearest directory with a `.git`
/// (a directory, or a file in worktrees and submodules).
pub fn repo_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf)
}

/// `git` in `root`, without pager or colors, with names as they are
/// (no `\303\251` quoting).
fn git(root: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root).args([
        "--no-pager",
        "-c",
        "color.ui=never",
        "-c",
        "core.quotepath=off",
    ]);
    cmd
}

/// Runs `git args -- paths` and returns whether it worked and what it wrote
/// (stdout, then stderr).
fn run(root: &Path, args: &[&str], paths: &[PathBuf]) -> io::Result<(bool, Vec<u8>)> {
    let mut cmd = git(root);
    cmd.args(args);
    if !paths.is_empty() {
        cmd.arg("--").args(paths);
    }
    let out = cmd.output().map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => io::Error::new(e.kind(), "git not found"),
        _ => e,
    })?;
    let mut text = out.stdout;
    text.extend(out.stderr);
    Ok((out.status.success(), text))
}

fn has_head(root: &Path) -> bool {
    run(root, &["rev-parse", "--verify", "-q", "HEAD"], &[]).is_ok_and(|(ok, _)| ok)
}

/// Text for the viewer: its title, contents and the kind of file whose
/// colors it gets.
pub struct Report {
    pub title: PathBuf,
    pub text: Vec<u8>,
    pub colors_of: &'static str,
}

/// Status, Log, Diff or Blame of `paths` (files, or a directory).
pub fn report(action: Action, root: &Path, paths: &[PathBuf]) -> io::Result<Report> {
    let (title, (_, mut text)) = match action {
        Action::Status => ("git status", run(root, &["status"], paths)?),
        Action::Log => ("git log", run(root, &["log", "-n", "1000"], paths)?),
        // Against HEAD: what is staged and what is not, together.
        Action::Diff if has_head(root) => ("git diff", run(root, &["diff", "HEAD"], paths)?),
        Action::Diff => ("git diff", run(root, &["diff", "--cached"], paths)?),
        Action::Blame => ("git blame", run(root, &["blame"], paths)?),
        _ => unreachable!("{action:?} does not make a report"),
    };
    if text.is_empty() {
        text = b"(nothing to show)\n".to_vec();
    }
    Ok(Report {
        title: title.into(),
        text,
        colors_of: if action == Action::Diff {
            "git.diff"
        } else {
            ""
        },
    })
}

/// Add or Unstage. Returns git's message when it fails.
pub fn change(action: Action, root: &Path, paths: &[PathBuf]) -> io::Result<Result<(), String>> {
    let args: &[&str] = match action {
        Action::Add => &["add"],
        Action::Unstage if has_head(root) => &["restore", "--staged"],
        // Before the first commit there is nothing to restore from.
        Action::Unstage => &["rm", "--cached", "-q"],
        _ => unreachable!("{action:?} does not change the index"),
    };
    let (ok, text) = run(root, args, paths)?;
    Ok(if ok {
        Ok(())
    } else {
        let text = String::from_utf8_lossy(&text);
        Err(text.lines().next().unwrap_or("git failed").to_string())
    })
}

/// How often the status is read again even without known changes (commits
/// or `git add` made in another terminal only touch `.git`).
const MAX_AGE: Duration = Duration::from_secs(10);
/// Minimum interval between two readings of the same repository.
const MIN_INTERVAL: Duration = Duration::from_millis(500);

/// Status of the files in the repositories being displayed: the two
/// letters of `git status --short` (`M `, ` M`, `??`...).
#[derive(Default)]
pub struct StatusCache {
    /// Directory → its repository (or none), so the screen never touches
    /// the disk.
    roots: HashMap<PathBuf, Option<PathBuf>>,
    repos: HashMap<PathBuf, RepoStatus>,
}

struct RepoStatus {
    files: HashMap<PathBuf, [u8; 2]>,
    /// Untracked directories: everything below them is `??`.
    untracked_dirs: HashSet<PathBuf>,
    read_at: Instant,
    stale: bool,
}

impl StatusCache {
    /// Something changed (an operation, a command, F3): read again.
    pub fn invalidate(&mut self) {
        self.roots.clear();
        for repo in self.repos.values_mut() {
            repo.stale = true;
        }
    }

    /// Makes sure the repositories of `dirs` are read and current. Called
    /// before drawing.
    pub fn refresh(&mut self, dirs: impl IntoIterator<Item = PathBuf>) {
        let mut wanted = HashSet::new();
        for dir in dirs {
            let root = self
                .roots
                .entry(dir)
                .or_insert_with_key(|d| repo_root(d))
                .clone();
            if let Some(root) = root {
                wanted.insert(root);
            }
        }
        self.repos.retain(|root, _| wanted.contains(root));
        for root in wanted {
            let due = match self.repos.get(&root) {
                None => true,
                Some(r) => {
                    let age = r.read_at.elapsed();
                    (r.stale && age >= MIN_INTERVAL) || age >= MAX_AGE
                }
            };
            if due {
                let status = read_status(&root);
                self.repos.insert(root, status);
            }
        }
    }

    /// Whether `dir` is inside a repository being displayed.
    pub fn in_repo(&self, dir: &Path) -> bool {
        self.roots.get(dir).is_some_and(Option::is_some)
    }

    /// The two status letters of a file; `None` if it is unchanged (or
    /// outside the repositories).
    pub fn marker(&self, file: &Path) -> Option<[u8; 2]> {
        let (root, repo) = self.repos.iter().find(|(root, _)| file.starts_with(root))?;
        let rel = file.strip_prefix(root).ok()?;
        if let Some(m) = repo.files.get(rel) {
            return Some(*m);
        }
        rel.ancestors()
            .skip(1)
            .any(|d| repo.untracked_dirs.contains(d))
            .then_some(*b"??")
    }
}

fn read_status(root: &Path) -> RepoStatus {
    let mut status = RepoStatus {
        files: HashMap::new(),
        untracked_dirs: HashSet::new(),
        read_at: Instant::now(),
        stale: false,
    };
    let Ok((true, out)) = run(root, &["status", "--porcelain=v1", "-z"], &[]) else {
        return status;
    };
    parse_status(&out, &mut status.files, &mut status.untracked_dirs);
    status
}

/// `git status --porcelain=v1 -z`: `XY path\0`, with the old name in a
/// second field for renames and copies.
fn parse_status(
    out: &[u8],
    files: &mut HashMap<PathBuf, [u8; 2]>,
    untracked_dirs: &mut HashSet<PathBuf>,
) {
    let mut fields = out.split(|&b| b == 0).filter(|f| !f.is_empty());
    while let Some(entry) = fields.next() {
        if entry.len() < 4 {
            continue;
        }
        let xy = [entry[0], entry[1]];
        if matches!(xy[0], b'R' | b'C') {
            fields.next(); // old name
        }
        let name = &entry[3..];
        let is_dir = name.ends_with(b"/");
        let path = rel_path(name.strip_suffix(b"/").unwrap_or(name));
        if is_dir && xy == *b"??" {
            untracked_dirs.insert(path);
        } else {
            files.insert(path, xy);
        }
    }
}

/// Git's `a/b/c` as a relative path for this system.
fn rel_path(name: &[u8]) -> PathBuf {
    name.split(|&b| b == b'/')
        .map(|part| {
            // SAFETY: git writes names as the bytes the system gave it (on
            // Windows, UTF-8): whole components, cut at the ASCII `/`.
            unsafe { OsStr::from_encoded_bytes_unchecked(part) }.to_os_string()
        })
        .collect::<Vec<OsString>>()
        .iter()
        .collect()
}

/// Runs git in tests (with an identity, for commits).
#[cfg(test)]
pub fn git_ok(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=xt", "-c", "user.email=xt@example.com"])
        .args(args)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Repository with `a.txt` committed and changed, `new/b.txt` untracked
    /// and `c.txt` staged.
    fn sample() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path();
        git_ok(p, &["init", "-q"]);
        fs::write(p.join("a.txt"), "one\n").unwrap();
        git_ok(p, &["add", "a.txt"]);
        git_ok(p, &["commit", "-q", "-m", "first"]);
        fs::write(p.join("a.txt"), "one\ntwo\n").unwrap();
        fs::create_dir(p.join("new")).unwrap();
        fs::write(p.join("new/b.txt"), "b").unwrap();
        fs::write(p.join("c.txt"), "c").unwrap();
        git_ok(p, &["add", "c.txt"]);
        tmp
    }

    #[test]
    fn menu_keys() {
        assert_eq!(Action::from_key('D'), Some(Action::Diff));
        assert_eq!(Action::from_key('x'), None);
        for word in MENU {
            let c = word.chars().next().unwrap();
            assert!(Action::from_key(c).is_some(), "{word}");
        }
    }

    #[test]
    fn parses_porcelain() {
        let (mut files, mut dirs) = (HashMap::new(), HashSet::new());
        parse_status(
            b" M src/a.rs\0R  new.rs\0old.rs\0?? tmp/\0A  b c.txt\0",
            &mut files,
            &mut dirs,
        );
        assert_eq!(files[Path::new("src/a.rs")], *b" M");
        assert_eq!(files[Path::new("new.rs")], *b"R ");
        assert_eq!(files[Path::new("b c.txt")], *b"A ");
        assert!(!files.contains_key(Path::new("old.rs")));
        assert!(dirs.contains(Path::new("tmp")));
    }

    #[test]
    fn markers_follow_the_repository() {
        let tmp = sample();
        let root = dunce::canonicalize(tmp.path()).unwrap();
        assert_eq!(repo_root(&root.join("new")), Some(root.clone()));

        let mut cache = StatusCache::default();
        cache.refresh([root.clone()]);
        assert!(cache.in_repo(&root));
        assert_eq!(cache.marker(&root.join("a.txt")), Some(*b" M"));
        assert_eq!(cache.marker(&root.join("c.txt")), Some(*b"A "));
        assert_eq!(cache.marker(&root.join("new/b.txt")), Some(*b"??"));

        change(Action::Add, &root, &[root.join("a.txt")])
            .unwrap()
            .unwrap();
        cache.invalidate();
        std::thread::sleep(MIN_INTERVAL);
        cache.refresh([root.clone()]);
        assert_eq!(cache.marker(&root.join("a.txt")), Some(*b"M "));

        change(Action::Unstage, &root, &[root.join("a.txt")])
            .unwrap()
            .unwrap();
        cache.invalidate();
        std::thread::sleep(MIN_INTERVAL);
        cache.refresh([root.clone()]);
        assert_eq!(cache.marker(&root.join("a.txt")), Some(*b" M"));

        let outside = tempfile::tempdir().unwrap();
        cache.refresh([outside.path().to_path_buf()]);
        assert!(!cache.in_repo(outside.path()));
        assert_eq!(cache.marker(&root.join("a.txt")), None, "no longer shown");
    }

    #[test]
    fn reports() {
        let tmp = sample();
        let root = tmp.path();
        let diff = report(Action::Diff, root, &[root.join("a.txt")]).unwrap();
        assert_eq!(diff.title, Path::new("git diff"));
        assert_eq!(diff.colors_of, "git.diff");
        assert!(String::from_utf8_lossy(&diff.text).contains("+two"));
        let log = report(Action::Log, root, &[]).unwrap();
        assert!(String::from_utf8_lossy(&log.text).contains("first"));
        let blame = report(Action::Blame, root, &[root.join("a.txt")]).unwrap();
        assert!(String::from_utf8_lossy(&blame.text).contains("one"));
        let status = report(Action::Status, root, &[]).unwrap();
        assert!(String::from_utf8_lossy(&status.text).contains("c.txt"));

        let err = change(Action::Add, root, &[root.join("missing")]).unwrap();
        assert!(err.is_err());
    }
}
