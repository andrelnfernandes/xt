//! Tabular file readers: CSV, TSV, JSON Lines, and Spreadsheets (calamine).
//!
//! Enforces security bounds (maximum rows, maximum columns, memory caps)
//! to prevent DoS from hostile or malformed inputs.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek};
use std::path::Path;

use calamine::{Reader, open_workbook_auto};
use serde_json::Value as JsonValue;

use super::table::{TableGrid, TableRow};

/// Hard security limits for in-memory tabular rendering.
pub const MAX_TABLE_ROWS: usize = 50_000;
pub const MAX_TABLE_COLS: usize = 256;
pub const MAX_LINE_BYTES: u64 = 512 * 1024; // 512 KiB per line max to prevent DoS

/// Helper to generate spreadsheet column names: 0 -> "A", 1 -> "B", 25 -> "Z", 26 -> "AA"...
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

/// Detects if a file looks like a CSV, TSV, JSONL, DBF, Spreadsheet or SQLite DB.
pub fn is_tabular_candidate(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    matches!(
        ext.as_str(),
        "csv"
            | "tsv"
            | "tab"
            | "jsonl"
            | "ndjson"
            | "xlsx"
            | "ods"
            | "xls"
            | "xlsb"
            | "dbf"
            | "sqlite"
            | "sqlite3"
            | "db"
    )
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

/// Load a CSV or TSV file into a TableGrid.
pub fn load_csv(path: &Path, delimiter: Option<u8>) -> Result<TableGrid, String> {
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
        None => {
            return Ok(TableGrid::new(
                path.display().to_string(),
                Vec::new(),
                Vec::new(),
            ));
        }
    };

    if first.is_empty() {
        return Ok(TableGrid::new(
            path.display().to_string(),
            Vec::new(),
            Vec::new(),
        ));
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

    let (headers, start_row) = if has_header {
        let h: Vec<String> = first
            .iter()
            .take(MAX_TABLE_COLS)
            .map(|s| TableGrid::sanitize_cell(s.trim()))
            .collect();
        (h, None)
    } else {
        let h: Vec<String> = (0..first.len().min(MAX_TABLE_COLS))
            .map(column_letter)
            .collect();
        let cells: Vec<String> = first
            .iter()
            .take(MAX_TABLE_COLS)
            .map(TableGrid::sanitize_cell)
            .collect();
        (h, Some(TableRow { cells }))
    };

    let mut rows = Vec::new();
    if let Some(r) = start_row {
        rows.push(r);
    }

    for result in records {
        if rows.len() >= MAX_TABLE_ROWS {
            break;
        }
        let record = result.map_err(|e| format!("CSV record error: {e}"))?;
        let cells: Vec<String> = record
            .iter()
            .take(MAX_TABLE_COLS)
            .map(TableGrid::sanitize_cell)
            .collect();
        rows.push(TableRow { cells });
    }

    let mut grid = TableGrid::new(path.display().to_string(), headers, rows);
    grid.sheets = vec!["Data".to_string()];
    Ok(grid)
}

/// Load a JSON Lines (.jsonl / .ndjson) file into a TableGrid.
pub fn load_jsonl(path: &Path) -> Result<TableGrid, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open JSONL: {e}"))?;
    let mut reader = BufReader::new(file);

    let mut raw_objects = Vec::new();
    let mut header_set = Vec::new();
    let mut line_buffer = String::new();

    while raw_objects.len() < MAX_TABLE_ROWS {
        line_buffer.clear();
        // Read with bounded capacity per line to prevent OOM
        let bytes_read = (&mut reader)
            .take(MAX_LINE_BYTES)
            .read_line(&mut line_buffer)
            .map_err(|e| format!("Read error: {e}"))?;

        if bytes_read == 0 {
            break;
        }

        let trimmed = line_buffer.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(JsonValue::Object(map)) = serde_json::from_str(trimmed) {
            for k in map.keys() {
                if !header_set.contains(k) && header_set.len() < MAX_TABLE_COLS {
                    header_set.push(k.clone());
                }
            }
            raw_objects.push(map);
        }
    }

    if header_set.is_empty() {
        return Ok(TableGrid::new(
            path.display().to_string(),
            Vec::new(),
            Vec::new(),
        ));
    }

    let headers: Vec<String> = header_set
        .iter()
        .map(|s| TableGrid::sanitize_cell(s))
        .collect();
    let mut rows = Vec::with_capacity(raw_objects.len());

    for map in raw_objects {
        let mut cells = Vec::with_capacity(headers.len());
        for h in &header_set {
            let val_str = match map.get(h) {
                Some(JsonValue::Null) | None => String::new(),
                Some(JsonValue::String(s)) => TableGrid::sanitize_cell(s),
                Some(other) => TableGrid::sanitize_cell(&other.to_string()),
            };
            cells.push(val_str);
        }
        rows.push(TableRow { cells });
    }

    let mut grid = TableGrid::new(path.display().to_string(), headers, rows);
    grid.sheets = vec!["JSONL".to_string()];
    Ok(grid)
}

/// Format a dbase::FieldValue into a string representation.
fn format_dbase_value(val: &dbase::FieldValue) -> String {
    match val {
        dbase::FieldValue::Character(Some(s)) => s.trim_end().to_string(),
        dbase::FieldValue::Numeric(Some(n)) => n.to_string(),
        dbase::FieldValue::Logical(Some(b)) => if *b { "T" } else { "F" }.to_string(),
        dbase::FieldValue::Date(Some(d)) => {
            format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
        }
        dbase::FieldValue::Float(Some(f)) => f.to_string(),
        dbase::FieldValue::Integer(i) => i.to_string(),
        dbase::FieldValue::Currency(c) => format!("{:.2}", c),
        dbase::FieldValue::Double(d) => d.to_string(),
        dbase::FieldValue::DateTime(dt) => {
            let d = dt.date();
            let t = dt.time();
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                d.year(),
                d.month(),
                d.day(),
                t.hours(),
                t.minutes(),
                t.seconds()
            )
        }
        dbase::FieldValue::Memo(m) => {
            // Take up to 2048 chars slice to avoid copying huge memo buffers
            let slice: String = m.chars().take(2048).collect();
            slice
        }
        _ => String::new(),
    }
}

/// Load a dBase / Clipper / FoxPro .dbf file (with companion .dbt memo file if present).
pub fn load_dbf(path: &Path) -> Result<TableGrid, String> {
    let mut reader = dbase::Reader::from_path(path)
        .map_err(|e| format!("Failed to open DBF {}: {e}", path.display()))?;

    let field_names: Vec<String> = reader
        .fields()
        .iter()
        .take(MAX_TABLE_COLS)
        .map(|f| f.name().to_string())
        .collect();

    if field_names.is_empty() {
        return Ok(TableGrid::new(
            path.display().to_string(),
            Vec::new(),
            Vec::new(),
        ));
    }

    let headers: Vec<String> = field_names
        .iter()
        .map(|name| TableGrid::sanitize_cell(name))
        .collect();

    let mut rows = Vec::new();
    let mut iter = reader.iter_records();

    while rows.len() < MAX_TABLE_ROWS {
        match iter.next() {
            Some(Ok(record)) => {
                let mut cells = Vec::with_capacity(headers.len());
                for name in &field_names {
                    let raw_str = match record.get(name) {
                        Some(val) => format_dbase_value(val),
                        None => String::new(),
                    };
                    cells.push(TableGrid::sanitize_cell(&raw_str));
                }
                rows.push(TableRow { cells });
            }
            Some(Err(e)) => return Err(format!("Error reading DBF record: {e}")),
            None => break,
        }
    }

    let mut grid = TableGrid::new(path.display().to_string(), headers, rows);
    grid.sheets = vec!["DBF".to_string()];
    Ok(grid)
}

/// Load an Excel / ODS spreadsheet using `calamine`.
pub fn load_spreadsheet(path: &Path) -> Result<TableGrid, String> {
    let mut workbook = open_workbook_auto(path)
        .map_err(|e| format!("Failed to open spreadsheet {}: {e}", path.display()))?;

    let sheet_names: Vec<String> = workbook
        .sheet_names()
        .into_iter()
        .map(|s| TableGrid::sanitize_cell(&s))
        .collect();

    if sheet_names.is_empty() {
        return Err("Spreadsheet has no sheets".to_string());
    }

    let first_sheet_name = sheet_names[0].clone();
    let range = workbook
        .worksheet_range(&first_sheet_name)
        .map_err(|e| format!("Failed to read sheet '{first_sheet_name}': {e}"))?;

    let (height, width) = range.get_size();
    let max_cols = width.min(MAX_TABLE_COLS);
    let max_rows = height.min(MAX_TABLE_ROWS);

    let headers: Vec<String> = (0..max_cols).map(column_letter).collect();
    let mut rows = Vec::with_capacity(max_rows);

    for r in 0..max_rows {
        let mut cells = Vec::with_capacity(max_cols);
        for c in 0..max_cols {
            let val = match range.get((r, c)) {
                Some(v) => TableGrid::sanitize_cell(&v.to_string()),
                None => String::new(),
            };
            cells.push(val);
        }
        rows.push(TableRow { cells });
    }

    let mut grid = TableGrid::new(path.display().to_string(), headers, rows);
    grid.sheets = sheet_names;
    grid.current_sheet = 0;
    Ok(grid)
}

/// Load a table or query result from an SQLite database in strictly defensive read-only mode.
pub fn load_sqlite(path: &Path, table_or_query: Option<&str>) -> Result<TableGrid, String> {
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

    // List user tables (bounded to prevent DoS from DB with millions of tables)
    let mut table_stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 1000",
        )
        .map_err(|e| format!("Failed to query tables: {e}"))?;

    let table_rows = table_stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Query error: {e}"))?;

    let mut raw_tables = Vec::new();
    let mut display_tables = Vec::new();
    for name in table_rows.flatten() {
        display_tables.push(TableGrid::sanitize_cell(&name));
        raw_tables.push(name);
    }

    if raw_tables.is_empty() {
        return Ok(TableGrid::new(
            path.display().to_string(),
            vec!["Status".to_string()],
            vec![TableRow {
                cells: vec!["(Database has no tables)".to_string()],
            }],
        ));
    }

    // Determine query: user SQL query, specific table, or first available table
    let (query, active_table_idx) = match table_or_query {
        Some(q) if q.trim().to_ascii_uppercase().starts_with("SELECT ") => (q.to_string(), 0),
        Some(tbl) => {
            let idx = raw_tables
                .iter()
                .position(|t| t == tbl)
                .or_else(|| display_tables.iter().position(|t| t == tbl))
                .unwrap_or(0);
            let escaped = raw_tables[idx].replace('"', "\"\"");
            (
                format!("SELECT * FROM \"{}\" LIMIT {}", escaped, MAX_TABLE_ROWS),
                idx,
            )
        }
        None => {
            let escaped = raw_tables[0].replace('"', "\"\"");
            (
                format!("SELECT * FROM \"{}\" LIMIT {}", escaped, MAX_TABLE_ROWS),
                0,
            )
        }
    };

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("SQL statement error: {e}"))?;

    let headers: Vec<String> = stmt
        .column_names()
        .into_iter()
        .take(MAX_TABLE_COLS)
        .map(TableGrid::sanitize_cell)
        .collect();

    let col_count = headers.len();
    let mut rows = Vec::new();

    let mut query_rows = stmt
        .query([])
        .map_err(|e| format!("SQL execution error: {e}"))?;

    while let Some(row) = query_rows
        .next()
        .map_err(|e| format!("Row read error: {e}"))?
    {
        if rows.len() >= MAX_TABLE_ROWS {
            break;
        }
        let mut cells = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let val_str = match row.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Null) => String::new(),
                Ok(rusqlite::types::ValueRef::Integer(i)) => i.to_string(),
                Ok(rusqlite::types::ValueRef::Real(r)) => r.to_string(),
                Ok(rusqlite::types::ValueRef::Text(t)) => {
                    const MAX_TEXT_BYTES: usize = 8192;
                    let slice = if t.len() > MAX_TEXT_BYTES {
                        &t[..MAX_TEXT_BYTES]
                    } else {
                        t
                    };
                    let s = String::from_utf8_lossy(slice);
                    let mut text = TableGrid::sanitize_cell(&s);
                    if t.len() > MAX_TEXT_BYTES && !text.ends_with('…') {
                        text.push('…');
                    }
                    text
                }
                Ok(rusqlite::types::ValueRef::Blob(b)) => {
                    format!("<Blob {} bytes>", b.len())
                }
                Err(_) => String::new(),
            };
            cells.push(val_str);
        }
        rows.push(TableRow { cells });
    }

    let mut grid = TableGrid::new(path.display().to_string(), headers, rows);
    grid.sheets = display_tables;
    grid.current_sheet = active_table_idx;
    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_column_letter() {
        assert_eq!(column_letter(0), "A");
        assert_eq!(column_letter(25), "Z");
        assert_eq!(column_letter(26), "AA");
        assert_eq!(column_letter(27), "AB");
    }

    #[test]
    fn test_load_csv_with_headers() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Name,Age,City").unwrap();
        writeln!(file, "Alice,30,Brasilia").unwrap();
        writeln!(file, "Bob,25,Curitiba").unwrap();

        let grid = load_csv(file.path(), Some(b',')).unwrap();
        assert_eq!(grid.headers, vec!["Name", "Age", "City"]);
        assert_eq!(grid.rows.len(), 2);
        assert_eq!(grid.rows[0].cells, vec!["Alice", "30", "Brasilia"]);
    }

    #[test]
    fn test_load_jsonl() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"id": 1, "task": "Test"}}"#).unwrap();
        writeln!(file, r#"{{"id": 2, "task": "Verify", "done": true}}"#).unwrap();

        let grid = load_jsonl(file.path()).unwrap();
        assert!(grid.headers.contains(&"id".to_string()));
        assert!(grid.headers.contains(&"task".to_string()));
        assert_eq!(grid.rows.len(), 2);
    }

    #[test]
    fn test_format_dbase_value_variants() {
        use dbase::{Date, FieldValue};

        assert_eq!(
            format_dbase_value(&FieldValue::Character(Some("Test  ".to_string()))),
            "Test"
        );
        assert_eq!(
            format_dbase_value(&FieldValue::Numeric(Some(123.45))),
            "123.45"
        );
        assert_eq!(format_dbase_value(&FieldValue::Logical(Some(true))), "T");
        assert_eq!(format_dbase_value(&FieldValue::Logical(Some(false))), "F");
        assert_eq!(format_dbase_value(&FieldValue::Logical(None)), "");
        assert_eq!(
            format_dbase_value(&FieldValue::Date(Date::new(30, 10, 2026).ok())),
            "2026-10-30"
        );
        assert_eq!(format_dbase_value(&FieldValue::Currency(42.5)), "42.50");
        assert_eq!(
            format_dbase_value(&FieldValue::Memo("Long memo note".to_string())),
            "Long memo note"
        );
    }

    #[test]
    fn test_load_sqlite_defensive_and_query() {
        use rusqlite::Connection;
        let file = NamedTempFile::new().unwrap();
        {
            let conn = Connection::open(file.path()).unwrap();
            conn.execute(
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, balance REAL)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO users (id, name, balance) VALUES (1, 'Alice', 100.50)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO users (id, name, balance) VALUES (2, 'Bob', 250.00)",
                [],
            )
            .unwrap();
        }

        // Test loading default table
        let grid = load_sqlite(file.path(), None).unwrap();
        assert_eq!(grid.sheets, vec!["users".to_string()]);
        assert_eq!(grid.headers, vec!["id", "name", "balance"]);
        assert_eq!(grid.rows.len(), 2);
        assert_eq!(grid.rows[0].cells, vec!["1", "Alice", "100.5"]);

        // Test custom SELECT query
        let grid_query = load_sqlite(
            file.path(),
            Some("SELECT name, balance FROM users WHERE balance > 150"),
        )
        .unwrap();
        assert_eq!(grid_query.headers, vec!["name", "balance"]);
        assert_eq!(grid_query.rows.len(), 1);
        assert_eq!(grid_query.rows[0].cells, vec!["Bob", "250"]);
    }
}
