//! File viewer (`V` key; Ctrl+V for the tagged files), in text or
//! hexadecimal mode, with search.

pub mod sql_filter;
pub mod table;
pub mod tabular_loader;

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui::style::Color;

use crate::charset::{self, Charset};
use crate::decompress::{
    self, AUTOVIEW_DICT, AUTOVIEW_TIME, Compression, Limits, VIEW_DICT, VIEW_TIME,
};
use crate::highlight::Highlighter;
use crate::search::Pattern;

/// The default viewer limit, the floor of `auto` and the cap for remote
/// files (16 MiB).
pub const MAX_BYTES: u64 = 16 * 1024 * 1024;
pub const HEX_WIDTH: usize = 16;
const TAB_WIDTH: usize = 8;
const PAN_STEP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Text,
    Hex,
    Table,
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
    /// Prompts for a SQL WHERE filter in table mode.
    FilterSql,
}

/// How a file is read: how much of it, and whether a compressed file is
/// shown decompressed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoadOptions {
    pub decompress: bool,
    pub limits: Limits,
    /// Fixed charset for files that are not UTF-8 or UTF-16 (`None`: guess).
    pub fallback: Option<Charset>,
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
            fallback: None,
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
            fallback: None,
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
    /// How `data` is decoded into `lines`.
    charset: Charset,
    /// The charset was picked with O: kept by `reload`.
    charset_manual: bool,
    /// The name the colors follow, when it is not `path` (see `colored_as`).
    color_name: Option<PathBuf>,
    /// The compression the file was shown through, and the file's size.
    pub compression: Option<(Compression, u64)>,
    /// Not all of the file (or of its decompressed contents) is shown.
    cut: bool,
    /// The line budget stopped `lines` before the end of the text.
    lines_cut: bool,
    /// What went wrong decompressing, for the header: the note says it
    /// too, but any key clears the note.
    problem: Option<String>,
    /// Tabular grid for CSV, spreadsheet, SQLite, or DBF files.
    pub table: Option<table::TableGrid>,
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
        let mut problem = None;
        let mut failed = false;
        let mut data = raw;
        if let Some(kind) = load.decompress.then(|| decompress::sniff(&data)).flatten() {
            let d = decompress::decompress(&data, kind, load.limits);
            if d.data.is_empty() && d.note.is_some() {
                failed = true;
            } else {
                data = d.data;
                cut |= d.cut;
                compression = Some((kind, size));
            }
            note = d.note;
            // The decoder running out of input is expected when the read
            // limit cut the compressed file.
            if cut && !failed && note.as_deref() == Some(decompress::TRUNCATED) {
                note = None;
            }
            problem = note.as_deref().and_then(|n| problem_text(n, failed, d.cut));
            // V helps only if its own limit is enough. xz does not say what
            // it needs, so V is suggested and may refuse too.
            let v_can = d.needed.is_none_or(|n| n <= u64::from(VIEW_DICT));
            if load.limits.dict == AUTOVIEW_DICT
                && v_can
                && note.as_deref() == Some(decompress::memory_note(AUTOVIEW_DICT).as_str())
            {
                note = note.map(|n| format!("{n} (V to view)"));
            }
        }
        if note.is_none() && data.get(257..262) == Some(b"ustar") {
            note = Some("TAR archive: Alt+F5 opens it as a folder".into());
        }
        // The colors follow the name inside the compressed file.
        let name = compression.and_then(|(kind, _)| {
            let s = path.to_str()?.strip_suffix(kind.extension())?;
            Some(PathBuf::from(s.strip_suffix('.')?))
        });
        let found = charset::detect(&data, load.fallback);
        let limit = load.limits.output;
        let mut v = Self::build(path.clone(), size, data, found.charset, found.binary, limit);
        if let Some(name) = name {
            v = v.colored_as(&name);
        }

        // Auto-load tabular candidate files into Table mode if on disk
        if tabular_loader::is_tabular_candidate(&path) {
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();

            let grid_res = match ext.as_str() {
                "csv" | "tsv" | "tab" => tabular_loader::load_csv(&path, None),
                "jsonl" | "ndjson" => tabular_loader::load_jsonl(&path),
                "xlsx" | "ods" | "xls" | "xlsb" => tabular_loader::load_spreadsheet(&path),
                "dbf" => tabular_loader::load_dbf(&path),
                "sqlite" | "sqlite3" | "db" => tabular_loader::load_sqlite(&path, None),
                _ => Err("Not supported".to_string()),
            };

            if let Ok(grid) = grid_res {
                v.table = Some(grid);
                v.mode = ViewMode::Table;
            }
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
        v.problem = problem;
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
        if old.charset_manual {
            self.set_charset(old.charset);
            self.charset_manual = true;
        }
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
                // Only as far as the next row: the line can be huge.
                let next_row = WrapRows::new(&self.lines[self.top], cw).nth(self.sub_top + 1);
                if next_row.is_some() {
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
                    let rows = WrapRows::new(&self.lines[self.top], cw).count();
                    self.sub_top = rows.saturating_sub(1);
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
        let binary = looks_binary(&data);
        Self::build(path, size, data, Charset::Utf8, binary, MAX_BYTES as usize)
    }

    /// Text with ANSI colors (git's graph), shown in those colors and
    /// without the escape sequences.
    pub fn from_ansi(path: PathBuf, data: &[u8]) -> Self {
        let (plain, colors) = crate::highlight::from_ansi(data);
        let mut v = Self::from_bytes(path, plain.len() as u64, plain);
        if v.mode == ViewMode::Text {
            v.colors_on = colors.is_some();
            v.colors = colors;
        }
        v
    }

    /// `data` shown as `charset` text, or as hex when it is `binary`.
    /// `limit` is the read limit, which sets the lines' memory budget.
    fn build(
        path: PathBuf,
        size: u64,
        data: Vec<u8>,
        charset: Charset,
        binary: bool,
        limit: usize,
    ) -> Self {
        let mode = if binary {
            ViewMode::Hex
        } else {
            ViewMode::Text
        };
        let data_len = data.len() as u64;
        let text = text_lines(&charset::decode(&data, charset), line_budget(limit));
        let (lines, bidi) = (text.lines, text.bidi);
        let colors = (!binary)
            .then(|| text_colors(&path, None, &lines))
            .flatten();
        Viewer {
            color_name: None,
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
            load: LoadOptions::view(limit as u64),
            charset,
            charset_manual: false,
            compression: None,
            cut: size > data_len,
            lines_cut: text.cut,
            problem: None,
            table: None,
        }
    }

    pub fn charset(&self) -> Charset {
        self.charset
    }

    /// O: the next codepage, decoded again from the bytes.
    pub fn cycle_charset(&mut self) {
        let next = self.charset.next();
        self.set_charset(next);
        self.charset_manual = true;
        self.note = Some(format!(
            "Codepage: {} ({})",
            next.name(),
            next.description()
        ));
    }

    /// Decodes `data` as `charset` again, keeping the colors' name and
    /// whether they are on. A UTF-16 pick on a file in hex whose text then
    /// looks right switches to text mode (at the top), since that is what
    /// the user asked to see; other picks keep the mode.
    fn set_charset(&mut self, charset: Charset) {
        self.charset = charset;
        let text = charset::decode(&self.data, charset);
        let readable = !looks_binary(text.as_bytes());
        if self.mode == ViewMode::Hex
            && readable
            && matches!(charset, Charset::Utf16Le | Charset::Utf16Be)
        {
            self.mode = ViewMode::Text;
            self.top = 0;
            self.left = 0;
        }
        let text = text_lines(&text, line_budget(self.load.limits.output));
        self.lines = text.lines;
        self.bidi = text.bidi;
        self.lines_cut = text.cut;
        let had_colors = self.colors.is_some();
        self.colors = readable
            .then(|| text_colors(&self.path, self.color_name.as_deref(), &self.lines))
            .flatten();
        self.colors_on = self.colors.is_some() && (self.colors_on || !had_colors);
        self.hit = None;
        if self.mode == ViewMode::Text {
            self.top = self.top.min(self.lines.len().saturating_sub(1));
        }
        self.sub_top = 0;
    }

    /// Colors as if it were the file `name` (`x.diff` for git's output, or
    /// `highlight::GIT_STATUS`).
    pub fn colored_as(mut self, name: &Path) -> Self {
        if self.mode == ViewMode::Text {
            self.colors = Highlighter::for_name(name, &self.lines);
            self.colors_on = self.colors.is_some();
            self.color_name = Some(name.to_path_buf());
        }
        self
    }

    /// How many bidi controls were hidden behind a `·`.
    pub fn bidi_count(&self) -> usize {
        self.bidi.len()
    }

    /// Not all of the file is shown: its bytes, or its text's lines.
    pub fn truncated(&self) -> bool {
        self.cut || self.lines_cut
    }

    /// The bytes were cut: by the read limit, or decompression stopped.
    pub fn bytes_cut(&self) -> bool {
        self.cut
    }

    /// The text has more lines than the line budget let in.
    pub fn lines_cut(&self) -> bool {
        self.lines_cut
    }

    /// The decompression failed or stopped on damaged data (the header
    /// keeps saying so).
    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    pub fn line_count(&self) -> usize {
        match self.mode {
            ViewMode::Text => self.lines.len(),
            ViewMode::Hex => self.data.len().div_ceil(HEX_WIDTH),
            ViewMode::Table => self
                .table
                .as_ref()
                .map(|t| t.visible_row_count())
                .unwrap_or(0),
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
            ViewMode::Table => Vec::new(),
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
                let width = self.content_width();
                let (left, colors, lines, bidi) =
                    (self.left, &mut self.colors, &self.lines, &self.bidi);
                range
                    .map(|i| {
                        // Only what fits: a line can be megabytes long.
                        let shown = visible_chars(&lines[i], left, width);
                        let mark = hit.filter(|h| h.line == i).and_then(|h| {
                            let s = h.start.saturating_sub(left);
                            let e = h.end.saturating_sub(left);
                            (e > s).then_some((s, e))
                        });
                        let per_char = colors
                            .as_mut()
                            .filter(|_| use_colors)
                            .and_then(|h| h.line(i, lines))
                            .map(|segs| colors_in(segs, left..left + shown.len()))
                            .unwrap_or_default();
                        StyledRow {
                            line_num: if self.line_numbers { Some(i + 1) } else { None },
                            segments: segments_of(shown, i, left, &per_char, bidi, mark),
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
                    let line = &self.lines[line_idx];
                    // Only the rows on screen: a line can be megabytes long.
                    let rows: Vec<WrapRow> = WrapRows::new(line, cw)
                        .skip(sub_idx)
                        .take(height - out.len())
                        .collect();
                    if let (Some(first), Some(last)) = (rows.first(), rows.last()) {
                        let shown = first.chars.start..last.chars.end;
                        let per_char = self
                            .colors
                            .as_mut()
                            .filter(|_| use_colors)
                            .and_then(|h| h.line(line_idx, &self.lines))
                            .map(|segs| colors_in(segs, shown.clone()))
                            .unwrap_or_default();
                        for (k, r) in rows.iter().enumerate() {
                            let mark = hit.filter(|h| h.line == line_idx).and_then(|h| {
                                let s = h.start.max(r.chars.start);
                                let e = h.end.min(r.chars.end);
                                (e > s).then(|| (s - r.chars.start, e - r.chars.start))
                            });
                            let colors = per_char
                                .get(r.chars.start - shown.start..)
                                .unwrap_or_default();
                            let chars = line[r.bytes.clone()].chars();
                            out.push(StyledRow {
                                line_num: (self.line_numbers && sub_idx + k == 0)
                                    .then_some(line_idx + 1),
                                segments: segments_of(
                                    chars,
                                    line_idx,
                                    r.chars.start,
                                    colors,
                                    &self.bidi,
                                    mark,
                                ),
                            });
                        }
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
            ViewMode::Table => Vec::new(),
        }
    }

    /// Handles a key; what does not belong to the viewer itself (close, switch
    /// file, search) comes back as an `Action`.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        // Route movement keys to table when in Table mode
        if self.mode == ViewMode::Table
            && let Some(t) = &mut self.table
        {
            match key.code {
                KeyCode::Up
                | KeyCode::Down
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Home
                | KeyCode::End => {
                    t.handle_key(key.code, self.page);
                    return Action::None;
                }
                _ => {}
            }
        }

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
            KeyCode::Char('t' | 'T') => {
                if self.table.is_some() {
                    self.mode = match self.mode {
                        ViewMode::Table => ViewMode::Text,
                        _ => ViewMode::Table,
                    };
                }
                Action::None
            }
            KeyCode::Char('\\') => {
                if self.mode == ViewMode::Table {
                    Action::FilterSql
                } else {
                    Action::Search {
                        forward: false,
                        from: SearchFrom::Page,
                    }
                }
            }
            KeyCode::Char(']') => {
                if let Some(t) = &mut self.table
                    && !t.sheets.is_empty()
                {
                    t.current_sheet = (t.current_sheet + 1) % t.sheets.len();
                    // Reload table for sqlite sheet change if needed
                    let sheet_name = t.sheets[t.current_sheet].clone();
                    if let Ok(new_grid) = tabular_loader::load_sqlite(&self.path, Some(&sheet_name))
                    {
                        *t = new_grid;
                    }
                }
                Action::None
            }
            KeyCode::Char('[') => {
                if let Some(t) = &mut self.table
                    && !t.sheets.is_empty()
                {
                    t.current_sheet = if t.current_sheet == 0 {
                        t.sheets.len() - 1
                    } else {
                        t.current_sheet - 1
                    };
                    let sheet_name = t.sheets[t.current_sheet].clone();
                    if let Ok(new_grid) = tabular_loader::load_sqlite(&self.path, Some(&sheet_name))
                    {
                        *t = new_grid;
                    }
                }
                Action::None
            }
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'h' => {
                    self.toggle_mode();
                    Action::None
                }
                'o' => {
                    self.cycle_charset();
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
            ViewMode::Hex => {
                if self.table.is_some() {
                    ViewMode::Table
                } else {
                    ViewMode::Text
                }
            }
            ViewMode::Table => ViewMode::Text,
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
            ViewMode::Table => None,
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
            self.sub_top = WrapRows::new(&self.lines[hit.line], cw)
                .position(|r| hit.start >= r.chars.start && hit.start < r.chars.end)
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

/// One wrapped row of a line: its characters, and the same as bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct WrapRow {
    pub chars: std::ops::Range<usize>,
    pub bytes: std::ops::Range<usize>,
}

/// The rows a line wraps into at `width` columns: at the last whitespace
/// that fits, or inside a word too long for a row, skipping the spaces
/// after a break. One row at a time, so that a huge line costs only the
/// rows looked at.
pub struct WrapRows<'a> {
    text: &'a str,
    width: usize,
    rest: std::str::CharIndices<'a>,
    /// Read but not in a row yet (byte offset, character): the next row
    /// starts at the first one.
    ahead: std::collections::VecDeque<(usize, char)>,
    /// Character index of the first one in `ahead`.
    start: usize,
    done: bool,
    /// A row came out already (an empty line has one, empty, row).
    any: bool,
}

impl<'a> WrapRows<'a> {
    pub fn new(text: &'a str, width: usize) -> Self {
        WrapRows {
            text,
            width: width.max(1),
            rest: text.char_indices(),
            ahead: std::collections::VecDeque::new(),
            start: 0,
            done: false,
            any: false,
        }
    }

    /// Byte offset of `ahead[k]`, or the end of the text.
    fn byte_at(&self, k: usize) -> usize {
        self.ahead.get(k).map_or(self.text.len(), |&(b, _)| b)
    }

    fn read_one(&mut self) -> bool {
        self.rest.next().map(|c| self.ahead.push_back(c)).is_some()
    }
}

impl Iterator for WrapRows<'_> {
    type Item = WrapRow;

    fn next(&mut self) -> Option<WrapRow> {
        if self.done {
            return None;
        }
        // A row looks at up to `width + 1` characters.
        while self.ahead.len() <= self.width && self.read_one() {}
        let (start, from) = (self.start, self.byte_at(0));
        if self.ahead.len() <= self.width {
            // The rest fits on this row.
            self.done = true;
            if self.ahead.is_empty() && self.any {
                return None;
            }
            let n = self.ahead.len();
            self.ahead.clear();
            self.any = true;
            return Some(WrapRow {
                chars: start..start + n,
                bytes: from..self.text.len(),
            });
        }
        self.any = true;
        let space = self
            .ahead
            .iter()
            .enumerate()
            .rposition(|(k, &(_, c))| k > 0 && c.is_whitespace());
        let row = match space {
            Some(k) => {
                // Breaks at the space, which goes, and so do the spaces after it.
                let row = WrapRow {
                    chars: start..start + k,
                    bytes: from..self.byte_at(k),
                };
                self.ahead.drain(..=k);
                self.start += k + 1;
                while (!self.ahead.is_empty() || self.read_one()) && self.ahead[0].1 == ' ' {
                    self.ahead.pop_front();
                    self.start += 1;
                }
                row
            }
            None => {
                let w = self.width;
                let row = WrapRow {
                    chars: start..start + w,
                    bytes: from..self.byte_at(w),
                };
                self.ahead.drain(..w);
                self.start += w;
                row
            }
        };
        Some(row)
    }
}

/// The character ranges of all the rows `text` wraps into (see `WrapRows`).
#[cfg(test)]
pub fn wrap_line_ranges(text: &str, width: usize) -> Vec<std::ops::Range<usize>> {
    WrapRows::new(text, width).map(|r| r.chars).collect()
}

/// Terminal cells a character takes: 2 for wide ones, 0 for combining marks.
fn char_cells(c: char) -> usize {
    let mut buf = [0; 4];
    ratatui::text::Span::raw(&*c.encode_utf8(&mut buf)).width()
}

/// The characters of `line` from column `left` on that fill `width` cells.
/// A line can be megabytes long, and only this much of it is drawn.
fn visible_chars(line: &str, left: usize, width: usize) -> Vec<char> {
    // Marks of no width ride along, but not without end.
    let most = width.saturating_mul(4).saturating_add(16);
    let mut cells = 0;
    let mut out = Vec::new();
    for c in line.chars().skip(left) {
        let w = char_cells(c);
        if (cells >= width && w > 0) || out.len() >= most {
            break;
        }
        cells += w;
        out.push(c);
    }
    out
}

/// The syntax color of each character in `chars` (character indexes in
/// the line), from the line's colored segments.
fn colors_in(segs: &[(Color, String)], chars: std::ops::Range<usize>) -> Vec<Color> {
    let mut out = Vec::with_capacity(chars.len());
    let mut at = 0;
    for (color, s) in segs {
        if at >= chars.end {
            break;
        }
        let n = s.chars().count();
        let (from, to) = (at.max(chars.start), (at + n).min(chars.end));
        if to > from {
            out.extend(std::iter::repeat_n(*color, to - from));
        }
        at += n;
    }
    out
}

/// The colored pieces of `chars`, the characters of line `line` from
/// column `first` on: `colors[k]` is the syntax color of the k-th one,
/// bidi marks take the error color, and `mark` (columns from `first`) is
/// the search match.
fn segments_of(
    chars: impl IntoIterator<Item = char>,
    line: usize,
    first: usize,
    colors: &[Color],
    bidi: &[(usize, usize)],
    mark: Option<(usize, usize)>,
) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    for (j, ch) in chars.into_iter().enumerate() {
        let color = if bidi.binary_search(&(line, first + j)).is_ok() {
            Some(crate::config::theme().error)
        } else {
            colors.get(j).copied()
        };
        let marked = mark.is_some_and(|(s, e)| j >= s && j < e);
        match segments.last_mut() {
            Some((text, c, m)) if *c == color && *m == marked => text.push(ch),
            _ => segments.push((ch.to_string(), color, marked)),
        }
    }
    segments
}

/// The header's lasting word on a decompression `note`: `failed` when
/// nothing came out (the raw bytes are shown), `cut` when the decoder
/// stopped on the time limit, which the cut notice already tells.
fn problem_text(note: &str, failed: bool, cut: bool) -> Option<String> {
    let truncated = note == decompress::TRUNCATED;
    let reason = if truncated {
        "truncated or corrupt"
    } else {
        note
    };
    if failed {
        Some(format!("not decompressed: {reason}"))
    } else if truncated {
        Some(reason.to_string())
    } else if cut {
        None
    } else {
        Some(format!("incomplete: {reason}"))
    }
}

/// How far `looks_binary` looks.
pub const BINARY_PROBE: usize = 8192;

/// Simple heuristic: there is a null byte near the start of the file.
pub fn looks_binary(data: &[u8]) -> bool {
    data.iter().take(BINARY_PROBE).any(|&b| b == 0)
}

/// The syntax colors of a text: by the name they follow, else by `path`.
fn text_colors(path: &Path, name: Option<&Path>, lines: &[String]) -> Option<Highlighter> {
    match name {
        Some(n) => Highlighter::for_name(n, lines),
        None => Highlighter::for_file(path, lines.first().map_or("", String::as_str)),
    }
}

/// The memory the lines of a text may take, by `line_cost`: twice the
/// read limit. Ordinary text costs about 1.4 × its size (~100-byte lines);
/// tiny lines, tabs and bidi marks cost many times the bytes they come from.
fn line_budget(limit: usize) -> usize {
    limit.saturating_mul(2)
}

/// What a line costs in memory, roughly: the `String` itself, plus its heap
/// block when it has one (allocators hand out at least 32 bytes, in steps
/// of 16).
fn line_cost(capacity: usize) -> usize {
    let heap = match capacity {
        0 => 0,
        n => n.next_multiple_of(16).max(32),
    };
    size_of::<String>() + heap
}

/// What each bidi position costs in `Viewer::bidi`.
const MARK_COST: usize = size_of::<(usize, usize)>();

/// A text split by `text_lines`.
struct TextLines {
    lines: Vec<String>,
    /// Where each bidi control was replaced: (line, character column after
    /// expanding tabs), in order.
    bidi: Vec<(usize, usize)>,
    /// The budget ran out before the end of the text.
    cut: bool,
}

/// Splits into lines, expands tabs and replaces control characters with `·`,
/// noting where the bidi controls were. Stops once the lines would cost
/// more than `budget` bytes: a file of newlines would otherwise take 25
/// times its size (16 MiB of them, a .bz2 of a few hundred bytes, became
/// 400 MiB).
fn text_lines(text: &str, budget: usize) -> TextLines {
    // The final '\n' does not open a new line.
    let body = text.strip_suffix('\n').unwrap_or(text);
    // Room for all the lines the budget allows, so the vector never grows.
    let count = body.bytes().filter(|&b| b == b'\n').count() + 1;
    let mut lines = Vec::with_capacity(count.min(budget / size_of::<String>() + 1));
    let mut bidi = Vec::new();
    let mut used = 0;
    let mut cut = false;
    let mut parts = body.split('\n').peekable();
    while let Some(line) = parts.next() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let i = lines.len();
        let mut out = String::with_capacity(line.len());
        let mut col = 0;
        let mut chars = line.chars();
        let mut full = false;
        for c in chars.by_ref() {
            if c == '\t' {
                let n = TAB_WIDTH - col % TAB_WIDTH;
                out.extend(std::iter::repeat_n(' ', n));
                col += n;
            } else {
                let hidden = crate::bidi::is_bidi(c);
                if hidden {
                    bidi.push((i, col));
                    used += MARK_COST;
                }
                out.push(if c.is_control() || hidden { '·' } else { c });
                col += 1;
            }
            // One long line (of tabs or bidi marks) can be over the budget.
            if used + out.len() > budget {
                full = true;
                break;
            }
        }
        used += line_cost(out.capacity());
        lines.push(out);
        if full || used > budget {
            cut = !chars.as_str().is_empty() || parts.peek().is_some();
            break;
        }
    }
    TextLines { lines, bidi, cut }
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
        fallback: None,
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
    fn truncated_tar_gz_keeps_the_truncated_note() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("a.tar.gz");
        crate::archive::tests::make_tar(&path, true);
        let mut raw = std::fs::read(&path).unwrap();
        raw.truncate(raw.len() / 2);
        let v = Viewer::from_file(path, raw.len() as u64, raw, LOAD);
        assert_eq!(v.note.as_deref(), Some(crate::decompress::TRUNCATED));
    }

    #[test]
    fn input_cut_by_the_read_limit_is_not_corrupt() {
        let mut x: u32 = 12345;
        let noise: Vec<u8> = (0..2 << 20)
            .map(|_| {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                (x >> 24) as u8
            })
            .collect();
        let mut raw = gz(&noise);
        let size = raw.len() as u64;
        raw.truncate(1 << 20);
        let v = Viewer::from_file(
            PathBuf::from("n.bin.gz"),
            size,
            raw,
            LoadOptions::view(1 << 20),
        );
        assert!(v.truncated());
        assert_eq!(v.note, None);
    }

    #[test]
    fn empty_gz_is_an_empty_text() {
        let v = from_gz("e.txt.gz", gz(b""), LOAD);
        assert_eq!(v.mode, ViewMode::Text);
        assert!(v.compression.is_some());
        assert_eq!(v.note, None);
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
    fn header_problem_by_case() {
        let mem = decompress::memory_note(VIEW_DICT);
        let truncated = decompress::TRUNCATED;
        assert_eq!(
            problem_text(truncated, false, false).as_deref(),
            Some("truncated or corrupt")
        );
        assert_eq!(
            problem_text(truncated, true, false).as_deref(),
            Some("not decompressed: truncated or corrupt")
        );
        assert_eq!(
            problem_text(&mem, false, false),
            Some(format!("incomplete: {mem}"))
        );
        // A time cut is already in the cut notice.
        assert_eq!(
            problem_text("decompression stopped after 3 s", false, true),
            None
        );
    }

    /// "(V to view)" only when V's own 64 MiB limit is enough.
    #[test]
    fn autoview_suggests_v_only_when_v_can_do_it() {
        let note = |raw: &[u8]| from_gz("a.zst", raw.to_vec(), LoadOptions::autoview()).note;
        let refused = decompress::memory_note(AUTOVIEW_DICT);
        // zstd frame headers asking for a 32 MiB and a 128 MiB window.
        let v_can = [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x78];
        let v_cannot = [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x88];
        assert_eq!(note(&v_can), Some(format!("{refused} (V to view)")));
        assert_eq!(note(&v_cannot), Some(refused.clone()));
        // xz does not say how much it needs: V may work, so it is suggested.
        let xz = crate::decompress::test_util::xz_header_with_dict_props(40);
        assert_eq!(note(&xz), Some(format!("{refused} (V to view)")));
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
        let lines = |t: &str| text_lines(t, usize::MAX).lines;
        assert_eq!(lines("a\r\nb\n"), ["a", "b"]);
        assert_eq!(lines("a\tb"), ["a       b"]);
        assert_eq!(lines("x\x07y"), ["x·y"]);
        assert_eq!(lines(""), [""]);
        assert_eq!(lines("\n"), [""]);
        assert_eq!(lines("a\n\n"), ["a", ""]);
    }

    /// Each empty line costs a `String` (24 bytes on 64 bits) and no heap
    /// block: 16 MiB of newlines, from a .gz of a few KiB, would be 16 M of
    /// them (400 MiB). The line budget, 2 × the read limit, stops it.
    #[test]
    fn many_tiny_lines_stop_at_the_line_budget() {
        let limit = 1 << 20;
        let budget = 2 * limit;
        let most = budget / size_of::<String>() + 1;
        let mut v = from_gz(
            "nl.txt.gz",
            gz(&vec![b'\n'; 16 << 20]),
            LoadOptions::view(limit as u64),
        );
        assert!(v.truncated());
        assert!(v.line_count() <= most, "{} lines", v.line_count());
        assert!(v.line_count() > most / 2, "{} lines", v.line_count());
        // O decodes again under the same budget.
        v.cycle_charset();
        assert!(v.truncated());
        assert!(v.line_count() <= most, "{} lines", v.line_count());
    }

    #[test]
    fn bidi_marks_and_tabs_count_against_the_line_budget() {
        let limit = 1 << 20;
        let budget = 2 * limit;
        let view = |text: String| {
            let n = text.len() as u64;
            Viewer::from_file("t.txt".into(), n, text.into_bytes(), LoadOptions::view(n))
        };
        // 16 bytes per mark: 349,525 of them would be 5.3 MiB.
        let v = view("\u{202E}".repeat(limit / 3));
        assert!(v.truncated());
        assert!(v.bidi_count() <= budget / 16 + 1, "{}", v.bidi_count());
        // A tab grows to 8 spaces: one line of 1 MiB of tabs would be 8 MiB.
        let v = view("\t".repeat(limit));
        assert!(v.truncated());
        assert!(
            v.lines[0].len() <= budget + TAB_WIDTH,
            "{}",
            v.lines[0].len()
        );
    }

    /// A log of ~100-byte lines costs ~1.4 × its size: it is not cut.
    #[test]
    fn ordinary_lines_fit_the_line_budget() {
        let limit = 1 << 20;
        let line =
            "2026-10-27 12:00:00 INFO an ordinary line of a log file, about a hundred bytes long\n";
        let text = line.repeat(limit / line.len());
        let n = text.len() as u64;
        let v = Viewer::from_file(
            "app.log".into(),
            n,
            text.into_bytes(),
            LoadOptions::view(limit as u64),
        );
        assert!(!v.truncated());
        assert_eq!(v.line_count(), limit / line.len());
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

    /// A frame costs what fits on screen, not the length of the line: an
    /// 8 MiB single line (a minified file) used to cost a pass over all of
    /// it, and a `Vec<char>` of it, on every key. The bound is loose, for
    /// slow CI machines; the old code took seconds here.
    #[test]
    fn a_huge_line_draws_only_what_fits() {
        let line = "{\"k\":1},".repeat(1 << 20);
        let mut v = viewer(line.as_bytes());
        v.width = 120;
        v.page = 40;
        let start = std::time::Instant::now();
        for _ in 0..10 {
            v.styled(40);
        }
        v.wrap = true;
        for _ in 0..10 {
            v.styled(40);
            v.scroll_down(1);
        }
        let took = start.elapsed();
        assert!(took < std::time::Duration::from_millis(500), "{took:?}");
        // And what is drawn is right: a row of the wrapped line, and the
        // piece of it at `left` without wrap.
        let text = |row: &StyledRow| row.iter().map(|(t, ..)| t.as_str()).collect::<String>();
        let rows = v.styled(2);
        assert_eq!(text(&rows[0]), &line[1200..1320]);
        v.wrap = false;
        v.left = 8;
        assert_eq!(text(&v.styled(1)[0]), &line[8..128]);
    }

    /// The lazy rows match the whole-line split, on lines that wrap at
    /// spaces, at other whitespace and inside words.
    #[test]
    fn lazy_wrap_rows_match_the_whole_line() {
        let samples = [
            "hello world this is a test",
            "abcdefghij",
            "",
            "a  b   c    d",
            "word\u{3000}wide space and   runs of     spaces at the end     ",
            "     leading spaces",
            "ação é só um teste com acentos e   espaços",
        ];
        for text in samples {
            for width in 1..=13 {
                let lazy: Vec<_> = WrapRows::new(text, width).collect();
                let chars: Vec<_> = lazy.iter().map(|r| r.chars.clone()).collect();
                assert_eq!(chars, reference_wrap(text, width), "{text:?} at {width}");
                for r in &lazy {
                    let by_bytes: String = text[r.bytes.clone()].to_string();
                    let by_chars: String = text
                        .chars()
                        .skip(r.chars.start)
                        .take(r.chars.len())
                        .collect();
                    assert_eq!(by_bytes, by_chars, "{text:?} at {width}");
                }
            }
        }
    }

    /// The split as it was (one `Vec<char>` of the whole line), to compare.
    #[allow(clippy::single_range_in_vec_init)]
    fn reference_wrap(text: &str, width: usize) -> Vec<std::ops::Range<usize>> {
        let chars: Vec<char> = text.chars().collect();
        let n = chars.len();
        if n == 0 {
            return vec![0..0];
        }
        let mut ranges = Vec::new();
        let mut start = 0;
        while start < n {
            if n - start <= width {
                ranges.push(start..n);
                break;
            }
            let limit = start + width;
            let break_at = chars[start..=limit]
                .iter()
                .enumerate()
                .rposition(|(idx, &c)| idx > 0 && c.is_whitespace());
            if let Some(pos) = break_at {
                ranges.push(start..start + pos);
                let mut next = start + pos + 1;
                while next < n && chars[next] == ' ' {
                    next += 1;
                }
                start = next;
            } else {
                ranges.push(start..limit);
                start = limit;
            }
        }
        ranges
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

    const PT: &str = "ação, coração e informação";

    fn from_raw(raw: Vec<u8>) -> Viewer {
        let n = raw.len() as u64;
        Viewer::from_file(PathBuf::from("a.txt"), n, raw, LOAD)
    }

    #[test]
    fn cp850_file_reads_as_portuguese() {
        use crate::charset::{CP850, Charset, test_util::encode};
        let v = from_raw(encode(PT, &CP850));
        assert_eq!(v.visible_lines(1), [PT]);
        assert_eq!(v.charset(), Charset::Cp850);
    }

    #[test]
    fn o_cycles_and_says_so() {
        use crate::charset::Charset;
        let mut v = from_raw(b"plain".to_vec());
        assert_eq!(v.charset(), Charset::Utf8);
        v.handle_key(KeyEvent::from(KeyCode::Char('o')));
        assert_eq!(v.charset(), Charset::Cp850);
        assert_eq!(
            v.note.as_deref(),
            Some("Codepage: CP850 (DOS, Western Europe)")
        );
        for _ in 0..5 {
            v.handle_key(KeyEvent::from(KeyCode::Char('o')));
        }
        assert_eq!(v.charset(), Charset::Utf8);
    }

    #[test]
    fn manual_choice_survives_reload() {
        use crate::charset::Charset;
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("f.txt");
        std::fs::write(&path, "hello").unwrap();
        let mut v = Viewer::open(path, LoadOptions::view(MAX_BYTES)).unwrap();
        v.cycle_charset(); // CP850
        v.cycle_charset(); // 1252
        v.cycle_charset(); // CP437
        assert_eq!(v.charset(), Charset::Cp437);
        v.reload().unwrap();
        assert_eq!(v.charset(), Charset::Cp437);
    }

    #[test]
    fn search_finds_accented_text_in_cp850() {
        use crate::charset::{CP850, test_util::encode};
        let mut v = from_raw(encode(&format!("one\n{PT}\n"), &CP850));
        assert!(v.search(&pat("ação"), true, SearchFrom::Edge));
        assert_eq!(v.hit.map(|h| h.line), Some(1));
    }

    #[test]
    fn reg_export_first_line_is_clean() {
        use crate::charset::Charset;
        let reg = "\u{FEFF}Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER]\r\n";
        let bytes: Vec<u8> = reg.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let v = from_raw(bytes);
        assert_eq!(v.charset(), Charset::Utf16Le);
        assert_eq!(v.mode, ViewMode::Text);
        assert_eq!(v.visible_lines(1), ["Windows Registry Editor Version 5.00"]);
    }

    #[test]
    fn cycling_to_utf16_on_binary_looking_data_shows_text() {
        let bytes: Vec<u8> = "hello there"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        // Xylem's own bytes are taken as UTF-8, so this one looks binary.
        let mut v = Viewer::from_bytes(PathBuf::from("a"), bytes.len() as u64, bytes);
        assert_eq!(v.mode, ViewMode::Hex);
        for _ in 0..3 {
            v.cycle_charset(); // CP850, 1252, CP437
            assert_eq!(v.mode, ViewMode::Hex);
        }
        v.cycle_charset(); // UTF-16LE
        assert_eq!(v.mode, ViewMode::Text);
        assert_eq!(v.visible_lines(1), ["hello there"]);
    }

    #[test]
    fn o_keeps_the_color_overrides() {
        let mut v = Viewer::from_bytes(PathBuf::from("git diff foo.rs"), 8, b"+a\n-b\n".to_vec())
            .colored_as(Path::new("x.diff"));
        let before = v.language().map(String::from);
        assert!(before.is_some());
        v.cycle_charset();
        assert_eq!(v.language().map(String::from), before);

        let raw = gz(b"fn main() {}\n");
        let mut v = from_gz("x.rs.gz", raw, LOAD);
        assert_eq!(v.language(), Some("Rust"));
        v.cycle_charset();
        assert_eq!(v.language(), Some("Rust"));
        v.handle_key(KeyEvent::from(KeyCode::Char('c')));
        assert_eq!(v.language(), None);
        v.cycle_charset();
        assert_eq!(v.language(), None);
    }

    #[test]
    fn bidi_survives_a_redecode() {
        let bytes: Vec<u8> = "\u{FEFF}a\u{202E}b\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let mut v = from_raw(bytes);
        assert_eq!(v.bidi_count(), 1);
        for _ in 0..6 {
            v.cycle_charset();
        }
        assert_eq!(v.charset(), crate::charset::Charset::Utf16Le);
        assert_eq!(v.bidi_count(), 1);
    }
}
