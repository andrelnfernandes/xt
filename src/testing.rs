//! Helpers for the tests.

/// A temporary folder inside the user's own space (`~/.cache/xt/tests`),
/// not in the shared `/tmp`: there, what other users leave around (a stray
/// `/tmp/.git`) gets in the way. Also outside xt's own repository, which
/// the Git tests need.
pub fn tempdir() -> tempfile::TempDir {
    let base = crate::platform::cache_dir()
        .expect("no home directory")
        .join("tests");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::Builder::new()
        .prefix("xt-")
        .tempdir_in(base)
        .unwrap()
}
