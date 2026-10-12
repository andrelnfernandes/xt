//! Avisos do sistema (inotify) quando um diretório mostrado muda.
//!
//! Só os diretórios atuais das janelas são observados, sem recursão: assim
//! árvores enormes não esgotam o limite de observações do sistema.

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

    /// Passa a observar exatamente `dirs`.
    pub fn watch_only(&mut self, dirs: HashSet<PathBuf>) {
        for gone in self.watched.difference(&dirs) {
            let _ = self.inner.unwatch(gone);
        }
        for new in dirs.difference(&self.watched) {
            // Sem permissão ou já apagado: simplesmente não é observado.
            let _ = self.inner.watch(new, RecursiveMode::NonRecursive);
        }
        self.watched = dirs;
    }

    /// Diretórios que mudaram desde a última chamada.
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

    /// O diretório observado afetado por uma mudança em `path`: ele mesmo
    /// ou o diretório onde ele está.
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

    /// Junta os avisos até chegar algum, por no máximo 3 s.
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
        let tmp = tempfile::tempdir().unwrap();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();

        let mut w = Watcher::new().unwrap();
        w.watch_only(HashSet::from([a.clone()]));
        std::fs::write(b.join("ignored"), "").unwrap();
        std::fs::write(a.join("new"), "").unwrap();
        assert_eq!(wait_for(&w), HashSet::from([a.clone()]));

        // Trocando o observado, `a` deixa de avisar.
        w.watch_only(HashSet::from([b.clone()]));
        std::fs::write(a.join("again"), "").unwrap();
        std::fs::remove_file(b.join("ignored")).unwrap();
        assert_eq!(wait_for(&w), HashSet::from([b]));
    }
}
