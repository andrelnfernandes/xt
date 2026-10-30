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

use chrono::{DateTime, Datelike, Local, Utc};
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
/// How many lines of a template's header the preview shows, so that a long one
/// does not hide the records.
const PREVIEW_HEADER_LINES: usize = 3;
/// A template is typed by hand; anything larger is not one.
const MAX_TEMPLATE_BYTES: u64 = 1024 * 1024;
const TEMPLATE_EXTENSION: &str = "tpl";

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
    /// A SQL literal for MySQL and MariaDB, where a backslash also escapes.
    Mysql,
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
            "mysql" => Escape::Mysql,
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
        Escape::Sql | Escape::Mysql => match cell {
            Cell::Null => out.extend_from_slice(b"NULL"),
            Cell::Bool(b) => out.extend_from_slice(if *b { b"TRUE" } else { b"FALSE" }),
            Cell::Int(i) => out.extend_from_slice(i.to_string().as_bytes()),
            Cell::Float(f) if f.is_finite() => out.extend_from_slice(f.to_string().as_bytes()),
            Cell::Float(_) => out.extend_from_slice(b"NULL"),
            Cell::Bytes(b) => out.extend_from_slice(format!("X'{}'", hex(b)).as_bytes()),
            Cell::Text(s) if escape == Escape::Mysql => {
                out.push(b'\'');
                for c in s.chars() {
                    match c {
                        '\\' => out.extend_from_slice(b"\\\\"),
                        '\'' => out.extend_from_slice(b"''"),
                        '\0' => out.extend_from_slice(b"\\0"),
                        // Ends the input of mysql on Windows.
                        '\u{1A}' => out.extend_from_slice(b"\\Z"),
                        c => {
                            let mut buf = [0; 4];
                            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                        }
                    }
                }
                out.push(b'\'');
            }
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

/// What a mask can say about where it is used, fixed when it is read: all the
/// records of a file get the same date.
struct Context<'a> {
    open: &'a Path,
    size: u64,
    out: &'a Path,
    template: Option<&'a str>,
    now: DateTime<Local>,
}

fn text_cell(text: impl Into<String>) -> Cell<'static> {
    Cell::Text(Cow::Owned(text.into()))
}

impl Context<'_> {
    /// A field that every record shares: unlike `%N`, it goes through an escape,
    /// and a file's name can be as hostile as its contents.
    fn field(&self, name: &str) -> Result<Option<Cell<'static>>, String> {
        let now = &self.now;
        let date = |format: &str| Some(text_cell(now.format(format).to_string()));
        let from_file = |var: &str| {
            let value = shell::batch_line(var, self.open, self.size);
            Some(text_cell(String::from_utf8_lossy(&value)))
        };
        let unknown = || format!("Mask: cannot tell %{{{name}}} here");
        Ok(match name {
            "#path" => from_file("%F"),
            "#dir" => from_file("%D"),
            "#name" => from_file("%N"),
            "#base" => from_file("%B"),
            "#ext" => from_file("%E"),
            "#size" => Some(Cell::Int(self.size as i64)),
            "#date" => date("%Y-%m-%d"),
            "#time" => date("%H:%M:%S"),
            "#datetime" => date("%Y-%m-%dT%H:%M:%S%:z"),
            "#utc" => Some(text_cell(
                now.with_timezone(&Utc)
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
            )),
            "#year" => date("%Y"),
            "#month" => date("%m"),
            "#day" => date("%d"),
            // ISO: Monday is 1.
            "#weekday" => Some(Cell::Int(i64::from(now.weekday().number_from_monday()))),
            // In English, so that a script does not change with the machine's language.
            "#weekday_name" => Some(text_cell(now.format("%A").to_string().to_lowercase())),
            "#epoch" => Some(Cell::Int(now.timestamp())),
            "#user" => {
                let user = std::env::var("USER").or_else(|_| std::env::var("USERNAME"));
                Some(text_cell(user.map_err(|_| unknown())?))
            }
            "#host" => Some(text_cell(
                crate::platform::system().host.ok_or_else(unknown)?,
            )),
            "#template" => Some(text_cell(self.template.ok_or_else(unknown)?)),
            "#output" => Some(text_cell(self.out.to_string_lossy())),
            _ => None,
        })
    }
}

/// A variable of the process, which a mask has to name. It is there as it was
/// when the file was generated; `$NAME` in a script is read when that runs.
fn environment_field(name: &str) -> Result<Cell<'static>, String> {
    if name.is_empty() {
        return Err("Mask: %{$} has no variable name".into());
    }
    let value =
        std::env::var_os(name).ok_or_else(|| format!("Mask: no environment variable {name}"))?;
    Ok(text_cell(value.to_string_lossy()))
}

/// Where a mask's text is in SQL, to know what is inside a string. A line starts
/// outside of one: a multi-line string is rare, and an apostrophe in text that
/// is not SQL should not count for the rest of the file.
#[derive(Default)]
struct SqlText {
    in_string: bool,
    in_comment: bool,
}

impl SqlText {
    fn read(&mut self, text: &str) {
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\n' => *self = SqlText::default(),
                _ if self.in_comment => {}
                '\'' => self.in_string = !self.in_string,
                '-' if !self.in_string && chars.peek() == Some(&'-') => self.in_comment = true,
                _ => {}
            }
        }
    }
}

/// Writes `bytes`, with each line ending in `newline`.
fn literal_into(bytes: &[u8], newline: &[u8], out: &mut Vec<u8>) {
    for (i, piece) in bytes.split(|&b| b == b'\n').enumerate() {
        if i > 0 {
            out.extend_from_slice(newline);
        }
        out.extend_from_slice(piece);
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
    /// The variables of the process it reads.
    environment: Vec<String>,
}

impl Mask {
    /// `%{name}` and `%{name:escape}` are fields of the record; `%f %d %n %b
    /// %e %s` are the open file's, as in the Batch of tagged files; `%%` is a `%`.
    /// `%{$NAME}` is a variable of the process, and `%{#...}` what `Context` knows.
    fn parse_in(text: &str, default: Escape, context: &Context<'_>) -> Result<Self, String> {
        if text.trim().is_empty() {
            return Err("Batch cancelled: empty line mask".into());
        }
        let mut parts = Vec::new();
        let mut environment = Vec::new();
        let mut literal = Vec::new();
        let mut rest = text;
        let mut sql = SqlText::default();
        // The first variable of the open file inside a SQL string.
        let mut in_string = None;
        while let Some(at) = rest.find('%') {
            literal.extend_from_slice(&rest.as_bytes()[..at]);
            sql.read(&rest[..at]);
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
                let shared = match name.strip_prefix('$') {
                    Some(variable) => {
                        let value = environment_field(variable)?;
                        environment.push(variable.to_string());
                        Some(value)
                    }
                    None => context.field(name)?,
                };
                if let Some(value) = shared {
                    escape_into(&value, escape, &mut literal);
                    rest = &inner[close + 1..];
                    continue;
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
                if sql.in_string && c != '%' {
                    in_string.get_or_insert_with(|| var.clone());
                }
                literal.extend(shell::batch_line(&var, context.open, context.size));
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
        let writes_sql = matches!(default, Escape::Sql | Escape::Mysql)
            || parts.iter().any(|part| {
                matches!(
                    part,
                    Part::Field {
                        escape: Escape::Sql | Escape::Mysql,
                        ..
                    }
                )
            });
        // LIKE '%foo%' would hold the file's path, with no escape for SQL.
        if let (true, Some(var)) = (writes_sql, in_string) {
            return Err(format!(
                "Mask: {var} in a SQL string is the open file's: \
                 write %{var} for a %, or a field like %{{#name}} for its value"
            ));
        }
        Ok(Mask { parts, environment })
    }

    /// The first field that `names` has no answer for.
    fn unknown(&self, row: &Row<'_>) -> Option<&str> {
        self.parts.iter().find_map(|part| match part {
            Part::Field { name, .. } if !row.knows(name) => Some(name.as_str()),
            _ => None,
        })
    }

    /// The first field that has no answer outside a record.
    fn record_field(&self, count_too: bool) -> Option<&str> {
        self.parts.iter().find_map(|part| match part {
            Part::Field { name, .. } if count_too || name != TOTAL_COUNT => Some(name.as_str()),
            _ => None,
        })
    }

    /// A header or footer: its text, and how many records there were.
    fn edge(&self, count: u64, newline: &[u8], out: &mut Vec<u8>) {
        for part in &self.parts {
            match part {
                Part::Literal(bytes) => literal_into(bytes, newline, out),
                Part::Field { escape, .. } => {
                    escape_into(&Cell::Int(count as i64), *escape, out);
                }
            }
        }
    }

    fn render(&self, row: &Row<'_>, newline: &[u8], stats: &mut Stats, out: &mut Vec<u8>) {
        for part in &self.parts {
            match part {
                Part::Literal(bytes) => literal_into(bytes, newline, out),
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
    /// Going into a `:sql` literal with a backslash, which in MySQL escapes
    /// the quote that ends it.
    pub backslashes: u64,
    /// Whole records only: columns of JSON Lines found after the preview,
    /// and so not written.
    pub late_columns: u64,
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
        if escape == Escape::Sql && text.contains('\\') {
            self.backslashes += 1;
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
        if self.backslashes > 0 {
            parts.push(format!(
                "{} values with a backslash (for MySQL or MariaDB, use :mysql)",
                self.backslashes
            ));
        }
        if self.late_columns > 0 {
            parts.push(format!(
                "{} columns appeared after the first {SCAN_RECORDS} records and were left out",
                self.late_columns
            ));
        }
        (!parts.is_empty()).then(|| parts.join(", "))
    }
}

/// The formats that hold whole records, with no mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// One array of objects.
    Json,
    /// An object per line.
    JsonLines,
    Csv,
    Tsv,
    Xml,
    Html,
}

/// `file`'s format can hold whole records, with no mask.
pub fn holds_records(file: &Path) -> bool {
    Format::for_file(file).is_some()
}

impl Format {
    fn for_file(file: &Path) -> Option<Self> {
        Some(match extension(file).as_str() {
            "json" => Format::Json,
            "jsonl" | "ndjson" => Format::JsonLines,
            "csv" => Format::Csv,
            "tsv" => Format::Tsv,
            "xml" => Format::Xml,
            "html" | "htm" => Format::Html,
            _ => return None,
        })
    }

    fn escape(self) -> Escape {
        match self {
            Format::Json | Format::JsonLines => Escape::Json,
            Format::Csv | Format::Tsv => Escape::Csv,
            Format::Xml | Format::Html => Escape::Xml,
        }
    }
}

/// The name in a mask that asks for a template, `:name`. It can only be a name,
/// so a template is always in its folder, and a mask with anything else in it,
/// like `: %{a}` in a script, stays a mask.
fn template_name(text: &str) -> Option<&str> {
    let name = text.trim().strip_prefix(':')?;
    let valid = !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    valid.then_some(name)
}

fn template_dir() -> Option<PathBuf> {
    Some(crate::platform::config_dir()?.join("batch"))
}

/// The templates there are, for the prompt.
pub fn template_names() -> Vec<String> {
    template_dir().map_or_else(Vec::new, |dir| names_in(&dir))
}

fn names_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != TEMPLATE_EXTENSION {
                return None;
            }
            let name = path.file_stem()?.to_str()?.to_string();
            template_name(&format!(":{name}")).is_some().then_some(name)
        })
        .collect();
    names.sort();
    names
}

fn read_template(name: &str, dir: Option<&Path>) -> Result<String, String> {
    let dir = dir.ok_or_else(|| format!("No template {name}: no configuration folder"))?;
    let path = dir.join(format!("{name}.{TEMPLATE_EXTENSION}"));
    let shown = path.display();
    let meta = fs::metadata(&path).map_err(|_| format!("No template {name}: no {shown}"))?;
    if !meta.is_file() {
        return Err(format!("Template {name}: {shown} is not a file"));
    }
    if meta.len() > MAX_TEMPLATE_BYTES {
        return Err(format!("Template {name}: {shown} is too large"));
    }
    fs::read_to_string(&path).map_err(|e| format!("Template {name}: {e}"))
}

/// Cuts a template into its header, record and footer. With no `%%` lines all
/// of it is the record, and a line starting with `%%#` is a comment. Lines are
/// kept with `\n`, and each record ends one, so that records never run together.
fn template_sections(text: &str) -> Result<[Option<String>; 3], String> {
    const RECORD: usize = 1;
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut sections: [Option<String>; 3] = [None, None, None];
    let mut before = String::new();
    let mut current = None;
    for line in text.lines() {
        if line.starts_with("%%#") {
            continue;
        }
        let word = line.trim_end().strip_prefix("%%");
        if let Some(word) = word.filter(|w| !w.is_empty() && w.chars().all(char::is_alphabetic)) {
            let i = match word {
                "header" => 0,
                "record" => RECORD,
                "footer" => 2,
                _ => return Err(format!("unknown section %%{word}")),
            };
            if sections[i].is_some() {
                return Err(format!("two sections %%{word}"));
            }
            sections[i] = Some(String::new());
            current = Some(i);
            continue;
        }
        let section = match current {
            Some(i) => sections[i].get_or_insert_default(),
            None => &mut before,
        };
        section.push_str(line);
        section.push('\n');
    }
    if current.is_none() {
        sections[RECORD] = Some(before);
    } else if !before.trim().is_empty() {
        return Err("text before the first section".into());
    }
    if sections[RECORD]
        .as_deref()
        .is_none_or(|r| r.trim().is_empty())
    {
        return Err("it has no record".into());
    }
    Ok(sections)
}

/// In a footer, `%{count}` is how many records were written.
const TOTAL_COUNT: &str = "count";

/// Header or footer: no fields of a record. The header is written before any
/// record is read, so only the footer knows how many there are.
fn template_edge(
    section: Option<String>,
    default: Escape,
    context: &Context<'_>,
    is_header: bool,
) -> Result<Option<Mask>, String> {
    let Some(text) = section.filter(|s| !s.trim().is_empty()) else {
        return Ok(None);
    };
    let mask = Mask::parse_in(&text, default, context)?;
    match mask.record_field(is_header) {
        Some(TOTAL_COUNT) => Err(format!("%{{{TOTAL_COUNT}}} is only known in the footer")),
        Some(field) => Err(format!(
            "%{{{field}}} is a field of a record, outside the record"
        )),
        None => Ok(Some(mask)),
    }
}

fn parse_template(text: &str, default: Escape, context: &Context<'_>) -> Result<Output, String> {
    let [header, record, footer] = template_sections(text)?;
    Ok(Output::Template {
        header: template_edge(header, default, context, true)?,
        record: Mask::parse_in(&record.unwrap_or_default(), default, context)?,
        footer: template_edge(footer, default, context, false)?,
    })
}

/// How a field is named in the output.
fn key_of(name: &str) -> &str {
    if name == "#line" { "line_number" } else { name }
}

/// What a Batch writes for each record.
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// A line from a mask.
    Lines(Mask),
    /// Whole records in a format that has its own rules for them.
    Records {
        format: Format,
        /// The fields to write, if the user named them.
        wanted: Option<Vec<String>>,
        /// The fields being written, known once the source was looked at.
        fields: Vec<String>,
        /// The title of an HTML page: the name of what is read.
        title: String,
    },
    /// Text of the user's, from a file, around the masks of the records.
    Template {
        header: Option<Mask>,
        record: Mask,
        footer: Option<Mask>,
    },
}

impl From<Mask> for Output {
    fn from(mask: Mask) -> Self {
        Output::Lines(mask)
    }
}

impl Output {
    /// An empty mask writes all fields, and `@a,b` those fields, in the
    /// formats that hold whole records; anything else is a mask.
    /// `:name` is the template with that name.
    pub fn parse(text: &str, out: &Path, open: &Path, size: u64) -> Result<Self, String> {
        Self::parse_in(text, out, open, size, template_dir().as_deref())
    }

    fn parse_in(
        text: &str,
        out: &Path,
        open: &Path,
        size: u64,
        templates: Option<&Path>,
    ) -> Result<Self, String> {
        let mut context = Context {
            open,
            size,
            out,
            template: None,
            now: Local::now(),
        };
        if let Some(name) = template_name(text) {
            let body = read_template(name, templates)?;
            context.template = Some(name);
            return parse_template(&body, Escape::for_file(out), &context)
                .map_err(|e| format!("Template {name}: {e}"));
        }
        let format = Format::for_file(out);
        let trimmed = text.trim();
        if let Some(format) = format
            && (trimmed.is_empty() || trimmed.starts_with('@'))
        {
            let wanted = match trimmed.strip_prefix('@') {
                None => None,
                Some(list) => {
                    let fields: Vec<String> = list
                        .split(',')
                        .map(|f| f.trim().to_string())
                        .filter(|f| !f.is_empty())
                        .collect();
                    if fields.is_empty() {
                        return Err("Mask: @ needs the names of the fields".into());
                    }
                    Some(fields)
                }
            };
            let title = open
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            return Ok(Output::Records {
                format,
                wanted,
                fields: Vec::new(),
                title,
            });
        }
        Mask::parse_in(text, Escape::for_file(out), &context).map(Output::Lines)
    }

    /// The variables of the process it reads, without repeating them.
    fn environment(&self) -> Vec<String> {
        let masks: Vec<&Mask> = match self {
            Output::Lines(mask) => vec![mask],
            Output::Template {
                header,
                record,
                footer,
            } => [header.as_ref(), Some(record), footer.as_ref()]
                .into_iter()
                .flatten()
                .collect(),
            Output::Records { .. } => Vec::new(),
        };
        let mut names: Vec<String> = masks
            .iter()
            .flat_map(|mask| mask.environment.iter().cloned())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// The first field the records do not have.
    fn unknown(&self, row: &Row<'_>) -> Option<String> {
        match self {
            Output::Lines(mask) => mask.unknown(row).map(str::to_string),
            Output::Template { record, .. } => record.unknown(row).map(str::to_string),
            Output::Records { wanted, .. } => wanted
                .iter()
                .flatten()
                .find(|name| !row.knows(name))
                .cloned(),
        }
    }

    /// Fixes the fields, from the last record looked at.
    fn settle(&mut self, row_fields: Vec<String>) {
        if let Output::Records { wanted, fields, .. } = self {
            *fields = wanted.clone().unwrap_or(row_fields);
        }
    }

    fn begin(&self, newline: &[u8], out: &mut Vec<u8>) {
        if let Output::Template {
            header: Some(header),
            ..
        } = self
        {
            header.edge(0, newline, out);
        }
        let Output::Records {
            format,
            fields,
            title,
            ..
        } = self
        else {
            return;
        };
        let separator = if *format == Format::Tsv { "\t" } else { "," };
        match format {
            Format::Json => out.extend_from_slice(b"[\n"),
            Format::JsonLines => {}
            Format::Csv | Format::Tsv => {
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        out.extend_from_slice(separator.as_bytes());
                    }
                    csv_field(key_of(field), out);
                }
                out.push(b'\n');
            }
            Format::Xml => {
                out.extend_from_slice(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<records>\n");
            }
            Format::Html => {
                out.extend_from_slice(
                    b"<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>",
                );
                escape_into(&Cell::Text(title.as_str().into()), Escape::Xml, out);
                out.extend_from_slice(b"</title>\n</head>\n<body>\n<table>\n<thead>\n<tr>");
                for field in fields {
                    out.extend_from_slice(b"<th>");
                    escape_into(&Cell::Text(key_of(field).into()), Escape::Xml, out);
                    out.extend_from_slice(b"</th>");
                }
                out.extend_from_slice(b"</tr>\n</thead>\n<tbody>\n");
            }
        }
    }

    /// What ends the file, after `count` records.
    fn end(&self, count: u64, newline: &[u8], out: &mut Vec<u8>) {
        if let Output::Template {
            footer: Some(footer),
            ..
        } = self
        {
            footer.edge(count, newline, out);
        }
        let Output::Records { format, .. } = self else {
            return;
        };
        match format {
            Format::Json => out.extend_from_slice(b"\n]\n"),
            Format::Xml => out.extend_from_slice(b"</records>\n"),
            Format::Html => out.extend_from_slice(b"</tbody>\n</table>\n</body>\n</html>\n"),
            _ => {}
        }
    }

    /// A record, with what ends its line. `first` is whether it is the first
    /// one written, since a JSON array has commas only between its objects.
    fn render(
        &self,
        row: &Row<'_>,
        first: bool,
        newline: &[u8],
        stats: &mut Stats,
        out: &mut Vec<u8>,
    ) {
        let Output::Records {
            format,
            wanted,
            fields,
            ..
        } = self
        else {
            match self {
                Output::Lines(mask) => {
                    mask.render(row, newline, stats, out);
                    out.extend_from_slice(newline);
                }
                // Its lines already end in what the file says.
                Output::Template { record, .. } => record.render(row, newline, stats, out),
                Output::Records { .. } => {}
            }
            return;
        };
        if wanted.is_none() && row.positional && row.names.len() > fields.len() {
            stats.late_columns = stats
                .late_columns
                .max((row.names.len() - fields.len()) as u64);
        }
        let escape = format.escape();
        let mut value = |name: &str, out: &mut Vec<u8>| {
            let cell = row.value(name);
            stats.look_at(&cell, escape);
            escape_into(&cell, escape, out);
        };
        match format {
            Format::Json | Format::JsonLines => {
                if *format == Format::Json {
                    out.extend_from_slice(if first { b"  " } else { b",\n  " });
                }
                out.push(b'{');
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        out.extend_from_slice(b", ");
                    }
                    let key = serde_json::Value::from(key_of(field)).to_string();
                    out.extend_from_slice(key.as_bytes());
                    out.extend_from_slice(b": ");
                    value(field, out);
                }
                out.push(b'}');
                if *format == Format::JsonLines {
                    out.push(b'\n');
                }
            }
            Format::Csv | Format::Tsv => {
                let separator = if *format == Format::Tsv { b'\t' } else { b',' };
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(separator);
                    }
                    value(field, out);
                }
                out.push(b'\n');
            }
            Format::Xml => {
                out.extend_from_slice(b"  <record>\n");
                for field in fields {
                    out.extend_from_slice(b"    <field name=\"");
                    escape_into(&Cell::Text(key_of(field).into()), Escape::Xml, out);
                    out.extend_from_slice(b"\">");
                    value(field, out);
                    out.extend_from_slice(b"</field>\n");
                }
                out.extend_from_slice(b"  </record>\n");
            }
            Format::Html => {
                out.extend_from_slice(b"<tr>");
                for field in fields {
                    out.extend_from_slice(b"<td>");
                    value(field, out);
                    out.extend_from_slice(b"</td>");
                }
                out.extend_from_slice(b"</tr>\n");
            }
        }
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
    /// All the fields of a record, for a format that writes it whole. A group
    /// with a name goes by the name; 0 repeats what the line already has.
    fn all_fields(&self) -> Vec<String> {
        if self.line.is_none() {
            return self.names.to_vec();
        }
        let mut fields = vec!["#line".to_string(), "line".to_string()];
        for name in self.names.iter().skip(1) {
            if name == "0" {
                continue;
            }
            // The number comes right before the name of the same group.
            let named = name.parse::<usize>().is_ok()
                && self
                    .names
                    .iter()
                    .position(|n| n == name)
                    .and_then(|i| self.names.get(i + 1))
                    .is_some_and(|next| next.parse::<usize>().is_err());
            if !named {
                fields.push(name.clone());
            }
        }
        fields
    }

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
        /// The viewer did not read all of the file.
        cut: bool,
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
                ..
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

    /// Part of the file will not be written.
    fn cut(&self) -> bool {
        matches!(self, Source::Text { cut: true, .. })
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
    /// The viewer read only part of the file, so the rest will not be written.
    pub cut: bool,
    pub stats: Stats,
    /// The variables of the process it reads: a template from someone else
    /// could send a secret away when its script runs.
    pub environment: Vec<String>,
    /// What to write, with its fields known.
    pub output: Output,
}

/// A line of output, safe for the screen: it may hold anything.
fn shown(line: &[u8]) -> String {
    TableGrid::sanitize_cell(&String::from_utf8_lossy(line))
}

/// Leaves only the last `count` lines of `text`, all ending in a newline.
fn keep_last_lines(text: &mut Vec<u8>, count: usize) {
    let ends = text.iter().enumerate().filter(|&(_, &b)| b == b'\n');
    let ends: Vec<usize> = ends.map(|(i, _)| i).collect();
    if ends.len() > count {
        text.drain(..=ends[ends.len() - count - 1]);
    }
}

/// Reads the start of `source` through `output`. A field that no record among
/// those read has is an error here, so that it never comes out empty.
pub fn preview(source: &Source, output: impl Into<Output>) -> Result<Preview, String> {
    let mut output = output.into();
    // A first look finds the fields, since JSON Lines finds them as it goes.
    let mut scanned = 0;
    let mut unknown = None;
    let mut fields = Vec::new();
    let end = source.run(Budget::new(SCAN_TIME, source.unpacked()), &mut |row| {
        if scanned >= SCAN_RECORDS {
            return ControlFlow::Break(());
        }
        scanned += 1;
        // The last answer counts, for the same reason.
        unknown = output.unknown(row);
        fields = row.all_fields();
        ControlFlow::Continue(())
    })?;
    if let Some(name) = unknown {
        return Err(format!("Mask: no field named {name}"));
    }
    output.settle(fields);

    // Then what those records become.
    let mut stats = Stats::default();
    let mut text = Vec::new();
    let mut count = 0;
    output.begin(b"\n", &mut text);
    if matches!(output, Output::Template { .. }) {
        keep_last_lines(&mut text, PREVIEW_HEADER_LINES);
    }
    source.run(Budget::new(SCAN_TIME, source.unpacked()), &mut |row| {
        if count >= scanned {
            return ControlFlow::Break(());
        }
        let first = count == 0;
        count += 1;
        // Only what the screen can show is kept; the rest is only looked at.
        let shown_enough = text.iter().filter(|&&b| b == b'\n').count() > PREVIEW_LINES;
        let mut line = Vec::new();
        output.render(row, first, b"\n", &mut stats, &mut line);
        if !shown_enough {
            text.extend_from_slice(&line);
        }
        ControlFlow::Continue(())
    })?;
    let mut lines: Vec<String> = text
        .split(|&b| b == b'\n')
        .map(shown)
        .take(PREVIEW_LINES)
        .collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    Ok(Preview {
        lines,
        scanned,
        complete: end == End::Complete,
        cut: source.cut(),
        stats,
        environment: output.environment(),
        output,
    })
}

/// How a Batch that ran ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Done {
    pub records: u64,
    pub stats: Stats,
    pub cancelled: bool,
    /// The viewer read only part of the file, so the rest was not written.
    pub cut: bool,
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
    pub fn start(source: Source, output: impl Into<Output>, file: PathBuf) -> Self {
        let output = output.into();
        let cancel = Arc::new(AtomicBool::new(false));
        let written = Arc::new(AtomicU64::new(0));
        let handle = {
            let (cancel, written, file) = (cancel.clone(), written.clone(), file.clone());
            thread::spawn(move || {
                let result = crate::panic_guard::catch(|| {
                    write_file(&source, &output, &file, cancel, &written)
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
    output: &Output,
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
    let mut line = Vec::new();
    output.begin(newline, &mut line);
    out.write_all(&line)
        .map_err(|e| format!("Cannot write {}: {e}", file.display()))?;

    let budget = Budget::cancellable(source.unpacked(), cancel);
    let mut stats = Stats::default();
    let mut records = 0;
    let mut failure = None;
    let end = source.run(budget, &mut |row| {
        line.clear();
        output.render(row, records == 0, newline, &mut stats, &mut line);
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
    line.clear();
    output.end(records, newline, &mut line);
    out.write_all(&line)
        .map_err(|e| format!("Cannot write {}: {e}", file.display()))?;
    let flushed: Result<File, _> = out.into_inner();
    flushed.map_err(|e| format!("Cannot write {}: {e}", file.display()))?;
    Ok(Done {
        records,
        stats,
        cancelled: end == End::OutOfTime,
        cut: source.cut(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    impl Mask {
        fn parse(text: &str, default: Escape, file: &Path, size: u64) -> Result<Self, String> {
            Self::parse_in(text, default, &test_context(file, size))
        }
    }

    fn test_context(open: &Path, size: u64) -> Context<'_> {
        Context {
            open,
            size,
            out: Path::new("/o/out.txt"),
            template: None,
            now: Local::now(),
        }
    }

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

    /// What a MySQL literal at the start of `text` holds and what comes after
    /// it, read with the rules of MySQL's default mode.
    fn read_mysql_literal(text: &str) -> Option<(String, &str)> {
        let mut chars = text.strip_prefix('\'')?.char_indices();
        let body = &text[1..];
        let mut value = String::new();
        while let Some((i, c)) = chars.next() {
            match c {
                '\\' => match chars.next()?.1 {
                    '0' => value.push('\0'),
                    'Z' => value.push('\u{1A}'),
                    other => value.push(other),
                },
                '\'' if body[i + 1..].starts_with('\'') => {
                    chars.next();
                    value.push('\'');
                }
                '\'' => return Some((value, &body[i + 1..])),
                c => value.push(c),
            }
        }
        None
    }

    #[test]
    fn a_mysql_literal_cannot_be_closed_by_a_backslash() {
        let hostile = r"\'; DROP TABLE t; -- ";
        let mysql = escaped(t(hostile), Escape::Mysql);
        assert_eq!(mysql, r"'\\''; DROP TABLE t; -- '");
        assert_eq!(read_mysql_literal(&mysql), Some((hostile.to_string(), "")));
        // The problem this avoids: read by MySQL, :sql ends the literal early.
        let sql = escaped(t(hostile), Escape::Sql);
        let (_, after) = read_mysql_literal(&sql).unwrap();
        assert!(after.contains("DROP TABLE"), "{after}");

        assert_eq!(escaped(t("a\0\u{1A}"), Escape::Mysql), r"'a\0\Z'");
        assert_eq!(escaped(Cell::Null, Escape::Mysql), "NULL");
        assert_eq!(escaped(Cell::Int(7), Escape::Mysql), "7");
        assert_eq!(escaped(Cell::Bytes(&[0, 255]), Escape::Mysql), "X'00ff'");
        assert_eq!(
            mask("%{a:mysql}", Escape::Line).parts,
            [Part::Field {
                name: "a".into(),
                escape: Escape::Mysql
            }]
        );
    }

    #[test]
    fn a_backslash_in_a_sql_literal_gets_a_warning() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.csv");
        fs::write(&path, "v\na\\b\nc\n").unwrap();
        let found = |m: &str| {
            preview(&table_source(&path, None), mask(m, Escape::Line))
                .unwrap()
                .stats
        };
        let stats = found("%{v:sql}");
        assert_eq!(stats.backslashes, 1);
        assert!(stats.warning().unwrap().contains(":mysql"));
        assert_eq!(found("%{v:mysql}").backslashes, 0);
        assert_eq!(found("%{v:line}").backslashes, 0);
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
    fn a_variable_of_the_file_is_refused_inside_a_sql_string() {
        let sql = |text: &str| Mask::parse(text, Escape::Sql, Path::new("/d/a.csv"), 10);
        let error = sql("SELECT 1 WHERE x LIKE '%foo%'").unwrap_err();
        assert!(error.starts_with("Mask: %f in a SQL string"), "{error}");
        assert!(error.contains("write %%f for a %"), "{error}");
        assert!(
            sql("WHERE x = 'a' AND y = '%N'").is_err(),
            "the second string"
        );
        assert!(sql("'it''s %d'").is_err(), "a doubled quote stays inside");

        // What a % is meant to be, and what is not in a string.
        let literal = |text: &str| match sql(text).unwrap().parts.as_slice() {
            [Part::Literal(bytes)] => String::from_utf8(bytes.clone()).unwrap(),
            other => panic!("{other:?}"),
        };
        assert_eq!(literal("LIKE '%%foo%%'"), "LIKE '%foo%'");
        assert_eq!(literal("LIKE '%a%'"), "LIKE '%a%'");
        assert_eq!(literal("-- %N"), "-- a.csv");
        assert_eq!(literal("-- don't\n-- %N\n"), "-- don't\n-- a.csv\n");
        assert_eq!(literal("-- it's %N"), "-- it's a.csv");
        assert_eq!(literal("'a'\n%N"), "'a'\na.csv");
        assert!(sql("'%{#name}' %{x}").is_ok(), "fields have an escape");

        // Only where SQL is written: a field asking for it counts too.
        let line = |text: &str| Mask::parse(text, Escape::Line, Path::new("/d/a.csv"), 10);
        assert!(line("'%N'").is_ok());
        assert!(line("'%N' %{a:sql}").is_err());
        assert!(line("'%N' %{a:mysql}").is_err());
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

    fn run_to_file(source: Source, output: impl Into<Output>, out: &Path) -> Done {
        let job = Job::start(source, output, out.to_path_buf());
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
        let p = preview(&source, mask("%{NAME} %{b} %{2}", Escape::Line)).unwrap();
        assert_eq!(p.lines, ["n1 n1 n1"]);
        assert_eq!(p.scanned, 1);
        assert!(p.complete);
    }

    #[test]
    fn a_field_that_does_not_exist_is_an_error_not_an_empty_value() {
        let tmp = csv_rows(1);
        let source = table_source(&tmp.path().join("in.csv"), None);
        let error = preview(&source, mask("%{nope}", Escape::Line)).unwrap_err();
        assert_eq!(error, "Mask: no field named nope");
    }

    #[test]
    fn a_late_column_of_json_lines_is_found_by_the_preview() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.jsonl");
        fs::write(&path, "{\"a\": 1}\n{\"a\": 2, \"b\": 3}\n").unwrap();
        let p = preview(&table_source(&path, None), mask("%{a}-%{b}", Escape::Line)).unwrap();
        assert_eq!(p.lines, ["1-", "2-3"]);
    }

    #[test]
    fn the_preview_stops_at_its_limits_and_shows_safe_lines() {
        let tmp = csv_rows(5_000);
        let source = table_source(&tmp.path().join("in.csv"), None);
        let p = preview(&source, mask("%{id}", Escape::Line)).unwrap();
        assert_eq!(p.lines.len(), PREVIEW_LINES);
        assert_eq!(p.scanned, SCAN_RECORDS);
        assert!(!p.complete);

        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.csv");
        fs::write(&path, "v,w\n\"a\u{1b}[31m\u{202E}\",=1\n").unwrap();
        let p = preview(
            &table_source(&path, None),
            mask("%{v:raw},%{w:csv}", Escape::Line),
        )
        .unwrap();
        assert_eq!(p.lines, ["a·[31m·,=1"]);
        assert_eq!(
            p.stats,
            Stats {
                suspicious: 1,
                formulas: 1,
                backslashes: 0,
                late_columns: 0,
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
            cut: false,
        }
    }

    #[test]
    fn text_gives_the_original_line_its_number_and_the_sequence() {
        let source = text_source("a\tb\r\nskip\nc\u{1b}\n", Some(vec![0, 2]), None);
        let p = preview(&source, mask("%{#}/%{#line}:%{line:raw}", Escape::Line)).unwrap();
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
        let p = preview(&source, m.clone()).unwrap();
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
        let p = preview(&source, mask("[%{n}]", Escape::Line)).unwrap();
        assert_eq!(p.lines, ["[]"]);
    }

    #[test]
    fn text_has_no_columns_by_position() {
        let source = text_source("x\n", None, None);
        assert!(preview(&source, mask("%{1}", Escape::Line)).is_err());
        assert!(preview(&source, mask("%{A}", Escape::Line)).is_err());
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

    fn output(text: &str, out: &str) -> Output {
        Output::parse(text, Path::new(out), Path::new("/d/in.csv"), 1).unwrap()
    }

    fn write_records(body: &str, out_name: &str, text: &str) -> String {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.csv");
        fs::write(&path, body).unwrap();
        let out = tmp.path().join(out_name);
        let source = table_source(&path, None);
        let found = preview(&source, output(text, out_name)).unwrap();
        run_to_file(source, found.output, &out);
        fs::read_to_string(&out).unwrap()
    }

    const PEOPLE: &str = "id,name\n1,\"a<b, \"\"c\"\"\"\n007,\n";

    #[test]
    fn an_empty_mask_means_whole_records_only_where_a_format_holds_them() {
        assert!(matches!(output("", "x.json"), Output::Records { .. }));
        assert!(matches!(output("  ", "x.HTML"), Output::Records { .. }));
        assert!(matches!(output("%{a}", "x.json"), Output::Lines(_)));
        let error = Output::parse("", Path::new("x.txt"), Path::new("a"), 0).unwrap_err();
        assert!(error.contains("empty"), "{error}");
        assert!(matches!(output("@a", "x.txt"), Output::Lines(_)));
        assert!(Output::parse("@ , ", Path::new("x.csv"), Path::new("a"), 0).is_err());
    }

    #[test]
    fn json_is_one_valid_array_with_nothing_lost() {
        let text = write_records(PEOPLE, "out.json", "");
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value,
            serde_json::json!([
                {"id": "1", "name": "a<b, \"c\""},
                {"id": "007", "name": ""},
            ])
        );
    }

    #[test]
    fn an_empty_json_array_is_still_valid() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.csv");
        fs::write(&path, "id,name\n1,a\n").unwrap();
        let out = tmp.path().join("out.json");
        let source = table_source(&path, Some("id > 5"));
        let wanted = Output::Records {
            format: Format::Json,
            wanted: None,
            fields: vec!["id".into()],
            title: String::new(),
        };
        run_to_file(source, wanted, &out);
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(value, serde_json::json!([]));
    }

    #[test]
    fn json_lines_give_an_object_per_line() {
        let text = write_records(PEOPLE, "out.jsonl", "@name");
        let lines: Vec<serde_json::Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines[1], serde_json::json!({"name": ""}));
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn csv_reads_back_the_same_table() {
        let text = write_records(PEOPLE, "out.csv", "");
        let mut reader = csv::Reader::from_reader(text.as_bytes());
        assert_eq!(reader.headers().unwrap(), vec!["id", "name"]);
        let rows: Vec<_> = reader.records().map(Result::unwrap).collect();
        assert_eq!(&rows[0], vec!["1", "a<b, \"c\""]);
        assert_eq!(&rows[1], vec!["007", ""]);
    }

    #[test]
    fn tsv_separates_with_tabs() {
        let text = write_records("a,b\n1,2\n", "out.tsv", "");
        assert_eq!(text, "a\tb\n1\t2\n");
    }

    #[test]
    fn xml_and_html_escape_values_names_and_title() {
        let body = "<evil>,b\n</td><script>,&\n";
        let xml = write_records(body, "out.xml", "");
        assert!(xml.starts_with("<?xml"));
        assert!(xml.contains("<field name=\"&lt;evil&gt;\">&lt;/td&gt;&lt;script&gt;</field>"));
        assert!(xml.contains("<field name=\"b\">&amp;</field>"));
        assert!(xml.ends_with("</records>\n"));

        let html = write_records(body, "out.html", "");
        assert!(html.contains("<th>&lt;evil&gt;</th>"));
        assert!(html.contains("<td>&lt;/td&gt;&lt;script&gt;</td>"));
        assert!(!html.contains("<script>"));
        assert!(html.ends_with("</html>\n"));
    }

    #[test]
    fn columns_can_be_chosen_by_name_letter_or_number() {
        let text = write_records("a,b\n1,2\n", "out.csv", "@b, A, #");
        assert_eq!(text, "b,A,#\n2,1,1\n");
    }

    #[test]
    fn a_wrong_column_is_an_error() {
        let tmp = csv_rows(1);
        let source = table_source(&tmp.path().join("in.csv"), None);
        let error = preview(&source, output("@nope", "x.csv")).unwrap_err();
        assert_eq!(error, "Mask: no field named nope");
    }

    #[test]
    fn whole_records_of_text_have_the_number_the_line_and_the_groups() {
        let source = text_source(
            "ip 1.2 get\nskip\n",
            Some(vec![0]),
            Some(r"ip (?P<addr>\S+) (\w+)"),
        );
        let found = preview(&source, output("", "x.jsonl")).unwrap();
        let value: serde_json::Value = serde_json::from_str(&found.lines[0]).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "line_number": 1, "line": "ip 1.2 get", "addr": "1.2", "2": "get"
            })
        );
    }

    #[test]
    fn a_column_found_after_the_preview_is_told_not_silently_lost() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.jsonl");
        let mut body = String::new();
        for i in 0..SCAN_RECORDS + 5 {
            body.push_str(&format!("{{\"a\": {i}}}\n"));
        }
        body.push_str("{\"a\": 1, \"late\": 2}\n");
        fs::write(&path, body).unwrap();
        let out = tmp.path().join("out.jsonl");
        let source = table_source(&path, None);
        let found = preview(&source, output("", "out.jsonl")).unwrap();
        let done = run_to_file(source, found.output, &out);
        assert_eq!(done.stats.late_columns, 1);
        assert!(done.stats.warning().unwrap().contains("left out"));
    }

    #[test]
    fn the_preview_and_the_end_say_when_text_was_cut() {
        let source = Source::Text {
            text: "a\nb\n".into(),
            total: 1,
            kept: None,
            regex: None,
            cut: true,
        };
        let found = preview(&source, mask("%{line}", Escape::Line)).unwrap();
        assert!(found.cut);
        assert_eq!(found.lines, ["a"]);
        let tmp = crate::testing::tempdir();
        let done = run_to_file(source, found.output, &tmp.path().join("o.txt"));
        assert!(done.cut);
        let tmp = csv_rows(1);
        let source = table_source(&tmp.path().join("in.csv"), None);
        assert!(!preview(&source, mask("%{id}", Escape::Line)).unwrap().cut);
    }

    /// Writes `in.csv` and the template, and the file `out_name` from them.
    fn through_template(template: &str, body: &str, out_name: &str, mask_text: &str) -> String {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.csv");
        fs::write(&path, body).unwrap();
        let dir = tmp.path().join("batch");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("t.tpl"), template).unwrap();
        let out = tmp.path().join(out_name);
        let parsed = Output::parse_in(mask_text, &out, &path, 1, Some(&dir)).unwrap();
        let source = table_source(&path, None);
        let found = preview(&source, parsed).unwrap();
        run_to_file(source, found.output, &out);
        fs::read_to_string(&out).unwrap()
    }

    fn template_error(template: &str) -> String {
        let tmp = crate::testing::tempdir();
        fs::write(tmp.path().join("t.tpl"), template).unwrap();
        Output::parse_in(
            ":t",
            Path::new("x.txt"),
            Path::new("a"),
            0,
            Some(tmp.path()),
        )
        .unwrap_err()
    }

    const NEWLINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

    #[test]
    fn a_template_writes_its_header_records_and_footer() {
        let template =
            "%%# a comment\n%%header\nfirst\n%%record\n- %{#}: %{name}\n  %{id}\n%%footer\nlast\n";
        let text = through_template(template, "id,name\n1,a\n2,b\n", "out.txt", ":t");
        let expected = "first\n- 1: a\n  1\n- 2: b\n  2\nlast\n";
        assert_eq!(text, expected.replace('\n', NEWLINE));
    }

    #[test]
    fn a_template_with_no_sections_is_all_record_and_ends_its_lines() {
        let text = through_template("%{id}", "id\n1\n2\n", "out.txt", " :t ");
        assert_eq!(text, "1\n2\n".replace('\n', NEWLINE));
    }

    #[test]
    fn a_template_follows_the_escape_of_the_output_and_reads_crlf() {
        let template = "%%header\r\n<ul>\r\n%%record\r\n<li>%{v}</li>\r\n<!-- %{v:raw} -->\r\n%%footer\r\n</ul>\r\n";
        let text = through_template(template, "v\n<x>\n", "out.html", ":t");
        let expected = "<ul>\n<li>&lt;x&gt;</li>\n<!-- <x> -->\n</ul>\n";
        assert_eq!(text, expected.replace('\n', NEWLINE));
    }

    #[test]
    fn a_template_quotes_a_hostile_value_in_a_script() {
        let text = through_template("echo %{v}\n", "v\n$(id)\n", "run.sh", ":t");
        assert_eq!(text, "#!/bin/sh\necho '$(id)'\n");
    }

    #[test]
    fn the_open_file_goes_through_the_escape_everywhere() {
        let m = Mask::parse(
            "%{#name} %{#base:url} %{#ext} %{#size} %{#dir:raw}",
            Escape::Xml,
            Path::new("/d/<a b>.csv"),
            10,
        )
        .unwrap();
        assert_eq!(
            m.parts,
            [Part::Literal(
                b"&lt;a b&gt;.csv %3Ca%20b%3E csv 10 /d".to_vec()
            )]
        );
        let text = through_template(
            "%%header\n<title>%{#name}</title>\n%%record\n%{v}\n",
            "v\n1\n",
            "out.html",
            ":t",
        );
        assert!(text.starts_with("<title>in.csv</title>"), "{text}");
    }

    #[test]
    fn a_footer_knows_how_many_records_were_written() {
        let template = "%%record\n%{id}\n%%footer\n-- %{count} records\n";
        let text = through_template(template, "id\n1\n2\n3\n", "out.txt", ":t");
        assert_eq!(text, "1\n2\n3\n-- 3 records\n".replace('\n', NEWLINE));
        // A number, whatever the file: no quotes in SQL.
        let text = through_template(template, "id\n1\n", "out.sql", ":t");
        assert!(text.ends_with(&format!("-- 1 records{NEWLINE}")), "{text}");
    }

    #[test]
    fn a_column_called_count_is_still_a_column_in_a_record() {
        let template = "%%record\n%{count}\n%%footer\n%{count}\n";
        let text = through_template(template, "count\n7\n8\n", "out.txt", ":t");
        assert_eq!(text, "7\n8\n2\n".replace('\n', NEWLINE));
    }

    #[test]
    fn the_date_is_the_same_in_every_field_and_comes_in_each_form() {
        use chrono::TimeZone;
        let moment = Utc.with_ymd_and_hms(2026, 10, 30, 17, 5, 9).unwrap();
        let context = Context {
            now: moment.with_timezone(&Local),
            ..test_context(Path::new("/d/a.csv"), 1)
        };
        let value = |field: &str| {
            let m = Mask::parse_in(&format!("%{{{field}:raw}}"), Escape::Line, &context).unwrap();
            match m.parts.as_slice() {
                [Part::Literal(bytes)] => String::from_utf8(bytes.clone()).unwrap(),
                other => panic!("{other:?}"),
            }
        };
        let now = &context.now;
        assert_eq!(value("#utc"), "2026-10-30T17:05:09Z");
        assert_eq!(value("#epoch"), "1793379909");
        assert_eq!(value("#date"), now.format("%Y-%m-%d").to_string());
        assert_eq!(value("#time"), now.format("%H:%M:%S").to_string());
        let datetime = now.format("%Y-%m-%dT%H:%M:%S%:z").to_string();
        assert_eq!(value("#datetime"), datetime);
        assert_eq!(value("#year"), now.format("%Y").to_string());
        assert_eq!(value("#month"), now.format("%m").to_string());
        assert_eq!(value("#day"), now.format("%d").to_string());
        // Fridays in UTC; the local date may be another day, around midnight.
        let weekday = now.weekday();
        assert_eq!(value("#weekday"), weekday.number_from_monday().to_string());
        assert_eq!(
            value("#weekday_name"),
            now.format("%A").to_string().to_lowercase()
        );
        assert_eq!(value("#output"), "/o/out.txt");
    }

    #[test]
    fn a_shared_field_goes_through_the_escape() {
        let in_sql = Mask::parse("%{#date}", Escape::Sql, Path::new("a"), 0).unwrap();
        let [Part::Literal(bytes)] = in_sql.parts.as_slice() else {
            panic!("{:?}", in_sql.parts);
        };
        assert!(bytes.starts_with(b"'") && bytes.ends_with(b"'"));

        let path = std::env::var("PATH").unwrap();
        let in_shell = Mask::parse("%{$PATH}", Escape::Shell, Path::new("a"), 0).unwrap();
        assert_eq!(
            in_shell.parts,
            [Part::Literal(format!("'{path}'").into_bytes())]
        );
    }

    #[test]
    fn the_environment_is_read_by_name_and_told() {
        let m = Mask::parse("%{$PATH:raw}", Escape::Line, Path::new("a"), 0).unwrap();
        assert_eq!(m.environment, ["PATH"]);

        let error = |text: &str| Mask::parse(text, Escape::Line, Path::new("a"), 0).unwrap_err();
        assert_eq!(
            error("%{$XY_NO_SUCH_VARIABLE}"),
            "Mask: no environment variable XY_NO_SUCH_VARIABLE"
        );
        assert_eq!(error("%{$}"), "Mask: %{$} has no variable name");

        let template =
            "%%header\n%{$PATH:raw}\n%%record\n%{id}%{$HOME:raw}\n%%footer\n%{$PATH:raw}\n";
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.csv");
        fs::write(&path, "id\n1\n").unwrap();
        fs::write(tmp.path().join("t.tpl"), template).unwrap();
        if std::env::var_os("HOME").is_some() {
            let parsed =
                Output::parse_in(":t", Path::new("x.txt"), &path, 0, Some(tmp.path())).unwrap();
            let found = preview(&table_source(&path, None), parsed).unwrap();
            assert_eq!(found.environment, ["HOME", "PATH"]);
        }
        let csv = csv_rows(1);
        let source = table_source(&csv.path().join("in.csv"), None);
        let found = preview(&source, mask("%{id}", Escape::Line)).unwrap();
        assert!(found.environment.is_empty());
    }

    #[test]
    fn the_template_name_is_known_only_in_a_template() {
        let error = Mask::parse("%{#template}", Escape::Line, Path::new("a"), 0).unwrap_err();
        assert_eq!(error, "Mask: cannot tell %{#template} here");
        let text = through_template("%{#template}-%{id}\n", "id\n1\n", "out.txt", ":t");
        assert_eq!(text.trim_end(), "t-1");
    }

    #[test]
    fn a_wrong_template_says_what_is_wrong() {
        let error = template_error;
        assert_eq!(error(""), "Template t: it has no record");
        assert_eq!(error("%%header\nx\n"), "Template t: it has no record");
        assert_eq!(error("%%record\n \n"), "Template t: it has no record");
        assert_eq!(error("%%heder\nx\n"), "Template t: unknown section %%heder");
        assert_eq!(
            error("%%record\na\n%%record\nb\n"),
            "Template t: two sections %%record"
        );
        assert_eq!(
            error("loose\n%%record\na\n"),
            "Template t: text before the first section"
        );
        assert_eq!(
            error("%%footer\n%{id}\n%%record\na\n"),
            "Template t: %{id} is a field of a record, outside the record"
        );
        assert_eq!(
            error("%%header\n%{count}\n%%record\na\n"),
            "Template t: %{count} is only known in the footer"
        );
        assert_eq!(
            error("%{a:nope}\n"),
            "Template t: Mask: unknown escape :nope"
        );
    }

    #[test]
    fn a_template_must_be_a_small_file_in_its_folder() {
        let tmp = crate::testing::tempdir();
        let dir = tmp.path();
        let parse =
            |text: &str| Output::parse_in(text, Path::new("x.txt"), Path::new("a"), 0, Some(dir));
        assert!(parse(":none").unwrap_err().starts_with("No template none"));
        fs::create_dir(dir.join("d.tpl")).unwrap();
        assert!(parse(":d").unwrap_err().contains("not a file"));
        let big = "x".repeat(MAX_TEMPLATE_BYTES as usize + 1);
        fs::write(dir.join("big.tpl"), big).unwrap();
        assert!(parse(":big").unwrap_err().contains("too large"));
        fs::write(dir.join("bin.tpl"), [0xFF, 0xFE]).unwrap();
        assert!(parse(":bin").is_err());
        assert!(
            Output::parse_in(":t", Path::new("x"), Path::new("a"), 0, None)
                .unwrap_err()
                .contains("no configuration folder")
        );
    }

    #[test]
    fn only_a_plain_name_asks_for_a_template() {
        assert_eq!(template_name(":a-b_c.1"), Some("a-b_c.1"));
        for mask in [
            ":", ":../up", ":a/b", ":a\\b", ":.hidden", ": %{a}", ":%{a}", "a",
        ] {
            assert_eq!(template_name(mask), None, "{mask}");
        }
        assert!(matches!(output(": %{id}", "x.sh"), Output::Lines(_)));
    }

    #[test]
    fn the_templates_there_are_come_sorted() {
        let tmp = crate::testing::tempdir();
        assert!(names_in(&tmp.path().join("none")).is_empty());
        for name in ["b.tpl", "a.tpl", "c.txt", ".h.tpl"] {
            fs::write(tmp.path().join(name), "x").unwrap();
        }
        assert_eq!(names_in(tmp.path()), ["a", "b"]);
    }

    #[test]
    fn the_preview_of_a_long_header_still_shows_records() {
        let tmp = crate::testing::tempdir();
        let path = tmp.path().join("in.csv");
        fs::write(&path, "v\nrow\n").unwrap();
        let header: String = (0..40).map(|i| format!("h{i}\n")).collect();
        fs::write(
            tmp.path().join("t.tpl"),
            format!("%%header\n{header}%%record\n%{{v}}\n"),
        )
        .unwrap();
        let parsed =
            Output::parse_in(":t", Path::new("x.txt"), &path, 0, Some(tmp.path())).unwrap();
        let found = preview(&table_source(&path, None), parsed).unwrap();
        assert_eq!(found.lines, ["h37", "h38", "h39", "row"]);
    }

    proptest! {
        /// Whatever the value, it stays one literal and reads back the same.
        #[test]
        fn a_mysql_literal_reads_back_the_value(text in "[\\\\'a0Z\\x00\\x1a\\n ;-]{0,30}") {
            let mysql = escaped(t(&text), Escape::Mysql);
            prop_assert_eq!(read_mysql_literal(&mysql), Some((text.clone(), "")));
        }

        #[test]
        fn a_template_never_panics(text in "(%%|%|\\{|\\}|header|record|footer|\\n|.){0,40}") {
            let open = Path::new("/a/b");
            let _ = parse_template(&text, Escape::Shell, &test_context(open, 1));
        }

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
