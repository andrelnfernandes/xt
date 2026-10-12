//! State that survives between runs: the prompt history, in
//! `~/.local/state/xt/history.toml` (or `$XDG_STATE_HOME/xt/...`).

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub type History = HashMap<String, Vec<String>>;

#[derive(Default, Serialize, Deserialize)]
struct StateFile {
    #[serde(default)]
    history: History,
}

pub fn history_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("state"))
        })?;
    Some(base.join("xt").join("history.toml"))
}

/// Saved history; empty if the file doesn't exist or can't be read.
pub fn load_history(path: &Path) -> History {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str::<StateFile>(&text).ok())
        .map(|s| s.history)
        .unwrap_or_default()
}

/// Saves the history: only the owner can read it (it may contain typed commands)
/// and the replacement is atomic (a temporary file renamed over the old one).
pub fn save_history(path: &Path, history: &History) -> io::Result<()> {
    let text = toml::to_string(&StateFile {
        history: history.clone(),
    })
    .map_err(io::Error::other)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("toml.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn saves_and_loads_privately() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("xt/history.toml");
        assert!(load_history(&path).is_empty());

        let mut history = History::new();
        history.insert("execute".into(), vec!["tar czf 'a b.tgz' %t".into()]);
        history.insert("filespec".into(), vec!["*.rs".into(), "*.md -x\"y".into()]);
        save_history(&path, &history).unwrap();
        assert_eq!(load_history(&path), history);

        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        assert!(!path.with_extension("toml.tmp").exists());
    }

    #[test]
    fn ignores_broken_files() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history.toml");
        fs::write(&path, "not [valid").unwrap();
        assert!(load_history(&path).is_empty());
    }
}
