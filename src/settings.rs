//! Settings screen (F12 or Alt+F10), in the style of ZTreeWin's:
//! pages of options, the option's letter changes the value, F3 saves, Esc
//! cancels and F4 picks the theme (with an instant preview).

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::{self, Choice, Config, CopyPaths, THEMES, TerminalTheme, Theme};
use crate::input::{LineEvent, LineInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opt {
    Editor,
    Opener,
    DateFormat,
    NumberFormat,
    ShowHidden,
    FileDisplay,
    SyntaxColors,
    Sort,
    SortDescending,
    EnterViews,
    ConfirmQuit,
    PruneConfirmation,
    HistorySkipAt,
    CompareIgnore2s,
    CompareIgnore1h,
    AutoRefresh,
    Git,
    CopyPaths,
    TerminalTheme,
    TerminalHistory,
}

type Page = (&'static str, &'static [(char, &'static str, Opt)]);

#[cfg(unix)]
const EDITOR_LABEL: &str = "Editor (empty: $VISUAL, $EDITOR, vi)";
#[cfg(unix)]
const OPENER_LABEL: &str = "Open with (empty: xdg-open)";
#[cfg(windows)]
const EDITOR_LABEL: &str = "Editor (empty: %VISUAL%, %EDITOR%, notepad)";
#[cfg(windows)]
const OPENER_LABEL: &str = "Open with (empty: start)";

pub const PAGES: &[Page] = &[
    (
        "Programs and formats",
        &[
            ('A', EDITOR_LABEL, Opt::Editor),
            ('B', OPENER_LABEL, Opt::Opener),
            ('C', "Date format", Opt::DateFormat),
            ('D', "Number format", Opt::NumberFormat),
            (
                'E',
                "Git commands (Ctrl+G) and status in the file list",
                Opt::Git,
            ),
        ],
    ),
    (
        "Display and sorting",
        &[
            ('A', "Show hidden files and directories", Opt::ShowHidden),
            ('B', "Initial sort", Opt::Sort),
            ('C', "Initial sort order", Opt::SortDescending),
            ('D', "Enter in the file window", Opt::EnterViews),
            ('E', "Initial file display (Alt+F)", Opt::FileDisplay),
            ('F', "Syntax colors in the viewer", Opt::SyntaxColors),
            ('G', "Built-in terminal colors (Alt+X)", Opt::TerminalTheme),
        ],
    ),
    (
        "Confirmations and history",
        &[
            ('A', "Quit confirmation", Opt::ConfirmQuit),
            ('B', "Prune confirmation (type)", Opt::PruneConfirmation),
            (
                'C',
                "Entries starting with @ are not saved in history",
                Opt::HistorySkipAt,
            ),
            (
                'D',
                "Built-in terminal keeps a shell history of its own",
                Opt::TerminalHistory,
            ),
        ],
    ),
    (
        "Copy, compare and refresh",
        &[
            (
                'A',
                "Compare ignores 2 second timestamp difference",
                Opt::CompareIgnore2s,
            ),
            (
                'B',
                "Compare ignores 1 hour difference (daylight saving)",
                Opt::CompareIgnore1h,
            ),
            ('C', "Automatically refresh directories", Opt::AutoRefresh),
            (
                'D',
                "Alt+C / Alt+M recreate paths from (F7 at the prompt)",
                Opt::CopyPaths,
            ),
        ],
    ),
];

fn yes_no(b: bool) -> String {
    if b { "Yes" } else { "No" }.into()
}

impl Opt {
    pub fn value(self, c: &Config) -> String {
        let o = &c.options;
        match self {
            Opt::Editor if o.editor.is_empty() => "(default)".into(),
            Opt::Editor => o.editor.clone(),
            Opt::Opener if o.opener.is_empty() => "(default)".into(),
            Opt::Opener => o.opener.clone(),
            Opt::DateFormat => o.date_format.as_str().into(),
            Opt::NumberFormat => o.number_format.as_str().into(),
            Opt::ShowHidden => yes_no(o.show_hidden),
            Opt::SyntaxColors => yes_no(o.syntax_highlight),
            Opt::FileDisplay => match o.file_display.as_str() {
                "names" => "Names in columns".into(),
                "names_size" => "Names and sizes in columns".into(),
                "details" => "Details".into(),
                _ => "Long names".into(),
            },
            Opt::Sort => o.sort.as_str().into(),
            Opt::SortDescending => if o.sort_descending {
                "Descending"
            } else {
                "Ascending"
            }
            .into(),
            Opt::EnterViews => if o.enter_views {
                "View file"
            } else {
                "Back to tree"
            }
            .into(),
            Opt::ConfirmQuit => yes_no(o.confirm_quit),
            Opt::PruneConfirmation => match o.prune_confirmation.as_str() {
                "prune" => "PRUNE".into(),
                "name" => "directory name".into(),
                _ => "Y".into(),
            },
            Opt::HistorySkipAt => yes_no(o.history_skip_at),
            Opt::TerminalHistory => yes_no(o.terminal_history),
            Opt::CompareIgnore2s => yes_no(o.compare_ignore_2s),
            Opt::CompareIgnore1h => yes_no(o.compare_ignore_1h),
            Opt::AutoRefresh => yes_no(o.auto_refresh),
            Opt::Git => yes_no(o.git),
            Opt::CopyPaths => match o.copy_paths {
                CopyPaths::Full => "Full (root of the disk)",
                CopyPaths::Tree => "Tree (root of xt's tree)",
                CopyPaths::Current => "Current (current directory, with its name)",
                CopyPaths::Relative => "Relative (below the current directory)",
            }
            .into(),
            Opt::TerminalTheme => match o.terminal_theme {
                TerminalTheme::Native => "Native (the terminal's own)",
                TerminalTheme::Xt => "xt (background and text of the theme)",
                TerminalTheme::Solarized => "Solarized",
                TerminalTheme::Gruvbox => "Gruvbox",
                TerminalTheme::Nord => "Nord",
                TerminalTheme::Dracula => "Dracula",
                TerminalTheme::OneDark => "One Dark",
            }
            .into(),
        }
    }

    /// Moves to the next value. The Editor is text: returns `false` and the
    /// screen opens a field for typing.
    fn cycle(self, c: &mut Config) -> bool {
        let o = &mut c.options;
        match self {
            Opt::Editor | Opt::Opener => return false,
            Opt::DateFormat => o.date_format = o.date_format.next(),
            Opt::NumberFormat => o.number_format = o.number_format.next(),
            Opt::ShowHidden => o.show_hidden = !o.show_hidden,
            Opt::SyntaxColors => o.syntax_highlight = !o.syntax_highlight,
            Opt::FileDisplay => o.file_display = o.file_display.next(),
            Opt::Sort => o.sort = o.sort.next(),
            Opt::SortDescending => o.sort_descending = !o.sort_descending,
            Opt::EnterViews => o.enter_views = !o.enter_views,
            Opt::ConfirmQuit => o.confirm_quit = !o.confirm_quit,
            Opt::PruneConfirmation => o.prune_confirmation = o.prune_confirmation.next(),
            Opt::HistorySkipAt => o.history_skip_at = !o.history_skip_at,
            Opt::TerminalHistory => o.terminal_history = !o.terminal_history,
            Opt::CompareIgnore2s => o.compare_ignore_2s = !o.compare_ignore_2s,
            Opt::CompareIgnore1h => o.compare_ignore_1h = !o.compare_ignore_1h,
            Opt::AutoRefresh => o.auto_refresh = !o.auto_refresh,
            Opt::Git => o.git = !o.git,
            Opt::CopyPaths => o.copy_paths = o.copy_paths.next(),
            Opt::TerminalTheme => o.terminal_theme = o.terminal_theme.next(),
        }
        true
    }
}

/// What the screen asks of the App after a key.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Stay,
    Close,
    Save(Box<Config>),
}

/// Theme list for F4.
pub struct ThemePicker {
    pub index: usize,
}

pub struct Settings {
    /// Copy being edited.
    pub config: Config,
    original: Config,
    pub page: usize,
    pub selected: usize,
    /// Editing a text option (editor, Open program).
    pub editing: Option<(Opt, LineInput)>,
    pub picker: Option<ThemePicker>,
    /// Esc with changes: the next Esc discards them.
    pub confirm_discard: bool,
    /// Footer message (e.g. error while saving).
    pub message: Option<String>,
    /// The file had errors when xt opened it: saving would erase what the user
    /// wrote, so F3 stays blocked until they fix it.
    pub load_error: Option<String>,
}

impl Settings {
    pub fn new(config: Config, load_error: Option<String>) -> Self {
        Settings {
            original: config.clone(),
            config,
            page: 0,
            selected: 0,
            editing: None,
            picker: None,
            confirm_discard: false,
            message: load_error
                .as_ref()
                .map(|_| "Config file has errors: fix it before saving (F3 is disabled)".into()),
            load_error,
        }
    }

    pub fn dirty(&self) -> bool {
        self.config != self.original
    }

    pub fn items(&self) -> &'static [(char, &'static str, Opt)] {
        PAGES[self.page].1
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if let Some((opt, input)) = &mut self.editing {
            match input.handle_key(key) {
                LineEvent::Submit => {
                    let text = input.text.trim().to_string();
                    match opt {
                        Opt::Opener => self.config.options.opener = text,
                        _ => self.config.options.editor = text,
                    }
                    self.editing = None;
                }
                LineEvent::Cancel => self.editing = None,
                LineEvent::Editing => {}
            }
            return Outcome::Stay;
        }
        if self.picker.is_some() {
            self.picker_key(key);
            return Outcome::Stay;
        }

        let discard_pending = std::mem::take(&mut self.confirm_discard);
        self.message = None;
        match key.code {
            KeyCode::Esc if self.dirty() && !discard_pending => {
                self.confirm_discard = true;
                self.message = Some("Unsaved changes: Esc again to discard, F3 to save".into());
            }
            KeyCode::Esc => {
                preview_theme(&self.original);
                return Outcome::Close;
            }
            KeyCode::F(3) => match &self.load_error {
                Some(e) => self.message = Some(format!("Cannot save: {e}")),
                None => return Outcome::Save(Box::new(self.config.clone())),
            },
            KeyCode::F(4) => {
                let index = THEMES
                    .iter()
                    .position(|(n, _)| *n == self.config.theme)
                    .unwrap_or(0);
                self.picker = Some(ThemePicker { index });
            }
            KeyCode::PageDown => self.go_to_page((self.page + 1).min(PAGES.len() - 1)),
            KeyCode::PageUp => self.go_to_page(self.page.saturating_sub(1)),
            KeyCode::Down => self.selected = (self.selected + 1).min(self.items().len() - 1),
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Enter | KeyCode::Char(' ') => self.change(self.selected),
            KeyCode::Char(c @ '1'..='9') => {
                let page = c as usize - '1' as usize;
                if page < PAGES.len() {
                    self.go_to_page(page);
                }
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let letter = c.to_ascii_uppercase();
                if let Some(i) = self.items().iter().position(|(l, ..)| *l == letter) {
                    self.selected = i;
                    self.change(i);
                }
            }
            _ => {}
        }
        Outcome::Stay
    }

    fn go_to_page(&mut self, page: usize) {
        self.page = page;
        self.selected = 0;
    }

    fn change(&mut self, i: usize) {
        let opt = self.items()[i].2;
        if !opt.cycle(&mut self.config) {
            let current = match opt {
                Opt::Opener => &self.config.options.opener,
                _ => &self.config.options.editor,
            };
            self.editing = Some((opt, LineInput::new(current)));
        }
    }

    fn picker_key(&mut self, key: KeyEvent) {
        let Some(picker) = &mut self.picker else {
            return;
        };
        match key.code {
            KeyCode::Up => picker.index = picker.index.saturating_sub(1),
            KeyCode::Down => picker.index = (picker.index + 1).min(THEMES.len() - 1),
            KeyCode::Enter => {
                self.config.theme = THEMES[picker.index].0.to_string();
                self.picker = None;
                return;
            }
            KeyCode::Esc => {
                self.picker = None;
                preview_theme(&self.config);
                return;
            }
            _ => return,
        }
        // Preview: the whole screen already shows the theme under the cursor.
        let preview = Config {
            theme: THEMES[picker.index].0.to_string(),
            ..self.config.clone()
        };
        preview_theme(&preview);
    }
}

/// Shows `config.theme` (with the colors overridden in `[colors]`).
fn preview_theme(config: &Config) {
    if let Ok(theme) = Theme::from_config(config) {
        config::set_theme(theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(s: &mut Settings, code: KeyCode) -> Outcome {
        s.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn letters_and_enter_change_options() {
        let mut s = Settings::new(Config::default(), None);
        press(&mut s, KeyCode::Char('c'));
        assert_eq!(s.config.options.date_format.as_str(), "dd/mm/yyyy");
        assert_eq!(s.selected, 2);
        press(&mut s, KeyCode::Down);
        press(&mut s, KeyCode::Enter);
        assert_eq!(s.config.options.number_format.as_str(), "1.234");

        press(&mut s, KeyCode::PageDown);
        assert_eq!((s.page, s.selected), (1, 0));
        press(&mut s, KeyCode::Char('a'));
        assert!(!s.config.options.show_hidden);
        press(&mut s, KeyCode::Char('4'));
        assert_eq!(s.page, 3);
        press(&mut s, KeyCode::Char('9'));
        assert_eq!(s.page, 3, "no such page");
        assert!(s.dirty());
    }

    #[test]
    fn editor_is_typed() {
        let mut s = Settings::new(Config::default(), None);
        press(&mut s, KeyCode::Char('a'));
        assert!(s.editing.is_some());
        for c in "nvim".chars() {
            press(&mut s, KeyCode::Char(c));
        }
        press(&mut s, KeyCode::Enter);
        assert_eq!(s.config.options.editor, "nvim");
        assert_eq!(Opt::Editor.value(&s.config), "nvim");

        press(&mut s, KeyCode::Char('b'));
        for c in "gio open".chars() {
            press(&mut s, KeyCode::Char(c));
        }
        press(&mut s, KeyCode::Enter);
        assert_eq!(s.config.options.opener, "gio open");
        assert_eq!(s.config.options.editor, "nvim");
    }

    #[test]
    fn f3_saves_and_esc_asks_before_discarding() {
        let mut s = Settings::new(Config::default(), None);
        assert_eq!(
            press(&mut s, KeyCode::Esc),
            Outcome::Close,
            "nothing changed"
        );

        let mut s = Settings::new(Config::default(), None);
        press(&mut s, KeyCode::PageDown);
        press(&mut s, KeyCode::Char('a'));
        assert_eq!(press(&mut s, KeyCode::Esc), Outcome::Stay);
        assert!(s.message.as_deref().unwrap().starts_with("Unsaved changes"));
        match press(&mut s, KeyCode::F(3)) {
            Outcome::Save(c) => assert!(!c.options.show_hidden),
            other => panic!("{other:?}"),
        }
        press(&mut s, KeyCode::Esc);
        assert_eq!(press(&mut s, KeyCode::Esc), Outcome::Close);
    }

    #[test]
    fn f3_is_blocked_when_file_had_errors() {
        let mut s = Settings::new(Config::default(), Some("bad".into()));
        assert_eq!(press(&mut s, KeyCode::F(3)), Outcome::Stay);
        assert_eq!(s.message.as_deref(), Some("Cannot save: bad"));
    }

    #[test]
    fn f4_picks_a_theme() {
        let mut s = Settings::new(Config::default(), None);
        press(&mut s, KeyCode::F(4));
        assert_eq!(s.picker.as_ref().unwrap().index, 0);
        press(&mut s, KeyCode::Down);
        press(&mut s, KeyCode::Down);
        press(&mut s, KeyCode::Esc);
        assert_eq!(s.config.theme, "classic", "Esc keeps the old theme");

        press(&mut s, KeyCode::F(4));
        press(&mut s, KeyCode::Down);
        press(&mut s, KeyCode::Enter);
        assert_eq!(s.config.theme, "dark");
        assert!(s.picker.is_none());
    }

    #[test]
    fn every_option_shows_a_value_and_letters_are_unique() {
        let c = Config::default();
        for (_, items) in PAGES {
            let mut letters: Vec<char> = items.iter().map(|(l, ..)| *l).collect();
            letters.dedup();
            assert_eq!(letters.len(), items.len());
            for (_, _, opt) in *items {
                assert!(!opt.value(&c).is_empty());
            }
        }
    }
}
