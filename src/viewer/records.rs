//! Where tables come from: each format as a stream of records.
//!
//! A record has the file's values as they are, typed when the format has
//! types, and neither cut nor cleaned: what they may do on a screen is the
//! grid's concern (`tabular_loader`), and it is the only one that keeps rows.
//! The stream has no row limit: it ends when the file does, when the sink
//! says so, or when the time is up.

use std::borrow::Cow;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek};
use std::ops::ControlFlow;
use std::path::Path;
use std::time::{Duration, Instant};

use arrow_array::Array;
use arrow_array::cast::AsArray;
use arrow_array::types::{
    Float32Type, Float64Type, Int8Type, Int16Type, Int32Type, Int64Type, UInt8Type, UInt16Type,
    UInt32Type, UInt64Type,
};
use arrow_schema::DataType;
use calamine::{Data, Reader, open_workbook_auto};
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::Value as JsonValue;

use super::table::TableGrid;

/// Columns read per record, in any format: a hostile file could have millions.
pub const MAX_COLS: usize = 256;
/// A line of JSON Lines longer than this is not read as a record.
const MAX_LINE_BYTES: u64 = 512 * 1024;
/// How many steps of SQLite's virtual machine between looks at the clock.
const SQLITE_STEPS: i32 = 10_000;

/// A value of a record. Text and bytes are borrowed whenever the reader allows
/// it, so that a huge cell costs nothing to whoever only wants its start.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(Cow<'a, str>),
    Bytes(&'a [u8]),
}

impl Cell<'_> {
    /// The cell as the grid shows it: safe for a terminal, and short.
    pub fn display(&self) -> String {
        match self {
            Cell::Null => String::new(),
            Cell::Bool(b) => b.to_string(),
            Cell::Int(i) => i.to_string(),
            Cell::Float(f) => f.to_string(),
            Cell::Text(s) => TableGrid::sanitize_cell(s),
            Cell::Bytes(b) => format!("<Blob {} bytes>", b.len()),
        }
    }
}

/// How far a reading may go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budget {
    /// When to stop. A reader looks at it now and then, between records.
    deadline: Option<Instant>,
    /// The most a spreadsheet may unpack to: its reader has no way to stop
    /// midway, so this is checked before it starts.
    unpacked: u64,
}

impl Budget {
    pub fn new(time: Duration, unpacked: u64) -> Self {
        Budget {
            deadline: Some(Instant::now() + time),
            unpacked,
        }
    }

    #[cfg(test)]
    pub fn unlimited() -> Self {
        Budget {
            deadline: None,
            unpacked: u64::MAX,
        }
    }

    fn over(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }
}

/// Why a stream ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// All of the file was read.
    Complete,
    /// The sink had enough.
    Refused,
    /// The time ran out.
    OutOfTime,
}

/// Who receives a stream. Columns come before the records they apply to and
/// may grow at the end (JSON Lines finds its keys as it goes), so a record may
/// be shorter or longer than the current columns.
pub trait RecordSink {
    /// The file's sheets or tables, and which one is being read.
    fn sheets(&mut self, names: &[String], current: usize);
    fn columns(&mut self, names: &[String]);
    /// The file has more than [`MAX_COLS`] columns.
    fn columns_cut(&mut self);
    fn record(&mut self, cells: &[Cell<'_>]) -> ControlFlow<()>;
}

/// Looks at the clock every so many records.
struct Pace {
    budget: Budget,
    count: u32,
}

impl Pace {
    fn new(budget: Budget) -> Self {
        Pace { budget, count: 0 }
    }

    fn over(&mut self) -> bool {
        self.count = self.count.wrapping_add(1);
        self.count & 0xFF == 0 && self.budget.over()
    }
}

/// Gives the sink a record, unless the time is up or it refuses; `None` to go on.
fn deliver(pace: &mut Pace, sink: &mut dyn RecordSink, cells: &[Cell<'_>]) -> Option<End> {
    if pace.over() {
        return Some(End::OutOfTime);
    }
    match sink.record(cells) {
        ControlFlow::Continue(()) => None,
        ControlFlow::Break(()) => Some(End::Refused),
    }
}

/// Spreadsheet column names: 0 -> "A", 25 -> "Z", 26 -> "AA"...
pub fn column_letter(mut col_idx: usize) -> String {
    let mut s = String::new();
    loop {
        let rem = (col_idx % 26) as u8;
        s.push((b'A' + rem) as char);
        if col_idx < 26 {
            break;
        }
        col_idx = (col_idx / 26) - 1;
    }
    s.chars().rev().collect()
}

/// Helper to pick best CSV delimiter by frequency outside quotes.
fn guess_csv_delimiter(sample: &str) -> u8 {
    let mut comma = 0usize;
    let mut semi = 0usize;
    let mut tab = 0usize;
    let mut pipe = 0usize;
    let mut in_quotes = false;

    for c in sample.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            match c {
                ',' => comma += 1,
                ';' => semi += 1,
                '\t' => tab += 1,
                '|' => pipe += 1,
                _ => {}
            }
        }
    }

    if tab > 0 && tab >= comma && tab >= semi {
        b'\t'
    } else if semi > comma && semi >= pipe {
        b';'
    } else if pipe > comma && pipe > semi {
        b'|'
    } else {
        b','
    }
}

fn text_cells(record: &csv::StringRecord) -> Vec<Cell<'_>> {
    record
        .iter()
        .take(MAX_COLS)
        .map(|field| Cell::Text(Cow::Borrowed(field)))
        .collect()
}

/// A CSV or TSV file: every value is text, since the format has no types.
pub fn read_csv(
    path: &Path,
    delimiter: Option<u8>,
    budget: Budget,
    sink: &mut dyn RecordSink,
) -> Result<End, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open CSV: {e}"))?;
    let mut reader = BufReader::new(file);

    // Auto-detect delimiter from bounded sample
    let delim = match delimiter {
        Some(d) => d,
        None => {
            let mut sample = String::new();
            let _ = (&mut reader).take(8192).read_to_string(&mut sample);
            let _ = reader.seek(std::io::SeekFrom::Start(0));
            guess_csv_delimiter(&sample)
        }
    };

    let mut csv_reader = csv::ReaderBuilder::new()
        .delimiter(delim)
        .has_headers(false)
        .flexible(true)
        .from_reader(reader);

    let mut records = csv_reader.records();
    let first = match records.next() {
        Some(r) => r.map_err(|e| format!("CSV read error: {e}"))?,
        None => return Ok(End::Complete),
    };
    if first.is_empty() {
        return Ok(End::Complete);
    }

    // Determine if first row is a header: at least half the fields are non-numeric strings
    let non_numeric_count = first
        .iter()
        .filter(|field| {
            let t = field.trim();
            !t.is_empty() && t.parse::<f64>().is_err()
        })
        .count();
    let has_header = non_numeric_count * 2 >= first.len();

    sink.sheets(&["Data".to_string()], 0);
    if first.len() > MAX_COLS {
        sink.columns_cut();
    }
    let mut pace = Pace::new(budget);
    if has_header {
        let names: Vec<String> = first
            .iter()
            .take(MAX_COLS)
            .map(|s| s.trim().to_string())
            .collect();
        sink.columns(&names);
    } else {
        let names: Vec<String> = (0..first.len().min(MAX_COLS)).map(column_letter).collect();
        sink.columns(&names);
        if let Some(end) = deliver(&mut pace, sink, &text_cells(&first)) {
            return Ok(end);
        }
    }

    for result in records {
        let record = result.map_err(|e| format!("CSV record error: {e}"))?;
        if let Some(end) = deliver(&mut pace, sink, &text_cells(&record)) {
            return Ok(end);
        }
    }
    Ok(End::Complete)
}

fn json_cell(value: Option<&JsonValue>) -> Cell<'_> {
    match value {
        None | Some(JsonValue::Null) => Cell::Null,
        Some(JsonValue::Bool(b)) => Cell::Bool(*b),
        Some(JsonValue::String(s)) => Cell::Text(Cow::Borrowed(s)),
        Some(JsonValue::Number(n)) => match (n.as_i64(), n.as_u64(), n.as_f64()) {
            (Some(i), _, _) => Cell::Int(i),
            // Above i64: as text, since a float would lose digits.
            (None, Some(_), _) => Cell::Text(Cow::Owned(n.to_string())),
            (None, None, Some(f)) => Cell::Float(f),
            _ => Cell::Text(Cow::Owned(n.to_string())),
        },
        Some(other) => Cell::Text(Cow::Owned(other.to_string())),
    }
}

/// A JSON Lines (.jsonl / .ndjson) file. Lines that are not objects are skipped.
pub fn read_jsonl(path: &Path, budget: Budget, sink: &mut dyn RecordSink) -> Result<End, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open JSONL: {e}"))?;
    let mut reader = BufReader::new(file);

    let mut names: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut pace = Pace::new(budget);
    let mut cut = false;

    loop {
        line.clear();
        // Read with bounded capacity per line to prevent OOM
        let bytes_read = (&mut reader)
            .take(MAX_LINE_BYTES)
            .read_line(&mut line)
            .map_err(|e| format!("Read error: {e}"))?;
        if bytes_read == 0 {
            return Ok(End::Complete);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(JsonValue::Object(map)) = serde_json::from_str(trimmed) else {
            continue;
        };

        let known = names.len();
        for key in map.keys() {
            if names.contains(key) {
                continue;
            }
            if names.len() < MAX_COLS {
                names.push(key.clone());
            } else if !cut {
                cut = true;
                sink.columns_cut();
            }
        }
        if names.len() > known {
            if known == 0 {
                sink.sheets(&["JSONL".to_string()], 0);
            }
            sink.columns(&names);
        }
        if names.is_empty() {
            continue;
        }

        let cells: Vec<Cell<'_>> = names.iter().map(|name| json_cell(map.get(name))).collect();
        if let Some(end) = deliver(&mut pace, sink, &cells) {
            return Ok(end);
        }
    }
}

/// A dBase value. Padding is how the format stores short text, not data.
fn dbase_cell(val: &dbase::FieldValue) -> Cell<'_> {
    use dbase::FieldValue as F;
    match val {
        F::Character(Some(s)) => Cell::Text(Cow::Borrowed(s.trim_end())),
        F::Numeric(Some(n)) => Cell::Float(*n),
        F::Logical(Some(b)) => Cell::Bool(*b),
        F::Date(Some(d)) => Cell::Text(Cow::Owned(format!(
            "{:04}-{:02}-{:02}",
            d.year(),
            d.month(),
            d.day()
        ))),
        // Through text, so that 1.1 stays 1.1 and not 1.100000023841858.
        F::Float(Some(f)) => Cell::Float(f.to_string().parse().unwrap_or(f64::from(*f))),
        F::Integer(i) => Cell::Int(i64::from(*i)),
        // As text: money keeps its two decimals.
        F::Currency(c) => Cell::Text(Cow::Owned(format!("{c:.2}"))),
        F::Double(d) => Cell::Float(*d),
        F::DateTime(dt) => {
            let (d, t) = (dt.date(), dt.time());
            Cell::Text(Cow::Owned(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                d.year(),
                d.month(),
                d.day(),
                t.hours(),
                t.minutes(),
                t.seconds()
            )))
        }
        F::Memo(m) => Cell::Text(Cow::Borrowed(m)),
        _ => Cell::Null,
    }
}

/// A dBase / Clipper / FoxPro .dbf file (with companion .dbt memo file if present).
pub fn read_dbf(path: &Path, budget: Budget, sink: &mut dyn RecordSink) -> Result<End, String> {
    let mut reader = dbase::Reader::from_path(path)
        .map_err(|e| format!("Failed to open DBF {}: {e}", path.display()))?;

    let total = reader.fields().len();
    let names: Vec<String> = reader
        .fields()
        .iter()
        .take(MAX_COLS)
        .map(|f| f.name().to_string())
        .collect();
    if names.is_empty() {
        return Ok(End::Complete);
    }
    sink.sheets(&["DBF".to_string()], 0);
    if total > MAX_COLS {
        sink.columns_cut();
    }
    sink.columns(&names);

    let mut pace = Pace::new(budget);
    for record in reader.iter_records() {
        let record = record.map_err(|e| format!("Error reading DBF record: {e}"))?;
        let cells: Vec<Cell<'_>> = names
            .iter()
            .map(|name| record.get(name).map_or(Cell::Null, dbase_cell))
            .collect();
        if let Some(end) = deliver(&mut pace, sink, &cells) {
            return Ok(end);
        }
    }
    Ok(End::Complete)
}

/// Refuses a spreadsheet that would unpack to more than the budget allows. A
/// spreadsheet is usually a ZIP of XML, so a small file can be enormous inside.
/// The sizes are the ones the ZIP declares.
fn check_unpacked(path: &Path, limit: u64) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("Failed to open spreadsheet: {e}"))?;
    let on_disk = file.metadata().map_err(|e| e.to_string())?.len();
    let size = match zip::ZipArchive::new(file) {
        Ok(mut archive) => {
            let mut sum = 0u64;
            for i in 0..archive.len() {
                let entry = archive
                    .by_index_raw(i)
                    .map_err(|e| format!("Failed to read spreadsheet: {e}"))?;
                sum = sum.saturating_add(entry.size());
            }
            sum
        }
        // An old .xls is not a ZIP; if a damaged one is, calamine will say.
        Err(_) => on_disk,
    };
    if size > limit {
        let mib = |n: u64| n.div_ceil(1 << 20);
        return Err(format!(
            "spreadsheet needs {} MiB to read, over the limit of {} MiB",
            mib(size),
            mib(limit)
        ));
    }
    Ok(())
}

fn sheet_cell(value: &Data) -> Cell<'_> {
    match value {
        Data::Empty => Cell::Null,
        Data::Int(i) => Cell::Int(*i),
        Data::Float(f) => Cell::Float(*f),
        Data::Bool(b) => Cell::Bool(*b),
        Data::String(s) => Cell::Text(Cow::Borrowed(s)),
        other => Cell::Text(Cow::Owned(other.to_string())),
    }
}

/// A sheet (by position) of an Excel / ODS spreadsheet using `calamine`, which
/// loads the whole sheet before the first record.
pub fn read_spreadsheet(
    path: &Path,
    sheet: usize,
    budget: Budget,
    sink: &mut dyn RecordSink,
) -> Result<End, String> {
    check_unpacked(path, budget.unpacked)?;
    let mut workbook = open_workbook_auto(path)
        .map_err(|e| format!("Failed to open spreadsheet {}: {e}", path.display()))?;

    let names = workbook.sheet_names();
    let Some(name) = names.get(sheet) else {
        return Err("Spreadsheet has no such sheet".to_string());
    };
    let range = workbook.worksheet_range(name).map_err(|e| {
        let shown = TableGrid::sanitize_cell(name);
        format!("Failed to read sheet '{shown}': {e}")
    })?;
    sink.sheets(&names, sheet);

    let (height, width) = range.get_size();
    let cols = width.min(MAX_COLS);
    if width > MAX_COLS {
        sink.columns_cut();
    }
    let letters: Vec<String> = (0..cols).map(column_letter).collect();
    sink.columns(&letters);

    let mut pace = Pace::new(budget);
    for r in 0..height {
        let cells: Vec<Cell<'_>> = (0..cols)
            .map(|c| range.get((r, c)).map_or(Cell::Null, sheet_cell))
            .collect();
        if let Some(end) = deliver(&mut pace, sink, &cells) {
            return Ok(end);
        }
    }
    Ok(End::Complete)
}

fn is_interrupted(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(e, _) if e.code == rusqlite::ErrorCode::OperationInterrupted
    )
}

/// A table or query result from an SQLite database in strictly defensive
/// read-only mode.
pub fn read_sqlite(
    path: &Path,
    table_or_query: Option<&str>,
    budget: Budget,
    sink: &mut dyn RecordSink,
) -> Result<End, String> {
    use rusqlite::{Connection, OpenFlags, ffi};

    // Open connection strictly READONLY (no URI scheme expansion)
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let path_str = path
        .to_str()
        .ok_or_else(|| "Invalid UTF-8 in SQLite path".to_string())?;

    let conn = Connection::open_with_flags(path_str, flags)
        .map_err(|e| format!("Failed to open SQLite database: {e}"))?;

    // Hardening: Defensive mode + Disable extensions
    // SAFETY: conn.handle() returns a valid sqlite3 connection handle managed by rusqlite.
    unsafe {
        let rc = ffi::sqlite3_db_config(
            conn.handle(),
            ffi::SQLITE_DBCONFIG_DEFENSIVE,
            1,
            std::ptr::null_mut::<i32>(),
        );
        if rc != ffi::SQLITE_OK {
            return Err("Failed to enable SQLITE_DBCONFIG_DEFENSIVE".to_string());
        }

        ffi::sqlite3_db_config(
            conn.handle(),
            ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION,
            0,
            std::ptr::null_mut::<i32>(),
        );
    }

    let _ = conn.execute_batch("PRAGMA query_only = ON;");

    // A query can take forever without giving a single row.
    if budget.deadline.is_some() {
        conn.progress_handler(SQLITE_STEPS, Some(move || budget.over()));
    }

    // List user tables (bounded to prevent DoS from DB with millions of tables)
    let mut table_stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 1000",
        )
        .map_err(|e| format!("Failed to query tables: {e}"))?;
    let tables: Vec<String> = table_stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Query error: {e}"))?
        .flatten()
        .collect();

    if tables.is_empty() {
        sink.columns(&["Status".to_string()]);
        let _ = sink.record(&[Cell::Text(Cow::Borrowed("(Database has no tables)"))]);
        return Ok(End::Complete);
    }

    // Determine query: user SQL query, specific table, or first available table
    let select = |idx: usize| format!("SELECT * FROM \"{}\"", tables[idx].replace('"', "\"\""));
    let (query, active) = match table_or_query {
        Some(q) if q.trim().to_ascii_uppercase().starts_with("SELECT ") => (q.to_string(), 0),
        Some(tbl) => {
            let idx = tables
                .iter()
                .position(|t| t == tbl)
                .or_else(|| {
                    tables
                        .iter()
                        .position(|t| TableGrid::sanitize_cell(t) == tbl)
                })
                .unwrap_or(0);
            (select(idx), idx)
        }
        None => (select(0), 0),
    };
    sink.sheets(&tables, active);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("SQL statement error: {e}"))?;

    let total = stmt.column_count();
    let names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .take(MAX_COLS)
        .map(str::to_string)
        .collect();
    let cols = names.len();
    if total > MAX_COLS {
        sink.columns_cut();
    }
    sink.columns(&names);

    let mut rows = stmt
        .query([])
        .map_err(|e| format!("SQL execution error: {e}"))?;

    let mut pace = Pace::new(budget);
    loop {
        let row = match rows.next() {
            Ok(Some(row)) => row,
            Ok(None) => return Ok(End::Complete),
            Err(e) if is_interrupted(&e) => return Ok(End::OutOfTime),
            Err(e) => return Err(format!("Row read error: {e}")),
        };
        let cells: Vec<Cell<'_>> = (0..cols)
            .map(|i| match row.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Null) | Err(_) => Cell::Null,
                Ok(rusqlite::types::ValueRef::Integer(i)) => Cell::Int(i),
                Ok(rusqlite::types::ValueRef::Real(r)) => Cell::Float(r),
                Ok(rusqlite::types::ValueRef::Text(t)) => Cell::Text(String::from_utf8_lossy(t)),
                Ok(rusqlite::types::ValueRef::Blob(b)) => Cell::Bytes(b),
            })
            .collect();
        if let Some(end) = deliver(&mut pace, sink, &cells) {
            return Ok(end);
        }
    }
}

/// A value of a Parquet column. Types with no [`Cell`] of their own (dates,
/// decimals, lists...) come as Arrow writes them.
fn parquet_cell(col: &dyn Array, r: usize) -> Cell<'_> {
    if col.is_null(r) {
        return Cell::Null;
    }
    match col.data_type() {
        DataType::Boolean => Cell::Bool(col.as_boolean().value(r)),
        DataType::Int8 => Cell::Int(i64::from(col.as_primitive::<Int8Type>().value(r))),
        DataType::Int16 => Cell::Int(i64::from(col.as_primitive::<Int16Type>().value(r))),
        DataType::Int32 => Cell::Int(i64::from(col.as_primitive::<Int32Type>().value(r))),
        DataType::Int64 => Cell::Int(col.as_primitive::<Int64Type>().value(r)),
        DataType::UInt8 => Cell::Int(i64::from(col.as_primitive::<UInt8Type>().value(r))),
        DataType::UInt16 => Cell::Int(i64::from(col.as_primitive::<UInt16Type>().value(r))),
        DataType::UInt32 => Cell::Int(i64::from(col.as_primitive::<UInt32Type>().value(r))),
        DataType::UInt64 => {
            let value = col.as_primitive::<UInt64Type>().value(r);
            i64::try_from(value).map_or_else(|_| Cell::Text(value.to_string().into()), Cell::Int)
        }
        // Through text, so that 1.1 stays 1.1 and not 1.100000023841858.
        DataType::Float32 => {
            let value = col.as_primitive::<Float32Type>().value(r);
            Cell::Float(value.to_string().parse().unwrap_or(f64::from(value)))
        }
        DataType::Float64 => Cell::Float(col.as_primitive::<Float64Type>().value(r)),
        DataType::Utf8 => Cell::Text(col.as_string::<i32>().value(r).into()),
        DataType::LargeUtf8 => Cell::Text(col.as_string::<i64>().value(r).into()),
        DataType::Utf8View => Cell::Text(col.as_string_view().value(r).into()),
        DataType::Binary => Cell::Bytes(col.as_binary::<i32>().value(r)),
        DataType::LargeBinary => Cell::Bytes(col.as_binary::<i64>().value(r)),
        DataType::BinaryView => Cell::Bytes(col.as_binary_view().value(r)),
        _ => Cell::Text(
            arrow_cast::display::array_value_to_string(col, r)
                .unwrap_or_else(|_| "<err>".to_string())
                .into(),
        ),
    }
}

/// An Apache Parquet file (.parquet). Columns past [`MAX_COLS`] are never
/// decoded from disk.
pub fn read_parquet(path: &Path, budget: Budget, sink: &mut dyn RecordSink) -> Result<End, String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open Parquet file: {e}"))?;

    // Validate size and magic markers (PAR1 at start and end)
    let file_len = file.metadata().map_err(|e| e.to_string())?.len();
    if file_len < 12 {
        return Err("File too small to be a valid Parquet file (min 12 bytes)".to_string());
    }

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|e| format!("Failed to read Parquet magic header: {e}"))?;
    if &magic != b"PAR1" {
        return Err("Invalid Parquet file: missing PAR1 magic header".to_string());
    }

    // Validate footer PAR1 marker
    file.seek(std::io::SeekFrom::End(-4))
        .map_err(|e| format!("Seek to footer failed: {e}"))?;
    file.read_exact(&mut magic)
        .map_err(|e| format!("Failed to read Parquet magic footer: {e}"))?;
    if &magic != b"PAR1" {
        return Err("Invalid Parquet file: missing PAR1 magic footer".to_string());
    }

    // Rewind back to beginning for Parquet reader
    file.seek(std::io::SeekFrom::Start(0))
        .map_err(|e| format!("Seek failed: {e}"))?;

    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| format!("Failed to parse Parquet metadata: {e}"))?;

    let schema = builder.schema().clone();
    let total = schema.fields().len();
    if total == 0 {
        return Ok(End::Complete);
    }
    let cols = total.min(MAX_COLS);
    let names: Vec<String> = schema.fields()[..cols]
        .iter()
        .map(|f| f.name().to_string())
        .collect();
    sink.sheets(&["Parquet".to_string()], 0);
    if total > MAX_COLS {
        sink.columns_cut();
    }
    sink.columns(&names);

    let mask = ProjectionMask::roots(builder.parquet_schema(), 0..cols);
    let reader = builder
        .with_projection(mask)
        .with_batch_size(1024)
        .build()
        .map_err(|e| format!("Failed to create Parquet batch reader: {e}"))?;

    let mut pace = Pace::new(budget);
    for batch in reader {
        let batch = batch.map_err(|e| format!("Error decoding Parquet batch: {e}"))?;
        let columns = batch.columns();
        for r in 0..batch.num_rows() {
            let cells: Vec<Cell<'_>> = columns
                .iter()
                .take(cols)
                .map(|col| parquet_cell(col.as_ref(), r))
                .collect();
            if let Some(end) = deliver(&mut pace, sink, &cells) {
                return Ok(end);
            }
        }
    }
    Ok(End::Complete)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// A value with nothing borrowed, to compare with.
    #[derive(Debug, Clone, PartialEq)]
    enum Owned {
        Null,
        Bool(bool),
        Int(i64),
        Float(f64),
        Text(String),
        Bytes(Vec<u8>),
    }

    fn text(s: &str) -> Owned {
        Owned::Text(s.to_string())
    }

    /// Keeps everything it receives, up to `stop` records.
    #[derive(Default)]
    struct Collect {
        columns: Vec<String>,
        rows: Vec<Vec<Owned>>,
        stop: Option<usize>,
        cut: bool,
    }

    impl RecordSink for Collect {
        fn sheets(&mut self, _: &[String], _: usize) {}
        fn columns(&mut self, names: &[String]) {
            self.columns = names.to_vec();
        }
        fn columns_cut(&mut self) {
            self.cut = true;
        }
        fn record(&mut self, cells: &[Cell<'_>]) -> ControlFlow<()> {
            if self.stop == Some(self.rows.len()) {
                return ControlFlow::Break(());
            }
            self.rows.push(
                cells
                    .iter()
                    .map(|c| match c {
                        Cell::Null => Owned::Null,
                        Cell::Bool(b) => Owned::Bool(*b),
                        Cell::Int(i) => Owned::Int(*i),
                        Cell::Float(f) => Owned::Float(*f),
                        Cell::Text(s) => Owned::Text(s.to_string()),
                        Cell::Bytes(b) => Owned::Bytes(b.to_vec()),
                    })
                    .collect(),
            );
            ControlFlow::Continue(())
        }
    }

    fn file_with(text: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(text.as_bytes()).unwrap();
        file
    }

    fn expired() -> Budget {
        Budget {
            deadline: Some(Instant::now()),
            unpacked: u64::MAX,
        }
    }

    #[test]
    fn values_come_as_they_are() {
        let long = "x".repeat(10_000);
        let file = file_with(&format!(
            "name,code\n\"a\u{1b}[31m\u{202E}b\",007\n{long},1\n"
        ));
        let mut sink = Collect::default();
        let end = read_csv(file.path(), None, Budget::unlimited(), &mut sink).unwrap();
        assert_eq!(end, End::Complete);
        assert_eq!(sink.columns, ["name", "code"]);
        assert_eq!(sink.rows[0], [text("a\u{1b}[31m\u{202E}b"), text("007")]);
        assert_eq!(sink.rows[1], [text(&long), text("1")]);
    }

    #[test]
    fn the_stream_has_no_row_limit() {
        let mut body = String::from("a,b\n");
        for i in 0..60_000 {
            body.push_str(&format!("{i},x\n"));
        }
        let file = file_with(&body);
        let mut sink = Collect::default();
        let end = read_csv(file.path(), None, Budget::unlimited(), &mut sink).unwrap();
        assert_eq!(end, End::Complete);
        assert_eq!(sink.rows.len(), 60_000);
        assert_eq!(sink.rows[59_999][0], text("59999"));
    }

    #[test]
    fn the_sink_can_stop_the_stream() {
        let file = file_with("a\nb\nc\nd\n");
        let mut sink = Collect {
            stop: Some(1),
            ..Default::default()
        };
        let end = read_csv(file.path(), Some(b','), Budget::unlimited(), &mut sink).unwrap();
        assert_eq!(end, End::Refused);
        assert_eq!(sink.rows.len(), 1);
    }

    #[test]
    fn a_stream_stops_when_the_time_is_up() {
        let mut body = String::from("a\n");
        for i in 0..2_000 {
            body.push_str(&format!("{i}x\n"));
        }
        let file = file_with(&body);
        let mut sink = Collect::default();
        let end = read_csv(file.path(), None, expired(), &mut sink).unwrap();
        assert_eq!(end, End::OutOfTime);
        assert!(sink.rows.len() < 2_000);
    }

    #[test]
    fn json_lines_keep_their_types_and_find_columns_as_they_go() {
        let file = file_with(concat!(
            r#"{"id": 1, "ok": true, "pi": 3.5, "s": "007"}"#,
            "\n\nnot json\n",
            r#"{"id": 18446744073709551615, "list": [1, 2], "s": null}"#,
            "\n"
        ));
        let mut sink = Collect::default();
        read_jsonl(file.path(), Budget::unlimited(), &mut sink).unwrap();
        assert_eq!(sink.columns, ["id", "ok", "pi", "s", "list"]);
        assert_eq!(
            sink.rows[0],
            [
                Owned::Int(1),
                Owned::Bool(true),
                Owned::Float(3.5),
                text("007")
            ]
        );
        assert_eq!(
            sink.rows[1],
            [
                text("18446744073709551615"),
                Owned::Null,
                Owned::Null,
                Owned::Null,
                text("[1,2]")
            ]
        );
    }

    #[test]
    fn dbase_values_have_types() {
        use dbase::{Date, FieldValue as F};
        assert_eq!(
            dbase_cell(&F::Character(Some("Test  ".to_string()))),
            Cell::Text("Test".into())
        );
        assert_eq!(dbase_cell(&F::Numeric(Some(123.45))), Cell::Float(123.45));
        assert_eq!(dbase_cell(&F::Logical(Some(true))), Cell::Bool(true));
        assert_eq!(dbase_cell(&F::Logical(None)), Cell::Null);
        assert_eq!(dbase_cell(&F::Float(Some(1.1))), Cell::Float(1.1));
        assert_eq!(
            dbase_cell(&F::Date(Date::new(30, 10, 2026).ok())),
            Cell::Text("2026-10-30".into())
        );
        assert_eq!(dbase_cell(&F::Currency(42.5)), Cell::Text("42.50".into()));
        let memo = "m".repeat(5_000);
        assert_eq!(
            dbase_cell(&F::Memo(memo.clone())),
            Cell::Text(memo.as_str().into()),
            "not cut"
        );
    }

    fn database(sql: &str) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        rusqlite::Connection::open(file.path())
            .unwrap()
            .execute_batch(sql)
            .unwrap();
        file
    }

    #[test]
    fn sqlite_gives_its_types_and_whole_values() {
        let long = "y".repeat(20_000);
        let file = database(&format!(
            "CREATE TABLE t (a, b, c, d, e);
             INSERT INTO t VALUES (1, 2.5, '{long}', x'00FF', NULL);"
        ));
        let mut sink = Collect::default();
        read_sqlite(file.path(), None, Budget::unlimited(), &mut sink).unwrap();
        assert_eq!(sink.columns, ["a", "b", "c", "d", "e"]);
        assert_eq!(
            sink.rows[0],
            [
                Owned::Int(1),
                Owned::Float(2.5),
                text(&long),
                Owned::Bytes(vec![0, 255]),
                Owned::Null
            ]
        );
    }

    #[test]
    fn a_query_that_never_gives_a_row_is_stopped() {
        let file = database("CREATE TABLE t (a);");
        let query = "SELECT x FROM (WITH RECURSIVE c(x) AS \
                     (SELECT 1 UNION ALL SELECT x + 1 FROM c) SELECT x FROM c) WHERE x < 0";
        let budget = Budget::new(Duration::from_millis(100), u64::MAX);
        let mut sink = Collect::default();
        let end = read_sqlite(file.path(), Some(query), budget, &mut sink).unwrap();
        assert_eq!(end, End::OutOfTime);
        assert!(sink.rows.is_empty());
    }

    #[test]
    fn a_spreadsheet_over_the_limit_is_not_read() {
        let file = NamedTempFile::new().unwrap();
        {
            let mut zip = zip::ZipWriter::new(file.reopen().unwrap());
            zip.start_file("xl/big.xml", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&vec![b' '; 3 << 20]).unwrap();
            zip.finish().unwrap();
        }
        let error = check_unpacked(file.path(), 1 << 20).unwrap_err();
        assert_eq!(
            error,
            "spreadsheet needs 3 MiB to read, over the limit of 1 MiB"
        );
        assert!(check_unpacked(file.path(), 4 << 20).is_ok());
    }

    #[test]
    fn parquet_gives_its_types() {
        use arrow_array::{BooleanArray, Float32Array, Int64Array, RecordBatch, StringArray};
        use arrow_schema::{Field, Schema};
        use parquet::arrow::arrow_writer::ArrowWriter;
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new("i", DataType::Int64, false),
            Field::new("f", DataType::Float32, false),
            Field::new("b", DataType::Boolean, false),
            Field::new("s", DataType::Utf8, true),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![7])),
                Arc::new(Float32Array::from(vec![1.1])),
                Arc::new(BooleanArray::from(vec![true])),
                Arc::new(StringArray::from(vec![None::<&str>])),
            ],
        )
        .unwrap();
        let file = NamedTempFile::new().unwrap();
        {
            let mut writer = ArrowWriter::try_new(file.reopen().unwrap(), schema, None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }
        let mut sink = Collect::default();
        read_parquet(file.path(), Budget::unlimited(), &mut sink).unwrap();
        assert_eq!(
            sink.rows[0],
            [
                Owned::Int(7),
                Owned::Float(1.1),
                Owned::Bool(true),
                Owned::Null
            ]
        );
    }

    proptest::proptest! {
        /// Whatever the bytes, a reader gives a result: a hostile file must
        /// not take the program down.
        #[test]
        fn any_bytes_never_panic(bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..2048)) {
            let mut file = NamedTempFile::new().unwrap();
            file.write_all(&bytes).unwrap();
            let path = file.path();
            let budget = Budget::unlimited();
            let _ = read_csv(path, None, budget, &mut Collect::default());
            let _ = read_jsonl(path, budget, &mut Collect::default());
            let _ = read_dbf(path, budget, &mut Collect::default());
            let _ = read_spreadsheet(path, 0, budget, &mut Collect::default());
            let _ = read_parquet(path, budget, &mut Collect::default());
            let _ = read_sqlite(path, None, budget, &mut Collect::default());
        }
    }

    #[test]
    fn columns_past_the_limit_are_cut_and_said() {
        let header: Vec<String> = (0..MAX_COLS + 4).map(|i| format!("c{i}")).collect();
        let file = file_with(&format!("{}\n", header.join(",")));
        let mut sink = Collect::default();
        read_csv(file.path(), None, Budget::unlimited(), &mut sink).unwrap();
        assert_eq!(sink.columns.len(), MAX_COLS);
        assert!(sink.cut);
    }
}
