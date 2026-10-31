use super::*;
use crate::testing::hide;
use ratatui::crossterm::event::KeyModifiers;
use std::fs::{create_dir_all, write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// root/{a.rs, b.txt}, root/sub/{c.rs}, root/sub/deep/{a.rs}
fn sample() -> (tempfile::TempDir, App) {
    let tmp = crate::testing::tempdir();
    let p = tmp.path();
    create_dir_all(p.join("sub/deep")).unwrap();
    write(p.join("a.rs"), "fn main() {}\n").unwrap();
    write(p.join("b.txt"), "b").unwrap();
    write(p.join("sub/c.rs"), "c").unwrap();
    write(p.join("sub/deep/a.rs"), "deep").unwrap();
    let app = App::new(p.to_path_buf());
    (tmp, app)
}

fn names(app: &App) -> Vec<String> {
    app.files
        .iter()
        .map(|&f| app.file(f).name.to_string_lossy().into_owned())
        .collect()
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

#[test]
fn left_on_leaf_goes_to_parent() {
    let (_tmp, mut app) = sample();
    app.apply(Command::Down);
    assert_eq!(app.tree_state.selected(), Some(1));
    app.apply(Command::Collapse);
    app.apply(Command::Collapse);
    assert_eq!(app.tree_state.selected(), Some(0));
}

#[test]
fn filespec_prompt_filters_files() {
    let (_tmp, mut app) = sample();
    assert_eq!(names(&app), ["a.rs", "b.txt"]);

    press(&mut app, KeyCode::Char('f'));
    press(&mut app, KeyCode::Backspace);
    type_text(&mut app, "*.rs");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.filespec.text(), "*.rs");
    assert_eq!(names(&app), ["a.rs"]);

    // Invalid pattern: keeps the previous one and shows the error.
    press(&mut app, KeyCode::Char('f'));
    type_text(&mut app, " [");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.filespec.text(), "*.rs");
    assert!(app.message.is_some());
}

#[test]
fn showall_lists_logged_files_sorted() {
    let (_tmp, mut app) = sample();
    app.apply(Command::Showall);
    assert_eq!(app.focus, Focus::Files);
    // "deep" has not been logged yet.
    assert_eq!(names(&app), ["a.rs", "b.txt", "c.rs"]);
}

#[test]
fn branch_logs_whole_branch() {
    let (_tmp, mut app) = sample();
    app.apply(Command::Branch);
    assert!(app.job.is_none());
    assert_eq!(app.view, FileView::Branch(ROOT));
    assert_eq!(names(&app), ["a.rs", "a.rs", "b.txt", "c.rs"]);
}

#[test]
fn leaving_showall_selects_file_dir() {
    let (_tmp, mut app) = sample();
    app.apply(Command::Branch);
    app.apply(Command::Down); // second a.rs, in sub/deep
    let dir = app.selected_file().unwrap().dir;
    assert_ne!(dir, ROOT);
    assert_eq!(app.header_dir(), dir);

    press(&mut app, KeyCode::Esc);
    assert_eq!(app.focus, Focus::Tree);
    assert_eq!(app.view, FileView::Dir);
    assert_eq!(app.current(), dir);
    assert_eq!(names(&app), ["a.rs"]);
}

#[test]
fn expand_all_runs_as_job() {
    let (_tmp, mut app) = sample();
    app.apply(Command::ExpandAll);
    assert!(app.job.is_none());
    assert_eq!(app.rows.len(), 3);
}

#[test]
fn viewer_opens_and_closes() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('v'));
    let viewer = app.viewer.as_ref().expect("viewer open");
    assert_eq!(viewer.visible_lines(1)[0], "fn main() {}");
    press(&mut app, KeyCode::Esc);
    assert!(app.viewer.is_none());
    assert_eq!(app.focus, Focus::Files);
}

fn ctrl(app: &mut App, c: char) {
    app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}

/// Answers an open prompt, replacing the suggested text.
fn answer_prompt(app: &mut App, text: &str) {
    assert!(app.prompt.is_some(), "prompt should be open");
    ctrl(app, 'u');
    type_text(app, text);
    press(app, KeyCode::Enter);
    // Finishes the operation, if any.
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
}

fn tagged(app: &App) -> Vec<String> {
    app.files
        .iter()
        .filter(|&&f| app.file(f).tagged)
        .map(|&f| app.file(f).name.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn tag_untag_and_invert() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('t'));
    assert_eq!(tagged(&app), ["a.rs"]);
    assert_eq!(app.file_state.selected(), Some(1), "cursor moves down");
    press(&mut app, KeyCode::Char('i'));
    assert_eq!(tagged(&app), ["b.txt"]);
    ctrl(&mut app, 't');
    assert_eq!(tagged(&app).len(), 2);
    ctrl(&mut app, 'u');
    assert!(tagged(&app).is_empty());

    // In the tree, T tags the whole current directory.
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('t'));
    assert_eq!(tagged(&app).len(), 2);
}

#[test]
fn ctrl_t_in_tree_tags_all_logged_matching_files() {
    let (_tmp, mut app) = sample();
    app.filespec = Filespec::parse("*.rs").unwrap();
    ctrl(&mut app, 't');
    app.apply(Command::Showall);
    app.filespec = Filespec::default();
    app.refresh_files();
    assert_eq!(tagged(&app), ["a.rs", "c.rs"]);
}

fn logged_tagged(app: &App) -> usize {
    app.logged_matching_files()
        .iter()
        .filter(|&&f| app.file(f).tagged)
        .count()
}

#[test]
fn minus_releases_the_branch_and_its_tags() {
    let (_tmp, mut app) = sample();
    app.apply(Command::ExpandAll);
    ctrl(&mut app, 't');
    assert_eq!(logged_tagged(&app), 4);

    press(&mut app, KeyCode::Down); // sub
    press(&mut app, KeyCode::Char('-'));
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .ends_with("(2 tags dropped)")
    );
    assert_eq!(app.tree_state.selected(), Some(1), "cursor stays on sub");
    assert!(tagged(&app).is_empty(), "sub/c.rs was read again");
    assert_eq!(app.logged_matching_files().len(), 3, "sub/deep unlogged");
    assert_eq!(logged_tagged(&app), 2, "a.rs and b.txt keep their tags");

    // Ctrl+T reaches only what is logged; * logs sub/deep again.
    press(&mut app, KeyCode::Char('*'));
    assert_eq!(app.logged_matching_files().len(), 4);
    assert_eq!(logged_tagged(&app), 2);
}

#[test]
fn left_only_collapses_and_keeps_tags() {
    let (_tmp, mut app) = sample();
    app.apply(Command::ExpandAll);
    ctrl(&mut app, 't');
    press(&mut app, KeyCode::Down); // sub
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Right);
    assert_eq!(logged_tagged(&app), 4);
}

#[test]
fn ctrl_s_in_tree_shows_only_the_tagged_files() {
    let (_tmp, mut app) = sample();
    ctrl(&mut app, 's');
    assert_eq!(
        app.message.as_deref(),
        Some("No tagged files match the filespec")
    );
    assert_eq!(app.focus, Focus::Tree);

    app.apply(Command::ExpandAll);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('t')); // a.rs
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::End); // sub/deep
    press(&mut app, KeyCode::Char('t')); // sub/deep/a.rs
    ctrl(&mut app, 's');
    assert_eq!(app.view, FileView::Showall);
    assert!(app.tags_only);
    assert_eq!(names(&app), ["a.rs", "a.rs"]);

    // Untagged files stay until the list is rebuilt, as in ZTreeWin.
    press(&mut app, KeyCode::Char('u'));
    assert_eq!(names(&app).len(), 2);
    app.refresh_files();
    assert_eq!(names(&app), ["a.rs"]);
    // With no tags left, the list goes back to all the files.
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Char('u'));
    app.refresh_files();
    assert!(!app.tags_only);
    assert_eq!(names(&app).len(), 4);

    press(&mut app, KeyCode::Esc);
    assert!(!app.tags_only && app.view == FileView::Dir);
}

#[test]
fn copy_tagged_files_to_directory() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    ctrl(&mut app, 't');
    ctrl(&mut app, 'c');
    let dest = tmp.path().join("sub");
    answer_prompt(&mut app, dest.to_str().unwrap());

    assert!(dest.join("a.rs").exists() && dest.join("b.txt").exists());
    assert!(app.message.as_deref().unwrap().starts_with("Copied 2 of 2"));
    // The tree already shows the new files in sub/.
    let sub = app.tree.find(&dest).unwrap();
    assert_eq!(app.tree.node(sub).files.len(), 3);
    // Tags remain at the source.
    assert_eq!(tagged(&app).len(), 2);
}

#[test]
fn copy_to_missing_folder_asks_to_create_it() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    ctrl(&mut app, 't');
    ctrl(&mut app, 'c');
    let dest = tmp.path().join("new").join("deeper");
    answer_prompt(&mut app, dest.to_str().unwrap());
    let question = &app.confirm.as_ref().expect("confirmation").question;
    assert_eq!(
        *question,
        format!("Create the missing folder {}?", dest.display())
    );
    assert!(!dest.exists());

    press(&mut app, KeyCode::Char('y'));
    assert!(dest.join("a.rs").exists() && dest.join("b.txt").exists());
    assert!(app.message.as_deref().unwrap().starts_with("Copied 2 of 2"));
}

#[test]
fn declining_to_create_the_folder_copies_nothing() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('c'));
    // One file into a missing folder: not a new name, since the parent
    // does not exist either.
    let dest = tmp.path().join("nope").join("x.rs");
    answer_prompt(&mut app, dest.to_str().unwrap());
    assert!(app.confirm.is_some());
    press(&mut app, KeyCode::Char('n'));
    assert!(app.op.is_none() && !tmp.path().join("nope").exists());

    // A new name in an existing folder is still a plain copy.
    press(&mut app, KeyCode::Char('c'));
    answer_prompt(&mut app, tmp.path().join("copy.rs").to_str().unwrap());
    assert!(app.confirm.is_none());
    assert!(tmp.path().join("copy.rs").exists());
}

#[test]
fn move_with_conflict_prompt() {
    let (tmp, mut app) = sample();
    write(tmp.path().join("sub/b.txt"), "old").unwrap();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('m'));
    answer_prompt(&mut app, "sub"); // relative to the current directory
    assert!(app.op.as_ref().unwrap().conflict.is_some());

    press(&mut app, KeyCode::Char('y'));
    assert!(app.op.is_none());
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("sub/b.txt")).unwrap(),
        "b"
    );
    assert_eq!(names(&app), ["a.rs"]);
}

#[test]
fn delete_asks_for_confirmation() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('d'));
    assert!(app.confirm.is_some());
    press(&mut app, KeyCode::Char('n'));
    assert!(tmp.path().join("a.rs").exists());

    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('y'));
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    assert!(!tmp.path().join("a.rs").exists());
    assert_eq!(names(&app), ["b.txt"]);
}

#[test]
fn delete_to_trash_and_oops_restore() {
    let (tmp, mut app) = sample();
    app.options.delete_to_trash = true;
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('d'));
    assert!(app.confirm.is_some());
    let prompt_q = app.confirm.as_ref().unwrap().question.clone();
    assert!(prompt_q.starts_with("MOVE a.rs to trash?"));
    press(&mut app, KeyCode::Char('y'));
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    assert!(!tmp.path().join("a.rs").exists());
    assert_eq!(app.message.as_deref(), Some("Trashed 1 of 1 files"));

    // Oops: restores the file
    ctrl(&mut app, 'z');
    assert!(app.confirm.is_some());
    let restore_q = app.confirm.as_ref().unwrap().question.clone();
    assert!(restore_q.starts_with("RESTORE a.rs from trash?"));
    press(&mut app, KeyCode::Char('y'));
    assert!(tmp.path().join("a.rs").exists());
    assert_eq!(app.message.as_deref(), Some("Restored a.rs from trash"));
}

#[test]
fn delete_prompt_toggles_between_trash_and_permanent() {
    let (tmp, mut app) = sample();
    app.options.delete_to_trash = false;
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('d'));
    assert!(
        app.confirm
            .as_ref()
            .unwrap()
            .question
            .starts_with("DELETE a.rs?")
    );
    press(&mut app, KeyCode::Char('t'));
    assert!(
        app.confirm
            .as_ref()
            .unwrap()
            .question
            .starts_with("MOVE a.rs to trash?")
    );
    press(&mut app, KeyCode::Char('p'));
    assert!(
        app.confirm
            .as_ref()
            .unwrap()
            .question
            .starts_with("DELETE a.rs?")
    );
    press(&mut app, KeyCode::Char('n'));
    assert!(tmp.path().join("a.rs").exists());
}

#[test]
fn shift_delete_permanently_deletes_even_when_delete_to_trash_is_true() {
    let (tmp, mut app) = sample();
    app.options.delete_to_trash = true;
    press(&mut app, KeyCode::Enter);
    app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
    assert!(app.confirm.is_some());
    assert!(
        app.confirm
            .as_ref()
            .unwrap()
            .question
            .starts_with("DELETE a.rs?")
    );
    press(&mut app, KeyCode::Char('y'));
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    assert!(!tmp.path().join("a.rs").exists());
    assert_eq!(app.message.as_deref(), Some("Deleted 1 of 1 files"));
}

#[test]
fn delete_empty_directory_to_trash_and_oops_restore() {
    let (tmp, mut app) = sample();
    app.options.delete_to_trash = true;
    press(&mut app, KeyCode::Char('m'));
    answer_prompt(&mut app, "empty_dir");
    assert!(tmp.path().join("empty_dir").is_dir());

    press(&mut app, KeyCode::Down);
    assert_eq!(app.tree.node(app.current()).name, "empty_dir");
    press(&mut app, KeyCode::Char('d'));
    assert!(
        app.confirm
            .as_ref()
            .unwrap()
            .question
            .starts_with("MOVE directory empty_dir to trash?")
    );
    press(&mut app, KeyCode::Char('y'));
    assert!(!tmp.path().join("empty_dir").exists());

    // Oops in tree window ('o')
    press(&mut app, KeyCode::Char('o'));
    assert!(
        app.confirm
            .as_ref()
            .unwrap()
            .question
            .starts_with("RESTORE empty_dir from trash?")
    );
    press(&mut app, KeyCode::Char('y'));
    assert!(tmp.path().join("empty_dir").exists());
}

#[test]
fn tagged_ops_without_tags_warn() {
    let (_tmp, mut app) = sample();
    ctrl(&mut app, 'd');
    assert!(app.confirm.is_none());
    assert_eq!(app.message.as_deref(), Some("No tagged files"));
}

#[test]
fn rename_file_keeps_cursor_on_it() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('r'));
    answer_prompt(&mut app, "z.rs");
    assert!(tmp.path().join("z.rs").exists());
    assert_eq!(names(&app), ["b.txt", "z.rs"]);
    assert_eq!(app.file_state.selected(), Some(1));

    press(&mut app, KeyCode::Char('r'));
    answer_prompt(&mut app, "b.txt");
    assert_eq!(app.message.as_deref(), Some("b.txt already exists"));
}

#[test]
fn make_rename_and_delete_directory() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Char('m'));
    answer_prompt(&mut app, "new");
    assert!(tmp.path().join("new").is_dir());
    assert!(app.rows.iter().any(|r| app.tree.node(r.id).name == "new"));

    // Cursor on "new" (order: new, sub).
    press(&mut app, KeyCode::Down);
    assert_eq!(app.tree.node(app.current()).name, "new");
    press(&mut app, KeyCode::Char('r'));
    answer_prompt(&mut app, "renamed");
    assert!(tmp.path().join("renamed").is_dir());
    assert_eq!(app.tree.node(app.current()).name, "renamed");

    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('y'));
    assert!(!tmp.path().join("renamed").exists());
    assert_eq!(app.current(), ROOT);

    // A directory with contents is not deleted.
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('d'));
    assert!(app.confirm.is_none());
    assert_eq!(app.message.as_deref(), Some("Directory is not empty"));
}

#[cfg(unix)]
#[test]
fn chmod_tagged_files() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    ctrl(&mut app, 't');
    ctrl(&mut app, 'a');
    answer_prompt(&mut app, "600");
    for f in ["a.rs", "b.txt"] {
        let mode = std::fs::metadata(tmp.path().join(f))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    assert_eq!(app.file(app.files[0]).mode & 0o777, 0o600);

    ctrl(&mut app, 'a');
    answer_prompt(&mut app, "999");
    assert!(app.message.as_deref().unwrap().contains("octal"));
}

#[test]
fn relog_picks_up_external_changes() {
    let (tmp, mut app) = sample();
    write(tmp.path().join("new.txt"), "").unwrap();
    assert_eq!(names(&app).len(), 2);
    press(&mut app, KeyCode::F(2));
    assert_eq!(names(&app), ["a.rs", "b.txt", "new.txt"]);
}

fn alt(app: &mut App, c: char) {
    app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT));
}

/// Tags the two a.rs of the root's Branch (root and sub/deep).
fn tag_both_a_rs(tmp: &tempfile::TempDir, app: &mut App) -> PathBuf {
    app.apply(Command::Branch);
    app.filespec = Filespec::parse("a.rs").unwrap();
    app.refresh_files();
    ctrl(app, 't');
    let dest = tmp.path().join("out");
    std::fs::create_dir(&dest).unwrap();
    dest
}

#[test]
fn alt_copy_recreates_paths() {
    let (tmp, mut app) = sample();
    app.options.copy_paths = CopyPaths::Relative;
    let dest = tag_both_a_rs(&tmp, &mut app);
    alt(&mut app, 'c');
    assert!(
        app.prompt
            .as_ref()
            .unwrap()
            .label
            .contains("with paths: relative")
    );
    answer_prompt(&mut app, dest.to_str().unwrap());

    assert!(dest.join("a.rs").exists());
    assert_eq!(
        std::fs::read_to_string(dest.join("sub/deep/a.rs")).unwrap(),
        "deep"
    );
    // The destination shows up in the tree and, when expanded, the created folders.
    let out = app.tree.find(&dest).expect("dest in tree");
    app.tree.expand(out);
    assert!(app.tree.find(&dest.join("sub")).is_some());
}

/// Alt+C of `sub/deep/a.rs` (listed from `sub/deep`) in each mode; F7
/// cycles them.
#[test]
fn alt_copy_path_modes() {
    let (tmp, mut app) = sample();
    let root = tmp.path().to_path_buf();
    let deep = root.join("sub/deep");
    let dest = root.join("out");
    std::fs::create_dir(&dest).unwrap();
    // Full: from the root of the disk.
    let full = deep
        .strip_prefix(deep.ancestors().last().unwrap())
        .unwrap()
        .to_path_buf();
    let expected = [
        (CopyPaths::Full, full.join("a.rs")),
        (CopyPaths::Tree, PathBuf::from("sub/deep/a.rs")),
        (CopyPaths::Current, PathBuf::from("deep/a.rs")),
        (CopyPaths::Relative, PathBuf::from("a.rs")),
    ];
    assert!(app.goto(&deep));
    press(&mut app, KeyCode::Enter);
    for (i, (mode, path)) in expected.iter().enumerate() {
        ctrl(&mut app, 't');
        alt(&mut app, 'c');
        // F7 from Full to this mode.
        for _ in 0..i {
            press(&mut app, KeyCode::F(7));
        }
        let label = &app.prompt.as_ref().unwrap().label;
        assert!(
            label.contains(&format!("with paths: {}", mode.as_str())),
            "{label}"
        );
        answer_prompt(&mut app, dest.to_str().unwrap());
        while app.busy() {
            app.tick(std::time::Duration::from_millis(50));
        }
        assert!(dest.join(path).exists(), "{mode:?}: {}", path.display());
        std::fs::remove_dir_all(&dest).unwrap();
        std::fs::create_dir(&dest).unwrap();
    }

    // The configured mode is where the prompt starts; F7 wraps around.
    app.options.copy_paths = CopyPaths::Current;
    ctrl(&mut app, 't');
    alt(&mut app, 'c');
    assert!(
        app.prompt
            .as_ref()
            .unwrap()
            .label
            .contains("paths: current")
    );
    press(&mut app, KeyCode::F(7));
    press(&mut app, KeyCode::F(7));
    assert!(app.prompt.as_ref().unwrap().label.contains("paths: full"));
    // Without paths, F7 turns them back on.
    press(&mut app, KeyCode::F(6));
    assert!(app.prompt.as_ref().unwrap().label.contains("no paths"));
    press(&mut app, KeyCode::F(7));
    assert!(app.prompt.as_ref().unwrap().label.contains("paths: full"));
}

#[test]
fn ctrl_copy_is_flat_and_f6_toggles() {
    let (tmp, mut app) = sample();
    let dest = tag_both_a_rs(&tmp, &mut app);
    ctrl(&mut app, 'c');
    assert!(app.prompt.as_ref().unwrap().label.contains("no paths"));
    press(&mut app, KeyCode::F(6));
    assert!(app.prompt.as_ref().unwrap().label.contains("with paths"));
    press(&mut app, KeyCode::F(6));
    answer_prompt(&mut app, dest.to_str().unwrap());

    // Two a.rs going to the same place: the second is a conflict.
    assert!(app.op.as_ref().unwrap().conflict.is_some());
    press(&mut app, KeyCode::Char('n'));
    assert!(dest.join("a.rs").exists());
    assert!(!dest.join("sub").exists());
}

#[test]
fn alt_move_with_paths_from_branch_root() {
    let (tmp, mut app) = sample();
    app.options.copy_paths = CopyPaths::Relative;
    // Branch from sub/: paths relative to sub.
    press(&mut app, KeyCode::Down);
    app.apply(Command::Branch);
    ctrl(&mut app, 't');
    let dest = tmp.path().join("out");
    std::fs::create_dir(&dest).unwrap();
    alt(&mut app, 'm');
    // Relative to the tree cursor's directory (sub), not the file's.
    answer_prompt(&mut app, "../out");
    // The next prompt, from another directory, suggests the already-resolved destination.
    app.apply(Command::Back);
    app.select_node(ROOT);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('c'));
    assert_eq!(
        app.prompt.as_ref().unwrap().input.text,
        dest.to_str().unwrap()
    );

    assert!(dest.join("c.rs").exists());
    assert!(dest.join("deep/a.rs").exists());
    assert!(!tmp.path().join("sub/deep/a.rs").exists());
}

fn modifier(app: &mut App, m: ModifierKeyCode, kind: KeyEventKind) {
    let mut key = KeyEvent::new(KeyCode::Modifier(m), KeyModifiers::NONE);
    key.kind = kind;
    app.handle_key(key);
}

#[test]
fn holding_ctrl_or_alt_switches_menu() {
    let (_tmp, mut app) = sample();
    assert_eq!(app.menu_mode(), MenuMode::Normal);
    modifier(&mut app, ModifierKeyCode::LeftControl, KeyEventKind::Press);
    assert_eq!(app.menu_mode(), MenuMode::Ctrl);
    modifier(&mut app, ModifierKeyCode::LeftControl, KeyEventKind::Repeat);
    assert_eq!(app.menu_mode(), MenuMode::Ctrl);
    modifier(
        &mut app,
        ModifierKeyCode::LeftControl,
        KeyEventKind::Release,
    );
    assert_eq!(app.menu_mode(), MenuMode::Normal);

    modifier(&mut app, ModifierKeyCode::RightAlt, KeyEventKind::Press);
    assert_eq!(app.menu_mode(), MenuMode::Alt);
    // Lost release: the next key without Alt goes back to normal.
    press(&mut app, KeyCode::Down);
    assert_eq!(app.menu_mode(), MenuMode::Normal);
}

#[test]
fn release_events_do_nothing() {
    let (_tmp, mut app) = sample();
    let mut key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    key.kind = KeyEventKind::Release;
    app.handle_key(key);
    assert_eq!(app.tree_state.selected(), Some(0));
    key.kind = KeyEventKind::Repeat;
    app.handle_key(key);
    assert_eq!(app.tree_state.selected(), Some(1), "held arrows repeat");
}

#[test]
fn f9_cycles_menus_and_applies_modifier_once() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::F(9));
    assert_eq!(app.menu_mode(), MenuMode::Ctrl);
    press(&mut app, KeyCode::F(9));
    assert_eq!(app.menu_mode(), MenuMode::Alt);
    press(&mut app, KeyCode::F(9));
    assert_eq!(app.menu_mode(), MenuMode::Normal);

    // F9 + T = Ctrl+T (tags the whole list); then back to normal.
    press(&mut app, KeyCode::F(9));
    press(&mut app, KeyCode::Char('t'));
    assert_eq!(tagged(&app).len(), 2);
    assert_eq!(app.menu_mode(), MenuMode::Normal);

    // F9 + M = Ctrl+M: move tagged files without paths.
    press(&mut app, KeyCode::F(9));
    press(&mut app, KeyCode::Char('m'));
    let label = &app.prompt.as_ref().unwrap().label;
    assert!(
        label.starts_with("MOVE 2 tagged files (no paths)"),
        "{label}"
    );
    press(&mut app, KeyCode::Esc);

    // F9 F9 + C = Alt+C: copy with paths.
    press(&mut app, KeyCode::F(9));
    press(&mut app, KeyCode::F(9));
    press(&mut app, KeyCode::Char('c'));
    assert!(app.prompt.as_ref().unwrap().label.contains("with paths"));
    press(&mut app, KeyCode::Esc);

    // Esc gives up on the menu without doing anything.
    press(&mut app, KeyCode::F(9));
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.menu_mode(), MenuMode::Normal);
    assert_eq!(app.focus, Focus::Files);
}

#[test]
fn f8_splits_and_tab_switches_panes() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Down); // sub
    press(&mut app, KeyCode::F(8));
    assert!(app.is_split());
    assert_eq!(app.active, 0);
    // The new window starts in the same directory.
    assert_eq!(app.panes[1].current_path(), tmp.path().join("sub"));

    press(&mut app, KeyCode::Tab);
    assert_eq!(app.active, 1);
    press(&mut app, KeyCode::Up);
    assert_eq!(app.current_path(), tmp.path());
    assert_eq!(app.panes[0].current_path(), tmp.path().join("sub"));

    // F8 again leaves only the active window.
    press(&mut app, KeyCode::F(8));
    assert!(!app.is_split());
    assert_eq!(app.current_path(), tmp.path());
    // Without a split, Tab goes back to toggling tree/files.
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus, Focus::Files);
}

#[test]
fn copy_defaults_to_other_pane_and_refreshes_both() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::F(8));
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Down); // window 2 in sub/
    press(&mut app, KeyCode::Tab);

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Down); // b.txt
    press(&mut app, KeyCode::Char('c'));
    let prompt = app.prompt.as_ref().unwrap();
    assert_eq!(prompt.input.text, tmp.path().join("sub").to_str().unwrap());
    press(&mut app, KeyCode::Enter);
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    assert!(tmp.path().join("sub/b.txt").exists());
    // The other window already shows the new file.
    let other = &app.panes[1];
    let names: Vec<_> = other
        .files
        .iter()
        .map(|&f| other.file(f).name.clone())
        .collect();
    assert_eq!(names, ["b.txt", "c.rs"]);
}

#[test]
fn log_moves_cursor_or_opens_new_root() {
    let (tmp, _) = sample();
    // As in `main`: the root is canonical (on Windows, the temporary
    // directory comes with a short 8.3 name).
    let root = dunce::canonicalize(tmp.path()).unwrap();
    let mut app = App::new(root.clone());
    // Inside the current tree: just moves the cursor.
    press(&mut app, KeyCode::Char('l'));
    answer_prompt(&mut app, "sub/deep");
    assert_eq!(app.current_path(), root.join("sub/deep"));
    assert_eq!(app.tree.path(ROOT), root);

    // Outside it: the window gets a new root.
    let other = crate::testing::tempdir();
    press(&mut app, KeyCode::Char('l'));
    answer_prompt(&mut app, other.path().to_str().unwrap());
    let canonical = dunce::canonicalize(other.path()).unwrap();
    assert_eq!(app.tree.path(ROOT), canonical);

    press(&mut app, KeyCode::Char('l'));
    answer_prompt(&mut app, "/caminho/que/nao/existe");
    assert!(app.message.as_deref().unwrap().starts_with("Cannot log"));
}

/// Two windows: the active one in `left/`, the other in `right/`.
fn compare_setup() -> (tempfile::TempDir, App) {
    use std::time::{Duration, SystemTime};
    let tmp = crate::testing::tempdir();
    let (l, r) = (tmp.path().join("left"), tmp.path().join("right"));
    create_dir_all(l.join("sub")).unwrap();
    create_dir_all(&r).unwrap();
    let at = |path: PathBuf, content: &str, secs: u64| {
        write(&path, content).unwrap();
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(t)
            .unwrap();
    };
    at(l.join("same.txt"), "x", 5_000);
    at(r.join("same.txt"), "x", 5_000);
    at(l.join("newer.txt"), "new", 9_000);
    at(r.join("newer.txt"), "old", 1_000);
    at(l.join("older.txt"), "old", 1_000);
    at(r.join("older.txt"), "new", 9_000);
    at(l.join("only-left.txt"), "", 5_000);
    at(l.join("sub/deep.txt"), "", 5_000);

    let mut app = App::new(l);
    app.apply(Command::Split);
    app.active = 1;
    app.log(r.to_str().unwrap()).unwrap();
    app.active = 0;
    (tmp, app)
}

#[test]
fn new_windows_keep_hidden_files_hidden() {
    let tmp = crate::testing::tempdir();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    hide(&tmp.path().join(".git"));
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    let mut config = Config::default();
    config.options.show_hidden = false;
    app.apply_config(&config).unwrap();

    app.apply(Command::Split);
    let names = |pane: &Pane| -> Vec<String> {
        pane.rows
            .iter()
            .map(|r| pane.tree.node(r.id).name.to_string_lossy().into_owned())
            .collect()
    };
    assert!(!app.panes[1].tree.show_hidden);
    assert!(!names(&app.panes[1]).contains(&".git".to_string()));

    // Log of a directory outside the tree: a new tree, same option.
    let other = crate::testing::tempdir();
    fs::create_dir_all(other.path().join(".cache")).unwrap();
    hide(&other.path().join(".cache"));
    app.log(other.path().to_str().unwrap()).unwrap();
    assert!(!app.tree.show_hidden);
    assert!(!names(&app).contains(&".cache".to_string()));
}

fn compare_with(app: &mut App, key: char) -> Vec<String> {
    press(app, KeyCode::Char('c'));
    assert!(app.confirm.is_some(), "compare question");
    press(app, KeyCode::Char(key));
    tagged(app)
}

#[test]
fn compare_tags_by_relation() {
    let (_tmp, mut app) = compare_setup();
    assert_eq!(compare_with(&mut app, 'u'), ["only-left.txt"]);
    assert_eq!(compare_with(&mut app, 'n'), ["newer.txt"]);
    assert_eq!(compare_with(&mut app, 'o'), ["older.txt"]);
    assert_eq!(compare_with(&mut app, 'i'), ["same.txt"]);
    assert_eq!(compare_with(&mut app, 'd'), ["newer.txt", "older.txt"]);
    assert_eq!(compare_with(&mut app, 'a'), ["newer.txt", "only-left.txt"]);
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .starts_with("Tagged 2 of 4 files")
    );
}

#[test]
fn compare_branch_follows_relative_paths() {
    let (_tmp, mut app) = compare_setup();
    press(&mut app, KeyCode::Char('c'));
    press(&mut app, KeyCode::Char('b'));
    let q = &app.confirm.as_ref().expect("asks again").question;
    assert!(q.starts_with("COMPARE branch left"), "{q}");
    press(&mut app, KeyCode::Char('u'));
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    // Opens the Branch; sub/deep.txt does not exist in right/sub/: it is unique.
    assert_eq!(app.view, FileView::Branch(ROOT));
    assert_eq!(tagged(&app), ["deep.txt", "only-left.txt"]);
}

#[test]
fn compare_needs_split_and_esc_cancels() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::Char('c'));
    assert!(app.confirm.is_none());
    assert_eq!(
        app.message.as_deref(),
        Some("Compare needs the split window (F8)")
    );

    let (_tmp, mut app) = compare_setup();
    press(&mut app, KeyCode::Char('c'));
    press(&mut app, KeyCode::Esc);
    assert!(app.confirm.is_none());
    assert!(tagged(&app).is_empty());
}

/// Root with a p.zip (src/a.rs, README) next to a.rs and b.txt.
fn archive_setup() -> (tempfile::TempDir, App) {
    let (tmp, _) = sample();
    crate::archive::tests::make_zip(&tmp.path().join("p.zip"));
    let mut app = App::new(tmp.path().to_path_buf());
    press(&mut app, KeyCode::Enter);
    let i = names(&app).iter().position(|n| n == "p.zip").unwrap();
    app.file_state.select(Some(i));
    (tmp, app)
}

fn alt_f5(app: &mut App) {
    app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::ALT));
}

#[test]
fn opens_browses_and_closes_archives() {
    let (tmp, mut app) = archive_setup();
    alt_f5(&mut app);
    assert_eq!(
        app.archive.as_deref(),
        Some(tmp.path().join("p.zip").as_path())
    );
    assert_eq!(app.focus, Focus::Tree);
    assert_eq!(names(&app), ["README"]);

    // View an entry without extracting anything to disk.
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('v'));
    assert_eq!(app.viewer.as_ref().unwrap().visible_lines(1)[0], "leia-me");
    press(&mut app, KeyCode::Esc);

    // Writing is forbidden inside the archive.
    press(&mut app, KeyCode::Char('d'));
    assert!(app.confirm.is_none());
    assert!(app.message.as_deref().unwrap().contains("read-only"));

    // Esc in the tree goes back exactly to where it was.
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);
    assert!(app.archive.is_none());
    assert_eq!(app.current_path(), tmp.path());
}

#[test]
fn copies_out_of_archives() {
    let (tmp, mut app) = archive_setup();
    alt_f5(&mut app);
    app.apply(Command::Branch);
    ctrl(&mut app, 't');
    alt(&mut app, 'c');
    let out = tmp.path().join("out");
    fs::create_dir(&out).unwrap();
    answer_prompt(&mut app, out.to_str().unwrap());
    assert!(app.message.as_deref().unwrap().starts_with("Copied 2 of 2"));
    assert_eq!(
        fs::read_to_string(out.join("src/a.rs")).unwrap(),
        "fn a() {}"
    );
    assert!(out.join("README").exists());
}

#[test]
fn open_rejects_non_archives() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    alt_f5(&mut app);
    assert!(app.archive.is_none());
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .starts_with("Not an archive")
    );
}

#[test]
fn fs_changes_refresh_now_or_after_operations() {
    let (tmp, mut app) = sample();
    assert_eq!(
        app.watched_dirs(),
        HashSet::from([tmp.path().to_path_buf()])
    );
    write(tmp.path().join("new.txt"), "").unwrap();
    assert!(app.fs_changed([tmp.path().to_path_buf()]));
    assert_eq!(names(&app), ["a.rs", "b.txt", "new.txt"]);

    // With an operation stopped on a conflict, waits.
    write(tmp.path().join("sub/b.txt"), "old").unwrap();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('c'));
    answer_prompt(&mut app, "sub");
    assert!(app.op.is_some());
    write(tmp.path().join("other.txt"), "").unwrap();
    assert!(!app.fs_changed([tmp.path().to_path_buf()]));
    assert_eq!(names(&app).len(), 3);

    press(&mut app, KeyCode::Char('n'));
    assert!(app.op.is_none());
    assert!(app.fs_changed([]), "pending change applied");
    assert_eq!(names(&app).len(), 4);
}

#[cfg(unix)]
#[test]
fn edit_and_execute_request_external_programs() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('e'));
    let ext = app.external.take().expect("editor requested");
    let quoted = format!(" '{}'", tmp.path().join("a.rs").display());
    assert!(ext.line.to_string_lossy().ends_with(&quoted));

    press(&mut app, KeyCode::Char('t'));
    press(&mut app, KeyCode::Char('t'));
    press(&mut app, KeyCode::Char('x'));
    type_text(&mut app, "wc -c %t");
    press(&mut app, KeyCode::Enter);
    let ext = app.external.take().expect("command requested");
    let a = tmp.path().join("a.rs");
    let b = tmp.path().join("b.txt");
    let expected = format!("wc -c '{}' '{}'", a.display(), b.display());
    assert_eq!(ext.line, expected.as_str());
    assert_eq!(ext.cwd, tmp.path());
    assert!(ext.wait);

    // Going back re-reads the disk and shows failures.
    write(tmp.path().join("made-by-command"), "").unwrap();
    let status = std::process::Command::new("false").status();
    app.after_external(status);
    assert!(names(&app).contains(&"made-by-command".to_string()));
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .starts_with("Command finished with")
    );
}

#[test]
fn prompts_remember_history_by_kind() {
    let (_tmp, mut app) = sample();
    for spec in ["*.rs", "*.txt"] {
        press(&mut app, KeyCode::Char('f'));
        answer_prompt(&mut app, spec);
    }
    press(&mut app, KeyCode::Char('f'));
    ctrl(&mut app, 'u');
    press(&mut app, KeyCode::Up);
    assert_eq!(app.prompt.as_ref().unwrap().input.text, "*.txt");
    press(&mut app, KeyCode::Up);
    assert_eq!(app.prompt.as_ref().unwrap().input.text, "*.rs");
    press(&mut app, KeyCode::Esc);

    // Another kind of prompt has its own history.
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Up);
    assert_eq!(app.prompt.as_ref().unwrap().input.text, "");
}

#[test]
fn quit_without_confirmation_when_configured() {
    let (_tmp, mut app) = sample();
    app.options.confirm_quit = false;
    press(&mut app, KeyCode::Char('q'));
    assert!(app.should_quit);
    assert!(app.confirm.is_none());
}

fn help_topic_after(app: &mut App, key: KeyEvent) -> String {
    app.handle_key(key);
    let topic = app.help.as_ref().expect("help open").topic.clone();
    press(app, KeyCode::Esc);
    assert!(app.help.is_none(), "Esc closes the help");
    topic
}

#[test]
fn f1_opens_the_help_of_the_window() {
    let (_tmp, mut app) = sample();
    let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
    assert_eq!(help_topic_after(&mut app, f1), "tree");
    press(&mut app, KeyCode::Enter);
    assert_eq!(help_topic_after(&mut app, f1), "files");
    press(&mut app, KeyCode::Char('v'));
    assert!(app.viewer.is_some());
    assert_eq!(help_topic_after(&mut app, f1), "viewer");
    assert!(app.viewer.is_some(), "the viewer is still open");
}

#[test]
fn help_in_a_prompt_returns_to_it_as_it_was() {
    let (_tmp, mut app) = sample();
    let alt_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::ALT);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('c'));
    type_text(&mut app, "dest");
    assert_eq!(help_topic_after(&mut app, alt_h), "copy-move");
    let prompt = app.prompt.as_ref().expect("the prompt is still open");
    assert!(prompt.input.text.ends_with("dest"), "{}", prompt.input.text);
    press(&mut app, KeyCode::Esc);

    press(&mut app, KeyCode::Char('f'));
    assert_eq!(help_topic_after(&mut app, alt_h), "filespec");
    press(&mut app, KeyCode::Esc);

    // A question too: after the help, it still takes its answer.
    press(&mut app, KeyCode::Char('d'));
    assert_eq!(help_topic_after(&mut app, alt_h), "delete");
    assert!(app.confirm.is_some());
    press(&mut app, KeyCode::Char('n'));
    assert!(app.confirm.is_none());
}

#[test]
fn help_shows_the_keys_in_effect() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::F(1));
    press(&mut app, KeyCode::Char('k'));
    let help = app.help.as_mut().expect("help open");
    assert_eq!(help.topic, "keys");
    let text: String = help
        .page(100)
        .lines
        .iter()
        .flatten()
        .map(|p| p.text.as_str())
        .collect();
    assert!(text.contains("help about what you are doing"), "{text}");
}

#[test]
fn history_is_saved_after_each_entry() {
    let (tmp, mut app) = sample();
    let file = tmp.path().join("state/history.toml");
    app.history_file = Some(file.clone());
    press(&mut app, KeyCode::Char('f'));
    answer_prompt(&mut app, "*.rs");
    assert_eq!(state::load_history(&file)["filespec"], ["*.rs"]);

    // A new run starts with the saved history.
    let mut again = App::new(tmp.path().to_path_buf());
    again.history = state::load_history(&file);
    press(&mut again, KeyCode::Char('f'));
    press(&mut again, KeyCode::Up);
    assert_eq!(again.prompt.as_ref().unwrap().input.text, "*.rs");
}

#[test]
fn alt_s_sorts_keeping_the_cursor() {
    let (tmp, mut app) = sample();
    write(tmp.path().join("big.bin"), vec![0u8; 100]).unwrap();
    app.apply(Command::Relog);
    press(&mut app, KeyCode::Enter);
    assert_eq!(names(&app), ["a.rs", "b.txt", "big.bin"]);
    press(&mut app, KeyCode::Down); // b.txt
    let selected = |app: &App| app.file(app.selected_file().unwrap()).name.clone();
    let before = selected(&app);

    alt(&mut app, 's');
    press(&mut app, KeyCode::Char('o')); // descending: asks again
    assert!(
        app.confirm
            .as_ref()
            .unwrap()
            .question
            .contains("descending")
    );
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.sort.label(), "Size↓");
    assert_eq!(names(&app), ["big.bin", "a.rs", "b.txt"]);
    assert_eq!(selected(&app), before, "cursor stays on the same file");

    // The F8 new window inherits the sort order.
    press(&mut app, KeyCode::F(8));
    assert_eq!(app.panes[1].sort, app.panes[0].sort);
}

#[test]
fn prune_needs_typed_confirmation() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Down); // sub (with sub/deep/a.rs)
    alt(&mut app, 'p');
    let label = app.prompt.as_ref().expect("asks to type").label.clone();
    assert!(label.starts_with("PRUNE sub: 2 files, 2 dirs"), "{label}");

    // Wrong text: nothing is deleted.
    answer_prompt(&mut app, "yes");
    assert!(tmp.path().join("sub/deep/a.rs").exists());
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .starts_with("Prune cancelled")
    );

    alt(&mut app, 'p');
    answer_prompt(&mut app, "prune");
    assert!(!tmp.path().join("sub").exists());
    assert!(app.message.as_deref().unwrap().starts_with("Pruned 4 of 4"));
    assert_eq!(app.current(), ROOT, "cursor goes to the parent");
    assert!(
        tmp.path().join("a.rs").exists(),
        "outside the branch is untouched"
    );
}

#[test]
fn prune_refuses_root_and_can_use_the_name() {
    let (tmp, mut app) = sample();
    alt(&mut app, 'p');
    assert!(app.prompt.is_none());
    assert_eq!(
        app.message.as_deref(),
        Some("Cannot prune the root directory")
    );

    app.options.prune_confirmation = PruneConfirm::Name;
    press(&mut app, KeyCode::Down);
    alt(&mut app, 'p');
    assert!(
        app.prompt
            .as_ref()
            .unwrap()
            .label
            .contains("Type sub to confirm")
    );
    answer_prompt(&mut app, "SUB");
    assert!(!tmp.path().join("sub").exists());
}

#[test]
fn hidden_files_option() {
    let (tmp, mut app) = sample();
    write(tmp.path().join(".env"), "").unwrap();
    create_dir_all(tmp.path().join(".git")).unwrap();
    hide(&tmp.path().join(".env"));
    hide(&tmp.path().join(".git"));
    app.apply(Command::Relog);
    assert!(names(&app).contains(&".env".to_string()));

    let config = Config {
        options: Options {
            show_hidden: false,
            ..Options::default()
        },
        ..Config::default()
    };
    app.apply_config(&config).unwrap();
    assert_eq!(names(&app), ["a.rs", "b.txt"]);
    assert!(!app.rows.iter().any(|r| app.tree.node(r.id).name == ".git"));
}

#[cfg(unix)]
#[test]
fn at_prefix_skips_history_and_is_stripped() {
    let (_tmp, mut app) = sample();
    app.options.history_skip_at = true;
    press(&mut app, KeyCode::Char('x'));
    type_text(&mut app, "@echo secret");
    press(&mut app, KeyCode::Enter);
    let ext = app.external.take().unwrap();
    assert_eq!(ext.line, "echo secret");
    assert!(!app.history.contains_key("execute"));
}

#[test]
fn git_menu() {
    let (tmp, mut app) = sample();
    ctrl(&mut app, 'g');
    assert_eq!(app.sticky, MenuMode::Normal, "off by default");

    let config = Config {
        options: Options {
            git: true,
            ..Options::default()
        },
        ..Config::default()
    };
    app.apply_config(&config).unwrap();
    ctrl(&mut app, 'g');
    assert_eq!(app.message.as_deref(), Some("Not a Git repository"));

    let root = tmp.path();
    git::git_ok(root, &["init", "-q"]);
    git::git_ok(root, &["add", "a.rs"]);
    git::git_ok(root, &["commit", "-q", "-m", "first"]);
    write(root.join("a.rs"), "fn main() { changed() }\n").unwrap();

    // Diff of the current file, in the viewer.
    press(&mut app, KeyCode::Enter);
    ctrl(&mut app, 'g');
    assert_eq!(app.sticky, MenuMode::Git);
    press(&mut app, KeyCode::Char('d'));
    let v = app.viewer.as_ref().expect("diff shown");
    assert!(
        v.visible_lines(20)
            .iter()
            .any(|l| l.contains("+fn main() { changed() }"))
    );
    press(&mut app, KeyCode::Esc);

    // Graph of the whole repository, in git's colors but without its codes.
    ctrl(&mut app, 'g');
    press(&mut app, KeyCode::Char('g'));
    let v = app.viewer.as_ref().expect("graph shown");
    let lines = v.visible_lines(20);
    assert!(lines[0].starts_with("* ") && lines[0].contains("(HEAD -> "));
    assert!(lines[0].ends_with(" first"), "{lines:?}");
    assert!(
        lines
            .iter()
            .all(|l| !l.contains('\u{1b}') && !l.contains('·'))
    );
    assert!(v.colors_on);
    press(&mut app, KeyCode::Esc);

    // Status letters in the list, before and after Add.
    app.refresh_git();
    let a = app.current_path().join("a.rs");
    assert_eq!(app.git_status.marker(&a), Some(*b" M"));
    ctrl(&mut app, 'g');
    press(&mut app, KeyCode::Char('a'));
    assert_eq!(app.message.as_deref(), Some("Added a.rs"));
    std::thread::sleep(std::time::Duration::from_millis(600));
    app.refresh_git();
    assert_eq!(app.git_status.marker(&a), Some(*b"M "));

    // Commit goes to git's editor, in the repository.
    ctrl(&mut app, 'g');
    press(&mut app, KeyCode::Char('c'));
    let ext = app.external.take().expect("git commit");
    assert_eq!(ext.line, "git commit");
    assert!(ext.wait);

    // Esc closes the menu; Blame needs a file.
    ctrl(&mut app, 'g');
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.sticky, MenuMode::Normal);
    assert!(app.viewer.is_none());
    press(&mut app, KeyCode::Esc);
    ctrl(&mut app, 'g');
    press(&mut app, KeyCode::Char('b'));
    assert_eq!(app.message.as_deref(), Some("Blame works on one file"));
}

#[test]
fn d_in_viewer_shows_the_git_diff() {
    let (tmp, mut app) = sample();
    let config = Config {
        options: Options {
            git: true,
            ..Options::default()
        },
        ..Config::default()
    };
    app.apply_config(&config).unwrap();
    let root = tmp.path();
    git::git_ok(root, &["init", "-q"]);
    git::git_ok(root, &["add", "."]);
    git::git_ok(root, &["commit", "-q", "-m", "first"]);
    write(root.join("a.rs"), "fn main() {}\nfn new() {}\n").unwrap();
    app.apply(Command::Relog);
    app.refresh_git();

    // b.txt did not change: no diff.
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('v'));
    assert!(!app.can_diff(&app.viewer.as_ref().unwrap().path));
    press(&mut app, KeyCode::Char('d'));
    let v = app.viewer.as_ref().unwrap();
    assert_eq!(v.note.as_deref(), Some("No changes since the last commit"));
    assert!(v.behind.is_none());
    press(&mut app, KeyCode::Esc);

    // a.rs changed: D shows the diff, D again the file, where it was.
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Char('v'));
    app.viewer.as_mut().unwrap().top = 1;
    press(&mut app, KeyCode::Char('d'));
    let v = app.viewer.as_ref().unwrap();
    assert_eq!(v.path, Path::new("git diff a.rs"));
    assert!(v.visible_lines(20).iter().any(|l| l == "+fn new() {}"));
    press(&mut app, KeyCode::Char('d'));
    let v = app.viewer.as_ref().unwrap();
    assert!(v.on_disk && v.behind.is_none());
    assert_eq!(v.top, 1);

    // E in the diff edits the file; on return, the diff is new.
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('e'));
    let ext = app.external.take().expect("editor");
    assert!(ext.file.is_some_and(|f| f.ends_with("a.rs")));
    write(root.join("a.rs"), "fn main() {}\nfn newer() {}\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(600));
    app.after_external(Ok(std::process::ExitStatus::default()));
    let v = app.viewer.as_ref().unwrap();
    assert!(v.behind.is_some(), "still in the diff");
    assert!(v.visible_lines(20).iter().any(|l| l == "+fn newer() {}"));

    // Esc closes both.
    press(&mut app, KeyCode::Esc);
    assert!(app.viewer.is_none());
}

#[test]
fn f2_picks_the_folder_in_a_tree() {
    let (tmp, mut app) = sample();
    let root = tmp.path().to_path_buf();
    // Copy of a.rs: the prompt asks for a folder.
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('c'));
    let typed = app.prompt.as_ref().unwrap().input.text.clone();
    press(&mut app, KeyCode::F(2));
    let picker = app.picker.as_ref().expect("tree open");
    assert_eq!(picker.pane().current_path(), root);

    // Esc goes back to the prompt as it was.
    press(&mut app, KeyCode::Esc);
    assert!(app.picker.is_none());
    assert_eq!(app.prompt.as_ref().unwrap().input.text, typed);

    // Enter puts the folder in the prompt, without copying yet.
    press(&mut app, KeyCode::F(2));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(app.picker.is_none());
    let prompt = app.prompt.as_ref().expect("still asking");
    assert_eq!(Path::new(&prompt.input.text), root.join("sub"));
    assert!(!root.join("sub/a.rs").exists());
    press(&mut app, KeyCode::Enter);
    assert!(app.op.is_some() || root.join("sub/a.rs").exists());
    while app.busy() {
        app.tick(std::time::Duration::from_millis(50));
    }
    assert!(root.join("sub/a.rs").exists());

    // Ctrl+B: only the folder changes, the file name stays.
    ctrl(&mut app, 't');
    ctrl(&mut app, 'b');
    press(&mut app, KeyCode::F(2));
    press(&mut app, KeyCode::Char('S'));
    press(&mut app, KeyCode::Enter);
    let text = &app.prompt.as_ref().unwrap().input.text;
    assert_eq!(Path::new(text), root.join("sub").join(shell::BATCH_FILE));

    // Not in prompts that take something else (Filespec): F2 does nothing.
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('f'));
    press(&mut app, KeyCode::F(2));
    assert!(app.picker.is_none());
}

#[test]
fn e_in_viewer_edits_and_reloads() {
    let (tmp, mut app) = sample();
    let file = tmp.path().join("a.rs");
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('v'));
    assert!(app.viewer.is_some());
    press(&mut app, KeyCode::Char('e'));
    let ext = app.external.take().expect("editor requested");
    assert!(ext.file.as_ref().is_some_and(|f| f.ends_with("a.rs")));
    assert_eq!(ext.cwd, tmp.path());

    // The "editor" changes the file; back in Xylem, the viewer shows it.
    write(&file, "one\ntwo\nthree\n").unwrap();
    app.after_external(Ok(std::process::ExitStatus::default()));
    let v = app.viewer.as_ref().expect("still viewing");
    assert_eq!(v.visible_lines(3), ["one", "two", "three"]);

    // Deleted by the editor: the viewer closes.
    press(&mut app, KeyCode::Char('e'));
    app.external.take().unwrap();
    std::fs::remove_file(&file).unwrap();
    app.after_external(Ok(std::process::ExitStatus::default()));
    assert!(app.viewer.is_none());
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .starts_with("Cannot view file")
    );

    // The help is not a file: E there edits nothing.
    press(&mut app, KeyCode::F(1));
    press(&mut app, KeyCode::Char('e'));
    assert!(app.help.is_some() && app.external.is_none());
}

#[test]
fn f12_settings_save_and_apply() {
    let (tmp, mut app) = sample();
    let file = tmp.path().join("cfg/config.toml");
    app.config_file = Some(file.clone());
    write(tmp.path().join(".hidden"), "").unwrap();
    hide(&tmp.path().join(".hidden"));
    app.apply(Command::Relog);

    press(&mut app, KeyCode::F(12));
    assert!(app.settings.is_some());
    press(&mut app, KeyCode::PageDown);
    press(&mut app, KeyCode::Char('a')); // hide hidden files
    press(&mut app, KeyCode::F(3));
    assert!(app.settings.is_none());
    assert!(!app.options.show_hidden);
    assert!(
        !names(&app).contains(&".hidden".to_string()),
        "applied at once"
    );
    let (saved, err) = Config::load_from(&file);
    assert!(err.is_none());
    assert!(!saved.options.show_hidden);
}

#[test]
fn settings_esc_discards() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::F(12));
    press(&mut app, KeyCode::Char('b'));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);
    assert!(app.settings.is_none());
    assert_eq!(app.options, Options::default());
}

/// A fake "default program" that copies the received file to `out/`.
fn fake_opener(tmp: &Path) -> (String, PathBuf) {
    let out = tmp.join("opened");
    fs::create_dir_all(&out).unwrap();
    (format!("cp -t {}", out.display()), out)
}

/// Runs the ongoing operation to the end, as the main loop would.
fn finish_op(app: &mut App) {
    while app.op.is_some() {
        app.tick(Duration::from_millis(30));
    }
}

fn wait_for(path: &Path) -> bool {
    (0..100).any(|_| {
        std::thread::sleep(Duration::from_millis(20));
        path.exists()
    })
}

/// Waits until `path` has exactly `text`: the opener runs in the
/// background and may have created the file without finishing it.
fn wait_for_text(path: &Path, text: &str) -> bool {
    (0..100).any(|_| {
        std::thread::sleep(Duration::from_millis(20));
        fs::read_to_string(path).is_ok_and(|t| t == text)
    })
}

#[test]
fn o_opens_with_the_default_program() {
    let (tmp, mut app) = sample();
    let (opener, out) = fake_opener(tmp.path());
    app.options.opener = opener;
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('o'));
    assert_eq!(app.message.as_deref(), Some("Opened 1 file(s)"));
    assert!(wait_for(&out.join("a.rs")));

    ctrl(&mut app, 't');
    ctrl(&mut app, 'o');
    assert_eq!(app.message.as_deref(), Some("Opened 2 file(s)"));
    assert!(wait_for(&out.join("b.txt")));
}

#[test]
fn o_inside_archive_extracts_first() {
    let (tmp, mut app) = archive_setup();
    let (opener, out) = fake_opener(tmp.path());
    app.options.opener = opener;
    alt_f5(&mut app);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('o'));
    finish_op(&mut app);
    assert!(wait_for_text(&out.join("README"), "leia-me"));
    // `main` removes the run's folder on exit; here, the test does.
    let run = platform::run_dir_path(&platform::cache_dir().unwrap());
    let _ = fs::remove_file(run.join("README"));
    let _ = fs::remove_dir(run);
}

#[test]
fn alt_c_in_tree_compares_the_branch() {
    let (_tmp, mut app) = compare_setup();
    alt(&mut app, 'c');
    let q = &app.confirm.as_ref().expect("compare question").question;
    assert!(q.starts_with("COMPARE branch"), "{q}");
}

#[test]
fn backspace_goes_to_parent_in_both_windows() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.current(), ROOT);

    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.current_path(), tmp.path().join("sub"));
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.current(), ROOT);
    assert_eq!(app.focus, Focus::Files);
    assert_eq!(names(&app), ["a.rs", "b.txt"]);
}

/// Three files; "senha" appears in two of them, twice in the first.
fn search_setup() -> (tempfile::TempDir, App) {
    let tmp = crate::testing::tempdir();
    write(
        tmp.path().join("1.txt"),
        "linha\na SENHA aqui\noutra\nsenha de novo\n",
    )
    .unwrap();
    write(tmp.path().join("2.txt"), "nada a ver\n").unwrap();
    write(tmp.path().join("3.txt"), "\n\n\nsó a senha no fim\n").unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    press(&mut app, KeyCode::Enter);
    ctrl(&mut app, 't');
    (tmp, app)
}

fn viewer_space(app: &mut App) -> (String, Option<usize>) {
    press(app, KeyCode::Char(' '));
    let v = app.viewer.as_ref().unwrap();
    let name = v.path.file_name().unwrap().to_string_lossy().into_owned();
    (name, v.hit.map(|h| h.line))
}

#[test]
fn ctrl_s_then_ctrl_v_then_space_walks_the_hits() {
    let (_tmp, mut app) = search_setup();
    ctrl(&mut app, 's');
    let label = app.prompt.as_ref().unwrap().label.clone();
    assert_eq!(label, "SEARCH tagged files (text, ignore case): ");
    type_text(&mut app, "senha");
    press(&mut app, KeyCode::Enter);
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    assert_eq!(tagged(&app), ["1.txt", "3.txt"], "2.txt was untagged");
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .starts_with("2 of 3 tagged files contain")
    );

    // Ctrl+V and Space: uses the Ctrl+S term and goes across the files.
    ctrl(&mut app, 'v');
    assert_eq!(app.tagged_view.as_ref().unwrap().items.len(), 2);
    assert_eq!(viewer_space(&mut app), ("1.txt".into(), Some(1)));
    assert_eq!(viewer_space(&mut app), ("1.txt".into(), Some(3)));
    assert_eq!(viewer_space(&mut app), ("3.txt".into(), Some(3)));
    let (_, hit) = viewer_space(&mut app);
    assert_eq!(hit, Some(3), "no more hits: stays");
    assert!(
        app.viewer
            .as_ref()
            .unwrap()
            .note
            .as_deref()
            .unwrap()
            .contains("not found")
    );

    // Backwards goes back to the previous file.
    press(&mut app, KeyCode::Char('-'));
    let v = app.viewer.as_ref().unwrap();
    assert_eq!(v.path.file_name().unwrap(), "1.txt");
    assert_eq!(v.hit.map(|h| h.line), Some(3));
}

#[test]
fn ctrl_v_next_prev_and_untag() {
    let (_tmp, mut app) = search_setup();
    ctrl(&mut app, 'v');
    let current = |app: &App| {
        let v = app.viewer.as_ref().unwrap();
        v.path.file_name().unwrap().to_string_lossy().into_owned()
    };
    assert_eq!(current(&app), "1.txt");
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(current(&app), "2.txt");
    press(&mut app, KeyCode::Char('u'));
    app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::ALT));
    assert_eq!(current(&app), "3.txt");
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(
        app.viewer.as_ref().unwrap().note.as_deref(),
        Some("Last tagged file")
    );
    press(&mut app, KeyCode::Esc);
    assert!(app.viewer.is_none() && app.tagged_view.is_none());
    assert_eq!(tagged(&app), ["1.txt", "3.txt"]);
}

#[test]
fn viewer_find_prompt_and_options() {
    let (_tmp, mut app) = search_setup();
    press(&mut app, KeyCode::Char('v')); // only 1.txt
    press(&mut app, KeyCode::Char('f'));
    assert!(
        app.prompt
            .as_ref()
            .unwrap()
            .label
            .starts_with("FIND (text, ignore case)")
    );
    press(&mut app, KeyCode::F(2));
    assert!(app.prompt.as_ref().unwrap().label.contains("match case"));
    type_text(&mut app, "SENHA");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.viewer.as_ref().unwrap().hit.map(|h| h.line), Some(1));
    // Case-sensitive, the "senha" on line 3 does not count.
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.viewer.as_ref().unwrap().hit.map(|h| h.line), Some(1));

    // The term and options come back on the next Ctrl+S.
    press(&mut app, KeyCode::Esc);
    ctrl(&mut app, 's');
    let prompt = app.prompt.as_ref().unwrap();
    assert_eq!(prompt.input.text, "SENHA");
    assert!(prompt.label.contains("match case"));
}

#[test]
fn search_can_be_cancelled_and_needs_tags() {
    let (_tmp, mut app) = search_setup();
    ctrl(&mut app, 'u');
    ctrl(&mut app, 's');
    assert!(app.prompt.is_none());
    assert_eq!(app.message.as_deref(), Some("No tagged files"));
    ctrl(&mut app, 'v');
    assert!(app.viewer.is_none());
}

#[test]
fn searches_inside_archives() {
    let (_tmp, mut app) = archive_setup();
    alt_f5(&mut app);
    app.apply(Command::Branch);
    ctrl(&mut app, 't');
    ctrl(&mut app, 's');
    type_text(&mut app, "fn a");
    press(&mut app, KeyCode::Enter);
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    assert_eq!(tagged(&app), ["a.rs"]);
}

fn treespec_setup() -> (tempfile::TempDir, App, String) {
    let tmp = crate::testing::tempdir();
    for d in ["Docs/rust/src", "Docs/notes", "downloads"] {
        create_dir_all(tmp.path().join(d)).unwrap();
    }
    write(tmp.path().join("Docs/rust/src/main.rs"), "").unwrap();
    let app = App::new(tmp.path().to_path_buf());
    let root = tmp.path().to_string_lossy().into_owned();
    (tmp, app, root)
}

/// `/` in the expected text becomes the platform's separator.
fn sep(text: String) -> String {
    text.replace('/', std::path::MAIN_SEPARATOR_STR)
}

fn current_name(app: &App) -> String {
    app.tree
        .node(app.current())
        .name
        .to_string_lossy()
        .into_owned()
}

#[test]
fn treespec_navigates_while_typing() {
    let (_tmp, mut app, root) = treespec_setup();
    press(&mut app, KeyCode::Char('\\'));
    assert_eq!(
        app.prompt.as_ref().unwrap().input.text,
        sep(format!("{root}/"))
    );
    type_text(&mut app, "do");
    assert_eq!(current_name(&app), "Docs", "cursor follows the typing");
    type_text(&mut app, "/");
    assert_eq!(
        app.prompt.as_ref().unwrap().input.text,
        sep(format!("{root}/Docs/"))
    );
    type_text(&mut app, "r\\s");
    assert_eq!(
        app.prompt.as_ref().unwrap().input.text,
        sep(format!("{root}/Docs/rust/s"))
    );
    assert_eq!(current_name(&app), "src");
    press(&mut app, KeyCode::Enter);
    assert!(app.prompt.is_none());
    assert_eq!(current_name(&app), "src");
    assert_eq!(names(&app), ["main.rs"]);
}

#[test]
fn treespec_tab_cycles_and_esc_returns() {
    let (_tmp, mut app, root) = treespec_setup();
    press(&mut app, KeyCode::Char('\\'));
    type_text(&mut app, "do");
    press(&mut app, KeyCode::Tab);
    assert_eq!(
        app.prompt.as_ref().unwrap().input.text,
        sep(format!("{root}/Docs"))
    );
    press(&mut app, KeyCode::Tab);
    assert_eq!(current_name(&app), "downloads");
    press(&mut app, KeyCode::BackTab);
    assert_eq!(current_name(&app), "Docs");
    press(&mut app, KeyCode::Esc);
    assert!(app.prompt.is_none());
    assert_eq!(app.current(), ROOT, "back to where it started");
}

#[test]
fn treespec_outside_the_tree_logs() {
    let (_tmp, mut app, _root) = treespec_setup();
    let other = crate::testing::tempdir();
    press(&mut app, KeyCode::Char('\\'));
    ctrl(&mut app, 'u');
    type_text(&mut app, other.path().to_str().unwrap());
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        app.tree.path(ROOT),
        dunce::canonicalize(other.path()).unwrap()
    );
}

#[test]
fn treespec_in_branch_goes_to_the_file() {
    let (_tmp, mut app, _root) = treespec_setup();
    app.apply(Command::Branch);
    assert_eq!(names(&app), ["main.rs"]);
    press(&mut app, KeyCode::Char('\\'));
    assert!(app.prompt.is_none());
    assert_eq!(app.view, FileView::Dir);
    assert_eq!(current_name(&app), "src");
    assert_eq!(app.file(app.selected_file().unwrap()).name, "main.rs");
}

#[test]
fn shift_letter_jumps_instead_of_running_commands() {
    let (_tmp, mut app, _root) = treespec_setup();
    app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
    assert_eq!(current_name(&app), "Docs");
    app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
    assert_eq!(current_name(&app), "downloads");
    // Lowercase is still a command (d = Delete).
    press(&mut app, KeyCode::Char('d'));
    assert!(app.confirm.is_some());
}

#[test]
fn alt_k_chooses_what_to_show() {
    let (_tmp, mut app) = sample();
    alt(&mut app, 'k');
    press(&mut app, KeyCode::Char('b'));
    let q = &app.confirm.as_ref().expect("asks again").question;
    assert!(q.contains("per branch"), "{q}");
    press(&mut app, KeyCode::Char('t'));
    press(&mut app, KeyCode::Char('c'));
    assert!(app.confirm.is_none());
    assert_eq!(
        app.kbyte,
        Kbyte {
            show: KbyteShow::Count,
            tagged_only: true,
            branch: true
        }
    );
    alt(&mut app, 'k');
    press(&mut app, KeyCode::Char('p'));
    assert_eq!(app.kbyte.show, KbyteShow::Percent);
    alt(&mut app, 'k');
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.kbyte.show, KbyteShow::None);
}

#[test]
fn alt_w_opens_the_usage_view() {
    let (_tmp, mut app) = sample();
    alt(&mut app, 'w');
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    let active = app.active;
    let pane = &mut app.panes[active];
    let (tree, view) = (&pane.tree, pane.usage.as_mut().expect("open"));
    let names: Vec<String> = view.rows(tree).into_iter().map(|r| r.name).collect();
    // sub (c.rs and deep/a.rs) is the largest on disk only by blocks; by
    // the apparent size, a.rs (13 bytes) wins.
    assert!(names.contains(&"sub".to_string()) && names.contains(&"a.rs".to_string()));
    view.apparent = true;
    assert_eq!(view.rows(tree)[0].name, "a.rs");
    // Into "sub", and from there to the tree.
    view.order = crate::usage::Order::Name;
    let at = view
        .rows(tree)
        .iter()
        .position(|r| r.name == "sub")
        .unwrap();
    view.list.select(Some(at));
    press(&mut app, KeyCode::Enter);
    let pane = &mut app.panes[app.active];
    let dir = pane.usage.as_ref().unwrap().dir;
    assert_eq!(pane.tree.node(dir).name, "sub");
    press(&mut app, KeyCode::Char('\\'));
    assert!(app.usage.is_none());
    assert_eq!(app.current_path().file_name().unwrap(), "sub");
    alt(&mut app, 'w');
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    press(&mut app, KeyCode::Esc);
    assert!(app.usage.is_none());
}

#[test]
fn alt_q_quits_remembering_the_directory() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Down);
    alt(&mut app, 'q');
    assert!(
        app.confirm
            .as_ref()
            .unwrap()
            .question
            .starts_with("Quit Xylem and change to")
    );
    press(&mut app, KeyCode::Char('y'));
    assert!(app.should_quit);
    assert_eq!(
        app.quit_dir.as_deref(),
        Some(tmp.path().join("sub").as_path())
    );

    // Normal Q does not change the directory.
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::Char('q'));
    press(&mut app, KeyCode::Char('y'));
    assert!(app.should_quit && app.quit_dir.is_none());
}

#[test]
fn filespec_with_size_and_date_in_configured_order() {
    let (tmp, mut app) = sample();
    write(tmp.path().join("big.bin"), vec![0u8; 5000]).unwrap();
    app.apply(Command::Relog);
    press(&mut app, KeyCode::Char('f'));
    answer_prompt(&mut app, ">s4K");
    assert_eq!(names(&app), ["big.bin"]);

    app.options.date_format = DateFormat::Dmy;
    let today = chrono::Local::now().format("%d/%m/%Y").to_string();
    press(&mut app, KeyCode::Char('f'));
    answer_prompt(&mut app, &format!("*.rs,={today}"));
    assert_eq!(names(&app), ["a.rs"]);
    press(&mut app, KeyCode::Char('f'));
    answer_prompt(&mut app, "<TODAY");
    assert!(names(&app).is_empty());
}

#[test]
fn ctrl_f7_f8_f9_tag_the_branch() {
    let (_tmp, mut app) = sample();
    app.apply(Command::ExpandAll);
    let fkey =
        |app: &mut App, n| app.handle_key(KeyEvent::new(KeyCode::F(n), KeyModifiers::CONTROL));
    let all_tagged = |app: &App| -> usize {
        app.logged_matching_files()
            .iter()
            .filter(|&&f| app.file(f).tagged)
            .count()
    };
    press(&mut app, KeyCode::Down); // sub
    fkey(&mut app, 7);
    assert_eq!(all_tagged(&app), 2, "sub/c.rs and sub/deep/a.rs");
    assert!(app.message.as_deref().unwrap().starts_with("2 of 2"));
    press(&mut app, KeyCode::Up);
    fkey(&mut app, 9);
    assert_eq!(
        all_tagged(&app),
        2,
        "root's 2 files now tagged, sub's untagged"
    );
    fkey(&mut app, 8);
    assert_eq!(all_tagged(&app), 0);
}

#[test]
fn f7_autoview_follows_the_cursor() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::F(7));
    assert_eq!(app.focus, Focus::Files);
    let shown = |app: &App| app.autoview.as_ref().unwrap().visible_lines(1)[0].clone();
    assert_eq!(shown(&app), "fn main() {}");
    press(&mut app, KeyCode::Down);
    assert_eq!(shown(&app), "b");
    // The files window commands remain valid.
    press(&mut app, KeyCode::Char('t'));
    assert_eq!(tagged(&app), ["b.txt"]);
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT));
    assert!(app.autoview.is_some());
    press(&mut app, KeyCode::F(7));
    assert!(app.autoview.is_none());
    assert_eq!(app.focus, Focus::Files);
}

#[test]
fn autoview_shows_a_gz_decompressed() {
    let tmp = crate::testing::tempdir();
    std::fs::write(
        tmp.path().join("app.log.gz"),
        crate::decompress::test_util::gz(b"hello log\n"),
    )
    .unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    press(&mut app, KeyCode::F(7));
    let v = app.autoview.as_ref().unwrap();
    assert_eq!(v.visible_lines(1)[0], "hello log");
}

#[test]
fn v_on_a_gz_shows_hex_when_viewer_decompress_is_off() {
    let tmp = crate::testing::tempdir();
    std::fs::write(
        tmp.path().join("app.log.gz"),
        crate::decompress::test_util::gz(b"hello log\n"),
    )
    .unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    app.options.viewer_decompress = false;
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('v'));
    let v = app.viewer.as_ref().unwrap();
    assert_eq!(v.mode, crate::viewer::ViewMode::Hex);
}

#[test]
fn v_reads_at_most_viewer_limit() {
    let tmp = crate::testing::tempdir();
    std::fs::write(tmp.path().join("big.txt"), "a".repeat(2 << 20)).unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    app.viewer_limit = 1 << 20;
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('v'));
    let v = app.viewer.as_ref().unwrap();
    assert!(v.truncated());
    assert_eq!(v.limit(), 1 << 20);
}

#[test]
fn n_sets_new_dates() {
    use chrono::{Datelike, Timelike};
    let (tmp, mut app) = sample();
    app.options.date_format = DateFormat::Dmy;
    let mtime = |p: &str| {
        chrono::DateTime::<chrono::Local>::from(
            fs::metadata(tmp.path().join(p))
                .unwrap()
                .modified()
                .unwrap(),
        )
    };
    let before = mtime("a.rs");

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('n'));
    let prompt = app.prompt.as_ref().unwrap();
    assert!(prompt.label.starts_with("NEW DATE for a.rs"));
    assert_eq!(
        prompt.input.text,
        before.format("%d/%m/%Y %H:%M").to_string()
    );
    answer_prompt(&mut app, "01/02/2020 13:45");
    let after = mtime("a.rs");
    assert_eq!((after.year(), after.month(), after.day()), (2020, 2, 1));
    assert_eq!((after.hour(), after.minute()), (13, 45));

    // Tagged files, date only: each keeps its time.
    ctrl(&mut app, 't');
    let b_time = mtime("b.txt").time();
    ctrl(&mut app, 'n');
    answer_prompt(&mut app, "2019-03-04");
    assert_eq!(mtime("b.txt").date_naive().to_string(), "2019-03-04");
    assert_eq!(mtime("b.txt").time().minute(), b_time.minute());
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .starts_with("New date set on 2 of 2")
    );

    press(&mut app, KeyCode::Char('n'));
    answer_prompt(&mut app, "ontem");
    assert!(app.message.as_deref().unwrap().starts_with("Invalid date"));

    // In the tree, the directory's date.
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('n'));
    answer_prompt(&mut app, "2018-01-01");
    assert_eq!(mtime("sub").date_naive().to_string(), "2018-01-01");
}

#[cfg(unix)]
#[test]
fn ctrl_b_writes_a_batch_file() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    ctrl(&mut app, 't');
    ctrl(&mut app, 'b');
    assert_eq!(
        app.prompt.as_ref().unwrap().input.text,
        tmp.path().join("xy-batch.sh").to_str().unwrap()
    );
    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "wc -c %f");
    press(&mut app, KeyCode::Enter);
    let script = tmp.path().join("xy-batch.sh");
    let text = fs::read_to_string(&script).unwrap();
    let a = tmp.path().join("a.rs");
    assert!(text.starts_with("#!/bin/sh\n"));
    assert!(text.contains(&format!("wc -c '{}'\n", a.display())));
    assert_eq!(
        fs::metadata(&script).unwrap().permissions().mode() & 0o111,
        0o111
    );
    assert!(
        names(&app).contains(&"xy-batch.sh".to_string()),
        "list refreshed"
    );

    // Data list, and does not overwrite.
    ctrl(&mut app, 'b');
    answer_prompt(&mut app, "lista.csv");
    type_text(&mut app, "%N;%S");
    press(&mut app, KeyCode::Enter);
    assert_eq!(
        fs::read_to_string(tmp.path().join("lista.csv")).unwrap(),
        "a.rs;13\nb.txt;1\n"
    );
    ctrl(&mut app, 'b');
    answer_prompt(&mut app, "lista.csv");
    assert!(app.message.as_deref().unwrap().ends_with("already exists"));
}

#[test]
fn f5_creates_archives_with_relative_paths() {
    let (tmp, mut app) = sample();
    app.apply(Command::Branch);
    ctrl(&mut app, 't');
    app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::CONTROL));
    let prompt = app.prompt.as_ref().unwrap();
    let root_name = tmp
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(prompt.input.text.ends_with(&format!("{root_name}.tar.gz")));
    answer_prompt(&mut app, "pack.zip");
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .starts_with("Archived 4 files"),
        "{:?}",
        app.message
    );

    let mut r = archive::Reader::open(&tmp.path().join("pack.zip")).unwrap();
    let mut names: Vec<String> = r
        .list()
        .unwrap()
        .iter()
        .map(|e| archive::entry_name(&e.path))
        .collect();
    names.sort();
    assert_eq!(names, ["a.rs", "b.txt", "sub/c.rs", "sub/deep/a.rs"]);

    // Does not overwrite.
    app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::CONTROL));
    answer_prompt(&mut app, "pack.zip");
    assert!(app.message.as_deref().unwrap().ends_with("already exists"));
}

#[test]
fn info_avail_and_stats_screens() {
    let (_tmp, mut app) = sample();
    let first_line = |app: &App| app.viewer.as_ref().unwrap().visible_lines(1)[0].clone();

    alt(&mut app, 'i');
    assert!(first_line(&app).starts_with("INFO "));
    let text = app.viewer.as_ref().unwrap().visible_lines(40).join("\n");
    assert!(
        text.contains("directory") && text.contains("LOGGED"),
        "{text}"
    );
    press(&mut app, KeyCode::Esc);

    press(&mut app, KeyCode::Enter);
    alt(&mut app, 'i');
    let text = app.viewer.as_ref().unwrap().visible_lines(40).join("\n");
    assert!(text.contains("a.rs") && text.contains("13 bytes"), "{text}");
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);

    press(&mut app, KeyCode::Char('a'));
    assert!(first_line(&app).starts_with("AVAIL"));
    press(&mut app, KeyCode::Esc);

    press(&mut app, KeyCode::Char('?'));
    let text = app.viewer.as_ref().unwrap().visible_lines(20).join("\n");
    assert!(text.contains("Logged") && text.contains("Tagged"), "{text}");
    let text = app.viewer.as_ref().unwrap().visible_lines(60).join("\n");
    assert!(
        text.contains("SYSTEM") && text.contains("XYLEM") && text.contains("Version"),
        "{text}"
    );
}

#[test]
fn slash_also_opens_treespec() {
    let (_tmp, mut app, root) = treespec_setup();
    press(&mut app, KeyCode::Char('/'));
    assert_eq!(
        app.prompt.as_ref().unwrap().input.text,
        sep(format!("{root}/"))
    );
    type_text(&mut app, "dow");
    assert_eq!(current_name(&app), "downloads");
    press(&mut app, KeyCode::Esc);
    // "?" still shows the statistics ("/" in the viewer still searches text).
    press(&mut app, KeyCode::Char('?'));
    assert!(app.viewer.is_some(), "? still shows statistics");
}

#[test]
fn test_alt_x_opens_terminal() {
    let (_tmp, mut app) = sample();
    assert!(!app.terminal_focus);
    alt(&mut app, 'x');
    assert!(
        app.terminal_focus,
        "terminal_focus should be true after alt+x"
    );
    assert!(app.terminal.is_some(), "terminal should be spawned");
}

#[test]
fn test_clipboard_copy_tree_dir() {
    let (tmp, mut app) = sample();
    assert_eq!(app.focus, Focus::Tree);
    press(&mut app, KeyCode::Char('y'));
    let msg = app.message.expect("message expected after y in tree");
    assert!(msg.starts_with("Copied to clipboard:"), "{msg}");
    let path = tmp.path().to_string_lossy();
    assert!(msg.contains(path.as_ref()), "{msg}");
    let copied = crate::clipboard::copied().expect("nothing copied");
    assert_eq!(msg, format!("Copied to clipboard: {copied}"));
}

#[test]
fn test_clipboard_copy_file() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.focus, Focus::Files);
    // Ctrl+Insert also works
    app.handle_key(KeyEvent::new(KeyCode::Insert, KeyModifiers::CONTROL));
    let msg = app
        .message
        .expect("message expected after Ctrl+Insert in files");
    assert!(msg.starts_with("Copied to clipboard:"), "{msg}");
    assert!(msg.ends_with("a.rs"), "{msg}");
    let copied = crate::clipboard::copied().expect("nothing copied");
    assert_eq!(msg, format!("Copied to clipboard: {copied}"));
}

#[test]
fn test_clipboard_copy_tagged() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    // No tagged files initially
    ctrl(&mut app, 'y');
    assert_eq!(app.message.as_deref(), Some("No tagged files to copy"));

    // Tag the file under cursor
    press(&mut app, KeyCode::Char('t'));
    ctrl(&mut app, 'y');
    let msg = app.message.expect("message expected after Ctrl+Y");
    assert_eq!(msg, "Copied 1 path to clipboard");
    let copied = crate::clipboard::copied().expect("nothing copied");
    assert!(
        copied.ends_with("a.rs") && !copied.contains('\n'),
        "{copied}"
    );
}

#[test]
fn test_clipboard_viewer_copy() {
    let (_tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('v'));
    assert!(app.viewer.is_some());
    press(&mut app, KeyCode::Char('y'));
    let note = app
        .viewer
        .as_ref()
        .unwrap()
        .note
        .as_ref()
        .expect("viewer note");
    assert!(note.starts_with("Copied to clipboard:"), "{note}");
    assert!(note.ends_with("a.rs"), "{note}");
    let copied = crate::clipboard::copied().expect("nothing copied");
    assert_eq!(*note, format!("Copied to clipboard: {copied}"));
}

#[test]
fn test_bookmark_set_and_quick_jump() {
    let (_tmp, mut app) = sample();
    let root = app.current_path();
    let deep = root.join("sub/deep");

    // Navigate into sub/deep
    assert!(app.goto(&deep));
    assert_eq!(app.current_path(), deep);

    // Press Ctrl+F11 to bookmark current directory
    app.handle_key(KeyEvent::new(KeyCode::F(11), KeyModifiers::CONTROL));
    assert!(app.prompt.as_ref().unwrap().is_bookmark_set());

    // Assign key 'd'
    press(&mut app, KeyCode::Char('d'));
    assert!(app.prompt.is_none());
    assert_eq!(app.bookmarks["d"].path, deep);

    // Go back to root
    assert!(app.goto(&root));
    assert_eq!(app.current_path(), root);

    // Quick jump with single quote (')
    press(&mut app, KeyCode::Char('\''));
    assert!(app.prompt.as_ref().unwrap().is_bookmark_jump());

    // Press 'd' -> jumps immediately to sub/deep
    press(&mut app, KeyCode::Char('d'));
    assert!(app.prompt.is_none());
    assert_eq!(app.current_path(), deep);
}

#[test]
fn test_bookmark_jump_outside_tree() {
    let (_tmp, mut app) = sample();
    let other = crate::testing::tempdir();
    let other_path = dunce::canonicalize(other.path()).unwrap();

    app.set_bookmark("o".into(), other_path.clone());

    // Jump with ' followed by o
    press(&mut app, KeyCode::Char('\''));
    press(&mut app, KeyCode::Char('o'));

    assert_eq!(app.current_path(), other_path);
}

#[test]
fn test_bookmark_jump_nonexistent_directory() {
    let (_tmp, mut app) = sample();
    let root = app.current_path();

    app.set_bookmark("z".into(), PathBuf::from("/nonexistent/directory/xyz"));

    press(&mut app, KeyCode::Char('\''));
    press(&mut app, KeyCode::Char('z'));

    assert_eq!(app.current_path(), root);
    assert!(app.message.as_ref().unwrap().contains("Cannot jump to"));
}

#[test]
fn test_bookmarks_manager_navigation_and_delete() {
    let (_tmp, mut app) = sample();
    let root = app.current_path();
    let deep = root.join("sub/deep");

    app.set_bookmark("1".into(), root.clone());
    app.set_bookmark("2".into(), deep.clone());

    // Open bookmarks with F11
    press(&mut app, KeyCode::F(11));
    assert!(app.bookmarks_view.is_some());
    assert_eq!(app.bookmarks_view.as_ref().unwrap().selected, 0);

    // Navigate down
    press(&mut app, KeyCode::Down);
    assert_eq!(app.bookmarks_view.as_ref().unwrap().selected, 1);

    // Enter jumps to selected bookmark ("2" -> deep)
    press(&mut app, KeyCode::Enter);
    assert!(app.bookmarks_view.is_none());
    assert_eq!(app.current_path(), deep);

    // Open bookmarks again and delete the selected one
    press(&mut app, KeyCode::F(11));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('d')); // Delete "2"
    assert_eq!(app.bookmarks.len(), 1);
    assert!(!app.bookmarks.contains_key("2"));
    assert!(app.bookmarks.contains_key("1"));

    // Close with Esc
    press(&mut app, KeyCode::Esc);
    assert!(app.bookmarks_view.is_none());
}

#[test]
fn test_bookmark_terminal_friendly_shortcuts() {
    let (_tmp, mut app) = sample();
    let root = app.current_path();
    let deep = root.join("sub/deep");

    // Initially bookmarks is empty; pressing single quote (') immediately prompts to set bookmark
    press(&mut app, KeyCode::Char('\''));
    assert!(app.prompt.as_ref().unwrap().is_bookmark_set());
    // Cancel with Esc
    press(&mut app, KeyCode::Esc);
    assert!(app.prompt.is_none());

    // In Tree, press Ctrl+B to bookmark current directory (fallback for Ctrl+F11)
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
    assert!(app.prompt.as_ref().unwrap().is_bookmark_set());
    press(&mut app, KeyCode::Char('1'));
    assert!(app.prompt.is_none());
    assert_eq!(app.bookmarks["1"].path, root);

    // Open bookmarks modal with Alt+B
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT));
    assert!(app.bookmarks_view.is_some());

    // Toggle close with Alt+B
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT));
    assert!(app.bookmarks_view.is_none());

    // Navigate to sub/deep
    assert!(app.goto(&deep));

    // Press ' to jump, then press '+' to set a bookmark for sub/deep
    press(&mut app, KeyCode::Char('\''));
    assert!(app.prompt.as_ref().unwrap().is_bookmark_jump());
    press(&mut app, KeyCode::Char('+'));
    assert!(app.prompt.as_ref().unwrap().is_bookmark_set());
    press(&mut app, KeyCode::Char('2'));
    assert_eq!(app.bookmarks["2"].path, deep);

    // Go back to root
    assert!(app.goto(&root));

    // Jump to '2' with ' 2
    press(&mut app, KeyCode::Char('\''));
    press(&mut app, KeyCode::Char('2'));
    assert_eq!(app.current_path(), deep);
}

#[test]
fn rename_tagged_files_with_wildcard_mask() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    // Tag both files
    press(&mut app, KeyCode::Char('t'));
    press(&mut app, KeyCode::Char('t'));
    assert_eq!(tagged(&app).len(), 2);

    // Ctrl+R opens prompt for tagged rename
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
    assert!(app.prompt.as_ref().unwrap().is_rename_tagged());

    // Enter mask prefix_*.*
    answer_prompt(&mut app, "prefix_*.*");
    assert!(tmp.path().join("prefix_a.rs").exists());
    assert!(tmp.path().join("prefix_b.txt").exists());
}

#[test]
fn rename_tagged_files_with_counters_and_search_replace() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('t'));
    press(&mut app, KeyCode::Char('t'));

    // Alt+R also triggers rename_tagged
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::ALT));
    assert!(app.prompt.as_ref().unwrap().is_rename_tagged());

    // Use counter %02d
    answer_prompt(&mut app, "item_%02d.*");
    assert!(tmp.path().join("item_01.rs").exists());
    assert!(tmp.path().join("item_02.txt").exists());

    // Tag the new files and use search & replace s/item_/test_/
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Char('t'));
    press(&mut app, KeyCode::Char('t'));
    assert_eq!(tagged(&app).len(), 2);
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
    answer_prompt(&mut app, "s/item_/test_/");
    assert!(tmp.path().join("test_01.rs").exists());
    assert!(tmp.path().join("test_02.txt").exists());
}

#[test]
fn rename_tagged_f2_cycles_case() {
    let (tmp, mut app) = sample();
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('t'));

    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
    assert!(app.prompt.as_ref().unwrap().label.contains("mask"));

    // Press F2 to cycle case to Lower
    press(&mut app, KeyCode::F(2));
    assert!(app.prompt.as_ref().unwrap().label.contains("lowercase"));

    // Press F2 to cycle case to Upper
    press(&mut app, KeyCode::F(2));
    assert!(app.prompt.as_ref().unwrap().label.contains("UPPERCASE"));

    answer_prompt(&mut app, "*.*");
    assert!(tmp.path().join("A.RS").exists());
}

/// The sample tree seen as a remote machine (OpenSSH's sftp-server spoken to
/// directly), in the only window. `None` without sftp-server.
#[cfg(unix)]
fn remote_sample() -> Option<(tempfile::TempDir, App)> {
    let (tmp, mut app) = sample();
    let remote = crate::remote::for_tests(tmp.path())?;
    app.panes[0] = Pane::from_remote(remote);
    Some((tmp, app))
}

#[cfg(unix)]
#[test]
fn o_on_a_remote_machine_downloads_first() {
    let Some((tmp, mut app)) = remote_sample() else {
        return;
    };
    let (opener, out) = fake_opener(tmp.path());
    app.options.opener = opener;
    press(&mut app, KeyCode::Enter);
    let i = names(&app).iter().position(|n| n == "b.txt").unwrap();
    app.file_state.select(Some(i));
    press(&mut app, KeyCode::Char('o'));
    finish_op(&mut app);
    assert_eq!(app.message.as_deref(), Some("Opened 1 file(s)"));
    let text = fs::read_to_string(tmp.path().join("b.txt")).unwrap();
    assert!(wait_for_text(&out.join("b.txt"), &text));
    let run = platform::run_dir_path(&platform::cache_dir().unwrap());
    let _ = fs::remove_file(run.join("b.txt"));
    let _ = fs::remove_dir(run);
}

#[cfg(unix)]
#[test]
fn remote_window_lists_views_and_copies_here() {
    let Some((tmp, mut app)) = remote_sample() else {
        return;
    };
    assert!(app.current_path().starts_with("test@testhost:"));
    assert_eq!(names(&app), ["a.rs", "b.txt"]);
    assert_eq!(app.help_topic(), "remote");

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('v'));
    let viewer = app.viewer.as_ref().expect("viewer open");
    assert_eq!(viewer.visible_lines(1), ["fn main() {}"]);
    press(&mut app, KeyCode::Esc);

    // Copy brings the file here, with its date.
    let local = tempfile::tempdir_in(tmp.path()).unwrap();
    press(&mut app, KeyCode::Char('c'));
    answer_prompt(&mut app, local.path().to_str().unwrap());
    let copied = local.path().join("a.rs");
    assert_eq!(std::fs::read_to_string(&copied).unwrap(), "fn main() {}\n");
    // SFTP v3 dates are whole seconds.
    let seconds = |p: &std::path::Path| {
        let t = std::fs::metadata(p).unwrap().modified().unwrap();
        t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    };
    assert_eq!(seconds(&copied), seconds(&tmp.path().join("a.rs")));

    // Writing on the remote machine is not there yet.
    press(&mut app, KeyCode::Char('d'));
    assert!(app.confirm.is_none());
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .starts_with("Read-only on a remote machine")
    );
    press(&mut app, KeyCode::Char('c'));
    answer_prompt(&mut app, "test@testhost:/tmp");
    assert!(
        app.message
            .as_deref()
            .unwrap()
            .contains("remote machine comes in a later version")
    );
}

#[cfg(unix)]
#[test]
fn remote_window_logs_and_searches() {
    let Some((_tmp, mut app)) = remote_sample() else {
        return;
    };
    // Branch logs the remote branch.
    press(&mut app, KeyCode::Char('b'));
    while app.busy() {
        app.tick(Duration::from_millis(50));
    }
    let mut all = names(&app);
    all.sort();
    assert_eq!(all, ["a.rs", "a.rs", "b.txt", "c.rs"]);
    // Ctrl+S reads the remote files.
    ctrl(&mut app, 't');
    ctrl(&mut app, 's');
    answer_prompt(&mut app, "deep");
    let tagged: Vec<String> = app
        .files
        .iter()
        .filter(|&&f| app.file(f).tagged)
        .map(|&f| app.file(f).name.to_string_lossy().into_owned())
        .collect();
    assert_eq!(tagged, ["a.rs"], "only sub/deep/a.rs has the word");
}

#[test]
fn e_on_a_binary_file_needs_an_editor_from_the_config() {
    let tmp = crate::testing::tempdir();
    write(
        tmp.path().join("photo.jpg"),
        b"\xff\xd8\xff\xe0\0\x10JFIF\0",
    )
    .unwrap();
    write(tmp.path().join("raw.bin"), b"\x7fELF\0\x01\x02").unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    // When edit_binary is false, E refuses binary files even if a hex editor is installed.
    app.options.edit_binary = false;
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('e'));
    assert!(app.external.is_none(), "no text editor for binary files");
    assert!(app.message.take().unwrap().starts_with("Binary file"));

    // The same from the viewer (in hex).
    press(&mut app, KeyCode::Char('v'));
    press(&mut app, KeyCode::Char('e'));
    assert!(app.external.is_none());
    let v = app.viewer.as_ref().expect("still viewing");
    assert!(v.note.as_deref().unwrap().starts_with("Binary file"));
    press(&mut app, KeyCode::Esc);

    // With edit_binary = true and binary_editor set in options (or [editors]), E opens it.
    let mut config = Config::default();
    config.options.edit_binary = true;
    config.options.binary_editor = "bvi".into();
    config.editors.0 = vec![
        ("*.png *.JPG".into(), "gimp %f &".into()),
        ("binary".into(), "hexedit".into()),
    ];
    app.apply_config(&config).unwrap();
    // photo.jpg matches the pattern "*.png *.JPG".
    press(&mut app, KeyCode::Char('e'));
    let ext = app.external.take().expect("editor");
    let line = ext.line.to_string_lossy();
    assert!(line.starts_with("gimp ") && line.ends_with(" &"), "{line}");
    assert!(ext.file.is_some_and(|f| f.ends_with("photo.jpg")), "{line}");

    // raw.bin uses options.binary_editor ("bvi"), or falls back to [editors].binary ("hexedit").
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('e'));
    let ext = app.external.take().expect("binary_editor");
    assert!(ext.line.to_string_lossy().starts_with("bvi "));

    app.options.binary_editor.clear();
    press(&mut app, KeyCode::Char('e'));
    let ext = app.external.take().expect("[editors].binary");
    assert!(ext.line.to_string_lossy().starts_with("hexedit "));
}

#[test]
fn viewer_goto_line_and_toggles() {
    let (tmp, mut app) = sample();
    let many_lines = (1..=50).map(|i| format!("line {i}\n")).collect::<String>();
    write(tmp.path().join("a.rs"), many_lines).unwrap();
    app.apply(Command::Relog);

    // Enter files window and view
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('v'));
    assert!(app.viewer.is_some());

    // Toggle wrap with 'w'
    press(&mut app, KeyCode::Char('w'));
    assert!(app.viewer.as_ref().unwrap().wrap);

    // Toggle line numbers with '#'
    press(&mut app, KeyCode::Char('#'));
    assert!(app.viewer.as_ref().unwrap().line_numbers);

    // Goto line 35 with 'g'
    press(&mut app, KeyCode::Char('g'));
    assert!(app.prompt.as_ref().unwrap().is_goto());
    answer_prompt(&mut app, "35");
    assert_eq!(app.viewer.as_ref().unwrap().top, 34);

    // Goto with ':' and ':10'
    press(&mut app, KeyCode::Char(':'));
    assert!(app.prompt.as_ref().unwrap().is_goto());
    answer_prompt(&mut app, ":10");
    assert_eq!(app.viewer.as_ref().unwrap().top, 9);
}

#[test]
fn line_filter_prompt_toggles_applies_and_opens_its_help() {
    let (tmp, mut app) = sample();
    let path = tmp.path().join("log.txt");
    std::fs::write(&path, "ok\nfail 12\nok\nFAIL 7\n").unwrap();
    app.viewer = Some(Viewer::open(path, viewer::LoadOptions::view(1 << 20)).unwrap());

    press(&mut app, KeyCode::Char('&'));
    let label = |app: &App| app.prompt.as_ref().unwrap().label.clone();
    assert_eq!(label(&app), "FILTER lines (text, ignore case): ");
    press(&mut app, KeyCode::F(4));
    assert_eq!(label(&app), "FILTER lines (regex, ignore case): ");
    press(&mut app, KeyCode::F(2));
    press(&mut app, KeyCode::F(3));
    assert_eq!(label(&app), "FILTER lines (regex, match case, inverted): ");
    press(&mut app, KeyCode::F(2));
    press(&mut app, KeyCode::F(3));

    // F1 explains regular expressions and comes back to the prompt.
    let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
    assert_eq!(help_topic_after(&mut app, f1), "filter");
    assert!(app.prompt.as_ref().unwrap().is_line_filter());

    answer_prompt(&mut app, r"^fail \d+$");
    let v = app.viewer.as_ref().unwrap();
    assert_eq!(v.line_count(), 2);
    assert_eq!(v.total_lines(), 4);
    assert_eq!(v.note.as_deref(), Some("Filter: 2 of 4 lines"));

    // The prompt comes back with the filter in effect, ready to change it.
    press(&mut app, KeyCode::Char('&'));
    assert_eq!(label(&app), "FILTER lines (regex, ignore case): ");
    assert_eq!(app.prompt.as_ref().unwrap().input.text, r"^fail \d+$");
    let prompt = app.prompt.as_mut().unwrap();
    prompt.input.text.clear();
    prompt.input.cursor = 0;
    press(&mut app, KeyCode::Enter);
    let v = app.viewer.as_ref().unwrap();
    assert_eq!(v.line_count(), 4);
    assert_eq!(v.note.as_deref(), Some("Filter off: 4 lines"));

    // Another file starts with no filter, set the same way.
    press(&mut app, KeyCode::Char('&'));
    assert_eq!(label(&app), "FILTER lines (regex, ignore case): ");
    assert_eq!(app.prompt.as_ref().unwrap().input.text, "");
}

#[test]
fn line_filter_survives_editing_the_file() {
    let (tmp, mut app) = sample();
    let path = tmp.path().join("log.txt");
    std::fs::write(&path, "a\nkeep\nb\n").unwrap();
    app.viewer = Some(Viewer::open(path.clone(), viewer::LoadOptions::view(1 << 20)).unwrap());
    press(&mut app, KeyCode::Char('&'));
    answer_prompt(&mut app, "keep");
    std::fs::write(&path, "keep\na\nkeep\nkeep\n").unwrap();
    let v = app.viewer.as_mut().unwrap();
    v.reload().unwrap();
    assert_eq!(v.line_count(), 3);
    assert_eq!(v.total_lines(), 4);
}

fn ctrl_b(app: &mut App) {
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
}

#[test]
fn batch_in_the_viewer_writes_the_filtered_records() {
    let (tmp, mut app) = sample();
    let path = tmp.path().join("in.csv");
    std::fs::write(&path, "id,name\n1,ana\n2,bob\n3,cid\n").unwrap();
    app.viewer = Some(Viewer::open(path, viewer::LoadOptions::view(1 << 20)).unwrap());
    press(&mut app, KeyCode::Char('&'));
    answer_prompt(&mut app, "id >= 2");

    let out = tmp.path().join("out.sh");
    ctrl_b(&mut app);
    answer_prompt(&mut app, out.to_str().unwrap());
    let guide = app.prompt.as_ref().unwrap().batch_guide().unwrap();
    assert!(guide.fields.contains("id, name, #"), "{}", guide.fields);
    assert_eq!(guide.example, "%{id} %{name}");
    answer_prompt(&mut app, "echo %{#} %{name}");

    // Nothing is written before the preview is accepted.
    let preview = app.batch_preview.as_ref().expect("preview");
    assert_eq!(preview.preview.lines, ["echo '1' 'bob'", "echo '2' 'cid'"]);
    assert!(!out.exists());
    press(&mut app, KeyCode::Enter);
    while app.busy() {
        app.tick(Duration::from_millis(5));
    }
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "#!/bin/sh\necho '1' 'bob'\necho '2' 'cid'\n"
    );
    assert_eq!(
        app.viewer.as_ref().unwrap().note,
        Some(format!("Wrote 2 records to {}", out.display()))
    );
}

#[test]
fn batch_of_text_gives_the_groups_of_the_line_filter() {
    let (tmp, mut app) = sample();
    let path = tmp.path().join("log.txt");
    std::fs::write(&path, "ip 1.2.3.4 ok\nnothing\nip 5.6.7.8 ok\n").unwrap();
    app.viewer = Some(Viewer::open(path, viewer::LoadOptions::view(1 << 20)).unwrap());
    press(&mut app, KeyCode::Char('&'));
    press(&mut app, KeyCode::F(4));
    answer_prompt(&mut app, r"ip (?P<addr>\S+)");

    let out = tmp.path().join("out.txt");
    ctrl_b(&mut app);
    answer_prompt(&mut app, out.to_str().unwrap());
    answer_prompt(&mut app, "%{#line}=%{addr}");
    let preview = app.batch_preview.as_ref().expect("preview");
    assert_eq!(preview.preview.lines, ["1=1.2.3.4", "3=5.6.7.8"]);
    assert!(preview.preview.complete);
    // Esc leaves everything as it was.
    press(&mut app, KeyCode::Esc);
    assert!(app.batch_preview.is_none());
    assert!(app.viewer.is_some(), "the viewer stays open");
    assert!(!out.exists());
}

#[test]
fn batch_refuses_what_it_cannot_do_safely() {
    let (tmp, mut app) = sample();
    let path = tmp.path().join("in.csv");
    std::fs::write(&path, "id\n1\n").unwrap();
    app.viewer = Some(Viewer::open(path, viewer::LoadOptions::view(1 << 20)).unwrap());
    let note = |app: &App| {
        app.viewer
            .as_ref()
            .unwrap()
            .note
            .clone()
            .unwrap_or_default()
    };

    ctrl_b(&mut app);
    let script = tmp.path().join("run.bat");
    answer_prompt(&mut app, script.to_str().unwrap());
    assert!(note(&app).contains(".bat"));
    assert!(app.prompt.is_none());

    let taken = tmp.path().join("taken.txt");
    std::fs::write(&taken, "mine").unwrap();
    ctrl_b(&mut app);
    answer_prompt(&mut app, taken.to_str().unwrap());
    assert!(note(&app).contains("already exists"));

    let out = tmp.path().join("out.txt");
    ctrl_b(&mut app);
    answer_prompt(&mut app, out.to_str().unwrap());
    answer_prompt(&mut app, "%{nope}");
    assert!(app.batch_preview.is_none());
    assert!(!out.exists());
    // The mask comes back to be corrected, saying what is wrong with it.
    let prompt = app.prompt.as_ref().expect("the prompt stays open");
    assert_eq!(prompt.input.text, "%{nope}");
    assert_eq!(
        prompt.batch_guide().unwrap().problem.as_deref(),
        Some("Mask: no field named nope")
    );
    answer_prompt(&mut app, "");
    let prompt = app.prompt.as_ref().expect("an empty mask asks again");
    let problem = prompt.batch_guide().unwrap().problem.clone().unwrap();
    assert!(problem.contains(":template"), "{problem}");
    press(&mut app, KeyCode::Esc);
    assert!(app.prompt.is_none());

    press(&mut app, KeyCode::Char('h'));
    press(&mut app, KeyCode::Char('h'));
    ctrl_b(&mut app);
    assert!(note(&app).contains("text and tables"));
    assert!(app.prompt.is_none());
}

#[test]
fn batch_says_what_works_on_the_open_file() {
    let (tmp, mut app) = sample();
    let path = tmp.path().join("log.txt");
    std::fs::write(&path, "ip 1.2.3.4\n").unwrap();
    app.viewer = Some(Viewer::open(path, viewer::LoadOptions::view(1 << 20)).unwrap());
    let guide = |app: &mut App| {
        ctrl_b(app);
        let out = tmp.path().join("out.txt");
        answer_prompt(app, out.to_str().unwrap());
        let prompt = app.prompt.as_ref().unwrap();
        let guide = prompt.batch_guide().unwrap();
        let found = (guide.example.clone(), guide.open_template.clone());
        press(app, KeyCode::Esc);
        found
    };
    assert_eq!(guide(&mut app), ("%{#line}: %{line}".to_string(), None));

    press(&mut app, KeyCode::Char('&'));
    press(&mut app, KeyCode::F(4));
    answer_prompt(&mut app, r"ip (?P<addr>\S+)");
    assert_eq!(guide(&mut app).0, "%{#line}: %{addr}");

    // A template opened by mistake is data to the Batch: it says so.
    let path = tmp.path().join("report.tpl");
    std::fs::write(&path, "%%record\n%{a}\n").unwrap();
    app.viewer = Some(Viewer::open(path, viewer::LoadOptions::view(1 << 20)).unwrap());
    let warning = guide(&mut app).1.expect("a warning");
    assert!(warning.contains("open the data"), "{warning}");
}

#[test]
fn batch_arrows_choose_a_template_and_leave_typing_alone() {
    let (tmp, mut app) = sample();
    let path = tmp.path().join("a.txt");
    std::fs::write(&path, "x\n").unwrap();
    app.viewer = Some(Viewer::open(path, viewer::LoadOptions::view(1 << 20)).unwrap());
    ctrl_b(&mut app);
    let out = tmp.path().join("out.txt");
    answer_prompt(&mut app, out.to_str().unwrap());
    // Not the templates of whoever runs the tests.
    {
        let prompt = app.prompt.as_mut().unwrap();
        let PromptKind::ViewerBatchMask { guide, .. } = &mut prompt.kind else {
            panic!("the mask prompt");
        };
        guide.templates = ["a", "b"]
            .map(|name| viewer::batch::TemplateInfo {
                name: name.to_string(),
                about: String::new(),
            })
            .to_vec();
    }
    let text = |app: &App| app.prompt.as_ref().unwrap().input.text.clone();
    let selected = |app: &App| app.prompt.as_ref().unwrap().batch_guide().unwrap().selected;

    press(&mut app, KeyCode::Down);
    assert_eq!((text(&app).as_str(), selected(&app)), (":a", Some(0)));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    assert_eq!((text(&app).as_str(), selected(&app)), (":b", Some(1)));
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Up);
    assert_eq!((text(&app).as_str(), selected(&app)), ("", None));

    // A mask being typed is not a choice, and the arrows do not replace it.
    press(&mut app, KeyCode::Char('z'));
    press(&mut app, KeyCode::Down);
    assert_eq!((text(&app).as_str(), selected(&app)), ("z", None));
    // Typing the name of one is the same as choosing it.
    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::Char(':'));
    press(&mut app, KeyCode::Char('b'));
    assert_eq!(selected(&app), Some(1));
    press(&mut app, KeyCode::Esc);
    assert!(app.prompt.is_none());
    assert!(app.viewer.is_some());
}

#[test]
fn ctrl_b_in_the_viewer_is_not_the_backward_search() {
    let (tmp, mut app) = sample();
    let path = tmp.path().join("a.txt");
    std::fs::write(&path, "x\n").unwrap();
    app.viewer = Some(Viewer::open(path, viewer::LoadOptions::view(1 << 20)).unwrap());
    ctrl_b(&mut app);
    assert!(app.prompt.as_ref().unwrap().label.starts_with("BATCH"));
    let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
    assert_eq!(help_topic_after(&mut app, f1), "execute");
}

#[test]
fn autoview_supports_wrap_and_line_number_keys() {
    let (_tmp, mut app) = sample();
    app.apply(Command::Autoview);
    assert!(app.autoview.is_some());

    // In autoview, 'w' toggles wrap
    press(&mut app, KeyCode::Char('w'));
    assert!(app.autoview.as_ref().unwrap().wrap);

    // In autoview, '#' toggles line numbers
    press(&mut app, KeyCode::Char('#'));
    assert!(app.autoview.as_ref().unwrap().line_numbers);
}

#[test]
fn o_asks_before_opening_a_name_with_hidden_bidi() {
    let tmp = crate::testing::tempdir();
    std::fs::write(tmp.path().join("fatura\u{202E}fdp.sh"), "").unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    press(&mut app, KeyCode::Enter);
    assert!(app.selected_name_has_bidi());
    press(&mut app, KeyCode::Char('o'));
    let c = app.confirm.as_ref().expect("asks first");
    assert!(
        c.question.contains("fatura\\u{202E}fdp.sh"),
        "{}",
        c.question
    );
    press(&mut app, KeyCode::Char('n'));
    assert!(app.confirm.is_none());
}

#[test]
fn o_on_a_hidden_bidi_name_opens_what_it_asked_about() {
    let tmp = crate::testing::tempdir();
    std::fs::write(tmp.path().join("fatura\u{202E}fdp.sh"), "").unwrap();
    std::fs::write(tmp.path().join("plain.txt"), "").unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    press(&mut app, KeyCode::Enter);
    let at = |app: &App, name: &str| names(app).iter().position(|n| n == name).unwrap();
    let i = at(&app, "fatura\u{202E}fdp.sh");
    app.file_state.select(Some(i));
    press(&mut app, KeyCode::Char('o'));
    assert!(app.confirm.is_some(), "asks first");
    // The selection changes before the answer (a rescan, say): 'y' still
    // opens the file the question named, not the one under the cursor now.
    let j = at(&app, "plain.txt");
    app.file_state.select(Some(j));
    crate::shell::OPENED.take();
    press(&mut app, KeyCode::Char('y'));
    let opened: Vec<String> = crate::shell::OPENED
        .take()
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(opened, ["fatura\u{202E}fdp.sh"]);
}

/// Opens Duplicates on a folder with a copy of a large file, a copy of a whole
/// folder, and a file with nothing like it.
fn duplicates_sample() -> (tempfile::TempDir, App) {
    let tmp = crate::testing::tempdir();
    let p = tmp.path();
    for dir in ["photos", "photos-copy"] {
        create_dir_all(p.join(dir)).unwrap();
        write(p.join(dir).join("trip.jpg"), vec![1u8; 5000]).unwrap();
    }
    write(p.join("alone.bin"), vec![2u8; 7000]).unwrap();
    let mut app = App::new(p.to_path_buf());
    alt(&mut app, 'y');
    finish(&mut app);
    (tmp, app)
}

fn finish(app: &mut App) {
    while app.busy() {
        app.tick(Duration::from_millis(5));
    }
}

#[test]
fn alt_y_finds_files_and_folders() {
    let (tmp, mut app) = duplicates_sample();
    let view = app.duplicates.as_ref().expect("open");
    assert_eq!(view.files.len(), 1);
    assert_eq!(view.folders.len(), 1);
    assert_eq!(
        view.groups()[0].items[0].path,
        tmp.path().join("photos/trip.jpg")
    );
    press(&mut app, KeyCode::Char('g'));
    let view = app.duplicates.as_ref().unwrap();
    assert!(view.show_folders);
    assert_eq!(
        view.groups()[0].items[1].path,
        tmp.path().join("photos-copy")
    );
    press(&mut app, KeyCode::Esc);
    assert!(app.duplicates.is_none());
}

#[test]
fn alt_y_with_nothing_equal_says_so() {
    let (_tmp, mut app) = sample();
    alt(&mut app, 'y');
    finish(&mut app);
    assert!(app.duplicates.is_none());
    assert!(app.message.as_deref().unwrap().contains("No duplicates"));
}

#[test]
fn deleting_the_tagged_copy_leaves_the_other() {
    let (tmp, mut app) = duplicates_sample();
    press(&mut app, KeyCode::Char('a'));
    press(&mut app, KeyCode::Char('d'));
    assert!(app.confirm.is_some());
    press(&mut app, KeyCode::Char('y'));
    finish(&mut app);
    let kept = [
        tmp.path().join("photos/trip.jpg").exists(),
        tmp.path().join("photos-copy/trip.jpg").exists(),
    ];
    assert_eq!(kept.iter().filter(|k| **k).count(), 1);
    assert!(
        app.duplicates.as_ref().unwrap().files.is_empty(),
        "no group left"
    );
}

#[test]
fn deleting_a_copy_of_a_folder_takes_what_it_holds() {
    let (tmp, mut app) = duplicates_sample();
    press(&mut app, KeyCode::Char('g'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('k')); // Keeps photos-copy.
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('y'));
    finish(&mut app);
    assert!(!tmp.path().join("photos").exists());
    assert!(tmp.path().join("photos-copy/trip.jpg").exists());
}

#[test]
fn a_copy_changed_after_the_analysis_is_left_alone() {
    let (tmp, mut app) = duplicates_sample();
    press(&mut app, KeyCode::Char('a'));
    write(tmp.path().join("photos/trip.jpg"), vec![9u8; 6000]).unwrap();
    press(&mut app, KeyCode::Char('d'));
    assert!(app.confirm.is_none());
    assert!(app.message.as_deref().unwrap().contains("changed since"));
    assert!(tmp.path().join("photos-copy/trip.jpg").exists());
}

/// The copy in `duplicates_sample`, as the system writes a path.
fn copy_path() -> String {
    Path::new("photos-copy")
        .join("trip.jpg")
        .display()
        .to_string()
}

#[test]
fn the_list_is_written_and_never_over_a_file() {
    let (tmp, mut app) = duplicates_sample();
    press(&mut app, KeyCode::Char('e'));
    press(&mut app, KeyCode::Enter);
    let text = std::fs::read_to_string(tmp.path().join("duplicates.txt")).unwrap();
    assert!(text.contains(&copy_path()));
    let before = text.len();
    press(&mut app, KeyCode::Char('e'));
    press(&mut app, KeyCode::Enter);
    assert!(
        app.message
            .as_deref()
            .unwrap_or_default()
            .contains("Cannot write")
    );
    let after = std::fs::read_to_string(tmp.path().join("duplicates.txt")).unwrap();
    assert_eq!(after.len(), before);
}

#[test]
fn the_viewer_opens_over_the_duplicates_and_goes_back() {
    let (_tmp, mut app) = duplicates_sample();
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(app.viewer.is_some());
    press(&mut app, KeyCode::Esc);
    assert!(app.viewer.is_none());
    assert!(app.duplicates.is_some());
}

#[test]
fn f3_sees_what_changed_after_the_tree_read_it() {
    let (tmp, mut app) = duplicates_sample();
    write(tmp.path().join("alone-too.bin"), vec![2u8; 7000]).unwrap();
    press(&mut app, KeyCode::F(3));
    finish(&mut app);
    assert_eq!(app.duplicates.as_ref().unwrap().files.len(), 2);
}

#[test]
fn the_duplicates_view_is_drawn() {
    use ratatui::{Terminal, backend::TestBackend};
    let (_tmp, mut app) = duplicates_sample();
    press(&mut app, KeyCode::Char('a'));
    let mut term = Terminal::new(TestBackend::new(120, 24)).unwrap();
    term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let text: String = term
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(text.contains("Duplicate files:"));
    assert!(text.contains("to free"));
    assert!(text.contains(&copy_path()));
    assert!(text.contains(" T "));
    assert!(text.contains("all but one"));
    assert!(text.contains("DUPLICATE"), "the panel on the right");
}

#[test]
fn usage_d_goes_to_the_duplicates_of_the_folder() {
    let (_tmp, mut app) = duplicates_sample();
    press(&mut app, KeyCode::Esc);
    alt(&mut app, 'w');
    finish(&mut app);
    press(&mut app, KeyCode::Char('d'));
    finish(&mut app);
    assert!(app.usage.is_none());
    assert!(app.duplicates.is_some());
}
