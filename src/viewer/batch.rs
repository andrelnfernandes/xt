//! Batch (Ctrl+B) over what the viewer shows: a line of output per record of a
//! table or per line of text, from a mask with fields such as `%{name:url}`.
//!
//! The values come from a file that may be hostile and go into something that
//! may run, such as a script. So each field has an escape, chosen by the output
//! file's extension unless the mask names another, and raw output must be asked
//! for by writing it.

use std::borrow::Cow;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use regex::Regex;

use super::records::{self, Budget, Cell, End, RecordSink};
use super::sql_filter::{self, Expr, RowContext, Value};
use super::table::TableGrid;
use crate::bidi;
use crate::shell;

/// How many records the preview looks at, also to find late columns.
const SCAN_RECORDS: u64 = 1_000;
/// How many lines of output the preview shows.
const PREVIEW_LINES: usize = 20;
/// How long the preview may take.
const SCAN_TIME: Duration = Duration::from_secs(3);

/// How a value goes into the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escape {
    /// One argument of a POSIX shell command, quoted.
    Shell,
    /// Percent-encoded, for a part of a URL.
    Url,
    /// A JSON value: numbers as numbers, text with its quotes.
    Json,
    /// Text or attribute value of XML or HTML.
    Xml,
    /// A SQL literal, with its quotes.
    Sql,
    /// A CSV field, quoted when needed.
    Csv,
    /// A CSV field that a spreadsheet will not take for a formula.
    CsvSafe,
    /// The value itself, with control characters as spaces, so that it never
    /// starts a new line of output.
    Line,
    /// The value itself.
    Raw,
}

impl Escape {
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "sh" => Escape::Shell,
            "url" => Escape::Url,
            "json" => Escape::Json,
            "xml" | "html" => Escape::Xml,
            "sql" => Escape::Sql,
            "csv" => Escape::Csv,
            "csvsafe" => Escape::CsvSafe,
            "line" => Escape::Line,
            "raw" => Escape::Raw,
            _ => return None,
        })
    }

    /// The escape of a field that does not name one.
    pub fn for_file(file: &Path) -> Self {
        let ext = extension(file);
        match ext.as_str() {
            "sh" | "bash" | "zsh" => Escape::Shell,
            "json" | "jsonl" | "ndjson" => Escape::Json,
            "xml" | "html" | "htm" | "xhtml" | "svg" => Escape::Xml,
            "sql" => Escape::Sql,
            "csv" | "tsv" => Escape::Csv,
            _ => Escape::Line,
        }
    }
}

fn extension(file: &Path) -> String {
    file.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

/// `cmd` expands `%VAR%` even inside quotes, and a value from a file could use
/// that. So there is no safe way to put one in these scripts yet.
pub fn refuses_records(file: &Path) -> bool {
    matches!(extension(file).as_str(), "bat" | "cmd")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The value as text, with no escape.
fn plain<'a>(cell: &'a Cell<'_>) -> Cow<'a, str> {
    match cell {
        Cell::Null => Cow::Borrowed(""),
        Cell::Bool(b) => Cow::Owned(b.to_string()),
        Cell::Int(i) => Cow::Owned(i.to_string()),
        Cell::Float(f) => Cow::Owned(f.to_string()),
        Cell::Text(s) => Cow::Borrowed(s),
        Cell::Bytes(b) => Cow::Owned(hex(b)),
    }
}

fn shell_quote(text: &str, out: &mut Vec<u8>) {
    out.push(b'\'');
    // A NUL cannot be in an argument, and sh stops reading a line at it.
    for piece in text.replace('\0', "").split_inclusive('\'') {
        match piece.strip_suffix('\'') {
            Some(head) => {
                out.extend_from_slice(head.as_bytes());
                out.extend_from_slice(b"'\\''");
            }
            None => out.extend_from_slice(piece.as_bytes()),
        }
    }
    out.push(b'\'');
}

fn csv_field(text: &str, out: &mut Vec<u8>) {
    if text.contains([',', ';', '\t', '"', '\n', '\r']) {
        out.push(b'"');
        out.extend_from_slice(text.replace('"', "\"\"").as_bytes());
        out.push(b'"');
    } else {
        out.extend_from_slice(text.as_bytes());
    }
}

fn starts_like_formula(text: &str) -> bool {
    text.starts_with(['=', '+', '-', '@', '\t', '\r'])
}

/// Writes the value of `cell` to `out`.
fn escape_into(cell: &Cell<'_>, escape: Escape, out: &mut Vec<u8>) {
    match escape {
        Escape::Raw => match cell {
            Cell::Bytes(b) => out.extend_from_slice(b),
            other => out.extend_from_slice(plain(other).as_bytes()),
        },
        Escape::Line => {
            let text = plain(cell);
            let clean: String = text
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect();
            out.extend_from_slice(clean.as_bytes());
        }
        Escape::Shell => shell_quote(&plain(cell), out),
        Escape::Url => {
            for b in plain(cell).bytes() {
                if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                    out.push(b);
                } else {
                    out.extend_from_slice(format!("%{b:02X}").as_bytes());
                }
            }
        }
        Escape::Json => {
            let value = match cell {
                Cell::Null => serde_json::Value::Null,
                Cell::Bool(b) => serde_json::Value::Bool(*b),
                Cell::Int(i) => serde_json::Value::from(*i),
                // Not a number in JSON (NaN, infinity) comes out as null.
                Cell::Float(f) => serde_json::Value::from(*f),
                other => serde_json::Value::String(plain(other).into_owned()),
            };
            out.extend_from_slice(value.to_string().as_bytes());
        }
        Escape::Xml => {
            for c in plain(cell).chars() {
                match c {
                    '&' => out.extend_from_slice(b"&amp;"),
                    '<' => out.extend_from_slice(b"&lt;"),
                    '>' => out.extend_from_slice(b"&gt;"),
                    '"' => out.extend_from_slice(b"&quot;"),
                    '\'' => out.extend_from_slice(b"&#39;"),
                    // XML 1.0 has no way to write these.
                    c if c.is_control() && !matches!(c, '\t' | '\n' | '\r') => {
                        out.extend_from_slice("\u{FFFD}".as_bytes());
                    }
                    c => {
                        let mut buf = [0; 4];
                        out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                    }
                }
            }
        }
        Escape::Sql => match cell {
            Cell::Null => out.extend_from_slice(b"NULL"),
            Cell::Bool(b) => out.extend_from_slice(if *b { b"TRUE" } else { b"FALSE" }),
            Cell::Int(i) => out.extend_from_slice(i.to_string().as_bytes()),
            Cell::Float(f) if f.is_finite() => out.extend_from_slice(f.to_string().as_bytes()),
            Cell::Float(_) => out.extend_from_slice(b"NULL"),
            Cell::Bytes(b) => out.extend_from_slice(format!("X'{}'", hex(b)).as_bytes()),
            Cell::Text(s) => {
                out.push(b'\'');
                out.extend_from_slice(s.replace('\0', "").replace('\'', "''").as_bytes());
                out.push(b'\'');
            }
        },
        Escape::Csv => csv_field(&plain(cell), out),
        Escape::CsvSafe => {
            let text = plain(cell);
            if matches!(cell, Cell::Text(_)) && starts_like_formula(&text) {
                csv_field(&format!("'{text}"), out);
            } else {
                csv_field(&text, out);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Part {
    Literal(Vec<u8>),
    Field { name: String, escape: Escape },
}

/// A parsed line mask.
#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    parts: Vec<Part>,
}

impl Mask {
    /// `%{name}` and `%{name:escape}` are fields of the record; `%f %d %n %b
    /// %e %s` are the open file's, as in the Batch of tagged files; `%%` is a `%`.
    pub fn parse(text: &str, default: Escape, file: &Path, size: u64) -> Result<Self, String> {
        if text.trim().is_empty() {
            return Err("Batch cancelled: empty line mask".into());
        }
        let mut parts = Vec::new();
        let mut literal = Vec::new();
        let mut rest = text;
        while let Some(at) = rest.find('%') {
            literal.extend_from_slice(&rest.as_bytes()[..at]);
            let after = &rest[at + 1..];
            if let Some(inner) = after.strip_prefix('{') {
                let Some(close) = inner.find('}') else {
                    return Err("Mask: %{ without its }".into());
                };
                let (name, escape) = match inner[..close].rsplit_once(':') {
                    Some((name, wanted)) => {
                        let escape = Escape::named(wanted)
                            .ok_or_else(|| format!("Mask: unknown escape :{wanted}"))?;
                        (name, escape)
                    }
                    None => (&inner[..close], default),
                };
                if name.is_empty() {
                    return Err("Mask: %{} has no field name".into());
                }
                if !literal.is_empty() {
                    parts.push(Part::Literal(std::mem::take(&mut literal)));
                }
                parts.push(Part::Field {
                    name: name.to_string(),
                    escape,
                });
                rest = &inner[close + 1..];
            } else if let Some(c) = after.chars().next()
                && matches!(
                    c.to_ascii_lowercase(),
                    'f' | 'd' | 'n' | 'b' | 'e' | 's' | '%'
                )
            {
                let var = format!("%{c}");
                literal.extend(shell::batch_line(&var, file, size));
                rest = &after[c.len_utf8()..];
            } else {
                literal.push(b'%');
                rest = after;
            }
        }
        literal.extend_from_slice(rest.as_bytes());
        if !literal.is_empty() {
            parts.push(Part::Literal(literal));
        }
        Ok(Mask { parts })
    }

    /// The first field that `names` has no answer for.
    fn unknown(&self, row: &Row<'_>) -> Option<&str> {
        self.parts.iter().find_map(|part| match part {
            Part::Field { name, .. } if !row.knows(name) => Some(name.as_str()),
            _ => None,
        })
    }

    fn render(&self, row: &Row<'_>, stats: &mut Stats, out: &mut Vec<u8>) {
        for part in &self.parts {
            match part {
                Part::Literal(bytes) => out.extend_from_slice(bytes),
                Part::Field { name, escape } => {
                    let cell = row.value(name);
                    stats.look_at(&cell, *escape);
                    escape_into(&cell, *escape, out);
                }
            }
        }
    }
}

/// Values worth a warning.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// With control or text direction characters.
    pub suspicious: u64,
    /// Going into a CSV starting like a spreadsheet formula.
    pub formulas: u64,
}

impl Stats {
    fn look_at(&mut self, cell: &Cell<'_>, escape: Escape) {
        let Cell::Text(text) = cell else { return };
        if text
            .chars()
            .any(|c| (c.is_control() && c != '\t') || bidi::is_bidi(c))
        {
            self.suspicious += 1;
        }
        if escape == Escape::Csv && starts_like_formula(text) {
            self.formulas += 1;
        }
    }

    /// The warnings, in one line.
    pub fn warning(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.suspicious > 0 {
            parts.push(format!(
                "{} values with control or text direction characters",
                self.suspicious
            ));
        }
        if self.formulas > 0 {
            parts.push(format!(
                "{} values a spreadsheet may take for formulas (:csvsafe avoids it)",
                self.formulas
            ));
        }
        (!parts.is_empty()).then(|| parts.join(", "))
    }
}

/// A record, as a mask sees it.
struct Row<'a> {
    names: &'a [String],
    cells: &'a [Cell<'a>],
    /// 1, 2, 3... among the records that passed the filter.
    sequence: u64,
    /// The line's number in the file (text only).
    line: Option<u64>,
    /// Columns can be reached by letter and position (tables only).
    positional: bool,
}

impl Row<'_> {
    fn index(&self, name: &str) -> Option<usize> {
        if let Some(i) = self.names.iter().position(|n| n.eq_ignore_ascii_case(name)) {
            return Some(i);
        }
        if !self.positional {
            return None;
        }
        if let Ok(n) = name.parse::<usize>() {
            return (n >= 1 && n <= self.names.len()).then(|| n - 1);
        }
        (0..self.names.len()).find(|&i| records::column_letter(i).eq_ignore_ascii_case(name))
    }

    fn knows(&self, name: &str) -> bool {
        name == "#" || (name == "#line" && self.line.is_some()) || self.index(name).is_some()
    }

    fn value(&self, name: &str) -> Cell<'_> {
        match name {
            "#" => Cell::Int(self.sequence as i64),
            "#line" if self.line.is_some() => Cell::Int(self.line.unwrap_or(0) as i64),
            _ => self
                .index(name)
                .and_then(|i| self.cells.get(i))
                .cloned()
                .unwrap_or(Cell::Null),
        }
    }
}

/// What a Batch reads: everything is read again, or kept, away from the screen.
pub enum Source {
    Table {
        path: PathBuf,
        /// The sheet of a spreadsheet.
        sheet: usize,
        /// The table of a database.
        table: Option<String>,
        filter: Option<Expr>,
        /// The most a spreadsheet may unpack to.
        unpacked: u64,
    },
    Text {
        /// The file decoded, with its tabs and control characters.
        text: String,
        /// How many lines the viewer has of it.
        total: usize,
        /// The lines the filter kept.
        kept: Option<Vec<usize>>,
        /// The filter's regular expression, for its groups.
        regex: Option<Regex>,
    },
}

/// Applies a table's filter to a stream, with the rules of the screen: a value
/// is judged by what the grid would show, so both keep the same rows.
struct TableSink<'a, F> {
    names: Vec<String>,
    shown_names: Vec<String>,
    filter: Option<&'a Expr>,
    sequence: u64,
    deliver: F,
}

struct CellContext<'a> {
    names: &'a [String],
    cells: &'a [Cell<'a>],
}

impl RowContext for CellContext<'_> {
    fn get_value(&self, column: &str) -> Option<Value> {
        let by_name = self
            .names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(column));
        let index = by_name.or_else(|| match column.as_bytes() {
            [b] if b.is_ascii_alphabetic() => Some((b.to_ascii_uppercase() - b'A') as usize),
            _ => None,
        })?;
        self.cells.get(index).map(|c| Value::infer(&c.display()))
    }
}

impl<F: FnMut(&Row<'_>) -> ControlFlow<()>> RecordSink for TableSink<'_, F> {
    fn sheets(&mut self, _: &[String], _: usize) {}

    fn columns(&mut self, names: &[String]) {
        self.names = names.to_vec();
        self.shown_names = names.iter().map(|n| TableGrid::sanitize_cell(n)).collect();
    }

    fn columns_cut(&mut self) {}

    fn record(&mut self, cells: &[Cell<'_>]) -> ControlFlow<()> {
        if let Some(filter) = self.filter {
            let ctx = CellContext {
                names: &self.shown_names,
                cells,
            };
            let keep = sql_filter::eval_expr(filter, &ctx).is_ok_and(|v| v.is_truthy());
            if !keep {
                return ControlFlow::Continue(());
            }
        }
        self.sequence += 1;
        (self.deliver)(&Row {
            names: &self.names,
            cells,
            sequence: self.sequence,
            line: None,
            positional: true,
        })
    }
}

impl Source {
    /// The names a text record has: the whole line and the filter's groups.
    fn text_names(regex: Option<&Regex>) -> Vec<String> {
        let mut names = vec!["line".to_string()];
        if let Some(regex) = regex {
            for (i, name) in regex.capture_names().enumerate() {
                names.push(i.to_string());
                if let Some(name) = name {
                    names.push(name.to_string());
                }
            }
        }
        names
    }

    /// What can be asked of this source, for the prompt.
    pub fn field_hint(&self, columns: &[String]) -> String {
        match self {
            Source::Table { .. } => {
                let shown: Vec<&str> = columns.iter().take(6).map(String::as_str).collect();
                let more = if columns.len() > shown.len() {
                    ", ..."
                } else {
                    ""
                };
                format!("{}{more}, #", shown.join(", "))
            }
            Source::Text { regex, .. } => {
                let names = Self::text_names(regex.as_ref());
                format!("{}, #, #line", names.join(", "))
            }
        }
    }

    /// Reads the source, stopping when `deliver` says so or `budget` runs out.
    fn run(
        &self,
        budget: Budget,
        deliver: &mut dyn FnMut(&Row<'_>) -> ControlFlow<()>,
    ) -> Result<End, String> {
        match self {
            Source::Table {
                path,
                sheet,
                table,
                filter,
                ..
            } => {
                let mut sink = TableSink {
                    names: Vec::new(),
                    shown_names: Vec::new(),
                    filter: filter.as_ref(),
                    sequence: 0,
                    deliver,
                };
                records::read_table(path, *sheet, table.as_deref(), budget, &mut sink)
            }
            Source::Text {
                text,
                total,
                kept,
                regex,
            } => {
                let names = Self::text_names(regex.as_ref());
                // Split as the viewer splits, so that a line's number is the same.
                let body = text.strip_suffix('\n').unwrap_or(text);
                let mut wanted = kept.as_ref().map(|k| k.iter().copied().peekable());
                let mut sequence = 0;
                for (i, line) in body.split('\n').take(*total).enumerate() {
                    if let Some(w) = &mut wanted {
                        if w.peek() != Some(&i) {
                            continue;
                        }
                        w.next();
                    }
                    if sequence & 0xFF == 0 && budget.over() {
                        return Ok(End::OutOfTime);
                    }
                    let line = line.strip_suffix('\r').unwrap_or(line);
                    let mut cells = vec![Cell::Text(Cow::Borrowed(line))];
                    if let Some(regex) = regex {
                        let captures = regex.captures(line);
                        for (g, name) in regex.capture_names().enumerate() {
                            let value = captures
                                .as_ref()
                                .and_then(|c| c.get(g))
                                .map_or(Cell::Null, |m| Cell::Text(Cow::Borrowed(m.as_str())));
                            if name.is_some() {
                                cells.push(value.clone());
                            }
                            cells.push(value);
                        }
                    }
                    sequence += 1;
                    let row = Row {
                        names: &names,
                        cells: &cells,
                        sequence,
                        line: Some(i as u64 + 1),
                        positional: false,
                    };
                    if deliver(&row).is_break() {
                        return Ok(End::Refused);
                    }
                }
                Ok(End::Complete)
            }
        }
    }

    fn unpacked(&self) -> u64 {
        match self {
            Source::Table { unpacked, .. } => *unpacked,
            Source::Text { .. } => 0,
        }
    }
}

/// What the preview found.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    /// The first lines of output.
    pub lines: Vec<String>,
    /// How many records it looked at.
    pub scanned: u64,
    /// Those are all of them.
    pub complete: bool,
    pub stats: Stats,
}

/// A line of output, safe for the screen: it may hold anything.
fn shown(line: &[u8]) -> String {
    TableGrid::sanitize_cell(&String::from_utf8_lossy(line))
}

/// Reads the start of `source` through `mask`. A field that no record among
/// those read has is an error here, so that it never comes out empty.
pub fn preview(source: &Source, mask: &Mask) -> Result<Preview, String> {
    let budget = Budget::new(SCAN_TIME, source.unpacked());
    let mut lines = Vec::new();
    let mut scanned = 0;
    let mut stats = Stats::default();
    let mut unknown: Option<String> = None;
    let mut line = Vec::new();
    let end = source.run(budget, &mut |row| {
        if scanned >= SCAN_RECORDS {
            return ControlFlow::Break(());
        }
        scanned += 1;
        // The last answer counts: JSON Lines finds columns as it goes.
        unknown = mask.unknown(row).map(str::to_string);
        line.clear();
        mask.render(row, &mut stats, &mut line);
        if lines.len() < PREVIEW_LINES {
            lines.push(shown(&line));
        }
        ControlFlow::Continue(())
    })?;
    if let Some(name) = unknown {
        return Err(format!("Mask: no field named {name}"));
    }
    Ok(Preview {
        lines,
        scanned,
        complete: end == End::Complete,
        stats,
    })
}

/// How a Batch that ran ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Done {
    pub records: u64,
    pub stats: Stats,
    pub cancelled: bool,
}

/// A Batch being written, in a thread of its own: a read can take long, and Esc
/// has to work meanwhile.
pub struct Job {
    pub file: PathBuf,
    cancel: Arc<AtomicBool>,
    written: Arc<AtomicU64>,
    handle: JoinHandle<Result<Done, String>>,
}

impl Job {
    pub fn start(source: Source, mask: Mask, file: PathBuf) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let written = Arc::new(AtomicU64::new(0));
        let handle = {
            let (cancel, written, file) = (cancel.clone(), written.clone(), file.clone());
            thread::spawn(move || {
                let result = crate::panic_guard::catch(|| {
                    write_file(&source, &mask, &file, cancel, &written)
                })
                .unwrap_or_else(|| Err("the file could not be read".into()));
                // Nothing incomplete stays behind, unless it was never ours.
                let keep = matches!(
                    result,
                    Ok(Done {
                        cancelled: false,
                        ..
                    })
                );
                if !keep && !matches!(&result, Err(e) if e.starts_with(NOT_CREATED)) {
                    let _ = fs::remove_file(&file);
                }
                result
            })
        };
        Job {
            file,
            cancel,
            written,
            handle,
        }
    }

    /// Asks it to stop; it ends at the next record.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelling(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn written(&self) -> u64 {
        self.written.load(Ordering::Relaxed)
    }

    pub fn is_done(&self) -> bool {
        self.handle.is_finished()
    }

    pub fn finish(self) -> Result<Done, String> {
        self.handle
            .join()
            .unwrap_or_else(|_| Err("the file could not be read".into()))
    }
}

const NOT_CREATED: &str = "Cannot create ";

fn write_file(
    source: &Source,
    mask: &Mask,
    file: &Path,
    cancel: Arc<AtomicBool>,
    written: &AtomicU64,
) -> Result<Done, String> {
    let created = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)
        .map_err(|e| format!("{NOT_CREATED}{}: {e}", file.display()))?;
    let mut out = BufWriter::new(created);
    let newline: &[u8] = if extension(file) == "sh" {
        b"\n"
    } else {
        shell::NEWLINE
    };
    if extension(file) == "sh" {
        out.write_all(b"#!/bin/sh\n").map_err(|e| e.to_string())?;
    }

    let budget = Budget::cancellable(source.unpacked(), cancel);
    let mut stats = Stats::default();
    let mut records = 0;
    let mut line = Vec::new();
    let mut failure = None;
    let end = source.run(budget, &mut |row| {
        line.clear();
        mask.render(row, &mut stats, &mut line);
        line.extend_from_slice(newline);
        if let Err(e) = out.write_all(&line) {
            failure = Some(e);
            return ControlFlow::Break(());
        }
        records += 1;
        written.store(records, Ordering::Relaxed);
        ControlFlow::Continue(())
    })?;
    if let Some(e) = failure {
        return Err(format!("Cannot write {}: {e}", file.display()));
    }
    let flushed: Result<File, _> = out.into_inner();
    flushed.map_err(|e| format!("Cannot write {}: {e}", file.display()))?;
    Ok(Done {
        records,
        stats,
        cancelled: end == End::OutOfTime,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn mask(text: &str, default: Escape) -> Mask {
        Mask::parse(text, default, Path::new("/d/a.csv"), 10).unwrap()
    }

    fn escaped(cell: Cell<'_>, escape: Escape) -> String {
        let mut out = Vec::new();
        escape_into(&cell, escape, &mut out);
        String::from_utf8(out).unwrap()
    }

    fn t(s: &str) -> Cell<'_> {
        Cell::Text(s.into())
    }

    /// Runs a command line through the real shell and returns what it prints.
    #[cfg(unix)]
    fn through_sh(line: &[u8]) -> Vec<u8> {
        use std::os::unix::ffi::OsStrExt;
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(std::ffi::OsStr::from_bytes(line))
            .env_remove("HOSTILE")
            .output()
            .unwrap();
        out.stdout
    }

    #[cfg(unix)]
    #[test]
    fn a_hostile_value_is_one_argument_in_a_real_shell() {
        let hostile = [
            "$(touch /nonexistent/pwned)",
            "`id`",
            "a'b",
            "'; echo injected; '",
            "line\nbreak",
            "* ? ~",
            "-n",
            "",
        ];
        for value in hostile {
            let mut line = b"printf '%s' ".to_vec();
            shell_quote(value, &mut line);
            assert_eq!(through_sh(&line), value.as_bytes(), "{value:?}");
        }
    }

    #[test]
    fn each_escape_writes_its_own_syntax() {
        assert_eq!(escaped(t("a b/é&"), Escape::Url), "a%20b%2F%C3%A9%26");
        assert_eq!(escaped(t("a-b_c.d~"), Escape::Url), "a-b_c.d~");
        assert_eq!(escaped(t("a\"b\n"), Escape::Json), "\"a\\\"b\\n\"");
        assert_eq!(escaped(t("007"), Escape::Json), "\"007\"");
        assert_eq!(escaped(Cell::Int(7), Escape::Json), "7");
        assert_eq!(escaped(Cell::Float(f64::NAN), Escape::Json), "null");
        assert_eq!(escaped(Cell::Null, Escape::Json), "null");
        assert_eq!(escaped(Cell::Bool(true), Escape::Json), "true");
        assert_eq!(
            escaped(t("<a href=\"x\">&'\u{1}"), Escape::Xml),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;\u{FFFD}"
        );
        assert_eq!(escaped(t("O'Neil"), Escape::Sql), "'O''Neil'");
        assert_eq!(escaped(Cell::Null, Escape::Sql), "NULL");
        assert_eq!(escaped(Cell::Bytes(&[0, 255]), Escape::Sql), "X'00ff'");
        assert_eq!(escaped(t("a,\"b"), Escape::Csv), "\"a,\"\"b\"");
        assert_eq!(escaped(t("=1+1"), Escape::Csv), "=1+1");
        assert_eq!(escaped(t("=1+1"), Escape::CsvSafe), "'=1+1");
        assert_eq!(escaped(Cell::Int(-5), Escape::CsvSafe), "-5");
        assert_eq!(escaped(t("a\nb\tc\u{1b}"), Escape::Line), "a b c ");
        assert_eq!(escaped(t("a\nb"), Escape::Raw), "a\nb");
        assert_eq!(escaped(Cell::Bytes(&[0xAB]), Escape::Line), "ab");
    }

    #[test]
    fn the_default_escape_follows_the_extension() {
        let of = |name: &str| Escape::for_file(Path::new(name));
        assert_eq!(of("x.sh"), Escape::Shell);
        assert_eq!(of("x.JSON"), Escape::Json);
        assert_eq!(of("x.html"), Escape::Xml);
        assert_eq!(of("x.sql"), Escape::Sql);
        assert_eq!(of("x.csv"), Escape::Csv);
        assert_eq!(of("x.txt"), Escape::Line);
        assert_eq!(of("x"), Escape::Line);
        assert!(refuses_records(Path::new("x.BAT")));
        assert!(!refuses_records(Path::new("x.sh")));
    }

    #[test]
    fn a_mask_is_parsed_or_says_what_is_wrong() {
        let m = mask("a%{x}b%{y:url}%%", Escape::Shell);
        assert_eq!(
            m.parts,
            [
                Part::Literal(b"a".to_vec()),
                Part::Field {
                    name: "x".into(),
                    escape: Escape::Shell
                },
                Part::Literal(b"b".to_vec()),
                Part::Field {
                    name: "y".into(),
                    escape: Escape::Url
                },
                Part::Literal(b"%".to_vec()),
            ]
        );
        let error = |text: &str| Mask::parse(text, Escape::Line, Path::new("a"), 0).unwrap_err();
        assert_eq!(error("%{x"), "Mask: %{ without its }");
        assert_eq!(error("%{x:nope}"), "Mask: unknown escape :nope");
        assert_eq!(error("%{}"), "Mask: %{} has no field name");
        assert!(error("  ").contains("empty"));
    }

    #[test]
    fn a_mask_keeps_the_variables_of_the_open_file() {
        let m = mask("%N %S %q", Escape::Line);
        assert_eq!(m.parts, [Part::Literal(b"a.csv 10 %q".to_vec())]);
    }

    #[test]
    fn a_field_name_may_hold_a_colon_before_its_escape() {
        let m = mask("%{a:b:url}", Escape::Line);
        assert_eq!(
            m.parts,
            [Part::Field {
                name: "a:b".into(),
                escape: Escape::Url
            }]
        );
    }

    fn table_source(path: &Path, filter: Option<&str>) -> Source {
        Source::Table {
            path: path.to_path_buf(),
            sheet: 0,
            table: None,
            filter: filter.map(|f| sql_filter::SqlFilter::parse(f).unwrap()),
            unpacked: u64::MAX,
        }
    }

    fn csv_rows(n: usize) -> tempfile::TempDir {
        let tmp = crate::testing::tempdir();
        let mut body = String::from("id,name\n");
        for i in 1..=n {
            body.push_str(&format!("{i},n{i}\n"));
        }
        fs::write(tmp.path().join("in.csv"), body).unwrap();
        tmp
    }

    fn run_to_file(source: Source, mask: Mask, out: &Path) -> Done {
        let job = Job::start(source, mask, out.to_path_buf());
        while !job.is_done() {
            thread::sleep(Duration::from_millis(2));
        }
        job.finish().unwrap()
    }

    #[test]
    fn a_batch_writes_every_row_past_the_limit_of_the_screen() {
        let tmp = csv_rows(60_000);
        let out = tmp.path().join("out.txt");
        let m = mask("%{#} %{name}", Escape::Line);
        let done = run_to_file(table_source(&tmp.path().join("in.csv"), None), m, &out);
        assert_eq!(done.records, 60_000);
        assert!(!done.cancelled);
        let text = fs::read_to_string(&out).unwrap();
        assert_eq!(text.lines().count(), 60_000);
        assert_eq!(text.lines().last(), Some("60000 n60000"));
    }

    #[test]
    fn the_filter_applies_to_every_row() {
        let tmp = csv_rows(60_000);
        let out = tmp.path().join("out.txt");
        let source = table_source(&tmp.path().join("in.csv"), Some("id > 59998"));
        let done = run_to_file(source, mask("%{#}:%{id}", Escape::Line), &out);
        assert_eq!(done.records, 2);
        assert_eq!(
            fs::read_to_string(&out).unwrap(),
            "1:59999\n2:60000\n".replace('\n', std::str::from_utf8(shell::NEWLINE).unwrap())
        );
    }

    #[test]
    fn a_value_keeps_what_the_screen_would_change() {
        let tmp = crate::testing::tempdir();
        let long = "z".repeat(5_000);
        fs::write(
            tmp.path().join("in.csv"),
            format!("code,long\n007,{long}\n"),
        )
        .unwrap();
        let out = tmp.path().join("out.txt");
        let source = table_source(&tmp.path().join("in.csv"), Some("code = 7"));
        run_to_file(source, mask("%{code} %{long}", Escape::Line), &out);
        let text = fs::read_to_string(&out).unwrap();
        assert_eq!(text.trim_end(), format!("007 {long}"));
    }

    #[test]
    fn columns_answer_to_name_letter_and_position() {
        let tmp = csv_rows(1);
        let source = table_source(&tmp.path().join("in.csv"), None);
        let p = preview(&source, &mask("%{NAME} %{b} %{2}", Escape::Line)).unwrap();
        assert_eq!(p.lines, ["n1 n1 n1"]);
        assert_eq!(p.scanned, 1);
        assert!(p.complete);
    }

    #[test]
    fn a_field_that_does_not_exist_is_an_error_not_an_empty_value() {
        let tmp = csv_rows(1);
        let source = table_source(&tmp.path().join("in.csv"), None);
        let error = preview(&source, &mask("%{nope}", Escape::Line)).unwrap_err();
        assert_eq!(error, "Mask: no field named nope");
    }

    #[test]
    fn a_late_column_of_json_lines_is_found_by_the_preview() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.jsonl");
        fs::write(&path, "{\"a\": 1}\n{\"a\": 2, \"b\": 3}\n").unwrap();
        let p = preview(&table_source(&path, None), &mask("%{a}-%{b}", Escape::Line)).unwrap();
        assert_eq!(p.lines, ["1-", "2-3"]);
    }

    #[test]
    fn the_preview_stops_at_its_limits_and_shows_safe_lines() {
        let tmp = csv_rows(5_000);
        let source = table_source(&tmp.path().join("in.csv"), None);
        let p = preview(&source, &mask("%{id}", Escape::Line)).unwrap();
        assert_eq!(p.lines.len(), PREVIEW_LINES);
        assert_eq!(p.scanned, SCAN_RECORDS);
        assert!(!p.complete);

        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.csv");
        fs::write(&path, "v,w\n\"a\u{1b}[31m\u{202E}\",=1\n").unwrap();
        let p = preview(
            &table_source(&path, None),
            &mask("%{v:raw},%{w:csv}", Escape::Line),
        )
        .unwrap();
        assert_eq!(p.lines, ["a·[31m·,=1"]);
        assert_eq!(
            p.stats,
            Stats {
                suspicious: 1,
                formulas: 1
            }
        );
        assert!(p.stats.warning().unwrap().contains(":csvsafe"));
    }

    fn text_source(text: &str, kept: Option<Vec<usize>>, regex: Option<&str>) -> Source {
        let body = text.strip_suffix('\n').unwrap_or(text);
        Source::Text {
            text: text.to_string(),
            total: body.split('\n').count(),
            kept,
            regex: regex.map(|r| Regex::new(r).unwrap()),
        }
    }

    #[test]
    fn text_gives_the_original_line_its_number_and_the_sequence() {
        let source = text_source("a\tb\r\nskip\nc\u{1b}\n", Some(vec![0, 2]), None);
        let p = preview(&source, &mask("%{#}/%{#line}:%{line:raw}", Escape::Line)).unwrap();
        assert_eq!(p.lines, ["1/1:a·b", "2/3:c·"]);
    }

    #[test]
    fn text_gives_the_groups_of_the_filter() {
        let source = text_source(
            "10.0.0.1 GET 200\nbad\n10.0.0.2 PUT 500\n",
            Some(vec![0, 2]),
            Some(r"^(?P<ip>\S+) (\S+) (?P<status>\d+)$"),
        );
        let m = mask("%{ip} %{2} %{status} %{0}", Escape::Line);
        let p = preview(&source, &m).unwrap();
        assert_eq!(
            p.lines,
            [
                "10.0.0.1 GET 200 10.0.0.1 GET 200",
                "10.0.0.2 PUT 500 10.0.0.2 PUT 500"
            ]
        );
    }

    #[test]
    fn a_line_with_no_match_has_empty_groups() {
        let source = text_source("x\n", None, Some(r"(?P<n>\d+)"));
        let p = preview(&source, &mask("[%{n}]", Escape::Line)).unwrap();
        assert_eq!(p.lines, ["[]"]);
    }

    #[test]
    fn text_has_no_columns_by_position() {
        let source = text_source("x\n", None, None);
        assert!(preview(&source, &mask("%{1}", Escape::Line)).is_err());
        assert!(preview(&source, &mask("%{A}", Escape::Line)).is_err());
    }

    #[test]
    fn a_shell_script_starts_with_its_interpreter_and_is_not_executable() {
        let tmp = csv_rows(1);
        let out = tmp.path().join("run.sh");
        let source = table_source(&tmp.path().join("in.csv"), None);
        run_to_file(source, mask("echo %{name}", Escape::Shell), &out);
        assert_eq!(fs::read_to_string(&out).unwrap(), "#!/bin/sh\necho 'n1'\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&out).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0, "a script made from data is reviewed first");
        }
    }

    #[test]
    fn an_existing_file_is_not_touched() {
        let tmp = csv_rows(1);
        let out = tmp.path().join("out.txt");
        fs::write(&out, "mine").unwrap();
        let source = table_source(&tmp.path().join("in.csv"), None);
        let job = Job::start(source, mask("%{id}", Escape::Line), out.clone());
        while !job.is_done() {
            thread::sleep(Duration::from_millis(2));
        }
        assert!(job.finish().unwrap_err().starts_with("Cannot create"));
        assert_eq!(fs::read_to_string(&out).unwrap(), "mine");
    }

    #[test]
    fn a_cancelled_batch_leaves_no_file() {
        let tmp = csv_rows(200_000);
        let out = tmp.path().join("out.txt");
        let source = table_source(&tmp.path().join("in.csv"), None);
        let job = Job::start(source, mask("%{id}", Escape::Line), out.clone());
        job.cancel();
        assert!(job.is_cancelling());
        while !job.is_done() {
            thread::sleep(Duration::from_millis(2));
        }
        let done = job.finish().unwrap();
        assert!(done.cancelled);
        assert!(done.records < 200_000);
        assert!(!out.exists());
    }

    #[test]
    fn a_bad_source_leaves_no_file() {
        let tmp = crate::testing::tempdir();
        let bad = tmp.path().join("in.parquet");
        fs::write(&bad, "not parquet").unwrap();
        let out = tmp.path().join("out.txt");
        let job = Job::start(
            table_source(&bad, None),
            mask("%{id}", Escape::Line),
            out.clone(),
        );
        while !job.is_done() {
            thread::sleep(Duration::from_millis(2));
        }
        assert!(job.finish().is_err());
        assert!(!out.exists());
    }

    proptest! {
        /// Whatever the text, each escape gives its format valid output.
        #[test]
        fn json_and_csv_read_back_the_value(text in "\\PC*") {
            let json = escaped(t(&text), Escape::Json);
            prop_assert_eq!(
                serde_json::from_str::<serde_json::Value>(&json).unwrap(),
                serde_json::Value::String(text.clone())
            );
            let csv = escaped(t(&text), Escape::Csv);
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .flexible(true)
                .from_reader(csv.as_bytes());
            if !text.is_empty() {
                let record = reader.records().next().unwrap().unwrap();
                prop_assert_eq!(record.get(0).unwrap(), text.as_str());
            }
        }

        #[test]
        fn a_line_never_breaks_and_a_url_is_plain_ascii(text in ".*\\n?.*") {
            prop_assert!(!escaped(t(&text), Escape::Line).contains('\n'));
            prop_assert!(escaped(t(&text), Escape::Url).bytes().all(|b| b.is_ascii_graphic()));
        }

        #[test]
        fn a_mask_never_panics(text in ".{0,60}") {
            let _ = Mask::parse(&text, Escape::Shell, Path::new("/a/b"), 1);
        }
    }
}
