//! The help (F1 / Alt+H): the topics of the manual (`docs/manual/*.md`, built
//! into the binary), opened on the topic of what the user is doing, with
//! links between them. The same files are the manual on GitHub and become
//! the man page (`xy --man`).
//!
//! The Markdown is a small subset: `#` headings, paragraphs, `-` lists,
//! tables, code blocks, and inline `code`, **bold** and `[links](topic.md)`.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::crossterm;

include!(concat!(env!("OUT_DIR"), "/manual_translations.rs"));

/// Known ISO 639-1 language codes and their native names, in display order.
pub const KNOWN_LANGS: &[(&str, &str)] = &[
    ("en", "English"),
    ("pt", "Português"),
    ("es", "Español"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("it", "Italiano"),
    ("ru", "Русский"),
    ("zh", "中文"),
    ("ja", "日本語"),
];

/// Display width of `s` in terminal columns (handles wide CJK characters).
fn str_width(s: &str) -> usize {
    ratatui::text::Span::raw(s).width()
}

/// Splits `s` into `(head, rest)` where `head` fits in `max_cols` columns
/// (at least one character when `s` is non-empty).
fn split_at_width(s: &str, max_cols: usize) -> (String, String) {
    let mut head = String::new();
    let mut cols = 0;
    let mut chars = s.chars();
    for ch in chars.by_ref() {
        let mut buf = [0u8; 4];
        let w = str_width(ch.encode_utf8(&mut buf));
        if !head.is_empty() && cols + w > max_cols {
            let mut rest = String::from(ch);
            rest.extend(chars);
            return (head, rest);
        }
        head.push(ch);
        cols += w;
        if cols >= max_cols {
            break;
        }
    }
    (head, chars.collect())
}

/// Human-readable native name for a language code (`"pt"` -> `"Português"`).
pub fn language_label(code: &str) -> &str {
    if code.is_empty() || code.eq_ignore_ascii_case("auto") {
        return "auto";
    }
    KNOWN_LANGS
        .iter()
        .find(|(c, _)| c.eq_ignore_ascii_case(code))
        .map(|(_, label)| *label)
        .unwrap_or(code)
}

/// Normalizes a locale or language tag (`"pt_BR.UTF-8"`, `"pt-BR"`) to its
/// primary lowercase language code (`"pt"`).
pub fn normalize_lang(raw: &str) -> String {
    raw.trim()
        .split(['_', '-', '.', '@'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Languages available for the help manual: `"en"`, any embedded language in
/// `docs/manual/<lang>/`, and any user directory in `~/.config/xylem/manual/<lang>/`.
pub fn available_languages() -> Vec<String> {
    let mut codes: Vec<String> = vec!["en".into()];
    for (lang, topics) in EMBEDDED_TRANSLATIONS {
        if !topics.is_empty() && !codes.iter().any(|c| c == lang) {
            codes.push((*lang).to_string());
        }
    }
    if let Some(dir) = crate::platform::config_dir().map(|d| d.join("manual"))
        && let Ok(entries) = std::fs::read_dir(dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|s| s.to_str())
                && !name.starts_with('.')
            {
                let code = normalize_lang(name);
                if !code.is_empty() && !codes.iter().any(|c| c == &code) {
                    codes.push(code);
                }
            }
        }
    }
    codes.sort_by_key(|c| {
        let rank = KNOWN_LANGS
            .iter()
            .position(|(k, _)| *k == c.as_str())
            .unwrap_or(KNOWN_LANGS.len());
        (rank, c.clone())
    });
    codes
}

/// Detects the preferred language from `$LC_ALL`, `$LC_MESSAGES` or `$LANG`,
/// falling back to `"en"` if not available.
pub fn detect_system_language() -> String {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let trimmed = val.trim();
            if trimmed.is_empty()
                || trimmed.eq_ignore_ascii_case("c")
                || trimmed.to_ascii_lowercase().starts_with("c.")
                || trimmed.eq_ignore_ascii_case("posix")
            {
                continue;
            }
            let code = normalize_lang(trimmed);
            if !code.is_empty() {
                if available_languages().iter().any(|l| l == &code) {
                    return code;
                }
                return "en".into();
            }
        }
    }
    "en".into()
}

/// Resolves the configured `help_language` (`"auto"` or an ISO code) to an
/// available language code (`"en"`, `"pt"`, `"es"`, ...).
pub fn resolve_language(configured: &str) -> String {
    let trimmed = configured.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return detect_system_language();
    }
    let code = normalize_lang(trimmed);
    if available_languages().iter().any(|l| l == &code) {
        code
    } else {
        "en".into()
    }
}

/// The topics, by name (the file name without `.md`), in the man page's order.
pub const TOPICS: &[(&str, &str)] = &[
    ("index", include_str!("../docs/manual/index.md")),
    ("tree", include_str!("../docs/manual/tree.md")),
    ("files", include_str!("../docs/manual/files.md")),
    ("split", include_str!("../docs/manual/split.md")),
    ("tagging", include_str!("../docs/manual/tagging.md")),
    ("copy-move", include_str!("../docs/manual/copy-move.md")),
    ("picker", include_str!("../docs/manual/picker.md")),
    ("delete", include_str!("../docs/manual/delete.md")),
    ("rename", include_str!("../docs/manual/rename.md")),
    ("attributes", include_str!("../docs/manual/attributes.md")),
    ("filespec", include_str!("../docs/manual/filespec.md")),
    ("treespec", include_str!("../docs/manual/treespec.md")),
    ("compare", include_str!("../docs/manual/compare.md")),
    ("sort", include_str!("../docs/manual/sort.md")),
    ("execute", include_str!("../docs/manual/execute.md")),
    ("archives", include_str!("../docs/manual/archives.md")),
    ("viewer", include_str!("../docs/manual/viewer.md")),
    ("filter", include_str!("../docs/manual/filter.md")),
    ("search", include_str!("../docs/manual/search.md")),
    ("info", include_str!("../docs/manual/info.md")),
    ("usage", include_str!("../docs/manual/usage.md")),
    ("duplicates", include_str!("../docs/manual/duplicates.md")),
    ("terminal", include_str!("../docs/manual/terminal.md")),
    ("remote", include_str!("../docs/manual/remote.md")),
    ("cloud", include_str!("../docs/manual/cloud.md")),
    ("git", include_str!("../docs/manual/git.md")),
    ("bookmarks", include_str!("../docs/manual/bookmarks.md")),
    (
        "configuration",
        include_str!("../docs/manual/configuration.md"),
    ),
    ("quit", include_str!("../docs/manual/quit.md")),
    ("windows", include_str!("../docs/manual/windows.md")),
    ("ztreewin", include_str!("../docs/manual/ztreewin.md")),
    ("keys", include_str!("../docs/manual/keys.md")),
];

pub fn topic(name: &str) -> Option<&'static str> {
    TOPICS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, text)| *text)
}

/// Looks up topic `name` in language `lang`:
/// 1. `~/.config/xylem/manual/<lang>/<name>.md` on disk (so translators can
///    preview changes live without recompiling),
/// 2. embedded translations from `docs/manual/<lang>/<name>.md`,
/// 3. fallback to the canonical English topic (`docs/manual/<name>.md`).
pub fn topic_in(lang: &str, name: &str) -> Option<Cow<'static, str>> {
    // Only known topic names are valid, preventing path traversal.
    let en_text = topic(name)?;
    let code = normalize_lang(lang);
    if !code.is_empty()
        && let Some(base) = crate::platform::config_dir()
    {
        let custom = base.join("manual").join(&code).join(format!("{name}.md"));
        if let Ok(disk_md) = std::fs::read_to_string(custom)
            && disk_md.starts_with("# ")
        {
            return Some(Cow::Owned(disk_md));
        }
    }
    if code != "en"
        && let Some((_, topics)) = EMBEDDED_TRANSLATIONS.iter().find(|(l, _)| *l == code)
        && let Some((_, text)) = topics.iter().find(|(n, _)| *n == name)
    {
        return Some(Cow::Borrowed(*text));
    }
    Some(Cow::Borrowed(en_text))
}

/// The topic's title: its first `#` heading.
pub fn title(text: &str) -> &str {
    text.lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or("Help")
        .trim()
}

// ---------------------------------------------------------------- blocks

#[derive(Debug, Clone, PartialEq)]
enum Block {
    Title(String),
    Heading(String),
    Para(String),
    /// Nesting level (0 at the margin) and text.
    Item(usize, String),
    /// Rows of cells; the first row is the header (may be all empty).
    Table(Vec<Vec<String>>),
    Code(Vec<String>),
    Blank,
}

fn blocks(md: &str) -> Vec<Block> {
    let lines: Vec<&str> = md.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let starts_block = |l: &str| {
        let t = l.trim_start();
        t.is_empty()
            || l.starts_with('#')
            || t.starts_with("- ")
            || t.starts_with('|')
            || t.starts_with("```")
            || t.starts_with("![")
    };
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            if out.last() != Some(&Block::Blank) {
                out.push(Block::Blank);
            }
            i += 1;
        } else if let Some(t) = line.strip_prefix("# ") {
            out.push(Block::Title(t.trim().into()));
            i += 1;
        } else if let Some(t) = line.strip_prefix("## ").or(line.strip_prefix("### ")) {
            out.push(Block::Heading(t.trim().into()));
            i += 1;
        } else if trimmed.starts_with("![") {
            i += 1; // images only make sense on GitHub
        } else if trimmed.starts_with("```") {
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                code.push(lines[i].to_string());
                i += 1;
            }
            out.push(Block::Code(code));
            i += 1;
        } else if trimmed.starts_with('|') {
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('|') {
                let row = lines[i].trim();
                let cells: Vec<String> = row
                    .trim_start_matches('|')
                    .trim_end_matches('|')
                    .split('|')
                    .map(|c| c.trim().to_string())
                    .collect();
                let rule = cells
                    .iter()
                    .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'));
                if !rule {
                    rows.push(cells);
                }
                i += 1;
            }
            out.push(Block::Table(rows));
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            let indent = line.len() - trimmed.len();
            let mut text = rest.to_string();
            i += 1;
            // Continuation lines, indented past the dash.
            while i < lines.len()
                && !starts_block(lines[i])
                && lines[i].len() - lines[i].trim_start().len() > indent
            {
                text.push(' ');
                text.push_str(lines[i].trim());
                i += 1;
            }
            out.push(Block::Item(indent / 2, text));
        } else {
            let mut text = line.trim().to_string();
            i += 1;
            while i < lines.len() && !starts_block(lines[i]) {
                text.push(' ');
                text.push_str(lines[i].trim());
                i += 1;
            }
            out.push(Block::Para(text));
        }
    }
    out
}

// ---------------------------------------------------------------- inline

/// How a piece of text is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Heading,
    Strong,
    Code,
    /// A link to another topic (its number in [`Page::links`]).
    Link(usize),
}

#[derive(Debug, Clone, PartialEq)]
struct Span {
    text: String,
    kind: Kind,
}

/// Splits inline Markdown into spans; links to topics (`x.md`) are numbered
/// from `links.len()` and their targets pushed to `links`.
fn inline(text: &str, base: Kind, links: &mut Vec<String>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let flush = |plain: &mut String, out: &mut Vec<Span>| {
        if !plain.is_empty() {
            out.push(Span {
                text: std::mem::take(plain),
                kind: base,
            });
        }
    };
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let rest: String = chars[i..].iter().collect();
        if c == '\\' && i + 1 < chars.len() && chars[i + 1].is_ascii_punctuation() {
            plain.push(chars[i + 1]);
            i += 2;
        } else if c == '`'
            && let Some(len) = rest[1..].find('`')
        {
            flush(&mut plain, &mut out);
            let code: String = rest[1..1 + len].to_string();
            i += code.chars().count() + 2;
            out.push(Span {
                text: code,
                kind: Kind::Code,
            });
        } else if rest.starts_with("**")
            && let Some(len) = rest[2..].find("**")
        {
            flush(&mut plain, &mut out);
            let inner = rest[2..2 + len].to_string();
            i += inner.chars().count() + 4;
            for mut s in inline(&inner, Kind::Strong, links) {
                if s.kind == Kind::Text {
                    s.kind = Kind::Strong;
                }
                out.push(s);
            }
        } else if c == '['
            && let Some(close) = rest.find("](")
            && let Some(end) = rest[close..].find(')')
        {
            flush(&mut plain, &mut out);
            let label = &rest[1..close];
            let target = &rest[close + 2..close + end];
            i += rest[..close + end + 1].chars().count();
            let kind = match target.strip_suffix(".md") {
                Some(name) if !target.contains("://") => {
                    links.push(name.to_string());
                    Kind::Link(links.len() - 1)
                }
                _ => base,
            };
            for mut s in inline(label, base, links) {
                if matches!(kind, Kind::Link(_)) {
                    s.kind = kind;
                }
                out.push(s);
            }
        } else {
            plain.push(c);
            i += 1;
        }
    }
    flush(&mut plain, &mut out);
    out
}

/// Plain text of inline Markdown (for widths and the man page's tables).
fn plain(text: &str) -> String {
    inline(text, Kind::Text, &mut Vec::new())
        .into_iter()
        .map(|s| s.text)
        .collect()
}

// ---------------------------------------------------------------- layout

/// One piece of a screen line.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub text: String,
    pub kind: Kind,
}

/// A topic laid out for a width.
#[derive(Debug, Default)]
pub struct Page {
    pub lines: Vec<Vec<Piece>>,
    /// Targets of the links, and the first line each one is on.
    pub links: Vec<(String, usize)>,
}

/// Breaks `spans` into lines of `width` columns; continuation lines start
/// with `indent` spaces. Long words are cut.
fn wrap(spans: &[Span], width: usize, first: &str, indent: usize) -> Vec<Vec<Piece>> {
    let width = width.max(indent + 1);
    let new_line = |lines: &mut Vec<Vec<Piece>>| {
        lines.push(vec![Piece {
            text: " ".repeat(indent),
            kind: Kind::Text,
        }]);
    };
    let mut lines = vec![vec![Piece {
        text: first.to_string(),
        kind: Kind::Text,
    }]];
    let mut col = str_width(first);
    let mut start = true; // at the start of a line: no space before a word
    let mut pending = false; // a space comes before the next word
    for span in spans {
        for (k, word) in span.text.split(' ').enumerate() {
            // Inside a link, the space between its words belongs to it.
            let space_kind = if k > 0 && matches!(span.kind, Kind::Link(_)) {
                span.kind
            } else {
                Kind::Text
            };
            if k > 0 {
                pending = true;
            }
            if word.is_empty() {
                continue;
            }
            let mut word = word.to_string();
            loop {
                let len = str_width(&word);
                let gap = usize::from(pending && !start);
                if col + gap + len <= width {
                    let line = lines.last_mut().unwrap();
                    if gap == 1 {
                        line.push(Piece {
                            text: " ".into(),
                            kind: space_kind,
                        });
                    }
                    line.push(Piece {
                        text: word,
                        kind: span.kind,
                    });
                    col += gap + len;
                    break;
                }
                if !start {
                    new_line(&mut lines);
                    col = indent;
                    start = true;
                    continue;
                }
                // A word longer than the line: cut it.
                let room = width.saturating_sub(col).max(1);
                let (head, tail) = split_at_width(&word, room);
                lines.last_mut().unwrap().push(Piece {
                    text: head,
                    kind: span.kind,
                });
                word = tail;
                new_line(&mut lines);
                col = indent;
            }
            start = false;
            pending = false;
        }
    }
    lines
}

/// Widths for a table's columns within `width` (two spaces between them).
fn column_widths(rows: &[Vec<String>], width: usize) -> Vec<usize> {
    let n = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut natural = vec![0; n];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            natural[i] = natural[i].max(str_width(&plain(cell)));
        }
    }
    let gaps = 2 * n.saturating_sub(1);
    if natural.iter().sum::<usize>() + gaps <= width {
        return natural;
    }
    // The last column wraps; the others keep their width up to a share.
    let room = width.saturating_sub(gaps).max(n);
    let mut widths = natural.clone();
    for w in widths.iter_mut().take(n - 1) {
        *w = (*w).clamp(1, room / n);
    }
    let used: usize = widths.iter().take(n - 1).sum();
    widths[n - 1] = room - used;
    widths
}

/// Lays out topic `md` for `width` columns.
pub fn layout(md: &str, width: usize) -> Page {
    let width = width.max(20);
    let mut page = Page::default();
    let mut names: Vec<String> = Vec::new();
    let push = |page: &mut Page, lines: Vec<Vec<Piece>>| {
        page.lines.extend(lines);
    };
    for block in blocks(md) {
        match block {
            Block::Blank => push(&mut page, vec![Vec::new()]),
            // The help screen shows the title in its header.
            Block::Title(_) => {}
            Block::Heading(t) => {
                let spans = inline(&t, Kind::Heading, &mut names);
                push(&mut page, wrap(&spans, width, "", 0));
            }
            Block::Para(t) => {
                let spans = inline(&t, Kind::Text, &mut names);
                push(&mut page, wrap(&spans, width, "", 0));
            }
            Block::Item(level, t) => {
                let spans = inline(&t, Kind::Text, &mut names);
                let lead = format!("{}• ", "  ".repeat(level));
                let indent = str_width(&lead);
                push(&mut page, wrap(&spans, width, &lead, indent));
            }
            Block::Code(code) => {
                let lines = code
                    .into_iter()
                    .map(|l| {
                        let (text, _) = split_at_width(&format!("  {l}"), width);
                        vec![Piece {
                            text,
                            kind: Kind::Code,
                        }]
                    })
                    .collect();
                push(&mut page, lines);
            }
            Block::Table(rows) => {
                let widths = column_widths(&rows, width);
                let header_empty = rows.first().is_some_and(|r| r.iter().all(String::is_empty));
                for (r, row) in rows.iter().enumerate() {
                    if r == 0 && header_empty {
                        continue;
                    }
                    let base = if r == 0 { Kind::Strong } else { Kind::Text };
                    let cells: Vec<Vec<Vec<Piece>>> = widths
                        .iter()
                        .enumerate()
                        .map(|(i, w)| {
                            let cell = row.get(i).map(String::as_str).unwrap_or("");
                            let mut spans = inline(cell, base, &mut names);
                            if base == Kind::Strong {
                                for s in &mut spans {
                                    if s.kind == Kind::Text {
                                        s.kind = Kind::Strong;
                                    }
                                }
                            }
                            wrap(&spans, *w, "", 0)
                        })
                        .collect();
                    let height = cells.iter().map(Vec::len).max().unwrap_or(1);
                    for k in 0..height {
                        let mut line = Vec::new();
                        for (i, cell) in cells.iter().enumerate() {
                            let pieces = cell.get(k).cloned().unwrap_or_default();
                            let len: usize = pieces.iter().map(|p| str_width(&p.text)).sum();
                            line.extend(pieces);
                            if i + 1 < cells.len() {
                                line.push(Piece {
                                    text: " ".repeat(widths[i].saturating_sub(len) + 2),
                                    kind: Kind::Text,
                                });
                            }
                        }
                        page.lines.push(line);
                    }
                }
            }
        }
    }
    // Where each link first appears, to scroll to it.
    page.links = names.into_iter().map(|n| (n, usize::MAX)).collect();
    for (i, line) in page.lines.iter().enumerate() {
        for p in line {
            if let Kind::Link(id) = p.kind {
                let first = &mut page.links[id].1;
                *first = (*first).min(i);
            }
        }
    }
    // Leading and trailing blank lines are noise.
    let blank = |l: &Vec<Piece>| l.iter().all(|p| p.text.trim().is_empty());
    let lead = page.lines.iter().take_while(|l| blank(l)).count();
    if lead > 0 {
        page.lines.drain(..lead);
        for (_, line) in &mut page.links {
            *line = line.saturating_sub(lead);
        }
    }
    while page
        .lines
        .last()
        .is_some_and(|l| l.iter().all(|p| p.text.trim().is_empty()))
    {
        page.lines.pop();
    }
    page
}

// ---------------------------------------------------------------- state

/// The help screen: a topic, where it is scrolled to, the selected link and
/// the way back.
pub struct Help {
    pub topic: String,
    pub lang: String,
    pub scroll: usize,
    pub selected: Option<usize>,
    back: Vec<(String, usize, Option<usize>)>,
    /// Text of the "keys" topic: the keys in effect.
    keys: String,
    /// The page laid out for a width, until the topic or the width changes.
    page: Option<(usize, Page)>,
    /// Lines that fit on the screen, as of the last draw.
    pub height: usize,
}

impl Help {
    #[cfg(test)]
    pub fn new(topic: &str, keys: String) -> Self {
        Self::with_lang(topic, keys, "auto")
    }

    pub fn with_lang(topic: &str, keys: String, configured_lang: &str) -> Self {
        Help {
            topic: if self::topic(topic).is_some() {
                topic.to_string()
            } else {
                "index".into()
            },
            lang: resolve_language(configured_lang),
            scroll: 0,
            selected: None,
            back: Vec::new(),
            keys,
            page: None,
            height: 20,
        }
    }

    pub fn title(&self) -> String {
        let raw = topic_in(&self.lang, &self.topic).unwrap_or_default();
        title(&raw).to_string()
    }

    /// Switches to the next available help language (`L` in the help screen).
    pub fn cycle_language(&mut self) {
        let langs = available_languages();
        if langs.len() <= 1 {
            return;
        }
        let next = match langs.iter().position(|l| l == &self.lang) {
            Some(i) => (i + 1) % langs.len(),
            None => 0,
        };
        self.lang = langs[next].clone();
        self.selected = None;
        self.page = None;
    }

    /// The Markdown of the current topic ("keys" is generated).
    fn markdown(&self) -> String {
        let raw = topic_in(&self.lang, &self.topic).unwrap_or_default();
        if self.topic == "keys" {
            let mut md = format!("# {}\n\n", title(&raw));
            md.push_str(raw.lines().skip(1).collect::<Vec<_>>().join("\n").trim());
            md.push_str("\n\n```\n");
            md.push_str(&self.keys);
            md.push_str("```\n");
            md
        } else {
            raw.into_owned()
        }
    }

    /// The current topic laid out for `width`.
    pub fn page(&mut self, width: usize) -> &Page {
        if self.page.as_ref().is_none_or(|(w, _)| *w != width) {
            let page = layout(&self.markdown(), width);
            self.page = Some((width, page));
        }
        &self.page.as_ref().unwrap().1
    }

    fn links(&mut self) -> Vec<(String, usize)> {
        let width = self.page.as_ref().map_or(80, |(w, _)| *w);
        self.page(width).links.clone()
    }

    fn line_count(&mut self) -> usize {
        let width = self.page.as_ref().map_or(80, |(w, _)| *w);
        self.page(width).lines.len()
    }

    fn go(&mut self, topic: &str) {
        if self::topic(topic).is_none() || topic == self.topic {
            return;
        }
        self.back
            .push((std::mem::take(&mut self.topic), self.scroll, self.selected));
        self.topic = topic.to_string();
        self.scroll = 0;
        self.selected = None;
        self.page = None;
    }

    fn select(&mut self, forward: bool) {
        let links = self.links();
        if links.is_empty() {
            return;
        }
        let next = match (self.selected, forward) {
            (Some(i), true) => (i + 1) % links.len(),
            (Some(i), false) => (i + links.len() - 1) % links.len(),
            // The first link on screen (or the last one before it).
            (None, true) => links
                .iter()
                .position(|(_, line)| *line >= self.scroll)
                .unwrap_or(0),
            (None, false) => links
                .iter()
                .rposition(|(_, line)| *line < self.scroll + self.height)
                .unwrap_or(links.len() - 1),
        };
        self.selected = Some(next);
        let line = links[next].1;
        if line < self.scroll || line >= self.scroll + self.height {
            self.scroll = line.saturating_sub(self.height / 3);
        }
    }

    /// Handles a key; returns `false` when the help should close.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        let page = self.height.max(1);
        let max = self.line_count().saturating_sub(page);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('q') => return false,
            KeyCode::Char('h' | 'H') if alt => return false,
            KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Down => self.scroll = (self.scroll + 1).min(max),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(page),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll = (self.scroll + page).min(max),
            KeyCode::Home => self.scroll = 0,
            KeyCode::End => self.scroll = max,
            KeyCode::Tab => self.select(true),
            KeyCode::BackTab => self.select(false),
            KeyCode::Enter | KeyCode::Right => {
                if let Some(i) = self.selected
                    && let Some((target, _)) = self.links().get(i).cloned()
                {
                    self.go(&target);
                }
            }
            KeyCode::Backspace | KeyCode::Left => {
                if let Some((topic, scroll, selected)) = self.back.pop() {
                    self.topic = topic;
                    self.scroll = scroll;
                    self.selected = selected;
                    self.page = None;
                }
            }
            KeyCode::Char('i' | 'I') => self.go("index"),
            KeyCode::Char('k' | 'K') => self.go("keys"),
            KeyCode::Char('l' | 'L') => self.cycle_language(),
            _ => {}
        }
        true
    }
}

// ---------------------------------------------------------------- man page

fn roff_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => escaped.push_str("\\e"),
            '-' => escaped.push_str("\\-"),
            // Non-ASCII as roff escapes, so the page needs no preprocessor.
            c if !c.is_ascii() => escaped.push_str(&format!("\\[u{:04X}]", c as u32)),
            c => escaped.push(c),
        }
    }
    let text = escaped;
    if text.starts_with('.') || text.starts_with('\'') {
        format!("\\&{text}")
    } else {
        text
    }
}

/// Inline Markdown as roff: code and bold in bold, links as their text.
fn roff_inline(text: &str) -> String {
    let mut out = String::new();
    for span in inline(text, Kind::Text, &mut Vec::new()) {
        let t = roff_escape(&span.text);
        match span.kind {
            Kind::Code | Kind::Strong => out.push_str(&format!("\\fB{t}\\fR")),
            _ => out.push_str(&t),
        }
    }
    // A line starting with a dot would be a request.
    if out.starts_with('.') || out.starts_with('\'') {
        out.insert_str(0, "\\&");
    }
    out
}

fn roff_blocks(md: &str, out: &mut String) {
    for block in blocks(md) {
        match block {
            Block::Title(_) | Block::Blank => {}
            Block::Heading(t) => out.push_str(&format!(".SS {}\n", roff_inline(&t))),
            Block::Para(t) => out.push_str(&format!(".PP\n{}\n", roff_inline(&t))),
            Block::Item(level, t) => {
                out.push_str(&format!(
                    ".IP \\(bu {}\n{}\n",
                    2 + 2 * level,
                    roff_inline(&t)
                ));
            }
            Block::Code(code) => {
                out.push_str(".PP\n.RS 4\n.nf\n");
                for l in code {
                    out.push_str(&roff_escape(&l));
                    out.push('\n');
                }
                out.push_str(".fi\n.RE\n");
            }
            // Each row as a tagged paragraph: the first cell as the tag,
            // the others (with their headers, if any) as the text.
            Block::Table(rows) => {
                let header = rows.first().cloned().unwrap_or_default();
                let named = header.iter().skip(1).any(|h| !h.is_empty()) && header.len() > 2;
                for row in rows.iter().skip(1) {
                    let tag = row.first().map(String::as_str).unwrap_or("");
                    let rest: Vec<String> = row
                        .iter()
                        .enumerate()
                        .skip(1)
                        .filter(|(_, c)| !c.is_empty())
                        .map(
                            |(i, c)| match header.get(i).filter(|h| named && !h.is_empty()) {
                                Some(h) => format!("{}: {}", plain(h), roff_inline(c)),
                                None => roff_inline(c),
                            },
                        )
                        .collect();
                    out.push_str(&format!(
                        ".TP\n\\fB{}\\fR\n{}\n",
                        roff_escape(&plain(tag)),
                        rest.join("; ")
                    ));
                }
            }
        }
    }
}

/// The man page, `xy(1)`, in language `lang` (falling back to English per topic).
pub fn man_page_in(lang: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut out = format!(
        ".TH XY 1 \"\" \"Xylem {version}\" \"User Commands\"\n\
         .SH NAME\n\
         xy \\- text\\-mode file manager inspired by XTree and ZTreeWin\n\
         .SH SYNOPSIS\n\
         .B xy\n\
         [\\fB\\-\\-cwd\\-file\\fR \\fIFILE\\fR] [\\fIDIRECTORY\\fR]\n\
         .SH OPTIONS\n\
         .TP\n\\fB\\-\\-cwd\\-file\\fR \\fIFILE\\fR\n\
         On Alt+Q (quit and change directory), write the current directory to \\fIFILE\\fR, \
         for a shell function to cd into it (see QUITTING AND CHANGING THE SHELL'S DIRECTORY).\n\
         .TP\n\\fB\\-\\-default\\-config\\fR\nPrint a commented configuration file.\n\
         .TP\n\\fB\\-\\-man\\fR\nShow this manual page; with the output redirected, print its source, \
         as in \\fBxy \\-\\-man > ~/.local/share/man/man1/xy.1\\fR.\n\
         .TP\n\\fB\\-h\\fR, \\fB\\-\\-help\\fR\nShow the options.\n\
         .TP\n\\fB\\-V\\fR, \\fB\\-\\-version\\fR\nShow the version.\n\
         .SH DESCRIPTION\n"
    );
    // The index's opening paragraphs describe Xylem; its link lists are for
    // the help screen.
    let index_text = topic_in(lang, "index").unwrap_or_default();
    for block in blocks(&index_text) {
        if let Block::Heading(_) = block {
            break;
        }
        if let Block::Para(t) = block {
            out.push_str(&format!(".PP\n{}\n", roff_inline(&t)));
        }
    }
    for (name, _) in TOPICS {
        if matches!(*name, "index" | "keys") {
            continue;
        }
        let text = topic_in(lang, name).unwrap_or_default();
        out.push_str(&format!(
            ".SH {}\n",
            roff_escape(&title(&text).to_uppercase())
        ));
        roff_blocks(&text, &mut out);
    }
    out.push_str(
        ".SH FILES\n\
         .TP\n\\fI~/.config/xylem/config.toml\\fR\nConfiguration (\\fB$XDG_CONFIG_HOME\\fR is honored).\n\
         .TP\n\\fI~/.local/state/xylem/history.toml\\fR\nPrompt history and bookmarks.\n\
         .SH SEE ALSO\n\
         The manual, also inside Xylem with F1 or Alt+H: https://github.com/andrelnfernandes/xylem/tree/main/docs/manual\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(page: &Page) -> Vec<String> {
        page.lines
            .iter()
            .map(|l| l.iter().map(|p| p.text.as_str()).collect::<String>())
            .collect()
    }

    #[test]
    fn every_link_points_to_a_topic() {
        let mut all: Vec<(&str, &str, &str)> =
            TOPICS.iter().map(|(n, md)| ("en", *n, *md)).collect();
        for (lang, topics) in EMBEDDED_TRANSLATIONS {
            for (name, md) in *topics {
                assert!(
                    topic(name).is_some(),
                    "{lang}/{name}.md: unknown topic name (not in English TOPICS)"
                );
                all.push((lang, name, md));
            }
        }
        for (lang, name, md) in all {
            assert!(
                md.starts_with("# "),
                "{lang}/{name}: starts with its `# ` title"
            );
            let mut links = Vec::new();
            for block in blocks(md) {
                let t = match block {
                    Block::Para(t) | Block::Item(_, t) | Block::Heading(t) => t,
                    Block::Table(rows) => rows.concat().join(" "),
                    _ => continue,
                };
                inline(&t, Kind::Text, &mut links);
            }
            for target in links {
                assert!(
                    topic(&target).is_some(),
                    "{lang}/{name}: link to unknown topic {target}"
                );
            }
        }
    }

    #[test]
    fn index_links_to_every_topic() {
        let mut indices = vec![("en", topic("index").unwrap())];
        for (lang, topics) in EMBEDDED_TRANSLATIONS {
            if let Some((_, md)) = topics.iter().find(|(n, _)| *n == "index") {
                indices.push((lang, *md));
            }
        }
        for (lang, index_md) in indices {
            let mut links = Vec::new();
            for block in blocks(index_md) {
                if let Block::Item(_, t) | Block::Para(t) = block {
                    inline(&t, Kind::Text, &mut links);
                }
            }
            for (name, _) in TOPICS {
                let reached =
                    *name == "index" || *name == "keys" || links.iter().any(|l| l == name);
                assert!(reached, "{lang}/index.md does not link to {name}");
            }
        }
    }

    #[test]
    fn lays_out_paragraphs_lists_tables_and_links() {
        let md = "# Title\n\nSome `code` and **bold** text that is long enough to wrap \
                  around.\n\n- an item that also wraps around the narrow width\n  and continues\n\n\
                  | Key | What |\n|---|---|\n| `F` | the [filter](filespec.md) |\n";
        let page = layout(md, 30);
        let lines = text(&page);
        // The title is in the screen's header, not in the text.
        assert!(
            lines[0].starts_with("Some code and bold text"),
            "{lines:#?}"
        );
        assert!(lines.iter().all(|l| str_width(l) <= 30), "{lines:#?}");
        assert!(
            lines.iter().any(|l| l.starts_with("• an item")),
            "{lines:#?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("  ") && l.contains("continues")),
            "{lines:#?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("Key") && l.contains("What")),
            "{lines:#?}"
        );
        assert_eq!(page.links.len(), 1);
        assert_eq!(page.links[0].0, "filespec");
        let link_line = &page.lines[page.links[0].1];
        assert!(
            link_line
                .iter()
                .any(|p| p.kind == Kind::Link(0) && p.text == "filter")
        );
        assert!(
            page.lines
                .iter()
                .flatten()
                .any(|p| p.kind == Kind::Code && p.text == "code")
        );
    }

    #[test]
    fn every_topic_lays_out_at_any_width() {
        let mut all: Vec<(&str, &str, &str)> =
            TOPICS.iter().map(|(n, md)| ("en", *n, *md)).collect();
        for (lang, topics) in EMBEDDED_TRANSLATIONS {
            for (name, md) in *topics {
                all.push((lang, name, md));
            }
        }
        for (lang, name, md) in all {
            for width in [20, 40, 80, 200] {
                let page = layout(md, width);
                for line in text(&page) {
                    assert!(
                        str_width(&line) <= width.max(20),
                        "{lang}/{name} at {width}: {line:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn navigation_follows_links_and_goes_back() {
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        let mut help = Help::with_lang("index", "F1  help\n".into(), "en");
        help.height = 10;
        help.page(80);
        assert!(help.handle_key(key(KeyCode::Tab)));
        let first = help.links()[help.selected.unwrap()].0.clone();
        help.handle_key(key(KeyCode::Enter));
        assert_eq!(help.topic, first);
        help.handle_key(key(KeyCode::Char('k')));
        assert_eq!(help.topic, "keys");
        assert!(help.markdown().contains("F1  help"));
        help.handle_key(key(KeyCode::Backspace));
        help.handle_key(key(KeyCode::Backspace));
        assert_eq!(help.topic, "index");
        assert!(!help.handle_key(key(KeyCode::Esc)));
        // An unknown topic opens the index.
        assert_eq!(Help::new("nope", String::new()).topic, "index");
    }

    #[test]
    fn man_page_has_every_topic() {
        let requests = [
            "TH", "SH", "SS", "PP", "IP", "TP", "RS", "RE", "nf", "fi", "B",
        ];
        for lang in available_languages() {
            let man = man_page_in(&lang);
            assert!(man.starts_with(".TH XY 1"));
            for (name, _) in TOPICS {
                if !matches!(*name, "index" | "keys") {
                    let md = topic_in(&lang, name).unwrap();
                    assert!(
                        man.contains(&format!(".SH {}", roff_escape(&title(&md).to_uppercase()))),
                        "{lang}/{name} missing in man page"
                    );
                }
            }
            // No text line starts with a dot: those would be read as requests.
            for line in man.lines() {
                if let Some(rest) = line.strip_prefix('.') {
                    let name = rest.split_whitespace().next().unwrap_or("");
                    assert!(requests.contains(&name), "{lang}: {line}");
                }
            }
        }
    }

    #[test]
    fn cycles_languages_with_l_and_falls_back_to_english() {
        let langs = available_languages();
        for expected in ["en", "pt", "es", "fr", "de", "zh"] {
            assert!(
                langs.iter().any(|l| l == expected),
                "missing language {expected} in {langs:?}"
            );
        }
        let mut help = Help::with_lang("index", "F1  help\n".into(), "en");
        let en_title = help.title();
        assert_eq!(help.lang, "en");
        help.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert_eq!(help.lang, "pt");
        assert_ne!(help.title(), en_title);

        // Unknown language code falls back to English.
        assert_eq!(resolve_language("xx_YY"), "en");
        assert_eq!(
            topic_in("xx", "index").unwrap().as_ref(),
            topic("index").unwrap()
        );
    }
}
