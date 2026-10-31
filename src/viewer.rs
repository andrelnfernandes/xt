//! File viewer (`V` key; Ctrl+V for the tagged files), in text or
//! hexadecimal mode, with search.

pub mod batch;
pub mod line_filter;
pub mod markdown;
pub mod records;
pub mod sql_filter;
pub mod table;
pub mod tabular_loader;

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui::style::{Color, Modifier};

use crate::charset::{self, Charset};
use crate::decompress::{
    self, AUTOVIEW_DICT, AUTOVIEW_TIME, Compression, Limits, VIEW_DICT, VIEW_TIME,
};
use crate::highlight::Highlighter;
use crate::search::Pattern;
use line_filter::{LineFilterSpec, LineMatcher};
use records::Budget;

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
    /// A Markdown file as a document (see `markdown`).
    Rendered,
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
    /// Prompts for the line filter in text mode.
    FilterLines,
    /// Ctrl+B: a line of output per record of what is shown.
    Batch,
}

/// How a file is read: how much of it, and whether a compressed file is
/// shown decompressed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoadOptions {
    pub decompress: bool,
    pub limits: Limits,
    /// Fixed charset for files that are not UTF-8 or UTF-16 (`None`: guess).
    pub fallback: Option<Charset>,
    /// May hand the file to a parser written in C (see
    /// [`tabular_loader::uses_c_parser`]): only when the user asks to see it.
    pub c_parsers: bool,
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
            c_parsers: true,
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
            c_parsers: false,
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
    /// Text mode shows only the lines this filter kept. `lines` stays whole:
    /// the margin keeps each line's number, and the colors their context.
    filter: Option<LineFilter>,
    /// The document of a Markdown file, unless it is too big for that.
    md: Option<markdown::Doc>,
    /// The file's name says Markdown: the document follows a new encoding.
    md_file: bool,
}

/// The line filter in effect and the lines it kept, in order.
struct LineFilter {
    matcher: LineMatcher,
    rows: Vec<usize>,
}

/// A piece of a displayed line: text, its color and whether it is part of
/// the search match, and its modifiers (bold, italic...).
pub type Segment = (String, Option<Color>, bool, Modifier);

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
        let mut v = Viewer {
            on_disk: true,
            ..Self::from_file(path, size, raw, load)
        };
        v.load_table();
        Ok(v)
    }

    /// Shows a tabular file as a table. Only for a file on disk: the loaders
    /// open it by its path, and a remote file or an archive entry has none
    /// here (the same path could even be another file on this machine).
    fn load_table(&mut self) {
        let path = self.path.clone();
        if !tabular_loader::is_tabular_candidate(&path)
            || self.compression.is_some()
            || self.problem.is_some()
        {
            return;
        }
        // Autoview runs on whatever the cursor passes over, so it never
        // reaches a parser written in C.
        if !self.load.c_parsers && tabular_loader::uses_c_parser(&path) {
            if self.note.is_none() {
                self.note = Some("Parquet or SQLite: V opens the table".into());
            }
            return;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        let budget = self.table_budget();
        let grid = crate::panic_guard::catch(|| match ext.as_str() {
            "csv" | "tsv" | "tab" => tabular_loader::load_csv(&path, None, budget),
            "jsonl" | "ndjson" => tabular_loader::load_jsonl(&path, budget),
            "xlsx" | "ods" | "xls" | "xlsb" => tabular_loader::load_spreadsheet(&path, 0, budget),
            "dbf" => tabular_loader::load_dbf(&path, budget),
            "sqlite" | "sqlite3" | "db" => tabular_loader::load_sqlite(&path, None, budget),
            "parquet" => tabular_loader::load_parquet(&path, budget),
            _ => Err("Not supported".to_string()),
        })
        .unwrap_or_else(|| Err("the file could not be read".into()));
        match grid {
            Ok(grid) => {
                // Autoview's title has only the note to say it.
                if self.note.is_none() {
                    self.note = grid.cut.clone();
                }
                self.table = Some(grid);
                self.mode = ViewMode::Table;
            }
            Err(e) if self.note.is_none() => self.note = Some(format!("Table mode: {e}")),
            Err(_) => {}
        }
    }

    /// A table follows the same limits as the rest of the viewer.
    fn table_budget(&self) -> Budget {
        Budget::new(self.load.limits.time, self.load.limits.output as u64)
    }

    /// What a Batch reads to give what is on screen, whatever the screen's limits.
    pub fn batch_source(&self) -> Result<batch::Source, String> {
        match self.mode {
            ViewMode::Table => {
                let Some(t) = &self.table else {
                    return Err("No table to read".into());
                };
                Ok(batch::Source::Table {
                    path: self.path.clone(),
                    sheet: t.current_sheet,
                    table: t.sheets.get(t.current_sheet).cloned(),
                    filter: t.filter_expr.clone(),
                    unpacked: self.load.limits.output as u64,
                })
            }
            ViewMode::Text => Ok(batch::Source::Text {
                text: charset::decode(&self.data, self.charset),
                total: self.lines.len(),
                kept: self.filter.as_ref().map(|f| f.rows.clone()),
                regex: self
                    .filter
                    .as_ref()
                    .and_then(|f| f.matcher.regex().cloned()),
                cut: self.truncated(),
            }),
            // The source's lines; the line filter is for the text mode.
            ViewMode::Rendered => Ok(batch::Source::Text {
                text: charset::decode(&self.data, self.charset),
                total: self.lines.len(),
                kept: None,
                regex: None,
                cut: self.truncated(),
            }),
            ViewMode::Hex => Err("Batch works on text and tables (H)".into()),
        }
    }

    /// `[` and `]`: another sheet of a spreadsheet or table of a database.
    /// Stays where it is, saying why, if that one cannot be read.
    fn switch_sheet(&mut self, forward: bool) {
        let Some(t) = &self.table else { return };
        let count = t.sheets.len();
        if count < 2 {
            return;
        }
        let target = if forward {
            (t.current_sheet + 1) % count
        } else {
            (t.current_sheet + count - 1) % count
        };
        let budget = self.table_budget();
        let grid = if tabular_loader::uses_c_parser(&self.path) {
            tabular_loader::load_sqlite(&self.path, Some(&t.sheets[target]), budget)
        } else {
            tabular_loader::load_spreadsheet(&self.path, target, budget)
        };
        match grid {
            Ok(grid) => self.table = Some(grid),
            Err(e) => self.note = Some(format!("Table mode: {e}")),
        }
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
        let shown_name = name.as_deref().unwrap_or(&path);
        v.md_file = !failed && !found.binary && markdown::is_markdown(shown_name);
        if let Some(name) = &name {
            v = v.colored_as(name);
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
        if v.render_markdown() {
            v.mode = ViewMode::Rendered;
        }
        v
    }

    /// Reads the text as a document, if it is Markdown and within the limits.
    /// Anything else leaves the source, and the note says why.
    fn render_markdown(&mut self) -> bool {
        self.md = None;
        if !self.md_file {
            return false;
        }
        if self.data.len() > markdown::MAX_INPUT {
            if self.note.is_none() {
                self.note = Some(format!(
                    "Too large to render (over {} MiB): showing the source",
                    markdown::MAX_INPUT >> 20
                ));
            }
            return false;
        }
        let text = charset::decode(&self.data, self.charset);
        let limits = self.load.limits;
        let budget = limits.output.min(markdown::MAX_INPUT).saturating_mul(16);
        let width = self.content_width();
        let doc = crate::panic_guard::catch(|| {
            let mut doc = markdown::parse(&text, limits.time, budget);
            doc.layout(width);
            doc
        });
        self.md = doc.filter(|d| !d.is_empty_cut());
        if self.md.is_none() && self.note.is_none() {
            self.note = Some("Could not render this file: showing the source".into());
        }
        self.md.is_some()
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
        self.mode = match old.mode {
            ViewMode::Rendered if self.md.is_none() => ViewMode::Text,
            mode => mode,
        };
        self.page = old.page;
        self.width = old.width;
        self.line_numbers = old.line_numbers;
        let width = self.content_width();
        if let Some(md) = &mut self.md {
            md.layout(width);
        }
        self.filter = old.filter.map(|f| LineFilter {
            matcher: f.matcher,
            rows: Vec::new(),
        });
        self.refilter();
        self.top = old.top.min(self.line_count().saturating_sub(1));
        self.left = old.left;
        self.colors_on = old.colors_on && self.colors.is_some();
        self.wrap = old.wrap;
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
                    self.top = self.text_rows().saturating_sub(1);
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
                if self.top >= self.text_rows() {
                    break;
                }
                // Only as far as the next row: the line can be huge.
                let line = &self.lines[self.line_index(self.top)];
                let next_row = WrapRows::new(line, cw).nth(self.sub_top + 1);
                if next_row.is_some() {
                    self.sub_top += 1;
                } else if self.top + 1 < self.text_rows() {
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
                    let rows = WrapRows::new(&self.lines[self.line_index(self.top)], cw).count();
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
        if self.mode == ViewMode::Rendered {
            self.note = Some("Rendered text always fits the window".into());
            return;
        }
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
            self.top = self
                .row_of_line(line - 1)
                .min(self.text_rows().saturating_sub(1));
            self.sub_top = 0;
            self.left = 0;
            let shown = self.top_line() + 1;
            self.note = Some(if shown == line {
                format!("Line {line} of {max_line}")
            } else {
                format!("Line {line} is filtered out: showing line {shown}")
            });
        } else if self.mode == ViewMode::Rendered {
            let max_line = self.lines.len().max(1);
            let line = target_1_indexed.clamp(1, max_line);
            self.top = self.md.as_ref().map_or(0, |d| d.row_of_source(line - 1));
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
        if self.line_numbers && matches!(self.mode, ViewMode::Text | ViewMode::Rendered) {
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
            filter: None,
            md: None,
            md_file: false,
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
        self.refilter();
        let had_colors = self.colors.is_some();
        self.colors = readable
            .then(|| text_colors(&self.path, self.color_name.as_deref(), &self.lines))
            .flatten();
        self.colors_on = self.colors.is_some() && (self.colors_on || !had_colors);
        self.hit = None;
        if !self.render_markdown() && self.mode == ViewMode::Rendered {
            self.mode = ViewMode::Text;
        }
        if self.mode == ViewMode::Text {
            self.top = self.top.min(self.text_rows().saturating_sub(1));
        } else if let Some(md) = self.md.as_ref().filter(|_| self.mode == ViewMode::Rendered) {
            self.top = self.top.min(md.row_count().saturating_sub(1));
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
        match &self.md {
            Some(md) if self.mode == ViewMode::Rendered => md.bidi(),
            _ => self.bidi.len(),
        }
    }

    /// The file has a rendered mode.
    pub fn can_render(&self) -> bool {
        self.md.is_some()
    }

    /// Not all of the document is rendered.
    pub fn rendered_cut(&self) -> bool {
        self.mode == ViewMode::Rendered && self.md.as_ref().is_some_and(markdown::Doc::cut)
    }

    /// The source line at the top of the rendered mode's window.
    pub fn rendered_line(&self) -> usize {
        self.md.as_ref().map_or(0, |d| d.source_of(self.top))
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
            ViewMode::Text => self.text_rows(),
            ViewMode::Hex => self.data.len().div_ceil(HEX_WIDTH),
            ViewMode::Table => self
                .table
                .as_ref()
                .map(|t| t.visible_row_count())
                .unwrap_or(0),
            ViewMode::Rendered => self.md.as_ref().map_or(0, markdown::Doc::row_count),
        }
    }

    /// Lines of text read, filtered or not.
    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    /// Rows of text mode: the lines the filter kept, or all of them. `top`
    /// and the scrolling count these.
    fn text_rows(&self) -> usize {
        self.filter
            .as_ref()
            .map_or(self.lines.len(), |f| f.rows.len())
    }

    /// The line (index in `lines`) shown at a row of text mode.
    fn line_index(&self, row: usize) -> usize {
        self.filter.as_ref().map_or(row, |f| f.rows[row])
    }

    /// The row showing `line`, or else the first one after it.
    fn row_of_line(&self, line: usize) -> usize {
        self.filter
            .as_ref()
            .map_or(line, |f| f.rows.partition_point(|&l| l < line))
    }

    /// The line at the top of the text mode's window.
    pub fn top_line(&self) -> usize {
        if self.top < self.text_rows() {
            self.line_index(self.top)
        } else {
            0
        }
    }

    /// The filter in effect, if any.
    pub fn line_filter(&self) -> Option<&LineFilterSpec> {
        self.filter.as_ref().map(|f| f.matcher.spec())
    }

    /// Runs the filter again, after `lines` changed.
    fn refilter(&mut self) {
        if let Some(f) = &mut self.filter {
            let matcher = &f.matcher;
            f.rows = (0..self.lines.len())
                .filter(|&i| matcher.matches(&self.lines[i]))
                .collect();
        }
    }

    /// Applies the prompt's filter (empty text turns it off) and says what
    /// happened. A filter that keeps nothing or is invalid changes nothing.
    /// The line at the top stays there, or the next one that is left.
    pub fn apply_line_filter(&mut self, spec: &LineFilterSpec) -> String {
        let total = self.lines.len();
        let anchor = self.top_line();
        let (filter, note) = if spec.text.is_empty() {
            (None, format!("Filter off: {total} lines"))
        } else {
            let matcher = match LineMatcher::new(spec) {
                Ok(m) => m,
                Err(e) => return e,
            };
            let rows: Vec<usize> = (0..total)
                .filter(|&i| matcher.matches(&self.lines[i]))
                .collect();
            if rows.is_empty() {
                return format!("No lines match \"{}\": filter not changed", spec.text);
            }
            let note = format!("Filter: {} of {total} lines", rows.len());
            (Some(LineFilter { matcher, rows }), note)
        };
        self.filter = filter;
        self.hit = self
            .hit
            .filter(|h| h.mode != ViewMode::Text || self.line_kept(h.line));
        self.top = self.row_of_line(anchor);
        if self.wrap {
            self.top = self.top.min(self.text_rows().saturating_sub(1));
        } else {
            self.top = self
                .top
                .min(self.text_rows().saturating_sub(self.page.max(1)));
        }
        self.sub_top = 0;
        note
    }

    /// `line` is on screen under the current filter.
    fn line_kept(&self, line: usize) -> bool {
        self.filter
            .as_ref()
            .is_none_or(|f| f.rows.binary_search(&line).is_ok())
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
                let range = self.top..(self.top + height).min(self.text_rows());
                range
                    .map(|row| {
                        let i = self.line_index(row);
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
                let mut row = self.top;
                let mut sub_idx = self.sub_top;
                while out.len() < height && row < self.text_rows() {
                    let line_idx = self.line_index(row);
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
                    row += 1;
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
            ViewMode::Table | ViewMode::Rendered => Vec::new(),
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
                let range = self.top..(self.top + height).min(self.text_rows());
                let width = self.content_width();
                let kept = self.filter.as_ref().map(|f| f.rows.as_slice());
                let (left, colors, lines, bidi) =
                    (self.left, &mut self.colors, &self.lines, &self.bidi);
                range
                    .map(|row| {
                        let i = kept.map_or(row, |rows| rows[row]);
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
                let mut row = self.top;
                let mut sub_idx = self.sub_top;
                let total = self.text_rows();
                while out.len() < height && row < total {
                    let line_idx = self.line_index(row);
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
                    row += 1;
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
                            segments: vec![(line, None, marked, Modifier::empty())],
                        }
                    })
                    .collect()
            }
            ViewMode::Table => Vec::new(),
            ViewMode::Rendered => self.rendered_rows(height),
        }
    }

    /// The rendered mode's rows on screen, laid out for the current width.
    /// The text at the top stays there when the width changes.
    fn rendered_rows(&mut self, height: usize) -> Vec<StyledRow> {
        let width = self.content_width();
        let (top, left, colors) = (self.top, self.left, self.colors_on);
        let numbers = self.line_numbers;
        let hit = self
            .hit
            .filter(|h| h.mode == ViewMode::Rendered)
            .map(|h| (h.line, h.start, h.end));
        let Some(md) = &mut self.md else {
            return Vec::new();
        };
        let mut top = top;
        if md.width() != width {
            // At the very top it stays there: above the first text there can
            // be a label or a rule.
            let anchor = md.anchor(top).filter(|_| top > 0);
            md.layout(width);
            if let Some((line, at)) = anchor {
                top = md.row_of(line, at);
            }
        }
        let end = (top + height).min(md.row_count());
        let rows = (top..end)
            .map(|row| StyledRow {
                line_num: md.number_of(row).filter(|_| numbers),
                segments: md.row_segments(row, left, colors, hit),
            })
            .collect();
        self.top = top;
        rows
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
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let action = match key.code {
            KeyCode::Char('b' | 'B') if ctrl => Action::Batch,
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
                    self.top = self.text_rows().saturating_sub(1);
                    self.sub_top = 0;
                } else {
                    self.top = max_top;
                }
                Action::None
            }
            KeyCode::Left if self.mode == ViewMode::Rendered || self.pans() => {
                self.left = self.left.saturating_sub(PAN_STEP);
                Action::None
            }
            KeyCode::Right if self.mode == ViewMode::Rendered || self.pans() => {
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
                } else if self.md.is_some() {
                    match self.mode {
                        ViewMode::Rendered => self.rendered_to_text(),
                        ViewMode::Text => self.text_to_rendered(),
                        _ => {
                            self.mode = ViewMode::Rendered;
                            self.top = 0;
                            self.left = 0;
                        }
                    }
                }
                Action::None
            }
            KeyCode::Char('&') => match self.mode {
                ViewMode::Text => Action::FilterLines,
                // Lines exist in the source: the filter opens over it.
                ViewMode::Rendered => {
                    self.rendered_to_text();
                    Action::FilterLines
                }
                ViewMode::Table => Action::FilterSql,
                ViewMode::Hex => {
                    self.note = Some("The line filter works in text mode (H)".into());
                    Action::None
                }
            },
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
                self.switch_sheet(true);
                Action::None
            }
            KeyCode::Char('[') => {
                self.switch_sheet(false);
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
                            let what = if self.mode == ViewMode::Rendered {
                                "Code"
                            } else {
                                h.language.as_str()
                            };
                            format!("{what} colors {state}")
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
            self.top = self.top.min(self.text_rows().saturating_sub(1));
        } else {
            // The mode may have just changed.
            self.top = self.top.min(self.line_count().saturating_sub(page));
        }
        action
    }

    /// The arrows scroll sideways in text mode without wrap.
    fn pans(&self) -> bool {
        self.mode == ViewMode::Text && !self.wrap
    }

    /// To the source, at the line the document was showing.
    fn rendered_to_text(&mut self) {
        let line = self.rendered_line();
        self.mode = ViewMode::Text;
        self.top = self.row_of_line(line);
        self.sub_top = 0;
        self.left = 0;
    }

    /// To the document, at the line the source was showing.
    fn text_to_rendered(&mut self) {
        let line = self.top_line();
        self.mode = ViewMode::Rendered;
        self.top = self.md.as_ref().map_or(0, |d| d.row_of_source(line));
        self.sub_top = 0;
        self.left = 0;
    }

    fn toggle_mode(&mut self) {
        match self.mode {
            ViewMode::Rendered => return self.rendered_to_text(),
            ViewMode::Text | ViewMode::Hex | ViewMode::Table => {}
        }
        self.mode = match self.mode {
            ViewMode::Hex if self.md.is_some() => ViewMode::Rendered,
            ViewMode::Text => ViewMode::Hex,
            ViewMode::Hex => {
                if self.table.is_some() {
                    ViewMode::Table
                } else {
                    ViewMode::Text
                }
            }
            ViewMode::Table | ViewMode::Rendered => ViewMode::Text,
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
        // The rendered mode searches its logical lines, not the rows.
        let pos = match (&self.md, self.mode, from) {
            (Some(md), ViewMode::Rendered, SearchFrom::Page) => {
                let at = if forward {
                    md.anchor(pos.0)
                } else {
                    md.anchor_back(pos.0)
                };
                at.unwrap_or(pos)
            }
            _ => pos,
        };
        self.find_from(pattern, forward, pos)
    }

    /// Next match after the last one found (or the page).
    pub fn search_next(&mut self, pattern: &Pattern, forward: bool) -> bool {
        self.forward = forward;
        let row = |h: Hit| match h.mode {
            ViewMode::Text => self.row_of_line(h.line),
            _ => h.line,
        };
        let pos = match self.hit.filter(|h| h.mode == self.mode) {
            Some(h) if forward => (row(h), h.start + 1),
            Some(h) => (row(h), h.start),
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
            ViewMode::Rendered => self.find_rendered(pattern, forward, line, col),
        };
        match hit {
            Some(h) => {
                self.show(h);
                true
            }
            None => false,
        }
    }

    /// Searches the document's logical lines; `line` and `col` are in them.
    fn find_rendered(
        &self,
        pattern: &Pattern,
        forward: bool,
        line: usize,
        col: usize,
    ) -> Option<Hit> {
        let md = self.md.as_ref()?;
        let n = md.line_count();
        if n == 0 {
            return None;
        }
        let at = |i: usize, (s, e): (usize, usize)| Hit {
            mode: ViewMode::Rendered,
            line: i,
            start: s,
            end: e,
        };
        if forward {
            if line >= n {
                return None;
            }
            if let Some(r) = pattern.find_in_line(md.line(line), col) {
                return Some(at(line, r));
            }
            (line + 1..n).find_map(|i| pattern.find_in_line(md.line(i), 0).map(|r| at(i, r)))
        } else {
            let line = line.min(n - 1);
            if let Some(r) = pattern.rfind_in_line(md.line(line), col) {
                return Some(at(line, r));
            }
            (0..line).rev().find_map(|i| {
                pattern
                    .rfind_in_line(md.line(i), usize::MAX)
                    .map(|r| at(i, r))
            })
        }
    }

    /// `row` counts the rows of text mode: a search sees only what the
    /// filter kept.
    fn find_text(&self, pattern: &Pattern, forward: bool, row: usize, col: usize) -> Option<Hit> {
        let n = self.text_rows();
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
            if row >= n {
                return None;
            }
            let line = self.line_index(row);
            if let Some(r) = pattern.find_in_line(&self.lines[line], col) {
                return Some(at(line, r));
            }
            (row + 1..n).find_map(|i| {
                let line = self.line_index(i);
                pattern
                    .find_in_line(&self.lines[line], 0)
                    .map(|r| at(line, r))
            })
        } else {
            let row = row.min(n - 1);
            let line = self.line_index(row);
            if let Some(r) = pattern.rfind_in_line(&self.lines[line], col) {
                return Some(at(line, r));
            }
            (0..row).rev().find_map(|i| {
                let line = self.line_index(i);
                pattern
                    .rfind_in_line(&self.lines[line], usize::MAX)
                    .map(|r| at(line, r))
            })
        }
    }

    /// Scrolls to the match, leaving a little context above.
    fn show(&mut self, hit: Hit) {
        self.hit = Some(hit);
        let page = self.page.max(1);
        if hit.mode == ViewMode::Rendered {
            let Some(md) = &self.md else { return };
            let row = md.row_of(hit.line, hit.start);
            if row < self.top || row >= self.top + page {
                self.top = row.saturating_sub(page / 3);
            }
            self.top = self.top.min(md.row_count().saturating_sub(page));
            // A column of a table too wide for the window.
            let col = md.column_of(row, hit.line, hit.start);
            let width = self.content_width().max(10);
            if col < self.left || col >= self.left + width {
                self.left = col.saturating_sub(width / 4);
            }
            return;
        }
        let row = match hit.mode {
            ViewMode::Text => self.row_of_line(hit.line),
            _ => hit.line,
        };
        if self.mode == ViewMode::Text && self.wrap {
            if row < self.top || row >= self.top + page {
                self.top = row.saturating_sub(page / 3);
            }
            let cw = self.content_width();
            self.sub_top = WrapRows::new(&self.lines[hit.line], cw)
                .position(|r| hit.start >= r.chars.start && hit.start < r.chars.end)
                .unwrap_or(0);
            self.left = 0;
        } else {
            if row < self.top || row >= self.top + page {
                self.top = row.saturating_sub(page / 3);
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
            Some((text, c, m, _)) if *c == color && *m == marked => text.push(ch),
            _ => segments.push((ch.to_string(), color, marked, Modifier::empty())),
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
        c_parsers: true,
    };

    fn from_gz(name: &str, raw: Vec<u8>, load: LoadOptions) -> Viewer {
        let n = raw.len() as u64;
        Viewer::from_file(PathBuf::from(name), n, raw, load)
    }

    fn sqlite_file(dir: &Path) -> PathBuf {
        let path = dir.join("data.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE t (a); INSERT INTO t VALUES (1);")
            .unwrap();
        path
    }

    #[test]
    fn autoview_never_reaches_the_parsers_in_c() {
        let tmp = crate::testing::tempdir();
        let path = sqlite_file(tmp.path());
        let v = Viewer::open(path.clone(), LoadOptions::autoview()).unwrap();
        assert!(v.table.is_none());
        assert_eq!(v.mode, ViewMode::Hex);
        assert_eq!(
            v.note.as_deref(),
            Some("Parquet or SQLite: V opens the table")
        );
        let v = Viewer::open(path, LoadOptions::view(1 << 20)).unwrap();
        assert!(v.table.is_some(), "V opens it");
        assert_eq!(v.mode, ViewMode::Table);
    }

    #[test]
    fn a_table_never_comes_from_a_local_file_with_the_path_of_a_remote_one() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("data.csv");
        std::fs::write(&path, "local,secret\n1,2\n").unwrap();
        // What the remote or the archive gave for that path.
        let data = b"remote,text\n".to_vec();
        let v = Viewer::from_file(path, data.len() as u64, data, LOAD);
        assert!(v.table.is_none());
        assert_eq!(v.mode, ViewMode::Text);
        assert_eq!(v.visible_lines(5), ["remote,text"]);
    }

    #[test]
    fn a_compressed_file_is_not_a_table() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("data.csv");
        std::fs::write(&path, gz(b"a,b\n1,2\n")).unwrap();
        let v = Viewer::open(path, LOAD).unwrap();
        assert!(v.table.is_none());
        assert_eq!(v.visible_lines(5), ["a,b", "1,2"]);
    }

    #[test]
    fn a_table_that_cannot_be_read_says_why() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("bad.xlsx");
        std::fs::write(&path, "not a spreadsheet").unwrap();
        let v = Viewer::open(path, LOAD).unwrap();
        assert!(v.table.is_none());
        assert!(v.note.as_deref().unwrap().starts_with("Table mode: "));
    }

    /// A spreadsheet with a sheet named after each string, with that string
    /// in its first cell.
    fn xlsx_file(path: &Path, sheets: &[&str]) {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        let mut add = |name: &str, body: String| {
            zip.start_file(name, options).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        };
        let ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
        let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
        let pkg = "http://schemas.openxmlformats.org/package/2006";
        let main = "application/vnd.openxmlformats-officedocument.spreadsheetml";
        let mut types = format!(
            r#"<Types xmlns="{pkg}/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/xl/workbook.xml" ContentType="{main}.sheet.main+xml"/>"#
        );
        let (mut list, mut rels) = (String::new(), String::new());
        for (i, name) in sheets.iter().enumerate() {
            let n = i + 1;
            types += &format!(
                r#"<Override PartName="/xl/worksheets/sheet{n}.xml" ContentType="{main}.worksheet+xml"/>"#
            );
            list += &format!(r#"<sheet name="{name}" sheetId="{n}" r:id="rId{n}"/>"#);
            rels += &format!(
                r#"<Relationship Id="rId{n}" Type="{rel}/worksheet" Target="worksheets/sheet{n}.xml"/>"#
            );
        }
        add("[Content_Types].xml", types + "</Types>");
        add(
            "_rels/.rels",
            format!(
                r#"<Relationships xmlns="{pkg}/relationships"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
            ),
        );
        add(
            "xl/workbook.xml",
            format!(r#"<workbook xmlns="{ns}" xmlns:r="{rel}"><sheets>{list}</sheets></workbook>"#),
        );
        add(
            "xl/_rels/workbook.xml.rels",
            format!(r#"<Relationships xmlns="{pkg}/relationships">{rels}</Relationships>"#),
        );
        for (i, name) in sheets.iter().enumerate() {
            add(
                &format!("xl/worksheets/sheet{}.xml", i + 1),
                format!(
                    r#"<worksheet xmlns="{ns}"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>{name}</t></is></c></row></sheetData></worksheet>"#
                ),
            );
        }
        zip.finish().unwrap();
    }

    #[test]
    fn brackets_switch_the_sheets_of_a_spreadsheet() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("book.xlsx");
        xlsx_file(&path, &["one", "two", "three"]);
        let mut v = Viewer::open(path, LOAD).unwrap();
        let state = |v: &Viewer| {
            let t = v.table.as_ref().unwrap();
            (t.current_sheet, t.active_cell().unwrap().to_string())
        };
        assert_eq!(state(&v), (0, "one".to_string()));
        press(&mut v, KeyCode::Char(']'));
        assert_eq!(state(&v), (1, "two".to_string()));
        press(&mut v, KeyCode::Char('['));
        press(&mut v, KeyCode::Char('['));
        assert_eq!(state(&v), (2, "three".to_string()), "wraps around");
        press(&mut v, KeyCode::Char(']'));
        assert_eq!(state(&v), (0, "one".to_string()));
    }

    #[test]
    fn brackets_switch_the_tables_of_a_database() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("data.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE a (x); INSERT INTO a VALUES ('in a');
             CREATE TABLE b (x); INSERT INTO b VALUES ('in b');",
        )
        .unwrap();
        drop(conn);
        let mut v = Viewer::open(path, LOAD).unwrap();
        let cell = |v: &Viewer| v.table.as_ref().unwrap().active_cell().unwrap().to_string();
        assert_eq!(cell(&v), "in a");
        press(&mut v, KeyCode::Char(']'));
        assert_eq!(cell(&v), "in b");
        assert_eq!(v.table.as_ref().unwrap().current_sheet, 1);
    }

    #[test]
    fn a_sheet_that_cannot_be_read_keeps_the_one_on_screen() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("book.xlsx");
        xlsx_file(&path, &["one", "two"]);
        let mut v = Viewer::open(path.clone(), LOAD).unwrap();
        std::fs::write(&path, "gone").unwrap();
        press(&mut v, KeyCode::Char(']'));
        let t = v.table.as_ref().unwrap();
        assert_eq!(t.current_sheet, 0);
        assert_eq!(t.active_cell(), Some("one"));
        assert!(v.note.as_deref().unwrap().starts_with("Table mode: "));
    }

    #[test]
    fn a_cut_table_is_told_in_the_note() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("big.csv");
        let mut body = String::from("n,w\n");
        for i in 0..tabular_loader::MAX_TABLE_ROWS + 1 {
            body.push_str(&format!("{i},x\n"));
        }
        std::fs::write(&path, body).unwrap();
        let v = Viewer::open(path, LOAD).unwrap();
        assert_eq!(v.note.as_deref(), Some("showing first 50,000 rows"));
    }

    #[test]
    fn autoview_does_not_read_a_spreadsheet_over_its_limit() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("big.xlsx");
        {
            use std::io::Write;
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            zip.start_file("xl/big.xml", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&vec![b' '; 2 << 20]).unwrap();
            zip.finish().unwrap();
        }
        let v = Viewer::open(path, LoadOptions::autoview()).unwrap();
        assert!(v.table.is_none());
        assert!(
            v.note
                .as_deref()
                .unwrap()
                .contains("over the limit of 1 MiB")
        );
    }

    #[test]
    fn autoview_still_shows_tables_read_in_rust() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("data.csv");
        std::fs::write(&path, "a,b\n1,2\n").unwrap();
        let v = Viewer::open(path, LoadOptions::autoview()).unwrap();
        assert!(v.table.is_some());
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
        assert!(
            rows[1]
                .iter()
                .any(|(t, _, marked, _)| *marked && t == "senha")
        );
        assert!(
            rows[1]
                .iter()
                .any(|(t, c, ..)| t.contains("let") && c.is_some())
        );

        // C turns colors off; the text stays the same.
        press(&mut v, KeyCode::Char('c'));
        assert_eq!(v.language(), None);
        assert!(v.styled(5)[1].iter().all(|(_, c, ..)| c.is_none()));
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

    fn filter_spec(text: &str, regex: bool) -> LineFilterSpec {
        LineFilterSpec {
            text: text.into(),
            regex,
            ..Default::default()
        }
    }

    fn numbers(v: &mut Viewer, height: usize) -> Vec<usize> {
        v.styled(height).iter().filter_map(|r| r.line_num).collect()
    }

    /// `n` lines: "line 1", "line 2"..., every third one with "error".
    fn logged(n: usize) -> Viewer {
        let text: String = (1..=n)
            .map(|i| {
                let word = if i % 3 == 0 { " error" } else { "" };
                format!("line {i}{word}\n")
            })
            .collect();
        let mut v = viewer(text.as_bytes());
        v.page = 4;
        v.line_numbers = true;
        v
    }

    #[test]
    fn line_filter_keeps_the_numbers_of_the_file() {
        let mut v = logged(10);
        assert_eq!(
            v.apply_line_filter(&filter_spec("error", false)),
            "Filter: 3 of 10 lines"
        );
        assert_eq!(numbers(&mut v, 10), [3, 6, 9]);
        assert_eq!(v.line_count(), 3);
        assert_eq!(v.total_lines(), 10);
        assert_eq!(v.visible_lines(10)[1], "line 6 error");
    }

    #[test]
    fn line_filter_with_regex_and_inverted() {
        let mut v = logged(10);
        v.apply_line_filter(&filter_spec(r"^line \d$", true));
        assert_eq!(numbers(&mut v, 20), [1, 2, 4, 5, 7, 8]);
        let spec = LineFilterSpec {
            invert: true,
            ..filter_spec("error", false)
        };
        v.apply_line_filter(&spec);
        assert_eq!(numbers(&mut v, 20), [1, 2, 4, 5, 7, 8, 10]);
    }

    #[test]
    fn line_filter_that_fails_changes_nothing() {
        let mut v = logged(10);
        v.apply_line_filter(&filter_spec("error", false));
        let note = v.apply_line_filter(&filter_spec("nothing", false));
        assert!(note.starts_with("No lines match"), "{note}");
        assert_eq!(numbers(&mut v, 20), [3, 6, 9]);
        let note = v.apply_line_filter(&filter_spec("(", true));
        assert!(note.starts_with("Invalid regex"), "{note}");
        assert_eq!(numbers(&mut v, 20), [3, 6, 9]);
    }

    #[test]
    fn empty_filter_turns_it_off_keeping_the_line_at_the_top() {
        let mut v = logged(30);
        v.apply_line_filter(&filter_spec("error", false));
        press(&mut v, KeyCode::Down);
        press(&mut v, KeyCode::Down);
        assert_eq!(v.top_line(), 8, "line 9");
        assert_eq!(
            v.apply_line_filter(&filter_spec("", false)),
            "Filter off: 30 lines"
        );
        assert!(v.line_filter().is_none());
        assert_eq!(v.top, 8);
        // Turning it on looks for the same line, or the next one kept.
        press(&mut v, KeyCode::Down);
        v.apply_line_filter(&filter_spec("error", false));
        assert_eq!(v.top_line(), 11, "line 12");
    }

    #[test]
    fn scrolling_stops_at_the_lines_kept() {
        let mut v = logged(30);
        v.apply_line_filter(&filter_spec("error", false));
        press(&mut v, KeyCode::End);
        assert_eq!(numbers(&mut v, 4), [18, 21, 24, 27, 30][1..]);
        press(&mut v, KeyCode::Home);
        assert_eq!(v.top_line(), 2);
        v.wrap = true;
        press(&mut v, KeyCode::End);
        assert_eq!(v.top_line(), 29);
        press(&mut v, KeyCode::Down);
        assert_eq!(v.top_line(), 29);
        press(&mut v, KeyCode::Up);
        assert_eq!(v.top_line(), 26);
        assert_eq!(numbers(&mut v, 2), [27, 30]);
    }

    #[test]
    fn search_sees_only_the_lines_kept() {
        let mut v = logged(30);
        v.apply_line_filter(&filter_spec("error", false));
        // "line 1" also starts lines 1, 10, 11... but only 12, 15, 18 are kept.
        assert!(v.search(&pat("line 1"), true, SearchFrom::Edge));
        assert_eq!(v.hit.unwrap().line, 11);
        assert!(v.search_next(&pat("line 1"), true));
        assert_eq!(v.hit.unwrap().line, 14);
        assert!(v.search_next(&pat("line 1"), false));
        assert_eq!(v.hit.unwrap().line, 11);
        assert!(!v.search_next(&pat("line 1"), false));
        assert!(v.search(&pat("line 1"), false, SearchFrom::Edge));
        assert_eq!(v.hit.unwrap().line, 17);
        // The match is on screen: its row, not its line, moves the window.
        assert_eq!(numbers(&mut v, 4)[0], v.top_line() + 1);
        assert!(numbers(&mut v, 4).contains(&18));
        assert!(!v.search(&pat("line 2\u{0}"), true, SearchFrom::Edge));
    }

    #[test]
    fn a_match_the_filter_hides_is_forgotten() {
        let mut v = logged(10);
        assert!(v.search(&pat("line 3"), true, SearchFrom::Edge));
        v.apply_line_filter(&filter_spec("error", false));
        assert!(v.hit.is_some(), "line 3 stays");
        assert!(v.search(&pat("line 6"), true, SearchFrom::Edge));
        v.apply_line_filter(&filter_spec("line 3", false));
        assert!(v.hit.is_none());
    }

    #[test]
    fn goto_takes_lines_of_the_file_under_a_filter() {
        let mut v = logged(30);
        v.apply_line_filter(&filter_spec("error", false));
        v.goto_line(9);
        assert_eq!(v.top_line(), 8);
        assert_eq!(v.note.as_deref(), Some("Line 9 of 30"));
        v.goto_line(10);
        assert_eq!(v.top_line(), 11);
        assert_eq!(
            v.note.as_deref(),
            Some("Line 10 is filtered out: showing line 12")
        );
        v.goto_line(999);
        assert_eq!(v.top_line(), 29);
    }

    #[test]
    fn colors_stay_those_of_the_whole_text() {
        let text = b"/* first\nkeep inside\n*/\nkeep outside\n";
        let mut whole = Viewer::from_bytes("x.c".into(), 40, text.to_vec());
        let mut v = Viewer::from_bytes("x.c".into(), 40, text.to_vec());
        v.apply_line_filter(&filter_spec("keep", false));
        let colors = |rows: &[StyledRow]| rows[0].segments[0].1;
        let all = whole.styled(4);
        let kept = v.styled(4);
        assert_eq!(colors(&kept), colors(&all[1..]), "inside the comment");
        assert_eq!(colors(&kept[1..]), colors(&all[3..]), "outside");
        assert_ne!(colors(&kept), colors(&kept[1..]));
    }

    #[test]
    fn line_filter_survives_a_change_of_encoding() {
        let mut v = logged(10);
        v.apply_line_filter(&filter_spec("error", false));
        v.cycle_charset();
        assert_eq!(numbers(&mut v, 20), [3, 6, 9]);
    }

    #[test]
    fn filter_key_depends_on_the_mode() {
        let mut v = logged(3);
        assert_eq!(press(&mut v, KeyCode::Char('&')), Action::FilterLines);
        press(&mut v, KeyCode::Char('h'));
        assert_eq!(press(&mut v, KeyCode::Char('&')), Action::None);
        assert!(v.note.as_deref().unwrap().contains("text mode"));
        // Hex keeps its bytes whatever the filter did to the text.
        v.mode = ViewMode::Text;
        v.apply_line_filter(&filter_spec("error", false));
        v.mode = ViewMode::Hex;
        assert_eq!(v.line_count(), v.data.len().div_ceil(HEX_WIDTH));
    }

    #[test]
    fn line_filter_in_wrap_mode_numbers_only_first_rows() {
        let mut v = viewer(b"short\nkeep aaaa bbbb cccc dddd\nskip\nkeep\n");
        v.page = 10;
        v.width = 12;
        v.line_numbers = true;
        v.wrap = true;
        v.apply_line_filter(&filter_spec("keep", false));
        let rows = v.styled(10);
        assert_eq!(rows[0].line_num, Some(2));
        assert_eq!(rows[1].line_num, None);
        assert_eq!(rows.last().unwrap().line_num, Some(4));
        assert_eq!(v.visible_lines(10).last().unwrap(), "keep");
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

    fn md(text: &str) -> Viewer {
        let n = text.len() as u64;
        let mut v = Viewer::from_file("doc.md".into(), n, text.as_bytes().to_vec(), LOAD);
        v.page = 10;
        v.width = 40;
        v
    }

    fn shown(v: &mut Viewer, height: usize) -> Vec<String> {
        v.styled(height)
            .iter()
            .map(|r| r.iter().map(|(t, ..)| t.as_str()).collect())
            .collect()
    }

    const DOC: &str = "# Title\n\nOne two three four five six seven eight nine ten eleven twelve\n\n- item\n\nlast\n";

    #[test]
    fn markdown_opens_rendered_by_name_only() {
        let mut v = md(DOC);
        assert_eq!(v.mode, ViewMode::Rendered);
        assert_eq!(shown(&mut v, 2), ["Title", "═════"]);
        let n = DOC.len() as u64;
        let raw = DOC.as_bytes().to_vec();
        for name in ["a.markdown", "A.MD"] {
            let v = Viewer::from_file(name.into(), n, raw.clone(), LOAD);
            assert_eq!(v.mode, ViewMode::Rendered, "{name}");
        }
        let v = Viewer::from_file("a.txt".into(), n, raw.clone(), LOAD);
        assert_eq!(v.mode, ViewMode::Text);
        // git's output and the program's own screens stay text.
        let v = Viewer::from_bytes("a.md".into(), n, raw.clone());
        assert_eq!(v.mode, ViewMode::Text);
        // The name inside the compressed file counts.
        let packed = gz(DOC.as_bytes());
        assert_eq!(from_gz("a.md.gz", packed, LOAD).mode, ViewMode::Rendered);
        // Autoview too.
        let v = Viewer::from_file("a.md".into(), n, raw, LoadOptions::autoview());
        assert_eq!(v.mode, ViewMode::Rendered);
        // A binary file does not.
        let v = Viewer::from_file("a.md".into(), 2, vec![0, 1], LOAD);
        assert_eq!(v.mode, ViewMode::Hex);
    }

    #[test]
    fn markdown_keys_cycle_the_modes_and_keep_the_place() {
        let mut v = md(DOC);
        v.page = 3;
        shown(&mut v, 3);
        press(&mut v, KeyCode::Char('h'));
        assert_eq!(v.mode, ViewMode::Text);
        press(&mut v, KeyCode::Char('h'));
        assert_eq!(v.mode, ViewMode::Hex);
        press(&mut v, KeyCode::Char('h'));
        assert_eq!(v.mode, ViewMode::Rendered);

        // T goes to the same line of the source, and comes back to it.
        press(&mut v, KeyCode::End);
        let line = v.rendered_line();
        assert!(line >= 4);
        press(&mut v, KeyCode::Char('t'));
        assert_eq!(v.mode, ViewMode::Text);
        assert_eq!(v.top_line(), line);
        press(&mut v, KeyCode::Char('t'));
        assert_eq!(v.mode, ViewMode::Rendered);
        assert_eq!(v.rendered_line(), line);
    }

    #[test]
    fn markdown_goto_and_line_numbers_follow_the_source() {
        let mut v = md(DOC);
        v.goto_line(5);
        assert_eq!(v.rendered_line(), 4);
        assert_eq!(v.note.as_deref(), Some("Line 5 of 7"));
        v.toggle_line_numbers();
        v.top = 0;
        let rows = v.styled(10);
        assert_eq!(rows[0].line_num, Some(1));
        assert_eq!(rows[1].line_num, None, "the heading's underline");
        assert!(rows.iter().any(|r| r.line_num == Some(7)));
        assert!(v.gutter_width() > 0);
    }

    #[test]
    fn markdown_search_finds_text_across_a_wrapped_line() {
        let mut v = md(DOC);
        shown(&mut v, 10);
        // On the screen these words are on different rows.
        let rows = shown(&mut v, 10);
        assert!(
            !rows
                .iter()
                .any(|r| r.contains("seven eight nine ten eleven"))
        );
        assert!(v.search(&pat("seven eight nine ten eleven"), true, SearchFrom::Edge));
        let hit = v.hit.unwrap();
        assert_eq!(hit.mode, ViewMode::Rendered);
        let marked: String = v
            .styled(10)
            .iter()
            .flat_map(|r| r.segments.clone())
            .filter(|s| s.2)
            .map(|s| s.0)
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(marked, "seven eight nine ten eleven");

        // Markup is not part of what is searched, and the address is.
        let mut v = md("**bold** text and [a](http://x.example)\n");
        assert!(v.search(&pat("bold text"), true, SearchFrom::Edge));
        assert!(v.search(&pat("x.example"), true, SearchFrom::Edge));
        assert!(!v.search(&pat("**"), true, SearchFrom::Edge));
    }

    #[test]
    fn markdown_search_goes_both_ways_from_the_page() {
        let mut v = md("alpha\n\nbeta\n\nalpha\n\nbeta\n\nalpha\n");
        v.page = 3;
        shown(&mut v, 3);
        assert!(v.search(&pat("alpha"), true, SearchFrom::Page));
        let first = v.hit.unwrap().line;
        assert!(v.search_next(&pat("alpha"), true));
        let second = v.hit.unwrap().line;
        assert!(second > first);
        assert!(v.search_next(&pat("alpha"), false));
        assert_eq!(v.hit.unwrap().line, first);
        assert!(!v.search_next(&pat("alpha"), false));
        assert!(v.search(&pat("beta"), false, SearchFrom::Edge));
        assert!(v.search(&pat("alpha"), false, SearchFrom::Page));
    }

    #[test]
    fn markdown_search_shows_a_column_past_the_window() {
        let header = (0..30).map(|i| format!("col{i}")).collect::<Vec<_>>();
        let text = format!("|{}|\n|{}|\n", header.join("|"), vec!["-"; 30].join("|"));
        let mut v = md(&text);
        shown(&mut v, 10);
        assert!(v.search(&pat("col29"), true, SearchFrom::Edge));
        assert!(v.left > 0);
        let row = &shown(&mut v, 1)[0];
        assert!(row.contains("col29"), "{row}");
    }

    #[test]
    fn batch_of_a_rendered_file_reads_the_source_lines() {
        let mut v = md(DOC);
        let batch::Source::Text {
            text, total, kept, ..
        } = v.batch_source().unwrap()
        else {
            panic!("not the text");
        };
        assert_eq!(text, DOC);
        assert_eq!(total, 7);
        assert_eq!(kept, None);
        assert_eq!(
            v.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)),
            Action::Batch
        );

        // A filter belongs to the text mode: the document shows all the lines.
        press(&mut v, KeyCode::Char('t'));
        v.apply_line_filter(&filter_spec("item", false));
        press(&mut v, KeyCode::Char('t'));
        let batch::Source::Text { kept, .. } = v.batch_source().unwrap() else {
            panic!("not the text");
        };
        assert_eq!(kept, None);
    }

    #[test]
    fn markdown_filter_opens_over_the_source_and_wrap_stays_on() {
        let mut v = md(DOC);
        assert_eq!(press(&mut v, KeyCode::Char('&')), Action::FilterLines);
        assert_eq!(v.mode, ViewMode::Text);
        let mut v = md(DOC);
        press(&mut v, KeyCode::Char('w'));
        assert!(!v.wrap);
        assert_eq!(
            v.note.as_deref(),
            Some("Rendered text always fits the window")
        );
    }

    #[test]
    fn markdown_follows_the_width_and_keeps_the_text_at_the_top() {
        let long = "word ".repeat(200);
        let mut v = md(&long);
        v.width = 20;
        shown(&mut v, 5);
        v.top = 10;
        let before = shown(&mut v, 1)[0].clone();
        v.width = 60;
        let after = shown(&mut v, 1)[0].clone();
        assert!(v.top < 10);
        assert!(after.starts_with("word"), "{before} {after}");
        assert!(after.len() > before.len());
    }

    #[test]
    fn markdown_at_the_top_stays_at_the_top_in_another_width() {
        let mut v = md("```rust\nfn a() {}\n```\n");
        v.width = 100;
        assert!(shown(&mut v, 2)[0].starts_with("─ rust ─"));
        assert_eq!(v.top, 0);
    }

    #[test]
    fn a_huge_markdown_file_opens_as_its_source() {
        let text = "x\n".repeat(markdown::MAX_INPUT / 2 + 1);
        let v = md(&text);
        assert_eq!(v.mode, ViewMode::Text);
        assert!(!v.can_render());
        assert!(
            v.note
                .as_deref()
                .unwrap()
                .starts_with("Too large to render")
        );
        // T and H then have nothing more to offer.
        let mut v = v;
        press(&mut v, KeyCode::Char('t'));
        assert_eq!(v.mode, ViewMode::Text);
    }

    #[test]
    fn markdown_past_the_read_limit_renders_what_was_read() {
        let v = Viewer::from_file("a.md".into(), 1 << 40, b"# start\n".to_vec(), LOAD);
        assert_eq!(v.mode, ViewMode::Rendered);
        assert!(v.truncated());
    }

    #[test]
    fn markdown_header_counts_the_controls_entities_become() {
        let mut v = md("a &#x202E;b\n");
        assert_eq!(v.bidi_count(), 1);
        press(&mut v, KeyCode::Char('t'));
        assert_eq!(v.bidi_count(), 0, "in the source it is plain text");
    }

    #[test]
    fn markdown_follows_a_new_encoding() {
        let mut v = md("# caf\u{e9}\n");
        assert_eq!(shown(&mut v, 1)[0], "café");
        v.cycle_charset();
        assert_eq!(v.mode, ViewMode::Rendered);
        assert_ne!(shown(&mut v, 1)[0], "café");
    }

    #[test]
    fn markdown_reload_keeps_the_mode_and_the_place() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("r.md");
        let long: String = (0..60).map(|i| format!("para {i}\n\n")).collect();
        std::fs::write(&path, &long).unwrap();
        let mut v = Viewer::open(path.clone(), LOAD).unwrap();
        assert!(v.on_disk);
        v.page = 5;
        v.width = 30;
        shown(&mut v, 5);
        v.top = 20;
        v.reload().unwrap();
        assert_eq!(v.mode, ViewMode::Rendered);
        assert_eq!(v.top, 20);
        // Too big now: the source, not an empty screen.
        std::fs::write(&path, "x\n".repeat(markdown::MAX_INPUT)).unwrap();
        v.reload().unwrap();
        assert_eq!(v.mode, ViewMode::Text);
    }
}
