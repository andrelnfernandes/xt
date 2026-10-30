//! F1 / Alt+H: help about what the user is doing.

use super::*;
use crate::help::Help;

impl App {
    /// The help key, as bound in the key map (F1 and Alt+H by default).
    pub(crate) fn is_help_key(&self, key: KeyEvent) -> bool {
        self.keymap.lookup(key, self.focus) == Some(Command::Help)
    }

    /// The topic for the current state: the prompt, question or screen in
    /// front, else the window that has the focus.
    pub(crate) fn help_topic(&self) -> &'static str {
        if let Some(confirm) = &self.confirm {
            return match confirm.kind {
                ConfirmKind::Quit | ConfirmKind::QuitChangeDir(_) => "quit",
                ConfirmKind::DeleteFiles { .. }
                | ConfirmKind::DeleteDir { .. }
                | ConfirmKind::RestoreTrash(_) => "delete",
                ConfirmKind::CreateDest { .. } => "copy-move",
                ConfirmKind::Compare { .. } => "compare",
                ConfirmKind::Sort(_) => "sort",
                ConfirmKind::Kbyte(_) => "info",
                ConfirmKind::OpenHidden { .. } => "files",
            };
        }
        if self.settings.is_some() {
            return "configuration";
        }
        if self.bookmarks_view.is_some() {
            return "bookmarks";
        }
        if let Some(op) = &self.op {
            return match op.kind {
                OpKind::Delete | OpKind::Prune => "delete",
                OpKind::Rename => "rename",
                _ => "copy-move",
            };
        }
        if self.archive_job.is_some() {
            return "archives";
        }
        if self.search_job.is_some() {
            return "search";
        }
        if self.picker.is_some() {
            return "picker";
        }
        if let Some(prompt) = &self.prompt {
            return match prompt.kind {
                PromptKind::Transfer { .. } => "copy-move",
                PromptKind::Filespec => "filespec",
                PromptKind::Log | PromptKind::Treespec { .. } => "treespec",
                PromptKind::Execute | PromptKind::BatchFile | PromptKind::BatchMask { .. } => {
                    "execute"
                }
                PromptKind::RenameFile(_)
                | PromptKind::RenameDir(_)
                | PromptKind::RenameTagged { .. } => "rename",
                PromptKind::Chmod(_) | PromptKind::NewDate(_) => "attributes",
                PromptKind::Prune { .. } => "delete",
                PromptKind::SearchTagged { .. } => "search",
                PromptKind::ViewerSearch { .. }
                | PromptKind::ViewerGoto
                | PromptKind::ViewerSqlFilter => "viewer",
                PromptKind::CreateArchive(_) => "archives",
                PromptKind::BookmarkJump | PromptKind::BookmarkSet => "bookmarks",
                PromptKind::MakeDir(_) => "tree",
                PromptKind::Askpass { .. } => "remote",
            };
        }
        if self.viewer.is_some() {
            return if self.tagged_view.is_some() {
                "search"
            } else {
                "viewer"
            };
        }
        if self.sticky == MenuMode::Git {
            return "git";
        }
        if self.usage.is_some() {
            return "usage";
        }
        if self.remote.is_some() || self.connecting.is_some() {
            return "remote";
        }
        match self.focus {
            Focus::Tree => "tree",
            Focus::Files => "files",
        }
    }

    pub(crate) fn open_help(&mut self) {
        self.help = Some(Help::with_lang(
            self.help_topic(),
            self.keymap.help_text(),
            &self.options.help_language,
        ));
    }
}
