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
    Back,
    Filespec,
    Showall,
    Branch,
    View,
    Quit,
}

pub fn map_key(key: KeyEvent, focus: Focus) -> Option<Command> {
    use Command::*;

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Quit);
    }

    let common = match key.code {
        KeyCode::Up => Some(Up),
        KeyCode::Down => Some(Down),
        KeyCode::PageUp => Some(PageUp),
        KeyCode::PageDown => Some(PageDown),
        KeyCode::Home => Some(Home),
        KeyCode::End => Some(End),
        KeyCode::Enter | KeyCode::Tab => Some(SwitchWindow),
        KeyCode::Char('f' | 'F') => Some(Filespec),
        KeyCode::Char('s' | 'S') => Some(Showall),
        KeyCode::Char('b' | 'B') => Some(Branch),
        KeyCode::Char('q' | 'Q') | KeyCode::F(10) => Some(Quit),
        _ => None,
    };
    if common.is_some() {
        return common;
    }

    match focus {
        Focus::Tree => match key.code {
            KeyCode::Right | KeyCode::Char('+') => Some(Expand),
            KeyCode::Left | KeyCode::Char('-') => Some(Collapse),
            KeyCode::Char('*') => Some(ExpandAll),
            _ => None,
        },
        Focus::Files => match key.code {
            KeyCode::Esc | KeyCode::Left => Some(Back),
            KeyCode::Char('v' | 'V') => Some(View),
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
