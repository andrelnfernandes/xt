//! Visualizador de arquivos (tecla `V`), em modo texto ou hexadecimal.

use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent};

/// Arquivos maiores são exibidos só até este limite.
pub const MAX_BYTES: u64 = 16 * 1024 * 1024;
pub const HEX_WIDTH: usize = 16;
const TAB_WIDTH: usize = 8;
const PAN_STEP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Text,
    Hex,
}

pub struct Viewer {
    pub path: PathBuf,
    pub size: u64,
    pub mode: ViewMode,
    data: Vec<u8>,
    lines: Vec<String>,
    /// Primeira linha visível.
    pub top: usize,
    /// Deslocamento horizontal (modo texto).
    pub left: usize,
    /// Altura útil no último desenho.
    pub page: usize,
}

impl Viewer {
    pub fn open(path: PathBuf) -> io::Result<Self> {
        let file = File::open(&path)?;
        let size = file.metadata()?.len();
        let mut data = Vec::new();
        file.take(MAX_BYTES).read_to_end(&mut data)?;
        Ok(Self::from_bytes(path, size, data))
    }

    /// Visualiza bytes já lidos (ex.: uma entrada de um arquivo compactado).
    pub fn from_bytes(path: PathBuf, size: u64, data: Vec<u8>) -> Self {
        let mode = if looks_binary(&data) {
            ViewMode::Hex
        } else {
            ViewMode::Text
        };
        Viewer {
            path,
            size,
            mode,
            lines: text_lines(&data),
            data,
            top: 0,
            left: 0,
            page: 1,
        }
    }

    pub fn truncated(&self) -> bool {
        self.size > self.data.len() as u64
    }

    pub fn line_count(&self) -> usize {
        match self.mode {
            ViewMode::Text => self.lines.len(),
            ViewMode::Hex => self.data.len().div_ceil(HEX_WIDTH),
        }
    }

    /// As linhas que cabem em `height`, já com o deslocamento aplicado.
    pub fn visible_lines(&self, height: usize) -> Vec<String> {
        let range = self.top..(self.top + height).min(self.line_count());
        match self.mode {
            ViewMode::Text => self.lines[range]
                .iter()
                .map(|l| l.chars().skip(self.left).collect())
                .collect(),
            ViewMode::Hex => range
                .map(|row| hex_row(&self.data, row * HEX_WIDTH))
                .collect(),
        }
    }

    /// Trata uma tecla. Devolve `false` quando o visualizador deve fechar.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let page = self.page.max(1);
        let max_top = self.line_count().saturating_sub(page);
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q' | 'Q' | 'v' | 'V') => {
                return false;
            }
            KeyCode::Up => self.top = self.top.saturating_sub(1),
            KeyCode::Down => self.top += 1,
            KeyCode::PageUp => self.top = self.top.saturating_sub(page),
            KeyCode::PageDown | KeyCode::Char(' ') => self.top += page,
            KeyCode::Home => self.top = 0,
            KeyCode::End => self.top = max_top,
            KeyCode::Left => self.left = self.left.saturating_sub(PAN_STEP),
            KeyCode::Right if self.mode == ViewMode::Text => self.left += PAN_STEP,
            KeyCode::Char('h' | 'H') => self.toggle_mode(),
            _ => {}
        }
        self.top = self.top.min(max_top);
        true
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ViewMode::Text => ViewMode::Hex,
            ViewMode::Hex => ViewMode::Text,
        };
        self.top = 0;
        self.left = 0;
    }
}

/// Heurística simples: há um byte nulo no começo do arquivo.
fn looks_binary(data: &[u8]) -> bool {
    data.iter().take(8192).any(|&b| b == 0)
}

/// Quebra em linhas, expande tabs e troca caracteres de controle por `·`.
fn text_lines(data: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(data);
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|line| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            let mut out = String::with_capacity(line.len());
            let mut col = 0;
            for c in line.chars() {
                if c == '\t' {
                    let n = TAB_WIDTH - col % TAB_WIDTH;
                    out.extend(std::iter::repeat_n(' ', n));
                    col += n;
                } else {
                    out.push(if c.is_control() { '·' } else { c });
                    col += 1;
                }
            }
            out
        })
        .collect();
    // O '\n' final não abre uma linha nova.
    if lines.len() > 1 && lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// `00000010  48 65 6C 6C 6F 0A 00 00  00 00 00 00 00 00 00 00  |Hello...........|`
fn hex_row(data: &[u8], offset: usize) -> String {
    let chunk = &data[offset..(offset + HEX_WIDTH).min(data.len())];
    let mut hex = String::with_capacity(HEX_WIDTH * 3 + 1);
    for i in 0..HEX_WIDTH {
        if i == HEX_WIDTH / 2 {
            hex.push(' ');
        }
        match chunk.get(i) {
            Some(b) => hex.push_str(&format!("{b:02X} ")),
            None => hex.push_str("   "),
        }
    }
    let ascii: String = chunk
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    format!("{offset:08X}  {hex} |{ascii}|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn viewer(data: &[u8]) -> Viewer {
        Viewer::from_bytes(PathBuf::from("t"), data.len() as u64, data.to_vec())
    }

    fn press(v: &mut Viewer, code: KeyCode) -> bool {
        v.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn detects_mode() {
        assert_eq!(viewer(b"hello\n").mode, ViewMode::Text);
        assert_eq!(viewer(b"\x7fELF\0\0").mode, ViewMode::Hex);
    }

    #[test]
    fn splits_text_lines() {
        assert_eq!(text_lines(b"a\r\nb\n"), ["a", "b"]);
        assert_eq!(text_lines(b"a\tb"), ["a       b"]);
        assert_eq!(text_lines(b"x\x07y"), ["x·y"]);
        assert_eq!(text_lines(b""), [""]);
    }

    #[test]
    fn formats_hex_rows() {
        let row = hex_row(b"Hello\n", 0);
        assert!(row.starts_with("00000000  48 65 6C 6C 6F 0A "));
        assert!(row.ends_with("|Hello.|"));
        let data = vec![b'a'; 20];
        assert!(hex_row(&data, 16).starts_with("00000010  61 61 61 61    "));
    }

    #[test]
    fn scrolls_within_bounds() {
        let text = (0..100).map(|i| format!("line {i}\n")).collect::<String>();
        let mut v = viewer(text.as_bytes());
        v.page = 10;
        press(&mut v, KeyCode::Up);
        assert_eq!(v.top, 0);
        press(&mut v, KeyCode::PageDown);
        assert_eq!(v.top, 10);
        press(&mut v, KeyCode::End);
        assert_eq!(v.top, 90);
        press(&mut v, KeyCode::Down);
        assert_eq!(v.top, 90);
        assert_eq!(v.visible_lines(10)[0], "line 90");
        assert!(!press(&mut v, KeyCode::Esc));
    }

    #[test]
    fn toggles_hex_and_pans() {
        let mut v = viewer(b"0123456789abcdef0123");
        press(&mut v, KeyCode::Right);
        assert_eq!(v.visible_lines(1)[0], "89abcdef0123");
        press(&mut v, KeyCode::Char('h'));
        assert_eq!(v.mode, ViewMode::Hex);
        assert_eq!(v.line_count(), 2);
        assert_eq!(v.left, 0);
    }

    #[test]
    fn opens_real_file_and_reports_truncation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("f.txt");
        std::fs::write(&path, "abc").unwrap();
        let v = Viewer::open(path).unwrap();
        assert_eq!(v.size, 3);
        assert!(!v.truncated());
        assert!(Viewer::open(tmp.path().join("missing")).is_err());
    }
}
