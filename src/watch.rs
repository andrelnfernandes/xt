//! System notifications (inotify) when a displayed directory changes.
//!
//! Only the windows' current directories are watched, non-recursively: this way
//! huge trees don't exhaust the system's watch limit.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};

pub struct Watcher {
    inner: RecommendedWatcher,
    events: Receiver<notify::Result<notify::Event>>,
    watched: HashSet<PathBuf>,
}

impl Watcher {
    pub fn new() -> notify::Result<Self> {
        let (tx, events) = channel();
        Ok(Watcher {
            inner: notify::recommended_watcher(tx)?,
            events,
            watched: HashSet::new(),
        })
    }

    /// Starts watching exactly `dirs`.
    pub fn watch_only(&mut self, dirs: HashSet<PathBuf>) {
        for gone in self.watched.difference(&dirs) {
            let _ = self.inner.unwatch(gone);
        }
        for new in dirs.difference(&self.watched) {
            // No permission or already deleted: it's simply not watched.
            let _ = self.inner.watch(new, RecursiveMode::NonRecursive);
        }
        self.watched = dirs;
    }

    /// Directories that changed since the last call.
    pub fn changed_dirs(&self) -> HashSet<PathBuf> {
        let mut dirs = HashSet::new();
        for event in self.events.try_iter().flatten() {
            if matches!(event.kind, EventKind::Access(_)) {
                continue;
            }
            for path in event.paths {
                dirs.extend(self.dir_of(&path));
            }
        }
        dirs
    }

    /// The watched directory affected by a change at `path`: itself
    /// or the directory it is in.
    fn dir_of(&self, path: &Path) -> Option<PathBuf> {
        if self.watched.contains(path) {
            return Some(path.to_path_buf());
        }
        path.parent()
            .filter(|p| self.watched.contains(*p))
            .map(Path::to_path_buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Gathers notifications until some arrive, for at most 3 s.
    fn wait_for(w: &Watcher) -> HashSet<PathBuf> {
        let start = Instant::now();
        let mut all = HashSet::new();
        while start.elapsed() < Duration::from_secs(3) {
            all.extend(w.changed_dirs());
            if !all.is_empty() {
                return all;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        all
    }

    #[test]
    fn reports_changes_in_watched_dirs_only() {
        let tmp = crate::testing::tempdir();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();

        let mut w = Watcher::new().unwrap();
        w.watch_only(HashSet::from([a.clone()]));
        std::fs::write(b.join("ignored"), "").unwrap();
        std::fs::write(a.join("new"), "").unwrap();
        assert_eq!(wait_for(&w), HashSet::from([a.clone()]));

        // After changing what is watched, `a` stops notifying.
        w.watch_only(HashSet::from([b.clone()]));
        std::fs::write(a.join("again"), "").unwrap();
        std::fs::remove_file(b.join("ignored")).unwrap();
        assert_eq!(wait_for(&w), HashSet::from([b]));
    }
}
