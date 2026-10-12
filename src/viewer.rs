//! File viewer (`V` key; Ctrl+V for the tagged files), in text or
//! hexadecimal mode, with search.

use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui::style::Color;

use crate::highlight::Highlighter;
use crate::search::Pattern;

/// Larger files are displayed only up to this limit.
pub const MAX_BYTES: u64 = 16 * 1024 * 1024;
pub const HEX_WIDTH: usize = 16;
const TAB_WIDTH: usize = 8;
const PAN_STEP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Text,
    Hex,
}

/// Where a new search starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFrom {
    /// Start of the file (F) or end, in a backward search (B).
    Edge,
    /// Top of the page, or its end in a backward search (`/` and `\`).
    Page,
}

/// What a key asks of whoever opened the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Close,
    /// Ctrl+V: next/previous/first/last tagged file.
    Next,
    Prev,
    First,
    Last,
    /// Ctrl+V: untags the current file.
    Untag,
    /// Asks for the term and searches.
    Search {
        forward: bool,
        from: SearchFrom,
    },
    /// Continues the last search (`None`: same direction, with Space).
    Continue {
        forward: Option<bool>,
    },
}

/// Match found: in text mode, line and columns (in characters); in
/// hex, `line` is the 16-byte row and `start..end` the bytes in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub mode: ViewMode,
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

pub struct Viewer {
    pub path: PathBuf,
    pub size: u64,
    pub mode: ViewMode,
    data: Vec<u8>,
    lines: Vec<String>,
    /// First visible line.
    pub top: usize,
    /// Horizontal offset (text mode).
    pub left: usize,
    /// Usable height and width as of the last draw.
    pub page: usize,
    pub width: usize,
    /// Last match found, highlighted on screen.
    pub hit: Option<Hit>,
    /// Direction of the last search.
    pub forward: bool,
    /// Footer notice (e.g. "Not found").
    pub note: Option<String>,
    /// Syntax highlighting, when the language is known.
    colors: Option<Highlighter>,
    /// Colors shown (C toggles).
    pub colors_on: bool,
}

/// A piece of a displayed line: text, its color and whether it is part of
/// the search match.
pub type Segment = (String, Option<Color>, bool);

impl Viewer {
    /// Reads only the start of the file (Autoview uses a smaller limit, to
    /// move quickly through large files).
    pub fn open_limited(path: PathBuf, limit: u64) -> io::Result<Self> {
        let file = File::open(&path)?;
        let size = file.metadata()?.len();
        let mut data = Vec::new();
        file.take(limit).read_to_end(&mut data)?;
        Ok(Self::from_bytes(path, size, data))
    }

    /// Autoview scrolling (Shift+arrows): only moves the content.
    pub fn scroll(&mut self, code: KeyCode) {
        let page = self.page.max(1);
        let max_top = self.line_count().saturating_sub(page);
        self.top = match code {
            KeyCode::Up => self.top.saturating_sub(1),
            KeyCode::Down => self.top + 1,
            KeyCode::PageUp => self.top.saturating_sub(page),
            KeyCode::PageDown => self.top + page,
            KeyCode::Home => 0,
            KeyCode::End => max_top,
            _ => self.top,
        }
        .min(max_top);
    }

    /// Views bytes already read (e.g. an entry of an archive).
    pub fn from_bytes(path: PathBuf, size: u64, data: Vec<u8>) -> Self {
        let mode = if looks_binary(&data) {
            ViewMode::Hex
        } else {
            ViewMode::Text
        };
        let lines = text_lines(&data);
        let colors = (mode == ViewMode::Text)
            .then(|| Highlighter::for_file(&path, lines.first().map_or("", String::as_str)))
            .flatten();
        Viewer {
            colors_on: colors.is_some(),
            colors,
            path,
            size,
            mode,
            lines,
            data,
            top: 0,
            left: 0,
            page: 1,
            width: 80,
            hit: None,
            forward: true,
            note: None,
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

    /// The lines that fit in `height`, with the offset already applied, and
    /// the match to highlight in each one (columns in the displayed line).
    #[cfg(test)]
    pub fn visible_lines(&self, height: usize) -> Vec<String> {
        self.visible(height).into_iter().map(|(l, _)| l).collect()
    }

    pub fn visible(&self, height: usize) -> Vec<(String, Option<(usize, usize)>)> {
        let range = self.top..(self.top + height).min(self.line_count());
        let hit = self.hit.filter(|h| h.mode == self.mode);
        match self.mode {
            ViewMode::Text => range
                .map(|i| {
                    let line: String = self.lines[i].chars().skip(self.left).collect();
                    let mark = hit.filter(|h| h.line == i).and_then(|h| {
                        let s = h.start.saturating_sub(self.left);
                        let e = h.end.saturating_sub(self.left);
                        (e > s).then_some((s, e))
                    });
                    (line, mark)
                })
                .collect(),
            ViewMode::Hex => range
                .map(|row| {
                    let line = hex_row(&self.data, row * HEX_WIDTH);
                    let mark = hit
                        .filter(|h| h.line == row)
                        .map(|_| (0, line.chars().count()));
                    (line, mark)
                })
                .collect(),
        }
    }

    /// Language being highlighted, if colors are on.
    pub fn language(&self) -> Option<&str> {
        self.colors
            .as_ref()
            .filter(|_| self.colors_on && self.mode == ViewMode::Text)
            .map(|h| h.language.as_str())
    }

    /// Like `visible`, split into colored pieces (syntax colors and the
    /// search match).
    pub fn styled(&mut self, height: usize) -> Vec<Vec<Segment>> {
        let rows = self.visible(height);
        let use_colors = self.colors_on && self.mode == ViewMode::Text;
        let (top, left) = (self.top, self.left);
        let (colors, lines) = (&mut self.colors, &self.lines);
        rows.into_iter()
            .enumerate()
            .map(|(k, (line, mark))| {
                // Color of each character of the whole line.
                let per_char: Vec<Color> = colors
                    .as_mut()
                    .filter(|_| use_colors)
                    .and_then(|h| h.line(top + k, lines))
                    .map(|segs| {
                        segs.iter()
                            .flat_map(|(c, s)| std::iter::repeat_n(*c, s.chars().count()))
                            .collect()
                    })
                    .unwrap_or_default();
                let mut out: Vec<Segment> = Vec::new();
                for (j, ch) in line.chars().enumerate() {
                    let color = per_char.get(left + j).copied();
                    let marked = mark.is_some_and(|(s, e)| j >= s && j < e);
                    match out.last_mut() {
                        Some((text, c, m)) if *c == color && *m == marked => text.push(ch),
                        _ => out.push((ch.to_string(), color, marked)),
                    }
                }
                out
            })
            .collect()
    }

    /// Handles a key; what does not belong to the viewer itself (close, switch
    /// file, search) comes back as an `Action`.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        let page = self.page.max(1);
        let max_top = self.line_count().saturating_sub(page);
        self.note = None;
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let action = match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q' | 'Q' | 'v' | 'V') => Action::Close,
            KeyCode::Home if alt => Action::First,
            KeyCode::End if alt => Action::Last,
            KeyCode::Up => {
                self.top = self.top.saturating_sub(1);
                Action::None
            }
            KeyCode::Down => {
                self.top += 1;
                Action::None
            }
            KeyCode::PageUp => {
                self.top = self.top.saturating_sub(page);
                Action::None
            }
            KeyCode::PageDown => {
                self.top += page;
                Action::None
            }
            KeyCode::Home => {
                self.top = 0;
                Action::None
            }
            KeyCode::End => {
                self.top = max_top;
                Action::None
            }
            KeyCode::Left => {
                self.left = self.left.saturating_sub(PAN_STEP);
                Action::None
            }
            KeyCode::Right if self.mode == ViewMode::Text => {
                self.left += PAN_STEP;
                Action::None
            }
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'h' => {
                    self.toggle_mode();
                    Action::None
                }
                'c' => {
                    self.note = Some(match &self.colors {
                        None => "No syntax colors for this file".into(),
                        Some(h) => {
                            self.colors_on = !self.colors_on;
                            let state = if self.colors_on { "on" } else { "off" };
                            format!("{} colors {state}", h.language)
                        }
                    });
                    Action::None
                }
                'f' => Action::Search {
                    forward: true,
                    from: SearchFrom::Edge,
                },
                'b' => Action::Search {
                    forward: false,
                    from: SearchFrom::Edge,
                },
                '/' | 's' => Action::Search {
                    forward: true,
                    from: SearchFrom::Page,
                },
                '\\' => Action::Search {
                    forward: false,
                    from: SearchFrom::Page,
                },
                '+' => Action::Continue {
                    forward: Some(true),
                },
                '-' => Action::Continue {
                    forward: Some(false),
                },
                ' ' => Action::Continue { forward: None },
                'n' => Action::Next,
                'p' => Action::Prev,
                'u' => Action::Untag,
                _ => Action::None,
            },
            KeyCode::F(9) => Action::Search {
                forward: true,
                from: SearchFrom::Page,
            },
            _ => Action::None,
        };
        self.top = self.top.min(max_top);
        action
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ViewMode::Text => ViewMode::Hex,
            ViewMode::Hex => ViewMode::Text,
        };
        self.top = 0;
        self.left = 0;
    }

    /// New search, from the start/end of the file or the current page.
    pub fn search(&mut self, pattern: &Pattern, forward: bool, from: SearchFrom) -> bool {
        self.forward = forward;
        let pos = match (from, forward) {
            (SearchFrom::Edge, true) => (0, 0),
            (SearchFrom::Edge, false) => (usize::MAX, usize::MAX),
            (SearchFrom::Page, true) => (self.top, 0),
            (SearchFrom::Page, false) => ((self.top + self.page).saturating_sub(1), usize::MAX),
        };
        self.find_from(pattern, forward, pos)
    }

    /// Next match after the last one found (or the page).
    pub fn search_next(&mut self, pattern: &Pattern, forward: bool) -> bool {
        self.forward = forward;
        let pos = match self.hit.filter(|h| h.mode == self.mode) {
            Some(h) if forward => (h.line, h.start + 1),
            Some(h) => (h.line, h.start),
            None => return self.search(pattern, forward, SearchFrom::Page),
        };
        self.find_from(pattern, forward, pos)
    }

    /// Searches from (line, column) — in hex, the column is ignored and
    /// the position is in bytes.
    fn find_from(&mut self, pattern: &Pattern, forward: bool, (line, col): (usize, usize)) -> bool {
        let hit = match self.mode {
            ViewMode::Text => self.find_text(pattern, forward, line, col),
            ViewMode::Hex => {
                let byte = match self.hit.filter(|h| h.mode == ViewMode::Hex) {
                    Some(h) if line == h.line && col == h.start + 1 => h.start + 1,
                    Some(h) if line == h.line && col == h.start => h.start,
                    _ if line == usize::MAX => self.data.len(),
                    _ => line.saturating_mul(HEX_WIDTH),
                };
                let found = if forward {
                    pattern.find_in_bytes(&self.data, byte)
                } else {
                    pattern.rfind_in_bytes(&self.data, byte)
                };
                found.map(|(s, e)| Hit {
                    mode: ViewMode::Hex,
                    line: s / HEX_WIDTH,
                    start: s,
                    end: e,
                })
            }
        };
        match hit {
            Some(h) => {
                self.show(h);
                true
            }
            None => false,
        }
    }

    fn find_text(&self, pattern: &Pattern, forward: bool, line: usize, col: usize) -> Option<Hit> {
        let n = self.lines.len();
        if n == 0 {
            return None;
        }
        let at = |i: usize, (s, e): (usize, usize)| Hit {
            mode: ViewMode::Text,
            line: i,
            start: s,
            end: e,
        };
        if forward {
            if line >= n {
                return None;
            }
            if let Some(r) = pattern.find_in_line(&self.lines[line], col) {
                return Some(at(line, r));
            }
            (line + 1..n).find_map(|i| pattern.find_in_line(&self.lines[i], 0).map(|r| at(i, r)))
        } else {
            let line = line.min(n - 1);
            if let Some(r) = pattern.rfind_in_line(&self.lines[line], col) {
                return Some(at(line, r));
            }
            (0..line).rev().find_map(|i| {
                pattern
                    .rfind_in_line(&self.lines[i], usize::MAX)
                    .map(|r| at(i, r))
            })
        }
    }

    /// Scrolls to the match, leaving a little context above.
    fn show(&mut self, hit: Hit) {
        self.hit = Some(hit);
        let page = self.page.max(1);
        if hit.line < self.top || hit.line >= self.top + page {
            self.top = hit.line.saturating_sub(page / 3);
        }
        self.top = self.top.min(self.line_count().saturating_sub(page));
        if hit.mode == ViewMode::Text {
            let width = self.width.max(10);
            if hit.start < self.left || hit.end > self.left + width {
                self.left = hit.start.saturating_sub(width / 4);
            }
        }
    }
}

/// Simple heuristic: there is a null byte near the start of the file.
fn looks_binary(data: &[u8]) -> bool {
    data.iter().take(8192).any(|&b| b == 0)
}

/// Splits into lines, expands tabs and replaces control characters with `·`.
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
    // The final '\n' does not open a new line.
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
    use crate::search::SearchSpec;

    fn viewer(data: &[u8]) -> Viewer {
        Viewer::from_bytes(PathBuf::from("t"), data.len() as u64, data.to_vec())
    }

    fn press(v: &mut Viewer, code: KeyCode) -> Action {
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
        assert_eq!(press(&mut v, KeyCode::Esc), Action::Close);
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
        let v = Viewer::open_limited(path, MAX_BYTES).unwrap();
        assert_eq!(v.size, 3);
        assert!(!v.truncated());
        assert!(Viewer::open_limited(tmp.path().join("missing"), MAX_BYTES).is_err());
    }

    fn pat(t: &str) -> Pattern {
        Pattern::new(&SearchSpec::new(t)).unwrap()
    }

    #[test]
    fn search_keys_map_to_actions() {
        let mut v = viewer(b"x");
        let act = |v: &mut Viewer, c| press(v, KeyCode::Char(c));
        assert!(matches!(
            act(&mut v, 'f'),
            Action::Search {
                forward: true,
                from: SearchFrom::Edge
            }
        ));
        assert!(matches!(
            act(&mut v, 'b'),
            Action::Search {
                forward: false,
                from: SearchFrom::Edge
            }
        ));
        assert!(matches!(
            act(&mut v, '/'),
            Action::Search {
                forward: true,
                from: SearchFrom::Page
            }
        ));
        assert!(matches!(
            act(&mut v, '\\'),
            Action::Search {
                forward: false,
                from: SearchFrom::Page
            }
        ));
        assert_eq!(act(&mut v, ' '), Action::Continue { forward: None });
        assert_eq!(
            act(&mut v, '-'),
            Action::Continue {
                forward: Some(false)
            }
        );
        assert_eq!(act(&mut v, 'n'), Action::Next);
        assert_eq!(act(&mut v, 'u'), Action::Untag);
    }

    #[test]
    fn finds_and_highlights_text_hits() {
        let text = (0..50)
            .map(|i| {
                if i % 10 == 5 {
                    format!("linha {i} com ALVO aqui\n")
                } else {
                    format!("linha {i}\n")
                }
            })
            .collect::<String>();
        let mut v = viewer(text.as_bytes());
        v.page = 10;
        let p = pat("alvo");
        assert!(v.search(&p, true, SearchFrom::Edge));
        assert_eq!(v.hit.map(|h| (h.line, h.start, h.end)), Some((5, 12, 16)));
        assert!(v.search_next(&p, true));
        assert_eq!(v.hit.unwrap().line, 15);
        assert!(v.top <= 15 && 15 < v.top + 10, "hit is visible");
        // Highlight in the right columns.
        let shown = v.visible(10);
        let (line, mark) = &shown[15 - v.top];
        assert!(line.contains("ALVO"));
        assert_eq!(*mark, Some((13, 17)));

        assert!(v.search_next(&p, false));
        assert_eq!(v.hit.unwrap().line, 5);
        assert!(!v.search_next(&p, false), "nothing before the first hit");
        assert!(v.search(&p, false, SearchFrom::Edge));
        assert_eq!(v.hit.unwrap().line, 45);
        assert!(!v.search_next(&p, true));
    }

    #[test]
    fn several_hits_on_one_line_and_hex_mode() {
        let mut v = viewer(b"ab ab ab\n");
        let p = pat("ab");
        assert!(v.search(&p, true, SearchFrom::Edge));
        assert!(v.search_next(&p, true));
        assert_eq!(v.hit.unwrap().start, 3);

        let mut data = vec![0u8; 64];
        data[40..43].copy_from_slice(b"MZP");
        let mut v = viewer(&data);
        assert_eq!(v.mode, ViewMode::Hex);
        assert!(v.search(&pat("mzp"), true, SearchFrom::Edge));
        assert_eq!(v.hit.map(|h| (h.line, h.start)), Some((2, 40)));
        assert!(!v.search_next(&pat("mzp"), true));
    }

    #[test]
    fn colors_code_and_keeps_the_search_mark() {
        let src = b"fn main() {\n    let senha = 1;\n}\n";
        let mut v = Viewer::from_bytes(PathBuf::from("a.rs"), src.len() as u64, src.to_vec());
        assert_eq!(v.language(), Some("Rust"));
        v.page = 5;
        assert!(v.search(&pat("senha"), true, SearchFrom::Edge));
        let rows = v.styled(5);
        let line1: String = rows[1].iter().map(|(t, ..)| t.as_str()).collect();
        assert_eq!(line1, "    let senha = 1;");
        assert!(rows[1].iter().any(|(t, _, marked)| *marked && t == "senha"));
        assert!(
            rows[1]
                .iter()
                .any(|(t, c, _)| t.contains("let") && c.is_some())
        );

        // C turns colors off; the text stays the same.
        press(&mut v, KeyCode::Char('c'));
        assert_eq!(v.language(), None);
        assert!(v.styled(5)[1].iter().all(|(_, c, _)| c.is_none()));
        assert_eq!(v.note.as_deref(), Some("Rust colors off"));

        let mut plain = viewer(b"just text\n");
        press(&mut plain, KeyCode::Char('c'));
        assert_eq!(
            plain.note.as_deref(),
            Some("No syntax colors for this file")
        );
    }
}
