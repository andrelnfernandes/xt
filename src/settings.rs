//! Tela de configuração (F12 ou Alt+F10), no estilo da do ZTreeWin:
//! páginas de opções, a letra da opção muda o valor, F3 grava, Esc
//! desiste e F4 escolhe o tema (com prévia na hora).

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::{self, Choice, Config, THEMES, Theme};
use crate::input::{LineEvent, LineInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opt {
    Editor,
    DateFormat,
    NumberFormat,
    ShowHidden,
    Sort,
    SortDescending,
    EnterViews,
    ConfirmQuit,
    PruneConfirmation,
    HistorySkipAt,
    CompareIgnore2s,
    CompareIgnore1h,
    AutoRefresh,
}

type Page = (&'static str, &'static [(char, &'static str, Opt)]);

pub const PAGES: &[Page] = &[
    (
        "Programs and formats",
        &[
            ('A', "Editor (empty: $VISUAL, $EDITOR, vi)", Opt::Editor),
            ('B', "Date format", Opt::DateFormat),
            ('C', "Number format", Opt::NumberFormat),
        ],
    ),
    (
        "Display and sorting",
        &[
            ('A', "Show hidden files and directories", Opt::ShowHidden),
            ('B', "Initial sort", Opt::Sort),
            ('C', "Initial sort order", Opt::SortDescending),
            ('D', "Enter in the file window", Opt::EnterViews),
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
        ],
    ),
    (
        "Compare and refresh",
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
            Opt::DateFormat => o.date_format.as_str().into(),
            Opt::NumberFormat => o.number_format.as_str().into(),
            Opt::ShowHidden => yes_no(o.show_hidden),
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
            Opt::CompareIgnore2s => yes_no(o.compare_ignore_2s),
            Opt::CompareIgnore1h => yes_no(o.compare_ignore_1h),
            Opt::AutoRefresh => yes_no(o.auto_refresh),
        }
    }

    /// Passa para o próximo valor. O Editor é texto: devolve `false` e a
    /// tela abre um campo para digitar.
    fn cycle(self, c: &mut Config) -> bool {
        let o = &mut c.options;
        match self {
            Opt::Editor => return false,
            Opt::DateFormat => o.date_format = o.date_format.next(),
            Opt::NumberFormat => o.number_format = o.number_format.next(),
            Opt::ShowHidden => o.show_hidden = !o.show_hidden,
            Opt::Sort => o.sort = o.sort.next(),
            Opt::SortDescending => o.sort_descending = !o.sort_descending,
            Opt::EnterViews => o.enter_views = !o.enter_views,
            Opt::ConfirmQuit => o.confirm_quit = !o.confirm_quit,
            Opt::PruneConfirmation => o.prune_confirmation = o.prune_confirmation.next(),
            Opt::HistorySkipAt => o.history_skip_at = !o.history_skip_at,
            Opt::CompareIgnore2s => o.compare_ignore_2s = !o.compare_ignore_2s,
            Opt::CompareIgnore1h => o.compare_ignore_1h = !o.compare_ignore_1h,
            Opt::AutoRefresh => o.auto_refresh = !o.auto_refresh,
        }
        true
    }
}

/// O que a tela pede ao App depois de uma tecla.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Stay,
    Close,
    Save(Box<Config>),
}

/// Lista de temas do F4.
pub struct ThemePicker {
    pub index: usize,
}

pub struct Settings {
    /// Cópia sendo editada.
    pub config: Config,
    original: Config,
    pub page: usize,
    pub selected: usize,
    /// Editando o texto do editor.
    pub editing: Option<LineInput>,
    pub picker: Option<ThemePicker>,
    /// Esc com mudanças: o próximo Esc descarta.
    pub confirm_discard: bool,
    /// Mensagem no rodapé (ex.: erro ao gravar).
    pub message: Option<String>,
    /// O arquivo tinha erros ao abrir o xt: gravar apagaria o que o usuário
    /// escreveu, então F3 fica bloqueado até ele corrigir.
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
        if let Some(input) = &mut self.editing {
            match input.handle_key(key) {
                LineEvent::Submit => {
                    self.config.options.editor = input.text.trim().to_string();
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
            self.editing = Some(LineInput::new(&self.config.options.editor));
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
        // Prévia: a tela inteira já aparece com o tema sob o cursor.
        let preview = Config {
            theme: THEMES[picker.index].0.to_string(),
            ..self.config.clone()
        };
        preview_theme(&preview);
    }
}

/// Mostra `config.theme` (com as cores trocadas em `[colors]`).
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
        press(&mut s, KeyCode::Char('b'));
        assert_eq!(s.config.options.date_format.as_str(), "dd/mm/yyyy");
        assert_eq!(s.selected, 1);
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
