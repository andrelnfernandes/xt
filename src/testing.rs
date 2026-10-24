//! Helpers for the tests.

/// A temporary folder inside the user's own space (`~/.cache/xylem/tests`),
/// not in the shared `/tmp`: there, what other users leave around (a stray
/// `/tmp/.git`) gets in the way. Also outside Xylem's own repository, which
/// the Git tests need.
pub fn tempdir() -> tempfile::TempDir {
    let base = crate::platform::cache_dir()
        .expect("no home directory")
        .join("tests");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::Builder::new()
        .prefix("xy-")
        .tempdir_in(base)
        .unwrap()
}

/// Hides a file or directory: the dot in the name is enough on Linux; on
/// Windows it takes the H attribute.
pub fn hide(path: &std::path::Path) {
    #[cfg(windows)]
    crate::platform::set_mode(path, 0x2).unwrap();
    let _ = path;
}
