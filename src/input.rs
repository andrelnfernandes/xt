//! Commands and the prompts' text field. The key map is in
//! `keymap.rs`.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Expand,
    Collapse,
    /// `-`: releases the branch from memory (unlogs it).
    Release,
    /// Logs and expands the whole branch.
    ExpandAll,
    /// Switches between the tree and the file window.
    SwitchWindow,
    /// Tab: switches window in split screen.
    SwitchPane,
    /// F8: splits the screen into two windows (or back to one).
    Split,
    /// Opens another directory in the window.
    Log,
    /// Compares with the other window's directory and tags files.
    Compare,
    /// Opens a ZIP/TAR as if it were a directory (Alt+F5).
    OpenArchive,
    /// Opens the file (or the tagged ones) in the system's default program.
    OpenWith {
        tagged: bool,
    },
    /// Alt+C in the tree: Compare of the whole branch.
    CompareBranch,
    /// Backspace: parent directory.
    Parent,
    /// Opens the file in the editor.
    Edit,
    /// Runs a shell command.
    Execute,
    /// Alt+X: shows the built-in terminal (or goes back to Xylem).
    Terminal,
    /// Alt+Z: the terminal in full screen (inside it: full screen / panel).
    TerminalFull,
    /// Alt+O: types the tagged files in the terminal.
    TerminalTagged,
    /// Ctrl+G: Git menu (when `git` is on in the configuration).
    Git,
    /// F1: key list.
    Help,
    /// Alt+S: chooses the file sort order.
    Sort,
    /// Alt+P: deletes the current directory with everything inside.
    Prune,
    /// F12 / Alt+F10: settings screen.
    Settings,
    /// Ctrl+S: searches for text in the tagged files and untags those without it.
    SearchTagged,
    /// Ctrl+V: views the tagged files, one after the other.
    ViewTagged,
    /// `\`: navigates by typing the path.
    Treespec,
    /// Alt+K: size or number of files per directory in the tree.
    Kbyte,
    /// Alt+W: where the space of the branch goes.
    Usage,
    /// Alt+Q: quits, leaving the shell in the current directory.
    QuitChangeDir,
    /// Alt+F: display mode of the file window.
    FileDisplay,
    /// Ctrl+F7 / Ctrl+F8 / Ctrl+F9 in the tree: tags, untags or inverts
    /// the branch's logged files.
    TagBranch,
    UntagBranch,
    InvertBranch,
    /// F7: the file list with the selected file's content beside it.
    Autoview,
    /// N: changes the modification date (of the file, the tagged ones or the
    /// directory, in the tree).
    NewDate {
        tagged: bool,
    },
    /// Ctrl+B: creates a file with one line per tagged file.
    Batch,
    /// Alt+I: details of the file or directory.
    Info,
    /// A in the tree: mounted file systems.
    Avail,
    /// ?: window statistics.
    Stats,
    /// F5 / Ctrl+F5: creates a ZIP/TAR with the file or the tagged ones.
    CreateArchive {
        tagged: bool,
    },
    /// ← and → in the file window: previous/next file in columns,
    /// page up/down in a single column.
    Left,
    Right,
    Back,
    Filespec,
    Showall,
    /// Ctrl+S in the tree: Showall of the tagged files only.
    ShowallTagged,
    Branch,
    View,
    /// Rereads the current directory from disk.
    Relog,
    Tag,
    Untag,
    TagAll,
    UntagAll,
    Invert,
    // `tagged`: acts on the tagged files instead of the current file.
    // `paths`: recreates the subdirectories at the destination (Alt+C / Alt+M).
    Copy {
        tagged: bool,
        paths: bool,
    },
    Move {
        tagged: bool,
        paths: bool,
    },
    Delete {
        tagged: bool,
        permanent: bool,
    },
    /// Restores the most recently trashed file(s) from system trash (Oops!).
    Oops,
    Attributes {
        tagged: bool,
    },
    Rename {
        tagged: bool,
    },
    MakeDir,
    /// Copies path(s) to the system clipboard (OSC 52 / native tools).
    Clipboard {
        tagged: bool,
    },
    /// F11: opens the directory bookmarks list / manager.
    Bookmarks,
    /// ': quick jump to a bookmark key.
    BookmarkJump,
    /// Ctrl+F11: saves a bookmark for the current directory.
    BookmarkSet,
    Quit,
}

/// Result of a key in a text field.
#[derive(Debug, PartialEq, Eq)]
pub enum LineEvent {
    Submit,
    Cancel,
    Editing,
}

/// Single-line text field, used in prompts (Filespec etc.).
pub struct LineInput {
    pub text: String,
    /// Cursor position, in characters.
    pub cursor: usize,
    /// Previous entries of this kind of prompt, oldest to newest.
    history: Vec<String>,
    /// Position in the history while navigating with ↑/↓...
    history_pos: Option<usize>,
    /// ...and the text that was being typed, to go back to it.
    draft: String,
}

impl LineInput {
    pub fn new(text: &str) -> Self {
        LineInput {
            text: text.to_string(),
            cursor: text.chars().count(),
            history: Vec::new(),
            history_pos: None,
            draft: String::new(),
        }
    }

    pub fn with_history(mut self, history: Vec<String>) -> Self {
        self.history = history;
        self
    }

    pub fn history(&self) -> Vec<String> {
        self.history.clone()
    }

    pub fn set_text(&mut self, text: String) {
        self.cursor = text.chars().count();
        self.text = text;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> LineEvent {
        let len = self.text.chars().count();
        match key.code {
            KeyCode::Enter => return LineEvent::Submit,
            KeyCode::Esc => return LineEvent::Cancel,
            KeyCode::Up if !self.history.is_empty() => {
                let pos = match self.history_pos {
                    None => {
                        self.draft = self.text.clone();
                        self.history.len() - 1
                    }
                    Some(p) => p.saturating_sub(1),
                };
                self.history_pos = Some(pos);
                self.set_text(self.history[pos].clone());
            }
            KeyCode::Down => match self.history_pos {
                Some(p) if p + 1 < self.history.len() => {
                    self.history_pos = Some(p + 1);
                    self.set_text(self.history[p + 1].clone());
                }
                Some(_) => {
                    self.history_pos = None;
                    let draft = std::mem::take(&mut self.draft);
                    self.set_text(draft);
                }
                None => {}
            },
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(len),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = len,
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.text.remove(self.byte_pos());
            }
            KeyCode::Delete if self.cursor < len => {
                self.text.remove(self.byte_pos());
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.text.clear();
                self.cursor = 0;
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.text.insert(self.byte_pos(), c);
                self.cursor += 1;
            }
            _ => {}
        }
        LineEvent::Editing
    }

    fn byte_pos(&self) -> usize {
        self.text
            .char_indices()
            .nth(self.cursor)
            .map_or(self.text.len(), |(i, _)| i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_keys(input: &mut LineInput, codes: &[KeyCode]) {
        for &code in codes {
            input.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
        }
    }

    #[test]
    fn edits_text() {
        let mut input = LineInput::new("*.rs");
        assert_eq!(input.cursor, 4);
        type_keys(&mut input, &[KeyCode::Char(' '), KeyCode::Char('*')]);
        assert_eq!(input.text, "*.rs *");
        type_keys(
            &mut input,
            &[KeyCode::Home, KeyCode::Delete, KeyCode::Char('x')],
        );
        assert_eq!(input.text, "x.rs *");
        type_keys(
            &mut input,
            &[KeyCode::End, KeyCode::Backspace, KeyCode::Backspace],
        );
        assert_eq!(input.text, "x.rs");
    }

    #[test]
    fn walks_history() {
        let mut input = LineInput::new("draft").with_history(vec!["one".into(), "two".into()]);
        type_keys(&mut input, &[KeyCode::Up]);
        assert_eq!((input.text.as_str(), input.cursor), ("two", 3));
        type_keys(&mut input, &[KeyCode::Up, KeyCode::Up]);
        assert_eq!(input.text, "one");
        type_keys(&mut input, &[KeyCode::Down]);
        assert_eq!(input.text, "two");
        type_keys(&mut input, &[KeyCode::Down]);
        assert_eq!(input.text, "draft", "back to what was typed");
        type_keys(&mut input, &[KeyCode::Down]);
        assert_eq!(input.text, "draft");

        let mut empty = LineInput::new("x");
        type_keys(&mut empty, &[KeyCode::Up]);
        assert_eq!(empty.text, "x");
    }

    #[test]
    fn handles_multibyte_chars() {
        let mut input = LineInput::new("ação");
        type_keys(&mut input, &[KeyCode::Left, KeyCode::Backspace]);
        assert_eq!(input.text, "aço");
    }

    #[test]
    fn submit_and_cancel() {
        let mut input = LineInput::new("");
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        assert_eq!(input.handle_key(key(KeyCode::Enter)), LineEvent::Submit);
        assert_eq!(input.handle_key(key(KeyCode::Esc)), LineEvent::Cancel);
    }
}
