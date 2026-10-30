//! The grid on screen, built from a stream of records (see `records`).
//!
//! Enforces security bounds: this is the one place that keeps rows in memory,
//! so it stops the stream at its limit, and what it keeps is safe for a terminal.

use std::ops::ControlFlow;
use std::path::Path;

use super::records::{self, Budget, Cell, End, MAX_COLS, RecordSink};
use super::table::{TableGrid, TableRow};

pub use super::records::column_letter;

/// Rows the grid keeps in memory (columns are limited by the stream).
pub const MAX_TABLE_ROWS: usize = 50_000;

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
            | "parquet"
    )
}

/// The formats read by libraries in C: SQLite and Parquet's zstd codec.
/// Everything else here is parsed in Rust.
pub fn uses_c_parser(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    matches!(ext.as_str(), "sqlite" | "sqlite3" | "db" | "parquet")
}

/// Builds a grid from a stream.
#[derive(Default)]
struct GridSink {
    sheets: Vec<String>,
    current_sheet: usize,
    headers: Vec<String>,
    rows: Vec<TableRow>,
    rows_cut: bool,
    columns_cut: bool,
}

impl RecordSink for GridSink {
    fn sheets(&mut self, names: &[String], current: usize) {
        self.sheets = names.iter().map(|s| TableGrid::sanitize_cell(s)).collect();
        self.current_sheet = current;
    }

    fn columns(&mut self, names: &[String]) {
        self.headers = names.iter().map(|s| TableGrid::sanitize_cell(s)).collect();
    }

    fn columns_cut(&mut self) {
        self.columns_cut = true;
    }

    fn record(&mut self, cells: &[Cell<'_>]) -> ControlFlow<()> {
        // One more than it keeps, to tell a table cut from one of that size.
        if self.rows.len() >= MAX_TABLE_ROWS {
            self.rows_cut = true;
            return ControlFlow::Break(());
        }
        let cells = cells.iter().take(MAX_COLS).map(Cell::display).collect();
        self.rows.push(TableRow { cells });
        ControlFlow::Continue(())
    }
}

/// `50000` -> `50,000`.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl GridSink {
    fn finish(mut self, path: &Path, end: End) -> TableGrid {
        // Columns found after a row was read leave it short.
        for row in &mut self.rows {
            row.cells
                .resize(self.headers.len().max(row.cells.len()), String::new());
        }
        let mut cut = Vec::new();
        if end == End::OutOfTime {
            cut.push(format!(
                "stopped by the time limit: showing {} rows",
                thousands(self.rows.len())
            ));
        } else if self.rows_cut {
            cut.push(format!("showing first {} rows", thousands(MAX_TABLE_ROWS)));
        }
        if self.columns_cut {
            cut.push(format!("showing first {MAX_COLS} columns"));
        }
        let mut grid = TableGrid::new(path.display().to_string(), self.headers, self.rows);
        grid.sheets = self.sheets;
        grid.current_sheet = self.current_sheet;
        grid.cut = (!cut.is_empty()).then(|| cut.join(", "));
        grid
    }
}

fn load(
    path: &Path,
    read: impl FnOnce(&mut GridSink) -> Result<End, String>,
) -> Result<TableGrid, String> {
    let mut sink = GridSink::default();
    let end = read(&mut sink)?;
    Ok(sink.finish(path, end))
}

/// Load a CSV or TSV file into a TableGrid.
pub fn load_csv(path: &Path, delimiter: Option<u8>, budget: Budget) -> Result<TableGrid, String> {
    load(path, |sink| {
        records::read_csv(path, delimiter, budget, sink)
    })
}

/// Load a JSON Lines (.jsonl / .ndjson) file into a TableGrid.
pub fn load_jsonl(path: &Path, budget: Budget) -> Result<TableGrid, String> {
    load(path, |sink| records::read_jsonl(path, budget, sink))
}

/// Load a dBase / Clipper / FoxPro .dbf file.
pub fn load_dbf(path: &Path, budget: Budget) -> Result<TableGrid, String> {
    load(path, |sink| records::read_dbf(path, budget, sink))
}

/// Load a sheet (by position) of an Excel / ODS spreadsheet.
pub fn load_spreadsheet(path: &Path, sheet: usize, budget: Budget) -> Result<TableGrid, String> {
    load(path, |sink| {
        records::read_spreadsheet(path, sheet, budget, sink)
    })
}

/// Load a table or query result from an SQLite database.
pub fn load_sqlite(
    path: &Path,
    table_or_query: Option<&str>,
    budget: Budget,
) -> Result<TableGrid, String> {
    load(path, |sink| {
        records::read_sqlite(path, table_or_query, budget, sink)
    })
}

/// Load an Apache Parquet file (.parquet).
pub fn load_parquet(path: &Path, budget: Budget) -> Result<TableGrid, String> {
    load(path, |sink| records::read_parquet(path, budget, sink))
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

        let grid = load_csv(file.path(), Some(b','), Budget::unlimited()).unwrap();
        assert_eq!(grid.headers, vec!["Name", "Age", "City"]);
        assert_eq!(grid.rows.len(), 2);
        assert_eq!(grid.rows[0].cells, vec!["Alice", "30", "Brasilia"]);
    }

    #[test]
    fn test_load_jsonl() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"id": 1, "task": "Test"}}"#).unwrap();
        writeln!(file, r#"{{"id": 2, "task": "Verify", "done": true}}"#).unwrap();

        let grid = load_jsonl(file.path(), Budget::unlimited()).unwrap();
        assert!(grid.headers.contains(&"id".to_string()));
        assert!(grid.headers.contains(&"task".to_string()));
        assert_eq!(grid.rows.len(), 2);
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
        let grid = load_sqlite(file.path(), None, Budget::unlimited()).unwrap();
        assert_eq!(grid.sheets, vec!["users".to_string()]);
        assert_eq!(grid.headers, vec!["id", "name", "balance"]);
        assert_eq!(grid.rows.len(), 2);
        assert_eq!(grid.rows[0].cells, vec!["1", "Alice", "100.5"]);

        // Test custom SELECT query
        let grid_query = load_sqlite(
            file.path(),
            Some("SELECT name, balance FROM users WHERE balance > 150"),
            Budget::unlimited(),
        )
        .unwrap();
        assert_eq!(grid_query.headers, vec!["name", "balance"]);
        assert_eq!(grid_query.rows.len(), 1);
        assert_eq!(grid_query.rows[0].cells, vec!["Bob", "250"]);
    }

    #[test]
    fn test_load_parquet_basic_and_sanitization() {
        use arrow_array::{Int32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use parquet::arrow::arrow_writer::ArrowWriter;
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("label", DataType::Utf8, true),
        ]));

        let ids = Arc::new(Int32Array::from(vec![10, 20, 30]));
        // Test normal string, null, and ANSI escape + bidi Trojan Source
        let labels = Arc::new(StringArray::from(vec![
            Some("Valid"),
            None,
            Some("Evil\x1b[31mColor\x1b[0m\u{202E}Bidi"),
        ]));

        let batch = RecordBatch::try_new(schema.clone(), vec![ids, labels]).unwrap();

        let file = NamedTempFile::new().unwrap();
        {
            let mut writer = ArrowWriter::try_new(file.reopen().unwrap(), schema, None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        let grid = load_parquet(file.path(), Budget::unlimited()).unwrap();
        assert_eq!(grid.headers, vec!["id", "label"]);
        assert_eq!(grid.sheets, vec!["Parquet"]);
        assert_eq!(grid.rows.len(), 3);
        assert_eq!(grid.rows[0].cells, vec!["10", "Valid"]);
        assert_eq!(grid.rows[1].cells, vec!["20", ""]);
        // Control chars (ESC) and bidi controls replaced with '·' to neutralize terminal injections
        assert_eq!(grid.rows[2].cells, vec!["30", "Evil·[31mColor·[0m·Bidi"]);
    }

    #[test]
    fn test_load_parquet_invalid_header_or_size() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"NOT_A_PARQUET_FILE_TOO_LONG").unwrap();
        let res = load_parquet(file.path(), Budget::unlimited());
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("missing PAR1 magic header"));

        let mut tiny_file = NamedTempFile::new().unwrap();
        tiny_file.write_all(b"PAR").unwrap();
        let tiny_res = load_parquet(tiny_file.path(), Budget::unlimited());
        assert!(tiny_res.is_err());
        assert!(tiny_res.unwrap_err().contains("too small"));

        let mut no_footer = NamedTempFile::new().unwrap();
        no_footer.write_all(b"PAR112345678NOT1").unwrap();
        let no_footer_res = load_parquet(no_footer.path(), Budget::unlimited());
        assert!(no_footer_res.is_err());
        assert!(
            no_footer_res
                .unwrap_err()
                .contains("missing PAR1 magic footer")
        );
    }

    #[test]
    fn test_load_parquet_sql_filtering() {
        use arrow_array::{Int32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use parquet::arrow::arrow_writer::ArrowWriter;
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("score", DataType::Int32, false),
        ]));

        let names = Arc::new(StringArray::from(vec!["Alice", "Bob", "Charlie", "David"]));
        let scores = Arc::new(Int32Array::from(vec![95, 45, 80, 60]));

        let batch = RecordBatch::try_new(schema.clone(), vec![names, scores]).unwrap();

        let file = NamedTempFile::new().unwrap();
        {
            let mut writer = ArrowWriter::try_new(file.reopen().unwrap(), schema, None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        let mut grid = load_parquet(file.path(), Budget::unlimited()).unwrap();
        assert_eq!(grid.rows.len(), 4);

        // Filter score >= 80
        let matches = grid.apply_filter(Some("score >= 80")).unwrap();
        assert_eq!(matches, 2);
        assert_eq!(grid.filtered_indices, vec![0, 2]);

        // Filter name LIKE '%a%' (case-sensitive: Charlie, David)
        let matches = grid.apply_filter(Some("name LIKE '%a%'")).unwrap();
        assert_eq!(matches, 2);

        // Filter name ILIKE '%a%' (case-insensitive: Alice, Charlie, David)
        let matches_ilike = grid.apply_filter(Some("name ILIKE '%a%'")).unwrap();
        assert_eq!(matches_ilike, 3);
    }

    fn numbers_file(rows: usize) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "n,w").unwrap();
        for i in 0..rows {
            writeln!(file, "{i},x").unwrap();
        }
        file
    }

    #[test]
    fn the_grid_stops_at_its_limit_and_says_so() {
        let file = numbers_file(MAX_TABLE_ROWS + 1);
        let grid = load_csv(file.path(), None, Budget::unlimited()).unwrap();
        assert_eq!(grid.rows.len(), MAX_TABLE_ROWS);
        assert_eq!(grid.cut.as_deref(), Some("showing first 50,000 rows"));
    }

    #[test]
    fn a_table_exactly_at_the_limit_is_not_cut() {
        let file = numbers_file(MAX_TABLE_ROWS);
        let grid = load_csv(file.path(), None, Budget::unlimited()).unwrap();
        assert_eq!(grid.rows.len(), MAX_TABLE_ROWS);
        assert_eq!(grid.cut, None);
    }

    #[test]
    fn the_grid_cleans_and_shortens_what_the_stream_gives() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "h\u{1b}\u{202E},b").unwrap();
        writeln!(file, "a\u{1b}[31m\u{202E}z,{}", "y".repeat(3_000)).unwrap();
        let grid = load_csv(file.path(), None, Budget::unlimited()).unwrap();
        assert_eq!(grid.headers[0], "h··");
        assert_eq!(grid.rows[0].cells[0], "a·[31m·z");
        assert_eq!(grid.rows[0].cells[1].chars().count(), 2049);
        assert!(grid.rows[0].cells[1].ends_with('…'));
    }

    #[test]
    fn rows_are_as_wide_as_the_columns_found_later() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"a": 1}}"#).unwrap();
        writeln!(file, r#"{{"a": 2, "b": 3}}"#).unwrap();
        let grid = load_jsonl(file.path(), Budget::unlimited()).unwrap();
        assert_eq!(grid.headers, ["a", "b"]);
        assert_eq!(grid.rows[0].cells, ["1", ""]);
        assert_eq!(grid.rows[1].cells, ["2", "3"]);
    }

    #[test]
    fn a_table_out_of_time_says_how_much_it_shows() {
        let file = numbers_file(2_000);
        let budget = Budget::new(std::time::Duration::ZERO, u64::MAX);
        let grid = load_csv(file.path(), None, budget).unwrap();
        assert!(grid.rows.len() < 2_000);
        assert_eq!(
            grid.cut,
            Some(format!(
                "stopped by the time limit: showing {} rows",
                grid.rows.len()
            ))
        );
    }

    #[test]
    fn a_table_with_too_many_columns_says_so() {
        let mut file = NamedTempFile::new().unwrap();
        let header: Vec<String> = (0..MAX_COLS + 1).map(|i| format!("c{i}")).collect();
        writeln!(file, "{}", header.join(",")).unwrap();
        let grid = load_csv(file.path(), None, Budget::unlimited()).unwrap();
        assert_eq!(grid.headers.len(), MAX_COLS);
        assert_eq!(grid.cut.as_deref(), Some("showing first 256 columns"));
    }

    #[test]
    fn a_blob_shows_as_its_size() {
        let file = NamedTempFile::new().unwrap();
        rusqlite::Connection::open(file.path())
            .unwrap()
            .execute_batch("CREATE TABLE t (b); INSERT INTO t VALUES (x'0001');")
            .unwrap();
        let grid = load_sqlite(file.path(), None, Budget::unlimited()).unwrap();
        assert_eq!(grid.rows[0].cells, ["<Blob 2 bytes>"]);
    }
}
