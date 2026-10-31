//! The viewer's rendered Markdown: `.md` and `.markdown` files shown as a
//! document, from a hostile file.
//!
//! In two steps. [`parse`] turns the parser's events into *logical lines* (a
//! paragraph, a list item, a line of code, a table cell), which are what a
//! search looks at, so a phrase broken across two rows of the screen is still
//! found. [`Doc::layout`] breaks those into rows for a width, again when the
//! width changes. Nothing here recurses, however deep the nesting.
//!
//! All the text goes through [`Line::push`], after the parser: `&#27;` and
//! `&#x202E;` only become control characters there. A link always shows its
//! real address, and nothing is ever fetched or opened.

use std::ops::Range;
use std::rc::Rc;
use std::time::{Duration, Instant};

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier};

use super::{Segment, TAB_WIDTH, char_cells, colors_in};
use crate::highlight::Highlighter;

/// A bigger file opens as its source (8 MiB): the parser's first pass cannot
/// be interrupted, and takes about 0.3 s on this much of the worst input.
pub const MAX_INPUT: usize = 8 << 20;
/// How many nested quotes, lists and items take up room on the screen. What
/// is deeper still shows, without more indentation.
const MAX_DEPTH: usize = 32;
/// Columns of a table, as in the table mode.
const MAX_COLUMNS: usize = 256;
/// Text gets at least this many cells, however deep its prefix.
const MIN_TEXT: usize = 8;
/// A column shrinks to no less than this.
const MIN_COLUMN: usize = 8;
/// Rough memory a logical line and a row take beyond their contents.
const LINE_COST: usize = 160;
const ROW_COST: usize = 96;
const PIECE_COST: usize = 24;

const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

/// True for the names that open rendered.
pub fn is_markdown(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Kind {
    #[default]
    Plain,
    Heading,
    Code,
    Link,
    Url,
    Html,
    Marker,
    Control,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Look {
    kind: Kind,
    bold: bool,
    italic: bool,
    strike: bool,
}

impl Look {
    const fn of(kind: Kind) -> Look {
        Look {
            kind,
            bold: false,
            italic: false,
            strike: false,
        }
    }
}

/// The color and modifiers of a look. Italic and strikethrough also change the
/// color, because not every terminal draws them.
fn paint(look: Look) -> (Option<Color>, Modifier) {
    let t = crate::config::theme();
    let mut m = Modifier::empty();
    if look.bold {
        m |= Modifier::BOLD;
    }
    if look.italic {
        m |= Modifier::ITALIC;
    }
    if look.strike {
        m |= Modifier::CROSSED_OUT;
    }
    let color = match look.kind {
        Kind::Plain if look.italic => Some(t.symlink),
        Kind::Plain if look.strike => Some(t.dim),
        Kind::Plain => None,
        Kind::Heading => Some(t.key),
        Kind::Code => Some(t.special),
        Kind::Link => {
            m |= Modifier::UNDERLINED;
            Some(t.key)
        }
        Kind::Url | Kind::Html | Kind::Marker => Some(t.dim),
        Kind::Control => Some(t.error),
    };
    (color, m)
}

/// A logical line being written: only clean text gets in.
#[derive(Default)]
struct Line {
    text: String,
    /// Where each look starts (character index), in order.
    runs: Vec<(u32, Look)>,
    chars: u32,
}

impl Line {
    fn put(&mut self, c: char, look: Look) {
        if self.runs.last().map(|r| r.1) != Some(look) {
            self.runs.push((self.chars, look));
        }
        self.text.push(c);
        self.chars += 1;
    }

    /// Adds text: tabs expanded, and every control character as a `·` in the
    /// error color. Counts the bidi controls.
    fn push(&mut self, s: &str, look: Look, bidi: &mut usize) {
        for c in s.chars() {
            if c == '\t' {
                let n = TAB_WIDTH - self.chars as usize % TAB_WIDTH;
                (0..n).for_each(|_| self.put(' ', look));
            } else if crate::bidi::is_bidi(c) {
                *bidi += 1;
                self.put('·', Look::of(Kind::Control));
            } else if c.is_control() {
                self.put('·', Look::of(Kind::Control));
            } else {
                self.put(c, look);
            }
        }
    }
}

struct CodeBlock {
    language: String,
    first: usize,
    len: usize,
    /// Looked up the first time the block is drawn.
    colors: Option<Option<Highlighter>>,
}

struct Table {
    prefix: Rc<str>,
    aligns: Vec<Alignment>,
    /// The cells' logical lines, by row, and the source line of each row.
    rows: Vec<(usize, Vec<usize>)>,
}

enum Item {
    Blank(Rc<str>),
    Rule(Rc<str>, usize),
    Label(Rc<str>, Rc<str>, usize),
    Text {
        line: usize,
        first: Rc<str>,
        rest: Rc<str>,
        /// Breaks only between words (code breaks anywhere).
        words: bool,
        underline: Option<char>,
        source: usize,
    },
    Table(Table),
}

#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Deco(Rc<str>, Kind),
    Pad(usize),
    Text {
        line: usize,
        chars: Range<usize>,
        bytes: Range<usize>,
    },
}

struct Row {
    pieces: Vec<Piece>,
    source: usize,
    /// First row of its source line: it gets the number in the margin.
    numbered: bool,
}

/// A parsed document, laid out for a width.
pub struct Doc {
    lines: Vec<String>,
    runs: Vec<Vec<(u32, Look)>>,
    items: Vec<Item>,
    blocks: Vec<CodeBlock>,
    bidi: usize,
    /// The budget or the time limit stopped the parser, or a table had too
    /// many columns.
    cut: bool,
    budget: usize,
    width: usize,
    rows: Vec<Row>,
    /// The first row of each logical line.
    line_row: Vec<usize>,
    /// The budget stopped the layout.
    rows_cut: bool,
}

enum Container {
    Quote,
    List(Option<u64>),
    Item {
        marker: Option<String>,
        width: usize,
    },
}

/// The block of raw lines being read: code, HTML or front matter.
struct Raw {
    look: Look,
    block: Option<usize>,
    first: usize,
    line: Line,
    source: usize,
}

struct Builder {
    doc: Doc,
    line_starts: Vec<usize>,
    containers: Vec<Container>,
    /// Containers open past `MAX_DEPTH`, which take no room.
    deeper: usize,
    open: Option<(Line, usize)>,
    raw: Option<Raw>,
    table: Option<Table>,
    row: Vec<usize>,
    row_source: usize,
    in_cell: bool,
    need_blank: bool,
    bold: usize,
    italic: usize,
    strike: usize,
    heading: Option<HeadingLevel>,
    /// The links open: address, and where the text of each one starts.
    links: Vec<(String, usize)>,
    images: Vec<String>,
    used: usize,
    /// The last prefix, to share it between the items that have the same one.
    last_prefix: Rc<str>,
}

/// Reads `text`, within `time` and about `budget` bytes of memory.
pub fn parse(text: &str, time: Duration, budget: usize) -> Doc {
    let start = Instant::now();
    let mut b = Builder::new(text, budget);
    let events = Parser::new_ext(text, OPTIONS).into_offset_iter();
    for (n, (event, range)) in events.enumerate() {
        if (n & 0x3ff == 0 && start.elapsed() >= time) || b.used > budget {
            b.doc.cut = true;
            break;
        }
        b.event(event, range);
    }
    b.finish()
}

impl Builder {
    fn new(source: &str, budget: usize) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            source
                .bytes()
                .enumerate()
                .filter(|&(_, b)| b == b'\n')
                .map(|(i, _)| i + 1),
        );
        Builder {
            doc: Doc {
                lines: Vec::new(),
                runs: Vec::new(),
                items: Vec::new(),
                blocks: Vec::new(),
                bidi: 0,
                cut: false,
                budget,
                width: 0,
                rows: Vec::new(),
                line_row: Vec::new(),
                rows_cut: false,
            },
            line_starts,
            containers: Vec::new(),
            deeper: 0,
            open: None,
            raw: None,
            table: None,
            row: Vec::new(),
            row_source: 0,
            in_cell: false,
            need_blank: false,
            bold: 0,
            italic: 0,
            strike: 0,
            heading: None,
            links: Vec::new(),
            images: Vec::new(),
            used: 0,
            last_prefix: Rc::from(""),
        }
    }

    fn line_of(&self, offset: usize) -> usize {
        self.line_starts
            .partition_point(|&s| s <= offset)
            .saturating_sub(1)
    }

    fn enter(&mut self, c: Container) {
        if self.deeper > 0 || self.containers.len() >= MAX_DEPTH {
            self.deeper += 1;
        } else {
            self.containers.push(c);
        }
    }

    fn leave(&mut self) {
        if self.deeper > 0 {
            self.deeper -= 1;
        } else {
            self.containers.pop();
        }
    }

    fn in_list(&self) -> bool {
        self.containers
            .iter()
            .any(|c| matches!(c, Container::List(_)))
    }

    fn shared(&mut self, s: String) -> Rc<str> {
        if *self.last_prefix != *s {
            self.last_prefix = Rc::from(s);
        }
        self.last_prefix.clone()
    }

    /// The prefix of the following rows.
    fn prefix_rest(&mut self) -> Rc<str> {
        let mut s = String::new();
        for c in &self.containers {
            match c {
                Container::Quote => s.push_str("│ "),
                Container::List(_) => {}
                Container::Item { width, .. } => s.extend(std::iter::repeat_n(' ', *width)),
            }
        }
        self.shared(s)
    }

    /// The prefix of a block's first row: it takes the item's bullet.
    fn prefix_first(&mut self) -> Rc<str> {
        let mut s = String::new();
        for c in &mut self.containers {
            match c {
                Container::Quote => s.push_str("│ "),
                Container::List(_) => {}
                Container::Item { marker, width } => match marker.take() {
                    Some(m) => s.push_str(&m),
                    None => s.extend(std::iter::repeat_n(' ', *width)),
                },
            }
        }
        self.shared(s)
    }

    /// Stores a finished logical line.
    fn add_line(&mut self, line: Line) -> usize {
        self.used +=
            LINE_COST + line.text.capacity() + line.runs.capacity() * size_of::<(u32, Look)>();
        self.doc.lines.push(line.text);
        self.doc.runs.push(line.runs);
        self.doc.lines.len() - 1
    }

    /// Closes the inline text in progress, if any.
    fn flush(&mut self, underline: Option<char>) {
        let Some((line, source)) = self.open.take() else {
            return;
        };
        if self.in_cell {
            if self.row.len() < MAX_COLUMNS {
                let id = self.add_line(line);
                self.row.push(id);
            } else {
                self.doc.cut = true;
            }
            return;
        }
        let mut first = self.prefix_first();
        let rest = self.prefix_rest();
        if let Some(level) = self.heading.filter(|l| *l > HeadingLevel::H2) {
            first = Rc::from(format!("{first}{} ", "#".repeat(level as usize)));
        }
        let id = self.add_line(line);
        self.doc.items.push(Item::Text {
            line: id,
            first,
            rest,
            words: true,
            underline,
            source,
        });
    }

    /// A new block: closes the text before it, with a blank row if due.
    fn begin_block(&mut self) {
        self.flush(None);
        if self.need_blank && !self.doc.items.is_empty() {
            let prefix = self.prefix_rest();
            self.doc.items.push(Item::Blank(prefix));
        }
        self.need_blank = false;
    }

    fn open_at(&mut self, offset: usize) {
        if self.open.is_none() {
            let source = self.line_of(offset);
            self.open = Some((Line::default(), source));
        }
    }

    fn look(&self, kind: Kind) -> Look {
        let kind = match kind {
            Kind::Plain if !self.links.is_empty() => Kind::Link,
            Kind::Plain if self.heading.is_some() => Kind::Heading,
            k => k,
        };
        Look {
            kind,
            bold: self.bold > 0,
            italic: self.italic > 0,
            strike: self.strike > 0,
        }
    }

    fn inline(&mut self, text: &str, kind: Kind, offset: usize) {
        self.open_at(offset);
        let look = self.look(kind);
        if let Some((line, _)) = &mut self.open {
            line.push(text, look, &mut self.doc.bidi);
        }
    }

    /// The real address after a link's text, unless it is the text itself.
    fn address(&mut self, url: &str, text_start: usize, offset: usize) {
        self.open_at(offset);
        let Some((line, _)) = &mut self.open else {
            return;
        };
        if url.is_empty() || line.text.get(text_start..) == Some(url) {
            return;
        }
        let look = Look::of(Kind::Url);
        if !line.text.is_empty() && !line.text.ends_with(' ') {
            line.put(' ', look);
        }
        line.put('<', look);
        line.push(url, look, &mut self.doc.bidi);
        line.put('>', look);
    }

    fn begin_raw(&mut self, look: Look, language: Option<String>, offset: usize) {
        self.begin_block();
        let source = self.line_of(offset);
        let first = self.doc.lines.len();
        let block = language.map(|language| {
            let mut clean = Line::default();
            clean.push(&language, Look::default(), &mut self.doc.bidi);
            let prefix = self.prefix_first();
            let label = format!("─ {} ", clean.text);
            self.doc
                .items
                .push(Item::Label(prefix, Rc::from(label), source));
            self.doc.blocks.push(CodeBlock {
                language: clean.text,
                first,
                len: 0,
                colors: None,
            });
            self.doc.blocks.len() - 1
        });
        self.raw = Some(Raw {
            look,
            block,
            first,
            line: Line::default(),
            source,
        });
    }

    /// Finishes the raw line in progress as an item of its own.
    fn raw_line(&mut self, mut raw: Raw) -> Raw {
        let line = std::mem::take(&mut raw.line);
        let first = self.prefix_first();
        let rest = self.prefix_rest();
        let id = self.add_line(line);
        // Code sits two columns in; HTML stays at the margin.
        let indent = |p: &Rc<str>| -> Rc<str> {
            if raw.look.kind == Kind::Code {
                Rc::from(format!("{p}  "))
            } else {
                p.clone()
            }
        };
        self.doc.items.push(Item::Text {
            line: id,
            first: indent(&first),
            rest: indent(&rest),
            words: false,
            underline: None,
            source: raw.source,
        });
        raw
    }

    fn raw_text(&mut self, text: &str, offset: usize) {
        let Some(mut raw) = self.raw.take() else {
            return;
        };
        let base = self.line_of(offset);
        for (k, piece) in text.split_inclusive('\n').enumerate() {
            if raw.line.chars == 0 {
                raw.source = base + k;
            }
            let (body, ended) = match piece.strip_suffix('\n') {
                Some(b) => (b.strip_suffix('\r').unwrap_or(b), true),
                None => (piece, false),
            };
            raw.line.push(body, raw.look, &mut self.doc.bidi);
            if ended {
                raw = self.raw_line(raw);
            }
        }
        self.raw = Some(raw);
    }

    fn end_raw(&mut self) {
        if let Some(mut raw) = self.raw.take() {
            if raw.line.chars > 0 {
                raw = self.raw_line(raw);
            }
            if let Some(block) = raw.block {
                self.doc.blocks[block].len = self.doc.lines.len() - raw.first;
            }
        }
        self.need_blank = true;
    }

    fn start(&mut self, tag: Tag, offset: usize) {
        match tag {
            Tag::Paragraph => {
                self.begin_block();
                self.open_at(offset);
            }
            Tag::Heading { level, .. } => {
                self.begin_block();
                self.heading = Some(level);
                self.bold += 1;
                self.open_at(offset);
            }
            Tag::BlockQuote(_) => {
                self.begin_block();
                self.enter(Container::Quote);
            }
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split(|c: char| c.is_whitespace() || c == ',')
                        .next()
                        .filter(|l| !l.is_empty())
                        .map(|l| l.chars().take(32).collect()),
                    CodeBlockKind::Indented => None,
                };
                self.begin_raw(Look::of(Kind::Code), language, offset);
            }
            Tag::HtmlBlock => self.begin_raw(Look::of(Kind::Html), None, offset),
            Tag::MetadataBlock(_) => {
                self.begin_raw(Look::of(Kind::Code), Some("yaml".into()), offset);
            }
            Tag::List(first) => {
                self.begin_block();
                self.enter(Container::List(first));
            }
            Tag::Item => {
                self.begin_block();
                let levels = self
                    .containers
                    .iter()
                    .filter(|c| matches!(c, Container::List(_)))
                    .count();
                let marker = match self.containers.last_mut() {
                    Some(Container::List(Some(n))) if self.deeper == 0 => {
                        let m = format!("{n}. ");
                        *n = n.saturating_add(1);
                        Some(m)
                    }
                    Some(Container::List(None)) if self.deeper == 0 => {
                        Some(["• ", "◦ ", "▪ "][levels.saturating_sub(1) % 3].to_string())
                    }
                    _ => None,
                };
                let width = marker.as_deref().map_or(0, cells);
                self.enter(Container::Item { marker, width });
            }
            Tag::Table(aligns) => {
                self.begin_block();
                let prefix = self.prefix_first();
                self.table = Some(Table {
                    prefix,
                    aligns,
                    rows: Vec::new(),
                });
            }
            Tag::TableHead | Tag::TableRow => {
                self.row = Vec::new();
                self.row_source = self.line_of(offset);
                self.bold += usize::from(matches!(tag, Tag::TableHead));
            }
            Tag::TableCell => {
                self.in_cell = true;
                self.open_at(offset);
            }
            Tag::Emphasis => self.italic += 1,
            Tag::Strong => self.bold += 1,
            Tag::Strikethrough => self.strike += 1,
            Tag::Link { dest_url, .. } => {
                self.open_at(offset);
                let at = self.open.as_ref().map_or(0, |(l, _)| l.text.len());
                self.links.push((dest_url.into_string(), at));
            }
            Tag::Image { dest_url, .. } => {
                self.open_at(offset);
                if let Some((line, _)) = &mut self.open {
                    "[image] "
                        .chars()
                        .for_each(|c| line.put(c, Look::of(Kind::Marker)));
                }
                self.images.push(dest_url.into_string());
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd, offset: usize) {
        match tag {
            TagEnd::Paragraph => {
                self.flush(None);
                self.need_blank = true;
            }
            TagEnd::Heading(level) => {
                let underline = match level {
                    HeadingLevel::H1 => Some('═'),
                    HeadingLevel::H2 => Some('─'),
                    _ => None,
                };
                self.flush(underline);
                self.heading = None;
                self.bold = self.bold.saturating_sub(1);
                self.need_blank = true;
            }
            TagEnd::BlockQuote(_) => {
                self.flush(None);
                self.leave();
                self.need_blank = true;
            }
            TagEnd::CodeBlock | TagEnd::HtmlBlock | TagEnd::MetadataBlock(_) => self.end_raw(),
            TagEnd::List(_) => {
                self.flush(None);
                self.leave();
                self.need_blank = !self.in_list();
            }
            TagEnd::Item => {
                self.flush(None);
                // An empty item still shows its bullet.
                if let Some(Container::Item {
                    marker: Some(_), ..
                }) = self.containers.last()
                    && self.deeper == 0
                {
                    self.open_at(offset);
                    self.flush(None);
                }
                self.leave();
            }
            TagEnd::TableCell => {
                self.open_at(offset);
                self.flush(None);
                self.in_cell = false;
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                let row = std::mem::take(&mut self.row);
                self.used += PIECE_COST + row.capacity() * size_of::<usize>();
                if let Some(t) = &mut self.table {
                    t.rows.push((self.row_source, row));
                }
                if matches!(tag, TagEnd::TableHead) {
                    self.bold = self.bold.saturating_sub(1);
                }
            }
            TagEnd::Table => {
                self.close_table();
                self.need_blank = true;
            }
            TagEnd::Emphasis => self.italic = self.italic.saturating_sub(1),
            TagEnd::Strong => self.bold = self.bold.saturating_sub(1),
            TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
            TagEnd::Link => {
                if let Some((url, at)) = self.links.pop() {
                    self.address(&url, at, offset);
                }
            }
            TagEnd::Image => {
                if let Some(url) = self.images.pop() {
                    // Never the same as the text, which starts with `[image]`.
                    self.address(&url, usize::MAX, offset);
                }
            }
            _ => {}
        }
    }

    fn close_table(&mut self) {
        if let Some(t) = self.table.take() {
            self.doc.items.push(Item::Table(t));
        }
    }

    fn event(&mut self, event: Event, range: Range<usize>) {
        let offset = range.start;
        match event {
            Event::Start(tag) => self.start(tag, offset),
            Event::End(tag) => self.end(tag, offset),
            Event::Text(t) if self.raw.is_some() => self.raw_text(&t, offset),
            Event::Text(t) => self.inline(&t, Kind::Plain, offset),
            Event::Code(t) => self.inline(&t, Kind::Code, offset),
            Event::Html(t) if self.raw.is_some() => self.raw_text(&t, offset),
            Event::Html(t) | Event::InlineHtml(t) => self.inline(&t, Kind::Html, offset),
            Event::SoftBreak => self.inline(" ", Kind::Plain, offset),
            Event::HardBreak if self.in_cell => self.inline(" ", Kind::Plain, offset),
            Event::HardBreak => self.flush(None),
            Event::Rule => {
                self.begin_block();
                let prefix = self.prefix_first();
                let source = self.line_of(offset);
                self.doc.items.push(Item::Rule(prefix, source));
                self.need_blank = true;
            }
            Event::TaskListMarker(done) => {
                let text = if done { "[x] " } else { "[ ] " };
                match self.containers.last_mut() {
                    Some(Container::Item { marker, width }) if self.deeper == 0 => {
                        *marker = Some(text.to_string());
                        *width = text.len();
                    }
                    _ => self.inline(text, Kind::Marker, offset),
                }
            }
            _ => {}
        }
    }

    /// What was still open when the parser stopped or the file ended.
    fn finish(mut self) -> Doc {
        if self.raw.is_some() {
            self.end_raw();
        }
        self.flush(None);
        self.close_table();
        self.doc
    }
}

fn cells(s: &str) -> usize {
    s.chars().map(char_cells).sum()
}

/// One row of a wrapped text.
#[derive(Debug, Clone, PartialEq)]
struct Cut {
    chars: Range<usize>,
    bytes: Range<usize>,
}

/// Breaks `text` into rows of `width` cells (wide characters count as two):
/// at the last whitespace that fits when `words`, else anywhere. Text is only
/// ever broken, never dropped, but for the whitespace at a break.
fn wrap(text: &str, width: usize, words: bool) -> Vec<Cut> {
    let width = width.max(2);
    let mut out = Vec::new();
    let (mut start_char, mut start_byte) = (0, 0);
    let mut used = 0;
    // The last whitespace in this row, and the cells after it.
    let mut space: Option<(usize, usize, usize)> = None;
    let mut after = 0;
    for (i, (b, c)) in text.char_indices().enumerate() {
        let w = char_cells(c);
        if used + w > width && used > 0 {
            if words && c.is_whitespace() {
                out.push(Cut {
                    chars: start_char..i,
                    bytes: start_byte..b,
                });
                (start_char, start_byte) = (i + 1, b + c.len_utf8());
                (used, after, space) = (0, 0, None);
                continue;
            }
            match space.filter(|&(si, ..)| words && si > start_char) {
                Some((si, sb, slen)) => {
                    out.push(Cut {
                        chars: start_char..si,
                        bytes: start_byte..sb,
                    });
                    (start_char, start_byte) = (si + 1, sb + slen);
                    used = after;
                }
                None => {
                    out.push(Cut {
                        chars: start_char..i,
                        bytes: start_byte..b,
                    });
                    (start_char, start_byte) = (i, b);
                    used = 0;
                }
            }
            (space, after) = (None, used);
        }
        if c.is_whitespace() {
            space = Some((i, b, c.len_utf8()));
            after = 0;
        } else {
            after += w;
        }
        used += w;
    }
    out.push(Cut {
        chars: start_char..text.chars().count().max(start_char),
        bytes: start_byte..text.len(),
    });
    out
}

/// The width of each column within `room` cells: the widest shrink first, down
/// to `MIN_COLUMN` (a table with too many columns goes past the window, and
/// ← → reach the rest).
fn column_widths(natural: &[usize], room: usize) -> Vec<usize> {
    if natural.iter().sum::<usize>() <= room {
        return natural.to_vec();
    }
    let fit = |level: usize| -> usize { natural.iter().map(|&n| n.min(level)).sum() };
    let (mut low, mut high) = (
        MIN_COLUMN,
        natural.iter().copied().max().unwrap_or(MIN_COLUMN),
    );
    while low < high {
        let mid = (low + high).div_ceil(2);
        if fit(mid) <= room {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    natural.iter().map(|&n| n.min(low)).collect()
}

/// Rows being laid out, within the budget.
struct Rows<'a> {
    rows: &'a mut Vec<Row>,
    line_row: &'a mut [usize],
    used: usize,
    source: usize,
}

impl Rows<'_> {
    fn push(&mut self, pieces: Vec<Piece>, source: Option<usize>) {
        for p in &pieces {
            if let Piece::Text { line, .. } = p
                && self.line_row[*line] == usize::MAX
            {
                self.line_row[*line] = self.rows.len();
            }
        }
        // Row sources never go back, so a row can be found by its source.
        let numbered = source.is_some_and(|s| s > self.source || self.rows.is_empty());
        self.source = self.source.max(source.unwrap_or(0));
        self.used += ROW_COST + pieces.capacity() * PIECE_COST;
        self.rows.push(Row {
            pieces,
            source: self.source,
            numbered,
        });
    }
}

fn deco(text: &Rc<str>, kind: Kind, pieces: &mut Vec<Piece>) {
    if !text.is_empty() {
        pieces.push(Piece::Deco(text.clone(), kind));
    }
}

impl Doc {
    /// Breaks the logical lines into rows of `width` cells.
    pub fn layout(&mut self, width: usize) {
        self.width = width;
        self.rows = Vec::new();
        self.rows_cut = false;
        self.line_row = vec![usize::MAX; self.lines.len()];
        let width = width.max(MIN_TEXT);
        let mut out = Rows {
            rows: &mut self.rows,
            line_row: &mut self.line_row,
            used: 0,
            source: 0,
        };
        for item in &self.items {
            if out.used > self.budget {
                self.rows_cut = true;
                break;
            }
            match item {
                Item::Blank(prefix) => {
                    let mut pieces = Vec::new();
                    deco(prefix, Kind::Marker, &mut pieces);
                    out.push(pieces, None);
                }
                Item::Rule(prefix, source) => {
                    let mut pieces = Vec::new();
                    deco(prefix, Kind::Marker, &mut pieces);
                    let n = width.saturating_sub(cells(prefix)).max(MIN_TEXT);
                    pieces.push(Piece::Deco(Rc::from("─".repeat(n)), Kind::Marker));
                    out.push(pieces, Some(*source));
                }
                Item::Label(prefix, label, source) => {
                    let mut pieces = Vec::new();
                    deco(prefix, Kind::Marker, &mut pieces);
                    let n = width
                        .saturating_sub(cells(prefix) + cells(label))
                        .clamp(2, 24);
                    let text = format!("{label}{}", "─".repeat(n));
                    pieces.push(Piece::Deco(Rc::from(text), Kind::Marker));
                    out.push(pieces, Some(*source));
                }
                Item::Text {
                    line,
                    first,
                    rest,
                    words,
                    underline,
                    source,
                } => {
                    let text = &self.lines[*line];
                    let prefix_width = cells(first).max(cells(rest));
                    let room = width.saturating_sub(prefix_width).max(MIN_TEXT);
                    let mut widest = 0;
                    for (k, cut) in wrap(text, room, *words).into_iter().enumerate() {
                        widest = widest.max(cells(&text[cut.bytes.clone()]));
                        let mut pieces = Vec::with_capacity(2);
                        deco(if k == 0 { first } else { rest }, Kind::Marker, &mut pieces);
                        pieces.push(Piece::Text {
                            line: *line,
                            chars: cut.chars,
                            bytes: cut.bytes,
                        });
                        out.push(pieces, Some(*source));
                    }
                    if let Some(c) = underline {
                        let mut pieces = Vec::with_capacity(2);
                        deco(rest, Kind::Marker, &mut pieces);
                        let bar = c.to_string().repeat(widest.max(1));
                        pieces.push(Piece::Deco(Rc::from(bar), Kind::Heading));
                        out.push(pieces, Some(*source));
                    }
                }
                Item::Table(t) => {
                    if !lay_out_table(t, &self.lines, width, self.budget, &mut out) {
                        self.rows_cut = true;
                        break;
                    }
                }
            }
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn bidi(&self) -> usize {
        self.bidi
    }

    /// Not all of the document is on the screen.
    pub fn cut(&self) -> bool {
        self.cut || self.rows_cut
    }

    /// The parser stopped before it gave anything to show.
    pub fn is_empty_cut(&self) -> bool {
        self.cut && self.items.is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, i: usize) -> &str {
        &self.lines[i]
    }

    /// The source line a row comes from.
    pub fn source_of(&self, row: usize) -> usize {
        self.rows.get(row).map_or(0, |r| r.source)
    }

    /// The source line in the margin, on the first row of each.
    pub fn number_of(&self, row: usize) -> Option<usize> {
        self.rows
            .get(row)
            .filter(|r| r.numbered)
            .map(|r| r.source + 1)
    }

    /// The first row at or after a source line.
    pub fn row_of_source(&self, source: usize) -> usize {
        self.rows
            .partition_point(|r| r.source < source)
            .min(self.rows.len().saturating_sub(1))
    }

    /// The first text at or after `row`: logical line and character.
    pub fn anchor(&self, row: usize) -> Option<(usize, usize)> {
        self.rows.iter().skip(row).find_map(|r| {
            r.pieces.iter().find_map(|p| match p {
                Piece::Text { line, chars, .. } => Some((*line, chars.start)),
                _ => None,
            })
        })
    }

    /// Where a backward search starts: the end of the last text at or
    /// before `row`.
    pub fn anchor_back(&self, row: usize) -> Option<(usize, usize)> {
        let end = (row + 1).min(self.rows.len());
        self.rows[..end].iter().rev().find_map(|r| {
            r.pieces.iter().rev().find_map(|p| match p {
                Piece::Text { line, chars, .. } => Some((*line, chars.end)),
                _ => None,
            })
        })
    }

    /// The row showing a character of a logical line.
    pub fn row_of(&self, line: usize, at: usize) -> usize {
        let last = self.rows.len().saturating_sub(1);
        let Some(&first) = self.line_row.get(line).filter(|&&r| r != usize::MAX) else {
            return last;
        };
        for (r, row) in self.rows.iter().enumerate().skip(first) {
            for p in &row.pieces {
                if let Piece::Text { line: l, chars, .. } = p
                    && *l == line
                    && (at < chars.end || at == chars.start)
                {
                    return r;
                }
            }
            // Past the table or text the line belongs to.
            if row.pieces.iter().all(|p| !matches!(p, Piece::Text { .. })) {
                break;
            }
        }
        first
    }

    /// How many cells of `row` come before a character of a logical line.
    pub fn column_of(&self, row: usize, line: usize, at: usize) -> usize {
        let mut col = 0;
        let Some(r) = self.rows.get(row) else {
            return 0;
        };
        for p in &r.pieces {
            match p {
                Piece::Deco(s, _) => col += cells(s),
                Piece::Pad(n) => col += n,
                Piece::Text {
                    line: l,
                    chars,
                    bytes,
                } => {
                    let text = &self.lines[*l][bytes.clone()];
                    if *l == line && at >= chars.start && at < chars.end {
                        return col
                            + text
                                .chars()
                                .take(at - chars.start)
                                .map(char_cells)
                                .sum::<usize>();
                    }
                    col += cells(text);
                }
            }
        }
        col
    }

    /// The colors of a line of code, if it is in a block of a known language.
    fn code_colors(
        lines: &[String],
        blocks: &mut [CodeBlock],
        line: usize,
        chars: Range<usize>,
    ) -> Vec<Color> {
        let i = blocks.partition_point(|b| b.first <= line);
        let Some(block) = i.checked_sub(1).and_then(|i| blocks.get_mut(i)) else {
            return Vec::new();
        };
        if line >= block.first + block.len {
            return Vec::new();
        }
        let (first, len) = (block.first, block.len);
        let colors = block
            .colors
            .get_or_insert_with(|| Highlighter::for_language(&block.language));
        colors
            .as_mut()
            .and_then(|h| h.line(line - first, &lines[first..first + len]))
            .map(|segs| colors_in(segs, chars))
            .unwrap_or_default()
    }

    /// What `row` shows, from cell `left` on. `hit` is a search match: logical
    /// line and character range.
    pub fn row_segments(
        &mut self,
        row: usize,
        left: usize,
        use_colors: bool,
        hit: Option<(usize, usize, usize)>,
    ) -> Vec<Segment> {
        let mut out: Vec<Segment> = Vec::new();
        let Some(r) = self.rows.get(row) else {
            return out;
        };
        for piece in &r.pieces {
            match piece {
                Piece::Deco(text, kind) => {
                    let (color, m) = paint(Look::of(*kind));
                    text.chars().for_each(|c| add(&mut out, c, color, false, m));
                }
                Piece::Pad(n) => (0..*n).for_each(|_| {
                    add(&mut out, ' ', None, false, Modifier::empty());
                }),
                Piece::Text { line, chars, bytes } => {
                    let colors = if use_colors {
                        Self::code_colors(&self.lines, &mut self.blocks, *line, chars.clone())
                    } else {
                        Vec::new()
                    };
                    let runs = &self.runs[*line];
                    let mut run = runs
                        .partition_point(|x| x.0 as usize <= chars.start)
                        .saturating_sub(1);
                    let mut look = runs.get(run).map_or(Look::default(), |x| x.1);
                    let mut painted = paint(look);
                    for (j, c) in self.lines[*line][bytes.clone()].chars().enumerate() {
                        let at = chars.start + j;
                        while runs.get(run + 1).is_some_and(|x| x.0 as usize <= at) {
                            run += 1;
                            look = runs[run].1;
                            painted = paint(look);
                        }
                        let color = match colors.get(j) {
                            Some(c) if look.kind == Kind::Code => Some(*c),
                            _ => painted.0,
                        };
                        let marked = hit.is_some_and(|(l, s, e)| l == *line && at >= s && at < e);
                        add(&mut out, c, color, marked, painted.1);
                    }
                }
            }
        }
        if left > 0 { clip_left(out, left) } else { out }
    }
}

fn add(out: &mut Vec<Segment>, c: char, color: Option<Color>, marked: bool, m: Modifier) {
    match out.last_mut() {
        Some((text, co, ma, mo)) if *co == color && *ma == marked && *mo == m => text.push(c),
        _ => out.push((c.to_string(), color, marked, m)),
    }
}

/// `segments` without their first `left` cells.
fn clip_left(segments: Vec<Segment>, left: usize) -> Vec<Segment> {
    let mut skip = left;
    let mut out = Vec::new();
    for (text, color, marked, m) in segments {
        if skip == 0 {
            out.push((text, color, marked, m));
            continue;
        }
        let mut kept = String::new();
        for c in text.chars() {
            if skip == 0 {
                kept.push(c);
            } else {
                skip = skip.saturating_sub(char_cells(c));
            }
        }
        if !kept.is_empty() {
            out.push((kept, color, marked, m));
        }
    }
    out
}

/// A table's rows; false when the budget ran out.
fn lay_out_table(t: &Table, lines: &[String], width: usize, budget: usize, out: &mut Rows) -> bool {
    let columns = t.rows.iter().map(|(_, r)| r.len()).max().unwrap_or(0);
    if columns == 0 {
        return true;
    }
    let mut natural = vec![1; columns];
    for (_, row) in &t.rows {
        for (i, &cell) in row.iter().enumerate() {
            natural[i] = natural[i].max(cells(&lines[cell]));
        }
    }
    let gaps = 3 * (columns - 1);
    let room = width
        .saturating_sub(cells(&t.prefix) + gaps)
        .max(MIN_COLUMN * columns);
    let widths = column_widths(&natural, room);
    let bar: Rc<str> = Rc::from(" │ ");
    let rest: Rc<str> = Rc::from(" ".repeat(cells(&t.prefix)));
    for (r, (source, row)) in t.rows.iter().enumerate() {
        if out.used > budget {
            return false;
        }
        let wrapped: Vec<Vec<Cut>> = row
            .iter()
            .zip(&widths)
            .map(|(&cell, &w)| wrap(&lines[cell], w, true))
            .collect();
        let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
        for k in 0..height {
            let mut pieces = Vec::with_capacity(columns * 3 + 1);
            let prefix = if r == 0 && k == 0 { &t.prefix } else { &rest };
            deco(prefix, Kind::Marker, &mut pieces);
            for (i, &w) in widths.iter().enumerate() {
                if i > 0 {
                    pieces.push(Piece::Deco(bar.clone(), Kind::Marker));
                }
                let cut = row.get(i).zip(wrapped.get(i).and_then(|c| c.get(k)));
                let Some((&cell, cut)) = cut else {
                    pieces.push(Piece::Pad(w));
                    continue;
                };
                let used = cells(&lines[cell][cut.bytes.clone()]);
                let free = w.saturating_sub(used);
                let before = match t.aligns.get(i) {
                    Some(Alignment::Right) => free,
                    Some(Alignment::Center) => free / 2,
                    _ => 0,
                };
                if before > 0 {
                    pieces.push(Piece::Pad(before));
                }
                pieces.push(Piece::Text {
                    line: cell,
                    chars: cut.chars.clone(),
                    bytes: cut.bytes.clone(),
                });
                if free > before {
                    pieces.push(Piece::Pad(free - before));
                }
            }
            out.push(pieces, Some(*source));
        }
        if r == 0 && t.rows.len() > 1 {
            let mut pieces = Vec::with_capacity(2);
            deco(&rest, Kind::Marker, &mut pieces);
            let rule = widths
                .iter()
                .map(|&w| "─".repeat(w))
                .collect::<Vec<_>>()
                .join("─┼─");
            pieces.push(Piece::Deco(Rc::from(rule), Kind::Marker));
            out.push(pieces, Some(*source));
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIME: Duration = Duration::from_secs(30);

    fn doc(md: &str, width: usize) -> Doc {
        let mut d = parse(md, TIME, 1 << 30);
        d.layout(width);
        d
    }

    fn text_of(d: &mut Doc, row: usize) -> String {
        d.row_segments(row, 0, false, None)
            .iter()
            .map(|s| s.0.as_str())
            .collect()
    }

    fn all(d: &mut Doc) -> Vec<String> {
        (0..d.row_count()).map(|r| text_of(d, r)).collect()
    }

    fn render(md: &str, width: usize) -> Vec<String> {
        all(&mut doc(md, width))
    }

    /// The modifiers and color of the piece holding `needle`.
    fn segment_of(d: &mut Doc, needle: &str) -> Segment {
        (0..d.row_count())
            .flat_map(|r| d.row_segments(r, 0, false, None))
            .find(|s| s.0.contains(needle))
            .unwrap_or_else(|| panic!("no {needle}"))
    }

    #[test]
    fn headings_paragraphs_and_inline_styles() {
        let md = "# Title\n\nSome **bold** *slanted* ~~gone~~ `code` text.\n\n### Third\n";
        let mut d = doc(md, 60);
        let rows = all(&mut d);
        assert_eq!(rows[0], "Title");
        assert_eq!(rows[1], "═════");
        assert_eq!(rows[3], "Some bold slanted gone code text.");
        assert_eq!(rows[5], "### Third");
        assert!(segment_of(&mut d, "bold").3.contains(Modifier::BOLD));
        assert!(segment_of(&mut d, "slanted").3.contains(Modifier::ITALIC));
        assert!(segment_of(&mut d, "gone").3.contains(Modifier::CROSSED_OUT));
        assert_eq!(
            segment_of(&mut d, "code").1,
            Some(crate::config::theme().special)
        );
        // Without terminal italics, the color still tells it apart.
        assert_eq!(
            segment_of(&mut d, "slanted").1,
            Some(crate::config::theme().symlink)
        );
    }

    #[test]
    fn nested_ordered_and_task_lists() {
        let md =
            "- one\n  - inner\n    - deep\n- two\n\n3. three\n4. four\n\n- [x] done\n- [ ] todo\n";
        let rows = render(md, 40);
        assert!(rows.contains(&"• one".to_string()), "{rows:#?}");
        assert!(rows.contains(&"  ◦ inner".to_string()), "{rows:#?}");
        assert!(rows.contains(&"    ▪ deep".to_string()), "{rows:#?}");
        assert!(rows.contains(&"3. three".to_string()), "{rows:#?}");
        assert!(rows.contains(&"4. four".to_string()), "{rows:#?}");
        assert!(rows.contains(&"[x] done".to_string()), "{rows:#?}");
        assert!(rows.contains(&"[ ] todo".to_string()), "{rows:#?}");
    }

    #[test]
    fn an_empty_item_keeps_its_bullet() {
        assert!(render("- \n- b\n", 20).contains(&"• ".to_string()));
    }

    #[test]
    fn quotes_rules_and_their_prefix_on_every_row() {
        let md = "> first words of a quote that wraps around\n>\n> > inner\n\n---\n\nafter\n";
        let rows = render(md, 20);
        assert!(rows[0].starts_with("│ first"), "{rows:#?}");
        assert!(rows[1].starts_with("│ "), "{rows:#?}");
        assert!(rows.iter().any(|r| r == "│ │ inner"), "{rows:#?}");
        assert!(rows.iter().any(|r| r == &"─".repeat(20)), "{rows:#?}");
        assert_eq!(rows.last().unwrap(), "after");
    }

    #[test]
    fn code_blocks_keep_their_lines_label_and_colors() {
        let md = "```rust\nfn main() {\n\tlet x = 1;\n}\n```\n\n    indented\n";
        let mut d = doc(md, 40);
        let rows = all(&mut d);
        assert!(rows[0].starts_with("─ rust ─"), "{rows:#?}");
        assert_eq!(rows[1], "  fn main() {");
        assert_eq!(rows[2], "          let x = 1;");
        assert!(rows.iter().any(|r| r == "  indented"), "{rows:#?}");
        // Colored only when asked for, and more than one color for Rust.
        let plain = d.row_segments(1, 0, false, None);
        assert_eq!(plain.iter().filter(|s| s.0.trim() != "").count(), 1);
        let colored = d.row_segments(1, 0, true, None);
        assert!(colored.len() > plain.len(), "{colored:?}");
    }

    #[test]
    fn mermaid_is_a_labelled_code_block() {
        let md = "```mermaid\ngraph TD; A-->B\n```\n";
        let mut d = doc(md, 40);
        let rows = all(&mut d);
        assert!(rows[0].starts_with("─ mermaid ─"), "{rows:#?}");
        assert_eq!(rows[1], "  graph TD; A-->B");
        assert_eq!(
            d.row_segments(1, 0, true, None).len(),
            2,
            "no syntax colors"
        );
    }

    #[test]
    fn front_matter_stays_visible() {
        let rows = render("---\ntitle: hello\n---\n\ntext\n", 40);
        assert!(rows[0].starts_with("─ yaml ─"), "{rows:#?}");
        assert_eq!(rows[1], "  title: hello");
    }

    #[test]
    fn tables_align_and_wrap_inside_their_columns() {
        let md = "| Left | Right | Mid |\n|:---|---:|:-:|\n| a | b | c |\n| long text that has to wrap | x | y |\n";
        let rows = render(md, 30);
        assert!(
            rows[0].contains("Left") && rows[0].contains('│'),
            "{rows:#?}"
        );
        assert!(rows[1].contains('┼'), "{rows:#?}");
        assert!(rows.iter().all(|r| cells(r) <= 30), "{rows:#?}");
        let rows = render(md, 60);
        assert_eq!(rows[2], "a                          │     b │  c ");
        assert_eq!(rows[3], "long text that has to wrap │     x │  y ");
    }

    #[test]
    fn a_table_too_wide_goes_past_the_window_and_pans() {
        let header = (0..30).map(|i| format!("col{i}")).collect::<Vec<_>>();
        let md = format!("|{}|\n|{}|\n", header.join("|"), vec!["-"; 30].join("|"));
        let mut d = doc(&md, 40);
        let first = text_of(&mut d, 0);
        assert!(first.contains("col29"), "{first}");
        let panned: String = d
            .row_segments(0, 150, false, None)
            .iter()
            .map(|s| s.0.as_str())
            .collect();
        assert!(
            panned.contains("col29") && !panned.contains("col0"),
            "{panned}"
        );
    }

    #[test]
    fn links_always_show_the_real_address() {
        let rows = render("[safe](https://evil.example/x)\n", 60);
        assert_eq!(rows[0], "safe <https://evil.example/x>");
        let rows = render("[https://good.example](https://evil.example)\n", 60);
        assert_eq!(rows[0], "https://good.example <https://evil.example>");
        // Reference links too, and an empty text.
        let rows = render(
            "[a][r] and [](https://x.example)\n\n[r]: https://y.example\n",
            60,
        );
        assert_eq!(rows[0], "a <https://y.example> and <https://x.example>");
        // Said once when the text is the address.
        assert_eq!(render("<https://z.example>\n", 40)[0], "https://z.example");
    }

    #[test]
    fn a_long_address_is_broken_but_whole_even_in_a_narrow_cell() {
        let url = format!("https://evil.example/{}", "abcdefghij".repeat(12));
        let md = format!("| a | b |\n|---|---|\n| [safe]({url}) | x |\n");
        let rows = render(&md, 24);
        let joined: String = rows
            .iter()
            .map(|r| r.split(" │ ").next().unwrap().trim_end())
            .collect();
        assert!(joined.contains(&url), "{rows:#?}");
    }

    #[test]
    fn images_are_a_placeholder_with_alt_and_path() {
        let rows = render("![a cat](img/cat.png)\n\n![](x.png)\n", 60);
        assert_eq!(rows[0], "[image] a cat <img/cat.png>");
        assert_eq!(rows[2], "[image] <x.png>");
        // A badge: the image and the link it opens both show.
        let rows = render("[![ci](b.svg)](https://ci.example)\n", 60);
        assert_eq!(rows[0], "[image] ci <b.svg> <https://ci.example>");
    }

    #[test]
    fn raw_html_stays_visible_as_text() {
        let rows = render(
            "<div>\n<script>evil()</script>\n</div>\n\nx <b>y</b>\n\n<!-- hidden -->\n",
            60,
        );
        assert!(
            rows.iter().any(|r| r == "<script>evil()</script>"),
            "{rows:#?}"
        );
        assert!(rows.iter().any(|r| r == "x <b>y</b>"), "{rows:#?}");
        assert!(rows.iter().any(|r| r == "<!-- hidden -->"), "{rows:#?}");
    }

    #[test]
    fn control_characters_never_get_through_even_as_entities() {
        let md = "a\u{1b}[2J &#27;[31m &#x202E;evil &#x7;\n\n```\nb\u{1b}[0m\u{202e}\n```\n\n\
                  [t\u{202e}](u\u{202e}v)\n\n| c\u{1b} |\n|---|\n";
        let mut d = doc(md, 60);
        let rows = all(&mut d);
        for row in &rows {
            assert!(
                row.chars()
                    .all(|c| !c.is_control() && !crate::bidi::is_bidi(c)),
                "{row:?}"
            );
        }
        assert!(
            rows.iter().any(|r| r.starts_with("a·[2J ·[31m ·evil ·")),
            "{rows:#?}"
        );
        // The entity, the code, the link's text and its address.
        assert_eq!(d.bidi(), 4, "{rows:#?}");
        assert_eq!(
            segment_of(&mut d, "·").1,
            Some(crate::config::theme().error)
        );
    }

    #[test]
    fn wrap_counts_cells_and_loses_only_the_spaces() {
        let text = "中文字符 mixed with 宽字符 and a verylongwordthatdoesnotfit end";
        for width in [2, 3, 7, 12, 40] {
            let cuts = wrap(text, width, true);
            let rows: Vec<&str> = cuts.iter().map(|c| &text[c.bytes.clone()]).collect();
            for r in &rows {
                assert!(cells(r) <= width, "{width}: {r}");
            }
            let squeezed: String = rows.concat().split_whitespace().collect();
            let expected: String = text.split_whitespace().collect();
            assert_eq!(squeezed, expected, "{width}: {rows:?}");
            for cut in &cuts {
                let by_chars: String = text
                    .chars()
                    .skip(cut.chars.start)
                    .take(cut.chars.len())
                    .collect();
                assert_eq!(by_chars, text[cut.bytes.clone()]);
            }
        }
        assert_eq!(wrap("", 10, true).len(), 1);
    }

    #[test]
    fn code_is_broken_anywhere_and_keeps_its_spaces() {
        let cuts = wrap("ab   cd", 3, false);
        let text = "ab   cd";
        let rows: Vec<&str> = cuts.iter().map(|c| &text[c.bytes.clone()]).collect();
        assert_eq!(rows, ["ab ", "  c", "d"]);
    }

    #[test]
    fn rows_know_their_source_lines() {
        let md = "one\n\n```\ncode1\ncode2\n```\n\n- item\n\n| h |\n|---|\n| r |\n";
        let d = doc(md, 40);
        let of = |needle: usize| d.row_of_source(needle);
        assert_eq!(d.source_of(of(0)), 0);
        assert_eq!(d.source_of(of(3)), 3, "code1");
        assert_eq!(d.source_of(of(4)), 4, "code2");
        assert_eq!(d.source_of(of(7)), 7, "the item");
        assert_eq!(d.source_of(of(11)), 11, "the table row");
        assert_eq!(d.number_of(0), Some(1));
    }

    #[test]
    fn anchors_find_a_place_again_after_another_width() {
        let long = "word ".repeat(100);
        let mut d = doc(&long, 20);
        let (line, at) = d.anchor(5).unwrap();
        d.layout(50);
        let row = d.row_of(line, at);
        assert!(row < 5);
        assert_eq!(d.anchor(row).unwrap().0, line);
    }

    #[test]
    fn a_stopped_parser_says_so_and_keeps_what_it_has() {
        let d = parse("# a\n", Duration::ZERO, 1 << 20);
        assert!(d.is_empty_cut());
        let md = "paragraph\n\n".repeat(1000);
        let d = parse(&md, TIME, 2000);
        assert!(d.cut());
        assert!(!d.is_empty_cut());
        let mut d = parse(&md, TIME, 1 << 30);
        d.budget = 5000;
        d.layout(40);
        assert!(d.cut() && d.row_count() < 2000);
    }

    #[test]
    fn too_many_columns_are_cut_and_said() {
        let n = MAX_COLUMNS + 10;
        let md = format!("{}|\n{}|\n", "|a".repeat(n), "|-".repeat(n));
        let d = doc(&md, 80);
        assert!(d.cut());
        // What is cut is not searched either: a match would have nowhere to show.
        assert_eq!(d.line_count(), MAX_COLUMNS);
        assert_eq!(d.row_count(), 1);
    }

    #[test]
    fn hostile_files_stay_within_time_and_memory() {
        let n = 1 << 20;
        let cases: Vec<(&str, String)> = vec![
            ("quotes", ">".repeat(n)),
            (
                "deep lists",
                (0..1500).map(|i| " ".repeat(i * 2) + "- x\n").collect(),
            ),
            ("list one line", "- ".repeat(n / 2)),
            ("brackets", "[".repeat(n)),
            ("links", "[a](".repeat(n / 4)),
            ("images", "![".repeat(n / 2)),
            ("emphasis", "*_".repeat(n / 2)),
            (
                "wide table",
                format!(
                    "{}\n{}\n{}",
                    "|a".repeat(n / 8),
                    "|-".repeat(n / 8),
                    "|b".repeat(n / 4)
                ),
            ),
            (
                "tall table",
                format!("|a|b|\n|-|-|\n{}", "|x|y|\n".repeat(n / 6)),
            ),
            ("short paragraphs", "a\n\n".repeat(n / 3)),
            ("tabs", format!("```\n{}\n```\n", "\t".repeat(n))),
            ("entities", "&#27;".repeat(n / 5)),
        ];
        let budget = 16 << 20;
        for (name, md) in cases {
            let start = Instant::now();
            let mut d = parse(&md, Duration::from_secs(20), budget);
            d.layout(80);
            d.layout(23);
            let rows = d.row_count();
            let segments = d.row_segments(rows.saturating_sub(1), 0, true, None);
            assert!(segments.len() <= 4 * MAX_COLUMNS, "{name}");
            // The time limit is for the machine under test, not a benchmark.
            assert!(start.elapsed() < Duration::from_secs(30), "{name}");
        }
    }
}
