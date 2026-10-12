//! Comandos e o campo de texto dos prompts. O mapa de teclas está em
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
    /// Compara com o diretório da outra janela e marca arquivos.
    Compare,
    /// Abre um ZIP/TAR como se fosse um diretório (Alt+F5).
    OpenArchive,
    /// Abre o arquivo (ou os marcados) no programa padrão do sistema.
    OpenWith {
        tagged: bool,
    },
    /// Alt+C na árvore: Compare do ramo inteiro.
    CompareBranch,
    /// Backspace: diretório pai.
    Parent,
    /// Abre o arquivo no editor.
    Edit,
    /// Executa um comando do shell.
    Execute,
    /// F1: lista das teclas.
    Help,
    /// Alt+S: escolhe a ordenação dos arquivos.
    Sort,
    /// Alt+P: apaga o diretório atual com tudo dentro.
    Prune,
    /// F12 / Alt+F10: tela de configuração.
    Settings,
    /// Ctrl+S: procura um texto nos marcados e desmarca os que não o têm.
    SearchTagged,
    /// Ctrl+V: visualiza os marcados, um depois do outro.
    ViewTagged,
    /// `\`: navega digitando o caminho.
    Treespec,
    /// Alt+K: tamanho ou nº de arquivos por diretório na árvore.
    Kbyte,
    /// Alt+Q: sai deixando o shell no diretório atual.
    QuitChangeDir,
    /// Alt+F: modo de exibição da janela de arquivos.
    FileDisplay,
    /// Ctrl+F7 / Ctrl+F8 / Ctrl+F9 na árvore: marca, desmarca ou inverte
    /// os arquivos logados do ramo.
    TagBranch,
    UntagBranch,
    InvertBranch,
    /// F7: a lista de arquivos com o conteúdo do selecionado ao lado.
    Autoview,
    /// N: muda a data de modificação (do arquivo, dos marcados ou do
    /// diretório, na árvore).
    NewDate {
        tagged: bool,
    },
    /// ← e → na janela de arquivos: arquivo anterior/próximo em colunas,
    /// página acima/abaixo numa coluna só.
    Left,
    Right,
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
    /// Entradas anteriores deste tipo de prompt, da mais antiga à mais nova.
    history: Vec<String>,
    /// Posição no histórico enquanto se navega com ↑/↓...
    history_pos: Option<usize>,
    /// ...e o texto que estava sendo digitado, para voltar a ele.
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

    fn set_text(&mut self, text: String) {
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
