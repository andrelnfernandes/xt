//! File viewer (`V` key; Ctrl+V for the tagged files), in text or
//! hexadecimal mode, with search.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui::style::Color;

use crate::decompress::{
    self, AUTOVIEW_DICT, AUTOVIEW_TIME, Compression, Limits, VIEW_DICT, VIEW_TIME,
};
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
    /// Opens the file in the editor.
    Edit,
    /// Git diff of the file, or back to the file.
    Diff,
    /// Asks for the term and searches.
    Search {
        forward: bool,
        from: SearchFrom,
    },
    /// Continues the last search (`None`: same direction, with Space).
    Continue {
        forward: Option<bool>,
    },
    /// Copies the file's path to the clipboard.
    CopyPath,
    /// Prompts for line number to go to.
    GotoLine,
}

/// How a file is read: how much of it, and whether a compressed file is
/// shown decompressed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoadOptions {
    pub decompress: bool,
    pub limits: Limits,
}

impl LoadOptions {
    /// The V key: up to `limit` bytes read, and as many decompressed.
    pub fn view(limit: u64) -> Self {
        LoadOptions {
            decompress: true,
            limits: Limits {
                output: limit as usize,
                dict: VIEW_DICT,
                time: VIEW_TIME,
            },
        }
    }

    /// Autoview: 1 MiB and tighter memory and time limits.
    pub fn autoview() -> Self {
        LoadOptions {
            decompress: true,
            limits: Limits {
                output: 1 << 20,
                dict: AUTOVIEW_DICT,
                time: AUTOVIEW_TIME,
            },
        }
    }
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
    /// Where a bidi control was swapped for `·`: (line, character column).
    bidi: Vec<(usize, usize)>,
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
    /// Read from a file on disk (not help, an info screen or an archive
    /// entry): it can be edited and reloaded.
    pub on_disk: bool,
    /// In a diff (D): the file it belongs to, shown again by D.
    pub behind: Option<Box<Viewer>>,
    /// Soft wrap long lines to fit `width` (W toggles).
    pub wrap: bool,
    /// Show line numbers in text mode (# / L toggles).
    pub line_numbers: bool,
    /// First visible wrapped sub-row within `lines[top]` when `wrap` is active.
    pub sub_top: usize,
    /// How the file was read (kept by `reload`).
    pub load: LoadOptions,
    /// The compression the file was shown through, and the file's size.
    pub compression: Option<(Compression, u64)>,
    /// Not all of the file (or of its decompressed contents) is shown.
    cut: bool,
}

/// A piece of a displayed line: text, its color and whether it is part of
/// the search match.
pub type Segment = (String, Option<Color>, bool);

/// A displayed visual row: its line number (if first visual row and line numbers
/// are enabled) and its styled text segments.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledRow {
    pub line_num: Option<usize>,
    pub segments: Vec<Segment>,
}

impl std::ops::Deref for StyledRow {
    type Target = [Segment];
    fn deref(&self) -> &Self::Target {
        &self.segments
    }
}

impl Viewer {
    /// Reads up to `load.limits.output` bytes of the file on disk.
    pub fn open(path: PathBuf, load: LoadOptions) -> io::Result<Self> {
        let file = File::open(&path)?;
        let size = file.metadata()?.len();
        let mut raw = Vec::new();
        file.take(load.limits.output as u64).read_to_end(&mut raw)?;
        Ok(Viewer {
            on_disk: true,
            ..Self::from_file(path, size, raw, load)
        })
    }

    /// Views the start of a file (`raw`, of `size` bytes in all), decompressed
    /// when it is a .gz/.xz/.zst/.bz2 and `load` says so.
    pub fn from_file(path: PathBuf, size: u64, raw: Vec<u8>, load: LoadOptions) -> Self {
        let mut cut = size > raw.len() as u64;
        let mut compression = None;
        let mut note = None;
        let mut failed = false;
        let mut data = raw;
        if let Some(kind) = load.decompress.then(|| decompress::sniff(&data)).flatten() {
            let d = decompress::decompress(&data, kind, load.limits);
            if d.data.is_empty() {
                failed = true;
            } else {
                data = d.data;
                cut |= d.cut;
                compression = Some((kind, size));
            }
            note = d.note;
            if load.limits.dict == AUTOVIEW_DICT
                && note.as_deref() == Some(decompress::memory_note(AUTOVIEW_DICT).as_str())
            {
                note = note.map(|n| format!("{n} (V to view)"));
            }
        }
        if data.get(257..262) == Some(b"ustar") {
            note = Some("TAR archive: Alt+F5 opens it as a folder".into());
        }
        // The colors follow the name inside the compressed file.
        let name = compression.and_then(|(kind, _)| {
            let s = path.to_str()?.strip_suffix(kind.extension())?;
            Some(PathBuf::from(s.strip_suffix('.')?))
        });
        let mut v = Self::from_bytes(path, size, data);
        if let Some(name) = name {
            v = v.colored_as(&name);
        }
        if failed {
            // The raw bytes of a compressed file that gave nothing.
            v.mode = ViewMode::Hex;
            v.colors = None;
            v.colors_on = false;
        }
        v.load = load;
        v.compression = compression;
        v.cut = cut;
        v.note = note;
        v
    }

    pub fn limit(&self) -> u64 {
        self.load.limits.output as u64
    }

    /// Bytes of contents shown (decompressed ones, when it is compressed).
    pub fn data_len(&self) -> u64 {
        self.data.len() as u64
    }

    /// Reads the file again (after editing), keeping the position, the mode
    /// and whether colors are on.
    pub fn reload(&mut self) -> io::Result<()> {
        let new = Self::open(self.path.clone(), self.load)?;
        let old = std::mem::replace(self, new);
        self.mode = old.mode;
        self.top = old.top.min(self.line_count().saturating_sub(1));
        self.left = old.left;
        self.page = old.page;
        self.width = old.width;
        self.colors_on = old.colors_on && self.colors.is_some();
        self.wrap = old.wrap;
        self.line_numbers = old.line_numbers;
        self.sub_top = old.sub_top;
        Ok(())
    }

    /// Autoview scrolling (Shift+arrows): only moves the content.
    pub fn scroll(&mut self, code: KeyCode) {
        let page = self.page.max(1);
        match code {
            KeyCode::Up => self.scroll_up(1),
            KeyCode::Down => self.scroll_down(1),
            KeyCode::PageUp => self.scroll_up(page),
            KeyCode::PageDown => self.scroll_down(page),
            KeyCode::Home => {
                self.top = 0;
                self.sub_top = 0;
            }
            KeyCode::End => {
                if self.mode == ViewMode::Text && self.wrap {
                    self.top = self.lines.len().saturating_sub(1);
                    self.sub_top = 0;
                } else {
                    self.top = self.line_count().saturating_sub(page);
                }
            }
            _ => {}
        }
    }

    pub fn scroll_down(&mut self, count: usize) {
        if self.mode == ViewMode::Text && self.wrap {
            let cw = self.content_width();
            for _ in 0..count {
                if self.top >= self.lines.len() {
                    break;
                }
                let sub_count = wrap_line_ranges(&self.lines[self.top], cw).len();
                if self.sub_top + 1 < sub_count {
                    self.sub_top += 1;
                } else if self.top + 1 < self.lines.len() {
                    self.top += 1;
                    self.sub_top = 0;
                } else {
                    break;
                }
            }
        } else {
            let max_top = self.line_count().saturating_sub(self.page.max(1));
            self.top = (self.top + count).min(max_top);
        }
    }

    pub fn scroll_up(&mut self, count: usize) {
        if self.mode == ViewMode::Text && self.wrap {
            let cw = self.content_width();
            for _ in 0..count {
                if self.sub_top > 0 {
                    self.sub_top -= 1;
                } else if self.top > 0 {
                    self.top -= 1;
                    let sub_count = wrap_line_ranges(&self.lines[self.top], cw).len();
                    self.sub_top = sub_count.saturating_sub(1);
                } else {
                    break;
                }
            }
        } else {
            self.top = self.top.saturating_sub(count);
        }
    }

    pub fn toggle_wrap(&mut self) {
        self.wrap = !self.wrap;
        self.sub_top = 0;
        self.left = 0;
        let state = if self.wrap { "on" } else { "off" };
        self.note = Some(format!("Wrap {state}"));
    }

    pub fn toggle_line_numbers(&mut self) {
        self.line_numbers = !self.line_numbers;
        let state = if self.line_numbers { "on" } else { "off" };
        self.note = Some(format!("Line numbers {state}"));
    }

    pub fn goto_line(&mut self, target_1_indexed: usize) {
        if self.mode == ViewMode::Text {
            let max_line = self.lines.len().max(1);
            let line = target_1_indexed.clamp(1, max_line);
            self.top = line.saturating_sub(1);
            self.sub_top = 0;
            self.left = 0;
            self.note = Some(format!("Line {line} of {max_line}"));
        } else {
            let max_offset = self.data.len();
            let offset = target_1_indexed.min(max_offset);
            self.top = (offset / HEX_WIDTH).min(self.line_count().saturating_sub(1));
            self.note = Some(format!("Offset {offset:X}"));
        }
    }

    pub fn gutter_width(&self) -> usize {
        if self.line_numbers && self.mode == ViewMode::Text {
            self.lines.len().to_string().len().max(1) + 3 // e.g. " 42 │ "
        } else {
            0
        }
    }

    pub fn content_width(&self) -> usize {
        self.width.saturating_sub(self.gutter_width()).max(1)
    }

    /// Views bytes already read (e.g. an entry of an archive).
    pub fn from_bytes(path: PathBuf, size: u64, data: Vec<u8>) -> Self {
        let mode = if looks_binary(&data) {
            ViewMode::Hex
        } else {
            ViewMode::Text
        };
        let data_len = data.len() as u64;
        let (lines, bidi) = text_lines(&String::from_utf8_lossy(&data));
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
            bidi,
            data,
            top: 0,
            left: 0,
            page: 1,
            width: 80,
            hit: None,
            forward: true,
            note: None,
            on_disk: false,
            behind: None,
            wrap: false,
            line_numbers: false,
            sub_top: 0,
            load: LoadOptions::view(MAX_BYTES),
            compression: None,
            cut: size > data_len,
        }
    }

    /// Colors as if it were the file `name` (`x.diff` for git's output, or
    /// `highlight::GIT_STATUS`).
    pub fn colored_as(mut self, name: &Path) -> Self {
        if self.mode == ViewMode::Text {
            self.colors = Highlighter::for_name(name, &self.lines);
            self.colors_on = self.colors.is_some();
        }
        self
    }

    /// How many bidi controls were hidden behind a `·`.
    pub fn bidi_count(&self) -> usize {
        self.bidi.len()
    }

    pub fn truncated(&self) -> bool {
        self.cut
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

    #[cfg(test)]
    pub fn visible(&self, height: usize) -> Vec<(String, Option<(usize, usize)>)> {
        let hit = self.hit.filter(|h| h.mode == self.mode);
        match self.mode {
            ViewMode::Text if !self.wrap => {
                let range = self.top..(self.top + height).min(self.lines.len());
                range
                    .map(|i| {
                        let line: String = self.lines[i].chars().skip(self.left).collect();
                        let mark = hit.filter(|h| h.line == i).and_then(|h| {
                            let s = h.start.saturating_sub(self.left);
                            let e = h.end.saturating_sub(self.left);
                            (e > s).then_some((s, e))
                        });
                        (line, mark)
                    })
                    .collect()
            }
            ViewMode::Text => {
                let cw = self.content_width();
                let mut out = Vec::new();
                let mut line_idx = self.top;
                let mut sub_idx = self.sub_top;
                while out.len() < height && line_idx < self.lines.len() {
                    let ranges = wrap_line_ranges(&self.lines[line_idx], cw);
                    while sub_idx < ranges.len() && out.len() < height {
                        let r = &ranges[sub_idx];
                        let chars: String = self.lines[line_idx]
                            .chars()
                            .skip(r.start)
                            .take(r.end - r.start)
                            .collect();
                        let mark = hit.filter(|h| h.line == line_idx).and_then(|h| {
                            let s = h.start.max(r.start);
                            let e = h.end.min(r.end);
                            if e > s {
                                Some((s - r.start, e - r.start))
                            } else {
                                None
                            }
                        });
                        out.push((chars, mark));
                        sub_idx += 1;
                    }
                    line_idx += 1;
                    sub_idx = 0;
                }
                out
            }
            ViewMode::Hex => {
                let range = self.top..(self.top + height).min(self.line_count());
                range
                    .map(|row| {
                        let line = hex_row(&self.data, row * HEX_WIDTH);
                        let mark = hit
                            .filter(|h| h.line == row)
                            .map(|_| (0, line.chars().count()));
                        (line, mark)
                    })
                    .collect()
            }
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
    pub fn styled(&mut self, height: usize) -> Vec<StyledRow> {
        let use_colors = self.colors_on && self.mode == ViewMode::Text;
        let hit = self.hit.filter(|h| h.mode == self.mode);
        match self.mode {
            ViewMode::Text if !self.wrap => {
                let range = self.top..(self.top + height).min(self.lines.len());
                let (left, colors, lines, bidi) =
                    (self.left, &mut self.colors, &self.lines, &self.bidi);
                range
                    .map(|i| {
                        let line_str = &lines[i];
                        let line_chars: Vec<char> = line_str.chars().skip(left).collect();
                        let mark = hit.filter(|h| h.line == i).and_then(|h| {
                            let s = h.start.saturating_sub(left);
                            let e = h.end.saturating_sub(left);
                            (e > s).then_some((s, e))
                        });
                        let per_char: Vec<Color> = colors
                            .as_mut()
                            .filter(|_| use_colors)
                            .and_then(|h| h.line(i, lines))
                            .map(|segs| {
                                segs.iter()
                                    .flat_map(|(c, s)| std::iter::repeat_n(*c, s.chars().count()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        let mut segments: Vec<Segment> = Vec::new();
                        for (j, ch) in line_chars.into_iter().enumerate() {
                            let color = if bidi.binary_search(&(i, left + j)).is_ok() {
                                Some(crate::config::theme().error)
                            } else {
                                per_char.get(left + j).copied()
                            };
                            let marked = mark.is_some_and(|(s, e)| j >= s && j < e);
                            match segments.last_mut() {
                                Some((text, c, m)) if *c == color && *m == marked => text.push(ch),
                                _ => segments.push((ch.to_string(), color, marked)),
                            }
                        }
                        StyledRow {
                            line_num: if self.line_numbers { Some(i + 1) } else { None },
                            segments,
                        }
                    })
                    .collect()
            }
            ViewMode::Text => {
                let cw = self.content_width();
                let mut out = Vec::new();
                let mut line_idx = self.top;
                let mut sub_idx = self.sub_top;
                while out.len() < height && line_idx < self.lines.len() {
                    let ranges = wrap_line_ranges(&self.lines[line_idx], cw);
                    let line_chars: Vec<char> = self.lines[line_idx].chars().collect();
                    let per_char: Vec<Color> = self
                        .colors
                        .as_mut()
                        .filter(|_| use_colors)
                        .and_then(|h| h.line(line_idx, &self.lines))
                        .map(|segs| {
                            segs.iter()
                                .flat_map(|(c, s)| std::iter::repeat_n(*c, s.chars().count()))
                                .collect()
                        })
                        .unwrap_or_default();

                    while sub_idx < ranges.len() && out.len() < height {
                        let r = &ranges[sub_idx];
                        let mark = hit.filter(|h| h.line == line_idx).and_then(|h| {
                            let s = h.start.max(r.start);
                            let e = h.end.min(r.end);
                            if e > s {
                                Some((s - r.start, e - r.start))
                            } else {
                                None
                            }
                        });
                        let mut segments: Vec<Segment> = Vec::new();
                        let slice = &line_chars[r.start..r.end];
                        for (j, &ch) in slice.iter().enumerate() {
                            let char_idx = r.start + j;
                            let color = if self.bidi.binary_search(&(line_idx, char_idx)).is_ok() {
                                Some(crate::config::theme().error)
                            } else {
                                per_char.get(char_idx).copied()
                            };
                            let marked = mark.is_some_and(|(s, e)| j >= s && j < e);
                            match segments.last_mut() {
                                Some((text, c, m)) if *c == color && *m == marked => text.push(ch),
                                _ => segments.push((ch.to_string(), color, marked)),
                            }
                        }
                        out.push(StyledRow {
                            line_num: if self.line_numbers && sub_idx == 0 {
                                Some(line_idx + 1)
                            } else {
                                None
                            },
                            segments,
                        });
                        sub_idx += 1;
                    }
                    line_idx += 1;
                    sub_idx = 0;
                }
                out
            }
            ViewMode::Hex => {
                let range = self.top..(self.top + height).min(self.line_count());
                range
                    .map(|row| {
                        let line = hex_row(&self.data, row * HEX_WIDTH);
                        let marked = hit.is_some_and(|h| h.line == row);
                        StyledRow {
                            line_num: None,
                            segments: vec![(line, None, marked)],
                        }
                    })
                    .collect()
            }
        }
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
                self.scroll_up(1);
                Action::None
            }
            KeyCode::Down => {
                self.scroll_down(1);
                Action::None
            }
            KeyCode::PageUp => {
                self.scroll_up(page);
                Action::None
            }
            KeyCode::PageDown => {
                self.scroll_down(page);
                Action::None
            }
            KeyCode::Home => {
                self.top = 0;
                self.sub_top = 0;
                Action::None
            }
            KeyCode::End => {
                if self.mode == ViewMode::Text && self.wrap {
                    self.top = self.lines.len().saturating_sub(1);
                    self.sub_top = 0;
                } else {
                    self.top = max_top;
                }
                Action::None
            }
            KeyCode::Left if self.mode == ViewMode::Text && !self.wrap => {
                self.left = self.left.saturating_sub(PAN_STEP);
                Action::None
            }
            KeyCode::Right if self.mode == ViewMode::Text && !self.wrap => {
                self.left += PAN_STEP;
                Action::None
            }
            KeyCode::Char('w' | 'W') => {
                self.toggle_wrap();
                Action::None
            }
            KeyCode::Char('#' | 'l' | 'L') => {
                self.toggle_line_numbers();
                Action::None
            }
            KeyCode::Char('g' | 'G' | ':') => Action::GotoLine,
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
                'e' => Action::Edit,
                'd' => Action::Diff,
                'y' => Action::CopyPath,
                _ => Action::None,
            },
            KeyCode::Insert if key.modifiers.contains(KeyModifiers::CONTROL) => Action::CopyPath,
            KeyCode::F(9) => Action::Search {
                forward: true,
                from: SearchFrom::Page,
            },
            _ => Action::None,
        };
        if self.mode == ViewMode::Text && self.wrap {
            self.top = self.top.min(self.lines.len().saturating_sub(1));
        } else {
            self.top = self.top.min(max_top);
        }
        action
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ViewMode::Text => ViewMode::Hex,
            ViewMode::Hex => ViewMode::Text,
        };
        self.top = 0;
        self.sub_top = 0;
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
        if self.mode == ViewMode::Text && self.wrap {
            if hit.line < self.top || hit.line >= self.top + page {
                self.top = hit.line.saturating_sub(page / 3);
            }
            let cw = self.content_width();
            let ranges = wrap_line_ranges(&self.lines[hit.line], cw);
            self.sub_top = ranges
                .iter()
                .position(|r| hit.start >= r.start && hit.start < r.end)
                .unwrap_or(0);
            self.left = 0;
        } else {
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
}

/// Returns the ranges of characters for each wrapped visual row of `text`.
/// Width must be at least 1.
#[allow(clippy::single_range_in_vec_init)]
pub fn wrap_line_ranges(text: &str, width: usize) -> Vec<std::ops::Range<usize>> {
    let width = width.max(1);
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    if n == 0 {
        return vec![0..0];
    }
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < n {
        let remaining = n - start;
        if remaining <= width {
            ranges.push(start..n);
            break;
        }
        let limit = start + width;
        let break_at = chars[start..=limit]
            .iter()
            .enumerate()
            .rposition(|(idx, &c)| idx > 0 && c.is_whitespace());

        if let Some(pos) = break_at {
            let space_pos = start + pos;
            ranges.push(start..space_pos);
            let mut next_start = space_pos + 1;
            while next_start < n && chars[next_start] == ' ' {
                next_start += 1;
            }
            start = next_start;
        } else {
            ranges.push(start..limit);
            start = limit;
        }
    }
    if ranges.is_empty() {
        ranges.push(0..0);
    }
    ranges
}

/// How far `looks_binary` looks.
pub const BINARY_PROBE: usize = 8192;

/// Simple heuristic: there is a null byte near the start of the file.
pub fn looks_binary(data: &[u8]) -> bool {
    data.iter().take(BINARY_PROBE).any(|&b| b == 0)
}

/// Splits into lines, expands tabs and replaces control characters with `·`.
/// Also returns where each bidi control was replaced: (line, character
/// column after expanding tabs), in order.
fn text_lines(text: &str) -> (Vec<String>, Vec<(usize, usize)>) {
    let mut bidi = Vec::new();
    let mut lines: Vec<String> = text
        .split('\n')
        .enumerate()
        .map(|(i, line)| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            let mut out = String::with_capacity(line.len());
            let mut col = 0;
            for c in line.chars() {
                if c == '\t' {
                    let n = TAB_WIDTH - col % TAB_WIDTH;
                    out.extend(std::iter::repeat_n(' ', n));
                    col += n;
                } else {
                    if crate::bidi::is_bidi(c) {
                        bidi.push((i, col));
                    }
                    out.push(if c.is_control() || crate::bidi::is_bidi(c) {
                        '·'
                    } else {
                        c
                    });
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
    (lines, bidi)
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
    use crate::decompress::test_util::gz;
    use crate::search::SearchSpec;

    const LOAD: LoadOptions = LoadOptions {
        decompress: true,
        limits: Limits {
            output: MAX_BYTES as usize,
            dict: VIEW_DICT,
            time: VIEW_TIME,
        },
    };

    fn from_gz(name: &str, raw: Vec<u8>, load: LoadOptions) -> Viewer {
        let n = raw.len() as u64;
        Viewer::from_file(PathBuf::from(name), n, raw, load)
    }

    #[test]
    fn gz_shows_text() {
        let raw = gz(b"one\ntwo\n");
        let n = raw.len() as u64;
        let v = Viewer::from_file(PathBuf::from("app.log.gz"), n, raw, LOAD);
        assert_eq!(v.visible_lines(2), ["one", "two"]);
        assert_eq!(v.compression, Some((Compression::Gzip, n)));
        assert!(!v.truncated());
    }

    #[test]
    fn cut_counts_the_decompressed_side() {
        let text = "0123456789abcdef\n".repeat(4 << 16);
        let raw = gz(text.as_bytes());
        let n = raw.len() as u64;
        assert!(n < 1 << 20);
        let v = Viewer::from_file(
            PathBuf::from("big.txt.gz"),
            n,
            raw,
            LoadOptions::view(1 << 20),
        );
        assert!(v.truncated());
        assert!(n < v.data.len() as u64);
    }

    #[test]
    fn plain_text_named_gz_stays_text() {
        let v = from_gz("notes.txt.gz", b"just text".to_vec(), LOAD);
        assert_eq!(v.compression, None);
        assert_eq!(v.visible_lines(1), ["just text"]);
    }

    #[test]
    fn signature_only_shows_raw_hex() {
        let v = from_gz("a.gz", b"\x1F\x8B".to_vec(), LOAD);
        assert_eq!(v.mode, ViewMode::Hex);
        assert_eq!(v.note.as_deref(), Some(crate::decompress::TRUNCATED));
    }

    #[test]
    fn decompressed_source_gets_its_colors() {
        let v = from_gz("x.rs.gz", gz(b"fn main() {}\n"), LOAD);
        assert_eq!(v.language(), Some("Rust"));
    }

    #[test]
    fn tar_gz_says_alt_f5() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("a.tar.gz");
        crate::archive::tests::make_tar(&path, true);
        let raw = std::fs::read(&path).unwrap();
        let v = Viewer::from_file(path, raw.len() as u64, raw, LOAD);
        assert_eq!(
            v.note.as_deref(),
            Some("TAR archive: Alt+F5 opens it as a folder")
        );
    }

    #[test]
    fn decompress_off_shows_hex() {
        let load = LoadOptions {
            decompress: false,
            ..LOAD
        };
        let v = from_gz("a.txt.gz", gz(b"one\n"), load);
        assert_eq!(v.mode, ViewMode::Hex);
        assert_eq!(v.compression, None);
    }

    #[test]
    fn reload_keeps_the_load_options() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("a.txt.gz");
        std::fs::write(&path, gz(b"one\n")).unwrap();
        let load = LoadOptions::view(1 << 20);
        let mut v = Viewer::open(path, load).unwrap();
        v.reload().unwrap();
        assert_eq!(v.load, load);
        assert_eq!(v.visible_lines(1), ["one"]);
    }

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
    fn bidi_controls_show_as_marked_dots() {
        let mut v = Viewer::from_bytes(
            PathBuf::from("x.txt"),
            0,
            b"a\xE2\x80\xAEb\xE2\x80\xACc\n\t\xE2\x81\xA6x".to_vec(),
        );
        assert_eq!(v.visible_lines(2), ["a·b·c", "        ·x"]);
        assert_eq!(v.bidi_count(), 3);
        let error = crate::config::theme().error;
        let marked = |row: &StyledRow| {
            row.iter()
                .filter(|s| s.1 == Some(error))
                .map(|s| s.0.clone())
                .collect::<Vec<_>>()
        };
        let rows = v.styled(2);
        assert_eq!(marked(&rows[0]), ["·", "·"]);
        assert_eq!(marked(&rows[1]), ["·"]);
        v.toggle_wrap();
        assert_eq!(marked(&v.styled(2)[0]), ["·", "·"]);
    }

    #[test]
    fn splits_text_lines() {
        assert_eq!(text_lines("a\r\nb\n").0, ["a", "b"]);
        assert_eq!(text_lines("a\tb").0, ["a       b"]);
        assert_eq!(text_lines("x\x07y").0, ["x·y"]);
        assert_eq!(text_lines("").0, [""]);
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
    fn reload_keeps_position_and_mode() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("a.txt");
        let lines = |n: usize| (0..n).map(|i| format!("line {i}\n")).collect::<String>();
        std::fs::write(&path, lines(100)).unwrap();
        let mut v = Viewer::open(path.clone(), LoadOptions::view(MAX_BYTES)).unwrap();
        assert!(v.on_disk);
        v.top = 50;
        std::fs::write(&path, lines(120)).unwrap();
        v.reload().unwrap();
        assert_eq!((v.top, v.line_count()), (50, 120));
        assert_eq!(v.visible_lines(1)[0], "line 50");

        // Shorter file: the position stays inside it; hex stays hex.
        press(&mut v, KeyCode::Char('h'));
        v.top = 0;
        std::fs::write(&path, "x").unwrap();
        v.reload().unwrap();
        assert_eq!((v.mode, v.top), (ViewMode::Hex, 0));
        assert!(!viewer(b"text").on_disk, "bytes are not a file on disk");
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
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("f.txt");
        std::fs::write(&path, "abc").unwrap();
        let v = Viewer::open(path, LoadOptions::view(MAX_BYTES)).unwrap();
        assert_eq!(v.size, 3);
        assert!(!v.truncated());
        assert!(Viewer::open(tmp.path().join("missing"), LoadOptions::view(MAX_BYTES)).is_err());
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

    #[test]
    fn test_wrap_line_ranges() {
        // Simple word wrap
        let ranges = wrap_line_ranges("hello world this is a test", 12);
        assert_eq!(ranges, vec![0..11, 12..21, 22..26]);

        // Long unbroken word
        let ranges = wrap_line_ranges("abcdefghij", 4);
        assert_eq!(ranges, vec![0..4, 4..8, 8..10]);

        // Empty line
        let ranges = wrap_line_ranges("", 10);
        assert_eq!(ranges, vec![0..0]);
    }

    #[test]
    fn test_wrap_toggle_and_scrolling() {
        let text = b"line 1\nthis is a very long line that should wrap into multiple visual lines when wrap is enabled\nline 3\n";
        let mut v = viewer(text);
        v.width = 30;
        v.page = 10;
        assert!(!v.wrap);

        // Toggle wrap with 'w'
        press(&mut v, KeyCode::Char('w'));
        assert!(v.wrap);
        assert_eq!(v.note.as_deref(), Some("Wrap on"));

        let lines = v.visible_lines(10);
        assert!(lines.len() >= 4);
        assert_eq!(lines[0], "line 1");

        // Scroll down advances sub-rows
        press(&mut v, KeyCode::Down);
        assert_eq!(v.top, 1);
        assert_eq!(v.sub_top, 0);

        press(&mut v, KeyCode::Down);
        assert_eq!(v.top, 1);
        assert_eq!(v.sub_top, 1);

        // Scroll up rewinds sub-rows
        press(&mut v, KeyCode::Up);
        assert_eq!(v.top, 1);
        assert_eq!(v.sub_top, 0);
    }

    #[test]
    fn test_line_numbers_toggle() {
        let text = b"first\nsecond\nthird\n";
        let mut v = viewer(text);
        assert!(!v.line_numbers);

        // Toggle with '#'
        press(&mut v, KeyCode::Char('#'));
        assert!(v.line_numbers);
        assert_eq!(v.note.as_deref(), Some("Line numbers on"));

        let rows = v.styled(3);
        assert_eq!(rows[0].line_num, Some(1));
        assert_eq!(rows[1].line_num, Some(2));
        assert_eq!(rows[2].line_num, Some(3));

        // Toggle with 'l'
        press(&mut v, KeyCode::Char('l'));
        assert!(!v.line_numbers);
        assert_eq!(v.note.as_deref(), Some("Line numbers off"));
    }

    #[test]
    fn test_goto_line() {
        let text = (1..=100).map(|i| format!("line {i}\n")).collect::<String>();
        let mut v = viewer(text.as_bytes());
        v.page = 10;

        // Key 'g' or ':' triggers Action::GotoLine
        assert_eq!(
            v.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
            Action::GotoLine
        );
        assert_eq!(
            v.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE)),
            Action::GotoLine
        );

        // goto_line directly
        v.goto_line(50);
        assert_eq!(v.top, 49);
        assert_eq!(v.note.as_deref(), Some("Line 50 of 100"));

        // Out of bounds clamp
        v.goto_line(999);
        assert_eq!(v.top, 99);
    }
}
