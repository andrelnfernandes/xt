//! Git (Ctrl+G, when `git = true` in the configuration): a menu of
//! commands and the status of each file in the file list. Not in XTree or
//! ZTreeWin, so it is off by default.
//!
//! Everything runs the `git` program: it follows the user's own
//! configuration (aliases, hooks, editor) and adds no dependencies.
//!
//! A repository's own `.git` can name commands, and one that was downloaded or
//! extracted comes with its `.git`. Looking at it must not run them, so what
//! only reads goes through `reading`.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// The menu (Ctrl+G), in display order; the first letter chooses.
pub const MENU: &[&str] = &[
    "Status", "Log", "Graph", "Diff", "Blame", "Add", "Unstage", "Commit",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Status,
    Log,
    /// `git log --graph` of the whole repository, in git's colors.
    Graph,
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
            'g' => Action::Graph,
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
            Action::Status | Action::Log | Action::Graph | Action::Diff | Action::Blame
        )
    }
}

/// The repository that contains `dir`: the nearest directory with a `.git`
/// (a directory, or a file in worktrees and submodules).
pub fn repo_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|d| is_git_dir(&d.join(".git")))
        .map(Path::to_path_buf)
}

/// Whether `.git` is really a repository's, as Git checks it: a directory
/// with `HEAD`, or a file pointing to one (`gitdir: ...`). A stray empty
/// `.git` (e.g. `/tmp/.git`, created by someone else) is not.
fn is_git_dir(git: &Path) -> bool {
    if git.is_dir() {
        return git.join("HEAD").is_file();
    }
    std::fs::read(git).is_ok_and(|text| text.starts_with(b"gitdir:"))
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

/// `git` for what only reads: nothing the repository names is run. The hooks
/// run when the index is written, and what would write it is an optional lock;
/// the options of each command do the rest (see `report`). Clean filters cannot
/// be turned off: see `has_own_filters`.
fn reading(root: &Path) -> Command {
    let mut cmd = git(root);
    cmd.arg("--no-optional-locks");
    for option in [
        "core.fsmonitor=false",
        // A format can check signatures too (`format.pretty = %G?`).
        "gpg.program=false",
        "gpg.openpgp.program=false",
        "gpg.x509.program=false",
        "gpg.ssh.program=false",
        // These show a submodule by running git in it.
        "diff.submodule=short",
        "status.submoduleSummary=false",
    ] {
        cmd.args(["-c", option]);
    }
    // A partial clone fetches what it lacks from a remote it names.
    cmd.env("GIT_NO_LAZY_FETCH", "1");
    cmd
}

/// Runs `git args -- paths` and returns whether it worked and what it wrote
/// (stdout, then stderr).
fn run(root: &Path, args: &[&str], paths: &[PathBuf]) -> io::Result<(bool, Vec<u8>)> {
    run_command(git(root), args, paths)
}

/// `run`, for what only reads.
fn run_reading(root: &Path, args: &[&str], paths: &[PathBuf]) -> io::Result<(bool, Vec<u8>)> {
    run_command(reading(root), args, paths)
}

fn run_command(mut cmd: Command, args: &[&str], paths: &[PathBuf]) -> io::Result<(bool, Vec<u8>)> {
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
    run_reading(root, &["rev-parse", "--verify", "-q", "HEAD"], &[]).is_ok_and(|(ok, _)| ok)
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
    // These read the files, and git would pass them through the filters.
    if matches!(action, Action::Status | Action::Diff | Action::Blame) && has_own_filters(root) {
        return Err(io::Error::other(
            "this repository defines filters that git would run; if you trust it, use a terminal",
        ));
    }
    const DIFF: [&str; 3] = [
        "--no-ext-diff",
        "--no-textconv",
        "--ignore-submodules=dirty",
    ];
    let run = run_reading;
    let (title, (_, mut text)) = match action {
        Action::Status => (
            "git status",
            run(root, &["status", "--ignore-submodules=dirty"], paths)?,
        ),
        Action::Log => (
            "git log",
            run(root, &["log", "--no-show-signature", "-n", "1000"], paths)?,
        ),
        // git's own colors (one per branch line), read by the viewer; no
        // gpg even with log.showSignature on.
        Action::Graph => (
            "git log --graph",
            run(
                root,
                &[
                    "log",
                    "--graph",
                    "--decorate",
                    "--oneline",
                    "--all",
                    "--color=always",
                    "--no-show-signature",
                    "-n",
                    "1000",
                ],
                paths,
            )?,
        ),
        // Against HEAD: what is staged and what is not, together.
        Action::Diff if has_head(root) => (
            "git diff",
            run(root, &["diff", DIFF[0], DIFF[1], DIFF[2], "HEAD"], paths)?,
        ),
        Action::Diff => (
            "git diff",
            run(
                root,
                &["diff", DIFF[0], DIFF[1], DIFF[2], "--cached"],
                paths,
            )?,
        ),
        Action::Blame => ("git blame", run(root, &["blame", "--no-textconv"], paths)?),
        _ => unreachable!("{action:?} does not make a report"),
    };
    if text.is_empty() {
        text = b"(nothing to show)\n".to_vec();
    }
    Ok(Report {
        title: title.into(),
        text,
        colors_of: match action {
            Action::Diff => "git.diff",
            Action::Status => crate::highlight::GIT_STATUS,
            Action::Graph => crate::highlight::ANSI,
            _ => "",
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

/// Reads the status by itself, just because the user walked into the
/// directory, so it must not run commands that the repository names: see
/// `reading`. Clean filters cannot be turned off, so a repository that defines
/// its own is left without markers.
fn read_status(root: &Path) -> RepoStatus {
    let mut status = RepoStatus {
        files: HashMap::new(),
        untracked_dirs: HashSet::new(),
        read_at: Instant::now(),
        stale: false,
    };
    if has_own_filters(root) {
        return status;
    }
    let args = [
        "status",
        "--porcelain=v1",
        "-z",
        "--ignore-submodules=dirty",
    ];
    let Ok((true, out)) = run_reading(root, &args, &[]) else {
        return status;
    };
    parse_status(&out, &mut status.files, &mut status.untracked_dirs);
    status
}

/// Whether the repository's own configuration defines filters (the user's
/// global ones, such as Git LFS, are trusted). Asks for all of it with where
/// each came from, since `--local` misses `include.path` and `config.worktree`.
/// In doubt, the answer is yes.
fn has_own_filters(root: &Path) -> bool {
    let out = reading(root)
        .args([
            "config",
            "--show-scope",
            "--name-only",
            "--get-regexp",
            r"^filter\.",
        ])
        .output();
    let Ok(out) = out else { return true };
    match out.status.code() {
        // Git's answer to a search that found nothing.
        Some(1) => false,
        Some(0) => !only_trusted_scopes(&out.stdout),
        _ => true,
    }
}

fn only_trusted_scopes(found: &[u8]) -> bool {
    found
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .all(|line| line.starts_with(b"global\t") || line.starts_with(b"system\t"))
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
        .args([
            "-c",
            "user.name=Xylem",
            "-c",
            "user.email=xylem@example.com",
        ])
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
        let tmp = crate::testing::tempdir();
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
    fn only_the_user_configuration_is_trusted_for_filters() {
        assert!(only_trusted_scopes(b""));
        assert!(only_trusted_scopes(
            b"global\tfilter.lfs.clean\nsystem\tfilter.x.clean\n"
        ));
        for scope in ["local", "worktree", "command", "other"] {
            let found = format!("global\tfilter.lfs.clean\n{scope}\tfilter.x.clean\n");
            assert!(!only_trusted_scopes(found.as_bytes()), "{scope}");
        }
    }

    #[test]
    fn a_stray_git_folder_is_not_a_repository() {
        let tmp = crate::testing::tempdir();
        let root = tmp.path();
        let sub = root.join("sub");
        std::fs::create_dir_all(sub.join(".git")).unwrap();
        assert_eq!(repo_root(&sub), None, "empty .git");
        std::fs::write(sub.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        assert_eq!(repo_root(&sub.join("x")), Some(sub.clone()));

        let worktree = root.join("wt");
        std::fs::create_dir(&worktree).unwrap();
        std::fs::write(worktree.join(".git"), "gitdir: /elsewhere\n").unwrap();
        assert_eq!(repo_root(&worktree), Some(worktree.clone()));
        std::fs::write(worktree.join(".git"), "junk").unwrap();
        assert_eq!(repo_root(&worktree), None);
    }

    #[test]
    fn walking_into_a_repository_runs_none_of_its_commands() {
        let tmp = sample();
        let root = dunce::canonicalize(tmp.path()).unwrap();
        let marker = root.join("ran");
        let touch = format!("touch '{}'", marker.display());
        git_ok(&root, &["config", "core.fsmonitor", &touch]);
        let mut cache = StatusCache::default();
        cache.refresh([root.clone()]);
        assert_eq!(cache.marker(&root.join("a.txt")), Some(*b" M"));

        git_ok(&root, &["config", "filter.evil.clean", &touch]);
        fs::write(root.join(".gitattributes"), "* filter=evil\n").unwrap();
        cache.invalidate();
        std::thread::sleep(MIN_INTERVAL);
        cache.refresh([root.clone()]);
        assert_eq!(cache.marker(&root.join("a.txt")), None, "not read");
        assert!(!marker.exists());
    }

    /// A repository whose `a.txt` has changed, with a command that would leave
    /// a marker if something ran it.
    #[cfg(unix)]
    struct Hostile {
        _tmp: tempfile::TempDir,
        root: PathBuf,
        marker: PathBuf,
        touch: String,
        /// Files that git has to read to know whether they changed.
        changed: Vec<PathBuf>,
        tick: std::cell::Cell<u64>,
    }

    #[cfg(unix)]
    impl Hostile {
        fn new() -> Self {
            let tmp = crate::testing::tempdir();
            let base = dunce::canonicalize(tmp.path()).unwrap();
            let root = base.join("repo");
            let marker = base.join("ran");
            let touch = format!("touch '{}'", marker.display());
            let hostile = Hostile {
                _tmp: tmp,
                changed: vec![root.join("a.txt")],
                root,
                marker,
                touch,
                tick: std::cell::Cell::new(0),
            };
            hostile.commit_a_file(&hostile.root);
            fs::write(hostile.root.join("a.txt"), "two\n").unwrap();
            hostile
        }

        fn commit_a_file(&self, dir: &Path) {
            fs::create_dir_all(dir).unwrap();
            git_ok(dir, &["init", "-q"]);
            fs::write(dir.join("a.txt"), "one\n").unwrap();
            git_ok(dir, &["add", "a.txt"]);
            git_ok(dir, &["commit", "-q", "-m", "first"]);
        }

        fn config(&self, key: &str, value: &str) {
            git_ok(&self.root, &["config", key, value]);
        }

        fn run_when(&self, key: &str) {
            self.config(key, &self.touch);
        }

        fn script(&self, path: &Path, body: &str) {
            use std::os::unix::fs::PermissionsExt;
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        /// Commits with `sign` as the signing program. It was just written: while
        /// another test's thread forks, running it can fail as "text file busy".
        fn sign_a_commit(&self, sign: &Path) {
            let program = format!("gpg.program={}", sign.display());
            let signed = || {
                Command::new("git")
                    .arg("-C")
                    .arg(&self.root)
                    .args([
                        "-c",
                        "user.name=Xylem",
                        "-c",
                        "user.email=xylem@example.com",
                    ])
                    .args(["-c", &program, "-c", "commit.gpgsign=true"])
                    .args(["commit", "-q", "--allow-empty", "-m", "signed"])
                    .output()
                    .unwrap()
                    .status
                    .success()
            };
            let deadline = Instant::now() + Duration::from_secs(10);
            while !signed() {
                assert!(Instant::now() < deadline, "the commit was not signed");
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        /// A new date on each changed file, with the same size, so that git
        /// must read the contents again.
        fn touch_changed(&self) {
            self.tick.set(self.tick.get() + 1);
            let date = std::time::SystemTime::now() + Duration::from_secs(self.tick.get());
            for file in &self.changed {
                let file = fs::OpenOptions::new().write(true).open(file).unwrap();
                file.set_modified(date).unwrap();
            }
        }

        /// The reads that ran the command.
        fn reads_that_ran_it(&self) -> Vec<&'static str> {
            let a = self.root.join("a.txt");
            let root = self.root.as_path();
            let reads: [(&str, &dyn Fn()); 6] = [
                ("markers", &|| {
                    StatusCache::default().refresh([root.to_path_buf()])
                }),
                ("status", &|| drop(report(Action::Status, root, &[]))),
                ("log", &|| drop(report(Action::Log, root, &[]))),
                ("graph", &|| drop(report(Action::Graph, root, &[]))),
                ("diff", &|| {
                    drop(report(Action::Diff, root, std::slice::from_ref(&a)))
                }),
                ("blame", &|| {
                    drop(report(Action::Blame, root, std::slice::from_ref(&a)))
                }),
            ];
            let mut ran = Vec::new();
            for (name, read) in reads {
                let _ = fs::remove_file(&self.marker);
                self.touch_changed();
                read();
                if self.marker.exists() {
                    ran.push(name);
                }
            }
            ran
        }
    }

    #[cfg(unix)]
    #[test]
    fn no_read_runs_what_the_configuration_names() {
        let cases: [(&str, &str); 3] = [
            ("core.fsmonitor", ""),
            ("diff.external", ""),
            ("diff.evil.command", "* diff=evil\n"),
        ];
        for (key, attributes) in cases {
            let hostile = Hostile::new();
            hostile.run_when(key);
            fs::write(hostile.root.join(".gitattributes"), attributes).unwrap();
            assert_eq!(hostile.reads_that_ran_it(), [""; 0], "{key}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn no_read_runs_a_text_conversion() {
        let hostile = Hostile::new();
        let convert = hostile.root.join(".git/convert");
        let body = format!("{}\ncat \"$1\"", hostile.touch);
        hostile.script(&convert, &body);
        hostile.config("diff.evil.textconv", convert.to_str().unwrap());
        fs::write(hostile.root.join(".gitattributes"), "* diff=evil\n").unwrap();
        assert_eq!(hostile.reads_that_ran_it(), [""; 0]);
    }

    #[cfg(unix)]
    #[test]
    fn no_read_checks_a_signature_with_the_repository_program() {
        for (key, value) in [("log.showSignature", "true"), ("format.pretty", "%G?")] {
            let hostile = Hostile::new();
            let sign = hostile.root.join(".git/sign");
            hostile.script(
                &sign,
                "echo '[GNUPG:] SIG_CREATED x' >&2\n\
                 echo '-----BEGIN PGP SIGNATURE-----'\n\
                 echo x\n\
                 echo '-----END PGP SIGNATURE-----'",
            );
            hostile.sign_a_commit(&sign);
            let check = hostile.root.join(".git/check");
            hostile.script(&check, &hostile.touch);
            hostile.config("gpg.program", check.to_str().unwrap());
            hostile.config(key, value);
            assert_eq!(hostile.reads_that_ran_it(), [""; 0], "{key}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn no_read_runs_a_hook() {
        let hostile = Hostile::new();
        let hook = hostile.root.join(".git/hooks/post-index-change");
        hostile.script(&hook, &hostile.touch);
        assert_eq!(hostile.reads_that_ran_it(), [""; 0], "its usual place");

        let hostile = Hostile::new();
        let hook = hostile.root.join("hooks/post-index-change");
        hostile.script(&hook, &hostile.touch);
        hostile.config("core.hooksPath", "hooks");
        assert_eq!(hostile.reads_that_ran_it(), [""; 0], "core.hooksPath");
    }

    /// Filters may hide in other files of the repository's configuration.
    #[cfg(unix)]
    #[test]
    fn no_read_runs_a_filter_of_the_repository() {
        let filter = |hostile: &Hostile| {
            fs::write(hostile.root.join(".gitattributes"), "* filter=evil\n").unwrap();
            format!("{}; cat", hostile.touch)
        };

        let hostile = Hostile::new();
        let clean = filter(&hostile);
        hostile.config("filter.evil.clean", &clean);
        assert_eq!(hostile.reads_that_ran_it(), [""; 0], ".git/config");

        let hostile = Hostile::new();
        let clean = filter(&hostile);
        let more = format!("[filter \"evil\"]\n\tclean = {clean}\n");
        fs::write(hostile.root.join(".git/more"), more).unwrap();
        hostile.config("include.path", "more");
        assert_eq!(hostile.reads_that_ran_it(), [""; 0], "an included file");

        let hostile = Hostile::new();
        let clean = filter(&hostile);
        hostile.config("extensions.worktreeConfig", "true");
        git_ok(
            &hostile.root,
            &["config", "--worktree", "filter.evil.clean", &clean],
        );
        assert_eq!(hostile.reads_that_ran_it(), [""; 0], "config.worktree");
    }

    #[cfg(unix)]
    #[test]
    fn a_repository_with_its_own_filters_says_why_it_shows_nothing() {
        let hostile = Hostile::new();
        hostile.config("filter.evil.clean", "cat");
        let (root, a) = (hostile.root.as_path(), hostile.root.join("a.txt"));
        for (action, paths) in [
            (Action::Status, &[][..]),
            (Action::Diff, std::slice::from_ref(&a)),
            (Action::Blame, std::slice::from_ref(&a)),
        ] {
            let error = report(action, root, paths).err().expect("no report");
            assert!(error.to_string().contains("filters"), "{error}");
        }
        assert!(report(Action::Log, root, &[]).is_ok());
        assert!(report(Action::Graph, root, &[]).is_ok());
    }

    /// `hostile` with a submodule `sub`, whose `a.txt` changed.
    #[cfg(unix)]
    fn with_submodule(hostile: &mut Hostile) -> PathBuf {
        let source = hostile.root.with_file_name("source");
        hostile.commit_a_file(&source);
        git_ok(
            &hostile.root,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "--quiet",
                "add",
                source.to_str().unwrap(),
                "sub",
            ],
        );
        git_ok(&hostile.root, &["commit", "-q", "-m", "sub"]);
        let sub = hostile.root.join("sub");
        fs::write(sub.join("a.txt"), "two\n").unwrap();
        hostile.changed.push(sub.join("a.txt"));
        sub
    }

    #[cfg(unix)]
    #[test]
    fn no_read_runs_what_a_submodule_names() {
        let mut hostile = Hostile::new();
        let sub = with_submodule(&mut hostile);
        let clean = format!("{}; cat", hostile.touch);
        git_ok(&sub, &["config", "filter.evil.clean", &clean]);
        fs::write(sub.join(".gitattributes"), "* filter=evil\n").unwrap();
        assert_eq!(hostile.reads_that_ran_it(), [""; 0], "a filter");

        let mut hostile = Hostile::new();
        with_submodule(&mut hostile);
        let hook = hostile
            .root
            .join(".git/modules/sub/hooks/post-index-change");
        hostile.script(&hook, &hostile.touch);
        assert_eq!(hostile.reads_that_ran_it(), [""; 0], "a hook");

        // Moved to another commit, which its repository shows if asked to.
        let mut hostile = Hostile::new();
        let sub = with_submodule(&mut hostile);
        git_ok(&sub, &["commit", "-q", "-am", "second"]);
        git_ok(&sub, &["config", "diff.external", &hostile.touch]);
        hostile.changed.pop();
        hostile.config("diff.submodule", "diff");
        hostile.config("status.submoduleSummary", "true");
        assert_eq!(hostile.reads_that_ran_it(), [""; 0], "its diff");
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

        let outside = crate::testing::tempdir();
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
        let graph = report(Action::Graph, root, &[]).unwrap();
        assert_eq!(graph.colors_of, crate::highlight::ANSI);
        let text = String::from_utf8_lossy(&graph.text);
        assert!(text.starts_with('*') && text.contains("first"), "{text}");
        assert!(text.contains('\u{1b}'), "git's colors");
        let blame = report(Action::Blame, root, &[root.join("a.txt")]).unwrap();
        assert!(String::from_utf8_lossy(&blame.text).contains("one"));
        let status = report(Action::Status, root, &[]).unwrap();
        assert!(String::from_utf8_lossy(&status.text).contains("c.txt"));
        assert_eq!(status.colors_of, crate::highlight::GIT_STATUS);

        let err = change(Action::Add, root, &[root.join("missing")]).unwrap();
        assert!(err.is_err());
    }
}
