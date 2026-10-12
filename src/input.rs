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
    ExpandAll,
    /// Alterna entre a árvore e a janela de arquivos.
    SwitchWindow,
    Back,
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
            _ => None,
        },
    }
}
