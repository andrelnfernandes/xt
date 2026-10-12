//! Traduz teclas em comandos. Mais tarde o mapa virá do arquivo de configuração.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::Focus;

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
    /// Loga e expande o ramo inteiro.
    ExpandAll,
    /// Alterna entre a árvore e a janela de arquivos.
    SwitchWindow,
    /// Tab: troca de janela com a tela dividida.
    SwitchPane,
    /// F8: divide a tela em duas janelas (ou volta a uma).
    Split,
    /// Abre outro diretório na janela.
    Log,
    Back,
    Filespec,
    Showall,
    Branch,
    View,
    /// Relê o diretório atual do disco.
    Relog,
    Tag,
    Untag,
    TagAll,
    UntagAll,
    Invert,
    // `tagged`: age sobre os arquivos marcados em vez do arquivo atual.
    // `paths`: recria os subdiretórios no destino (Alt+C / Alt+M).
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
    },
    Attributes {
        tagged: bool,
    },
    Rename,
    MakeDir,
    Quit,
}

pub fn map_key(key: KeyEvent, focus: Focus) -> Option<Command> {
    use Command::*;

    // Ctrl+letra (ou Alt+letra) = versão "arquivos marcados" do comando.
    // No Copy/Move, Ctrl copia tudo direto no destino e Alt recria os
    // caminhos; F6 no prompt troca entre os dois.
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        let KeyCode::Char(c) = key.code else {
            return None;
        };
        let paths = key.modifiers.contains(KeyModifiers::ALT);
        return match c.to_ascii_lowercase() {
            't' => Some(TagAll),
            'u' => Some(UntagAll),
            'c' => Some(Copy {
                tagged: true,
                paths,
            }),
            'm' => Some(Move {
                tagged: true,
                paths,
            }),
            'd' => Some(Delete { tagged: true }),
            'a' => Some(Attributes { tagged: true }),
            _ => None,
        };
    }

    let key = match key.code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        other => other,
    };

    let common = match key {
        KeyCode::Up => Some(Up),
        KeyCode::Down => Some(Down),
        KeyCode::PageUp => Some(PageUp),
        KeyCode::PageDown => Some(PageDown),
        KeyCode::Home => Some(Home),
        KeyCode::End => Some(End),
        KeyCode::Enter => Some(SwitchWindow),
        KeyCode::Tab => Some(SwitchPane),
        KeyCode::F(8) => Some(Split),
        KeyCode::F(2) => Some(Relog),
        KeyCode::Char('f') => Some(Filespec),
        KeyCode::Char('s') => Some(Showall),
        KeyCode::Char('b') => Some(Branch),
        KeyCode::Char('t') => Some(Tag),
        KeyCode::Char('u') => Some(Untag),
        KeyCode::Char('i') => Some(Invert),
        KeyCode::Char('d') => Some(Delete { tagged: false }),
        KeyCode::Char('r') => Some(Rename),
        KeyCode::Char('q') | KeyCode::F(10) => Some(Quit),
        _ => None,
    };
    if common.is_some() {
        return common;
    }

    match focus {
        Focus::Tree => match key {
            KeyCode::Right | KeyCode::Char('+') => Some(Expand),
            KeyCode::Left | KeyCode::Char('-') => Some(Collapse),
            KeyCode::Char('*') => Some(ExpandAll),
            KeyCode::Char('m') => Some(MakeDir),
            KeyCode::Char('l') => Some(Log),
            _ => None,
        },
        Focus::Files => match key {
            KeyCode::Esc | KeyCode::Left => Some(Back),
            KeyCode::Char('v') => Some(View),
            KeyCode::Char('c') => Some(Copy {
                tagged: false,
                paths: false,
            }),
            KeyCode::Char('m') => Some(Move {
                tagged: false,
                paths: false,
            }),
            KeyCode::Char('a') => Some(Attributes { tagged: false }),
            _ => None,
        },
    }
}

/// Resultado de uma tecla num campo de texto.
#[derive(Debug, PartialEq, Eq)]
pub enum LineEvent {
    Submit,
    Cancel,
    Editing,
}

/// Campo de texto de uma linha, usado nos prompts (Filespec etc.).
pub struct LineInput {
    pub text: String,
    /// Posição do cursor, em caracteres.
    pub cursor: usize,
}

impl LineInput {
    pub fn new(text: &str) -> Self {
        LineInput {
            text: text.to_string(),
            cursor: text.chars().count(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> LineEvent {
        let len = self.text.chars().count();
        match key.code {
            KeyCode::Enter => return LineEvent::Submit,
            KeyCode::Esc => return LineEvent::Cancel,
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

    fn map(code: KeyCode, mods: KeyModifiers, focus: Focus) -> Option<Command> {
        map_key(KeyEvent::new(code, mods), focus)
    }

    #[test]
    fn maps_letters_by_window() {
        let none = KeyModifiers::NONE;
        assert_eq!(
            map(KeyCode::Char('m'), none, Focus::Tree),
            Some(Command::MakeDir)
        );
        assert_eq!(
            map(KeyCode::Char('M'), KeyModifiers::SHIFT, Focus::Files),
            Some(Command::Move {
                tagged: false,
                paths: false
            })
        );
        assert_eq!(map(KeyCode::Char('c'), none, Focus::Tree), None);
    }

    #[test]
    fn ctrl_and_alt_mean_tagged() {
        for (mods, paths) in [(KeyModifiers::CONTROL, false), (KeyModifiers::ALT, true)] {
            assert_eq!(
                map(KeyCode::Char('c'), mods, Focus::Files),
                Some(Command::Copy {
                    tagged: true,
                    paths
                })
            );
            assert_eq!(
                map(KeyCode::Char('m'), mods, Focus::Tree),
                Some(Command::Move {
                    tagged: true,
                    paths
                })
            );
            assert_eq!(
                map(KeyCode::Char('t'), mods, Focus::Tree),
                Some(Command::TagAll)
            );
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
