//! The files of `demo/`: writes them, and checks that every template works with
//! every table. They are all made here, from the same customers, so that any
//! template can be tried with any format and nothing in them comes from outside.
//!
//! To write them again: `cargo test --bin xy write_demo_files -- --ignored`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use regex::Regex;

use super::*;

struct Customer {
    id: i64,
    code: &'static str,
    name: &'static str,
    email: &'static str,
    city: &'static str,
    plan: &'static str,
    balance: f64,
    active: bool,
    joined: (u32, u32, i32),
}

#[allow(clippy::too_many_arguments)]
const fn customer(
    id: i64,
    code: &'static str,
    name: &'static str,
    email: &'static str,
    city: &'static str,
    plan: &'static str,
    balance: f64,
    active: bool,
    joined: (u32, u32, i32),
) -> Customer {
    Customer {
        id,
        code,
        name,
        email,
        city,
        plan,
        balance,
        active,
        joined,
    }
}

/// The first ones are ordinary, with what breaks careless code: a leading zero,
/// accents, quotes, a comma. The last ones are hostile on purpose: each attacks
/// a kind of output if its escape fails. All names and addresses are made up.
const CUSTOMERS: [Customer; 9] = [
    customer(
        1,
        "007",
        "Ana Souza",
        "ana@example.com",
        "São Paulo",
        "pro",
        1234.5,
        true,
        (15, 1, 2024),
    ),
    customer(
        2,
        "042",
        "Patrick O'Neil",
        "patrick@example.org",
        "Dublin",
        "free",
        0.0,
        true,
        (3, 2, 2024),
    ),
    customer(
        3,
        "108",
        "Bruno \"Bê\" Lima",
        "bruno@example.net",
        "Rio de Janeiro",
        "pro",
        -87.25,
        false,
        (20, 3, 2024),
    ),
    customer(
        4,
        "0815",
        "Zoë Müller",
        "zoe@example.com",
        "Zürich, ZH",
        "team",
        9876.0,
        true,
        (9, 5, 2024),
    ),
    customer(
        5,
        "1000",
        "Li Lei",
        "lilei@example.com",
        "Beijing",
        "team",
        42.0,
        true,
        (30, 6, 2024),
    ),
    customer(
        6,
        "2001",
        "<script>alert(1)</script>",
        "html@example.com",
        "</td><td>",
        "free",
        0.0,
        false,
        (1, 7, 2024),
    ),
    customer(
        7,
        "2002",
        "$(echo injected)",
        "shell@example.com",
        "`id`; echo injected",
        "free",
        0.0,
        false,
        (2, 7, 2024),
    ),
    customer(
        8,
        "2003",
        "Robert'); DROP TABLE customers;--",
        "sql@example.com",
        "back\\slash'",
        "free",
        0.0,
        false,
        (3, 7, 2024),
    ),
    customer(
        9,
        "2004",
        "=HYPERLINK(\"http://example.com\")",
        "formula@example.com",
        "+1+1",
        "free",
        0.0,
        false,
        (4, 7, 2024),
    ),
];

const ORDERS: [(i64, i64, &str, f64); 5] = [
    (1, 1, "Keyboard", 149.9),
    (2, 1, "Monitor", 899.0),
    (3, 2, "Mouse", 59.5),
    (4, 4, "Desk", 1200.0),
    (5, 5, "Chair", 750.25),
];

const ORDER_COLUMNS: [&str; 4] = ["id", "customer", "item", "price"];

const COLUMNS: [&str; 9] = [
    "id", "code", "name", "email", "city", "plan", "balance", "active", "joined",
];

/// Hits of a site, for the line filter: `demo/README.md` has the expression.
const ACCESS_LOG: &str = "\
203.0.113.7 - - [30/Oct/2026:08:01:12 -0300] \"GET /index.html HTTP/1.1\" 200 5120
203.0.113.7 - - [30/Oct/2026:08:01:13 -0300] \"GET /style.css HTTP/1.1\" 200 1024
198.51.100.23 - - [30/Oct/2026:08:02:40 -0300] \"GET /missing.png HTTP/1.1\" 404 312
192.0.2.55 - - [30/Oct/2026:08:03:05 -0300] \"POST /login HTTP/1.1\" 302 0
192.0.2.55 - - [30/Oct/2026:08:03:06 -0300] \"GET /account HTTP/1.1\" 200 2048
198.51.100.23 - - [30/Oct/2026:08:04:18 -0300] \"GET /admin?q=' OR 1=1 -- HTTP/1.1\" 403 128
203.0.113.9 - - [30/Oct/2026:08:05:51 -0300] \"GET /report.pdf HTTP/1.1\" 500 256
203.0.113.9 - - [30/Oct/2026:08:05:58 -0300] \"GET /report.pdf HTTP/1.1\" 500 256
192.0.2.55 - - [30/Oct/2026:08:06:30 -0300] \"PUT /account HTTP/1.1\" 200 64
198.51.100.23 - - [30/Oct/2026:08:07:02 -0300] \"GET /<script>alert(1)</script> HTTP/1.1\" 404 312
";

const LOG_FILTER: &str =
    r#"^(?P<ip>\S+) .*"(?P<method>[A-Z]+) (?P<path>.*) HTTP/[\d.]+" (?P<status>\d{3})"#;

fn demo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("demo")
}

fn joined_text(c: &Customer) -> String {
    let (day, month, year) = c.joined;
    format!("{year}-{month:02}-{day:02}")
}

fn xml_text(text: &str) -> String {
    let mut out = Vec::new();
    escape_into(&Cell::Text(text.into()), Escape::Xml, &mut out);
    String::from_utf8(out).unwrap()
}

fn cells(c: &Customer) -> [String; 9] {
    [
        c.id.to_string(),
        c.code.to_string(),
        c.name.to_string(),
        c.email.to_string(),
        c.city.to_string(),
        c.plan.to_string(),
        c.balance.to_string(),
        c.active.to_string(),
        joined_text(c),
    ]
}

fn write_separated(path: &Path, delimiter: u8) {
    let mut out = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_path(path)
        .unwrap();
    out.write_record(COLUMNS).unwrap();
    for c in &CUSTOMERS {
        out.write_record(cells(c)).unwrap();
    }
    out.flush().unwrap();
}

fn customer_json(c: &Customer) -> serde_json::Value {
    serde_json::json!({
        "id": c.id, "code": c.code, "name": c.name, "email": c.email, "city": c.city,
        "plan": c.plan, "balance": c.balance, "active": c.active, "joined": joined_text(c),
    })
}

fn write_json_files(data: &Path) {
    let all: Vec<_> = CUSTOMERS.iter().map(customer_json).collect();
    let pretty = serde_json::to_string_pretty(&all).unwrap();
    fs::write(data.join("customers.json"), pretty + "\n").unwrap();
    let lines: Vec<String> = all.iter().map(|c| c.to_string() + "\n").collect();
    fs::write(data.join("customers.jsonl"), lines.concat()).unwrap();
}

fn write_xml(path: &Path) {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<customers>\n");
    for c in &CUSTOMERS {
        xml += &format!("  <customer id=\"{}\">\n", c.id);
        for (column, value) in COLUMNS.iter().zip(cells(c)).skip(1) {
            xml += &format!("    <{column}>{}</{column}>\n", xml_text(&value));
        }
        xml += "  </customer>\n";
    }
    fs::write(path, xml + "</customers>\n").unwrap();
}

fn write_sqlite(path: &Path) {
    let db = rusqlite::Connection::open(path).unwrap();
    db.execute_batch(
        "CREATE TABLE customers (id INTEGER PRIMARY KEY, code TEXT, name TEXT, email TEXT,
             city TEXT, plan TEXT, balance REAL, active INTEGER, joined TEXT);
         CREATE TABLE orders (id INTEGER PRIMARY KEY, customer INTEGER, item TEXT, price REAL);",
    )
    .unwrap();
    for c in &CUSTOMERS {
        db.execute(
            "INSERT INTO customers VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                c.id,
                c.code,
                c.name,
                c.email,
                c.city,
                c.plan,
                c.balance,
                c.active,
                joined_text(c)
            ],
        )
        .unwrap();
    }
    for (id, customer, item, price) in ORDERS {
        db.execute(
            "INSERT INTO orders VALUES (?, ?, ?, ?)",
            rusqlite::params![id, customer, item, price],
        )
        .unwrap();
    }
}

fn write_parquet(path: &Path) {
    use arrow_array::{ArrayRef, BooleanArray, Float64Array, Int64Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use parquet::arrow::arrow_writer::ArrowWriter;

    let text = |pick: fn(&Customer) -> String| -> ArrayRef {
        Arc::new(StringArray::from_iter_values(CUSTOMERS.iter().map(pick)))
    };
    let columns: Vec<ArrayRef> = vec![
        Arc::new(Int64Array::from_iter_values(CUSTOMERS.iter().map(|c| c.id))),
        text(|c| c.code.into()),
        text(|c| c.name.into()),
        text(|c| c.email.into()),
        text(|c| c.city.into()),
        text(|c| c.plan.into()),
        Arc::new(Float64Array::from_iter_values(
            CUSTOMERS.iter().map(|c| c.balance),
        )),
        Arc::new(BooleanArray::from_iter(
            CUSTOMERS.iter().map(|c| Some(c.active)),
        )),
        text(joined_text),
    ];
    let kind = |name: &str| match name {
        "id" => DataType::Int64,
        "balance" => DataType::Float64,
        "active" => DataType::Boolean,
        _ => DataType::Utf8,
    };
    let fields: Vec<Field> = COLUMNS
        .iter()
        .map(|name| Field::new(*name, kind(name), false))
        .collect();
    let schema = Arc::new(Schema::new(fields));
    let batch = RecordBatch::try_new(schema.clone(), columns).unwrap();
    let mut writer = ArrowWriter::try_new(fs::File::create(path).unwrap(), schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
}

fn write_dbf(path: &Path) {
    use dbase::{Date, FieldName, FieldValue as F, Record, TableWriterBuilder};
    let name = |text: &str| FieldName::try_from(text).unwrap();
    let mut writer = TableWriterBuilder::new()
        .add_numeric_field(name("id"), 6, 0)
        .add_character_field(name("code"), 8)
        .add_character_field(name("name"), 40)
        .add_character_field(name("email"), 30)
        .add_character_field(name("city"), 24)
        .add_character_field(name("plan"), 8)
        .add_numeric_field(name("balance"), 12, 2)
        .add_logical_field(name("active"))
        .add_date_field(name("joined"))
        .build_with_file_dest(path)
        .unwrap();
    for c in &CUSTOMERS {
        let (day, month, year) = c.joined;
        let mut record = Record::default();
        let text = |value: &str| F::Character(Some(value.to_string()));
        record.insert("id".into(), F::Numeric(Some(c.id as f64)));
        record.insert("code".into(), text(c.code));
        record.insert("name".into(), text(c.name));
        record.insert("email".into(), text(c.email));
        record.insert("city".into(), text(c.city));
        record.insert("plan".into(), text(c.plan));
        record.insert("balance".into(), F::Numeric(Some(c.balance)));
        record.insert("active".into(), F::Logical(Some(c.active)));
        record.insert(
            "joined".into(),
            F::Date(Some(Date::new(day, month, year as u32).unwrap())),
        );
        writer.write_record(&record).unwrap();
    }
}

/// The cells of a sheet, whose first row is a row like any other.
fn sheet_xml(rows: &[Vec<String>]) -> String {
    let mut sheet = String::new();
    for (r, row) in rows.iter().enumerate() {
        sheet += &format!("<row r=\"{}\">", r + 1);
        for (c, value) in row.iter().enumerate() {
            let at = format!("{}{}", records::column_letter(c), r + 1);
            sheet += &format!(
                "<c r=\"{at}\" t=\"inlineStr\"><is><t>{}</t></is></c>",
                xml_text(value)
            );
        }
        sheet += "</row>";
    }
    sheet
}

/// The sheets of the spreadsheet, the same data as the tables of the database.
fn spreadsheet_sheets() -> [(&'static str, Vec<Vec<String>>); 2] {
    let customers = std::iter::once(COLUMNS.map(String::from).to_vec())
        .chain(CUSTOMERS.iter().map(|c| cells(c).to_vec()))
        .collect();
    let orders = std::iter::once(ORDER_COLUMNS.map(String::from).to_vec())
        .chain(ORDERS.iter().map(|(id, customer, item, price)| {
            vec![
                id.to_string(),
                customer.to_string(),
                item.to_string(),
                price.to_string(),
            ]
        }))
        .collect();
    [("customers", customers), ("orders", orders)]
}

fn write_xlsx(path: &Path) {
    use std::io::Write;
    let ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let pkg = "http://schemas.openxmlformats.org/package/2006";
    let main = "application/vnd.openxmlformats-officedocument.spreadsheetml";
    let sheets = spreadsheet_sheets();
    let (mut types, mut list, mut rels) = (String::new(), String::new(), String::new());
    for (i, (name, _)) in sheets.iter().enumerate() {
        let n = i + 1;
        types += &format!(
            r#"<Override PartName="/xl/worksheets/sheet{n}.xml" ContentType="{main}.worksheet+xml"/>"#
        );
        list += &format!(r#"<sheet name="{name}" sheetId="{n}" r:id="rId{n}"/>"#);
        rels += &format!(
            r#"<Relationship Id="rId{n}" Type="{rel}/worksheet" Target="worksheets/sheet{n}.xml"/>"#
        );
    }
    let mut parts = vec![
        (
            "[Content_Types].xml".to_string(),
            format!(
                r#"<Types xmlns="{pkg}/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/xl/workbook.xml" ContentType="{main}.sheet.main+xml"/>{types}</Types>"#
            ),
        ),
        (
            "_rels/.rels".to_string(),
            format!(
                r#"<Relationships xmlns="{pkg}/relationships"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
            ),
        ),
        (
            "xl/workbook.xml".to_string(),
            format!(r#"<workbook xmlns="{ns}" xmlns:r="{rel}"><sheets>{list}</sheets></workbook>"#),
        ),
        (
            "xl/_rels/workbook.xml.rels".to_string(),
            format!(r#"<Relationships xmlns="{pkg}/relationships">{rels}</Relationships>"#),
        ),
    ];
    for (i, (_, rows)) in sheets.iter().enumerate() {
        parts.push((
            format!("xl/worksheets/sheet{}.xml", i + 1),
            format!(
                r#"<worksheet xmlns="{ns}"><sheetData>{}</sheetData></worksheet>"#,
                sheet_xml(rows)
            ),
        ));
    }
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    for (name, body) in parts {
        zip.start_file(name, options).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
#[ignore = "writes the files of demo/ again"]
fn write_demo_files() {
    let data = demo().join("data");
    fs::create_dir_all(&data).unwrap();
    for (name, write) in [
        ("customers.db", write_sqlite as fn(&Path)),
        ("customers.dbf", write_dbf),
        ("customers.parquet", write_parquet),
        ("customers.xlsx", write_xlsx),
        ("customers.xml", write_xml),
    ] {
        let path = data.join(name);
        let _ = fs::remove_file(&path);
        write(&path);
    }
    write_separated(&data.join("customers.csv"), b',');
    write_separated(&data.join("customers.tsv"), b'\t');
    write_json_files(&data);
    fs::write(data.join("access.log"), ACCESS_LOG).unwrap();
}

/// The tables that have the names of the columns; a spreadsheet has A, B, C.
const NAMED_TABLES: [&str; 6] = [
    "customers.csv",
    "customers.tsv",
    "customers.jsonl",
    "customers.dbf",
    "customers.db",
    "customers.parquet",
];

/// A template and the file it is meant for.
const TEMPLATES: [(&str, &str); 10] = [
    ("report", "report.html"),
    ("update", "update.sql"),
    ("bulk_fix", "bulk_fix.sql"),
    ("undo", "undo.sql"),
    ("missing", "missing.sql"),
    ("requests", "requests.sh"),
    ("one_file_each", "split.sh"),
    ("contacts", "contacts.vcf"),
    ("table", "customers.md"),
    ("elastic_bulk", "bulk.ndjson"),
];

fn table(name: &str) -> Source {
    Source::Table {
        path: demo().join("data").join(name),
        sheet: 0,
        table: name.ends_with(".db").then(|| "customers".to_string()),
        filter: None,
        unpacked: u64::MAX,
    }
}

/// Runs a template as the user would, and returns what it wrote.
fn through(source: Source, open: &Path, template: &str, out_name: &str) -> (Preview, Done, String) {
    let tmp = crate::testing::tempdir();
    let out = tmp.path().join(out_name);
    let templates = demo().join("templates");
    let output = Output::parse_in(&format!(":{template}"), &out, open, 1, Some(&templates))
        .unwrap_or_else(|e| panic!("{template}: {e}"));
    let found = preview(&source, output).unwrap_or_else(|e| panic!("{template}: {e}"));
    let job = Job::start(source, found.output.clone(), out.clone());
    while !job.is_done() {
        thread::sleep(Duration::from_millis(2));
    }
    let done = job.finish().unwrap();
    let text = fs::read_to_string(&out).unwrap();
    (found, done, text)
}

fn customers_db_copy(dir: &Path) -> rusqlite::Connection {
    let copy = dir.join("copy.db");
    fs::copy(demo().join("data/customers.db"), &copy).unwrap();
    rusqlite::Connection::open(copy).unwrap()
}

#[test]
fn every_table_holds_the_same_customers() {
    for name in NAMED_TABLES {
        let m = Mask::parse_in(
            "%{id}|%{code}|%{name}|%{city}",
            Escape::Raw,
            &Context {
                open: Path::new("a"),
                size: 0,
                out: Path::new("b"),
                template: None,
                now: Local::now(),
            },
        )
        .unwrap();
        let found = preview(&table(name), m).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(found.scanned, CUSTOMERS.len() as u64, "{name}");
        let wanted: Vec<String> = CUSTOMERS
            .iter()
            .map(|c| format!("{}|{}|{}|{}", c.id, c.code, c.name, c.city))
            .collect();
        assert_eq!(found.lines, wanted, "{name}");
    }
}

#[test]
fn every_template_works_with_every_table() {
    for name in NAMED_TABLES {
        for (template, out) in TEMPLATES {
            let open = demo().join("data").join(name);
            let (_, done, text) = through(table(name), &open, template, out);
            assert_eq!(done.records, CUSTOMERS.len() as u64, "{template} {name}");
            assert!(!text.is_empty(), "{template} {name}");
        }
    }
}

#[test]
fn the_page_keeps_hostile_values_as_text() {
    let open = demo().join("data/customers.csv");
    let (_, _, page) = through(table("customers.csv"), &open, "report", "report.html");
    assert!(page.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!page.contains("<script>"));
    assert!(page.contains("<p>9 customers.</p>"));
}

#[test]
fn the_sql_templates_run_and_nothing_gets_out_of_its_string() {
    let open = demo().join("data/customers.parquet");
    for (template, out) in [
        ("update", "u.sql"),
        ("bulk_fix", "b.sql"),
        ("undo", "n.sql"),
    ] {
        let tmp = crate::testing::tempdir();
        let db = customers_db_copy(tmp.path());
        let (_, _, sql) = through(table("customers.parquet"), &open, template, out);
        db.execute_batch(&sql)
            .unwrap_or_else(|e| panic!("{template}: {e}\n{sql}"));
        let left: i64 = db
            .query_row("SELECT count(*) FROM customers", [], |row| row.get(0))
            .unwrap();
        assert_eq!(left, CUSTOMERS.len() as i64, "{template}");
    }

    let tmp = crate::testing::tempdir();
    let db = customers_db_copy(tmp.path());
    let (_, _, sql) = through(table("customers.csv"), &open, "missing", "m.sql");
    let mut lines = sql.lines();
    let all_there = lines.all(|select| {
        db.query_row(select, [], |_| Ok(())) == Err(rusqlite::Error::QueryReturnedNoRows)
    });
    assert!(all_there, "{sql}");
    db.execute_batch("DELETE FROM customers WHERE id = 5")
        .unwrap();
    let found: Vec<String> = sql
        .lines()
        .filter_map(|select| db.query_row(select, [], |row| row.get(0)).ok())
        .collect();
    assert_eq!(found, ["5"]);
}

#[cfg(unix)]
#[test]
fn the_scripts_are_valid_and_a_hostile_value_stays_a_value() {
    let open = demo().join("data/customers.csv");
    for (template, out) in [("requests", "r.sh"), ("one_file_each", "s.sh")] {
        let (_, _, script) = through(table("customers.csv"), &open, template, out);
        let checked = std::process::Command::new("sh")
            .args(["-n", "-c", &script])
            .output()
            .unwrap();
        assert!(checked.status.success(), "{template}\n{script}");
    }

    // Really run, in a folder of its own: printf writes the files.
    let (_, _, script) = through(table("customers.csv"), &open, "one_file_each", "s.sh");
    let tmp = crate::testing::tempdir();
    let done = std::process::Command::new("sh")
        .arg("-c")
        .arg(&script)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(done.status.success(), "{done:?}");
    let seventh = fs::read_to_string(tmp.path().join("customer-7.md")).unwrap();
    assert!(seventh.starts_with("# $(echo injected)\n"), "{seventh}");
    assert!(seventh.contains("City: `id`; echo injected\n"), "{seventh}");
}

#[test]
fn the_bulk_file_has_two_valid_json_lines_per_record() {
    let open = demo().join("data/customers.parquet");
    let (_, _, bulk) = through(
        table("customers.parquet"),
        &open,
        "elastic_bulk",
        "b.ndjson",
    );
    let lines: Vec<serde_json::Value> = bulk
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}")))
        .collect();
    assert_eq!(lines.len(), CUSTOMERS.len() * 2);
    assert_eq!(lines[11]["name"], CUSTOMERS[5].name);
    assert_eq!(lines[0]["index"]["_id"], 1);
    assert_eq!(lines[1]["active"], true);
}

#[test]
fn a_log_becomes_a_database() {
    let regex = Regex::new(LOG_FILTER).unwrap();
    let path = demo().join("data/access.log");
    let text = fs::read_to_string(&path).unwrap();
    let kept: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| regex.is_match(line))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        kept.len(),
        text.lines().count(),
        "the filter takes all of it"
    );
    let source = Source::Text {
        total: text.lines().count(),
        text: text.clone(),
        kept: Some(kept),
        regex: Some(regex),
        cut: false,
    };
    let (_, _, sql) = through(source, &path, "log_to_sql", "log.sql");
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch(&sql)
        .unwrap_or_else(|e| panic!("{e}\n{sql}"));
    let errors: i64 = db
        .query_row("SELECT count(*) FROM hits WHERE status >= 500", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(errors, 2);
    let attack: String = db
        .query_row("SELECT path FROM hits WHERE line = 6", [], |row| row.get(0))
        .unwrap();
    assert_eq!(attack, "/admin?q=' OR 1=1 --");
}

fn first_lines(source: &Source, text: &str) -> (u64, Vec<String>) {
    let context = Context {
        open: Path::new("a"),
        size: 0,
        out: Path::new("b"),
        template: None,
        now: Local::now(),
    };
    let m = Mask::parse_in(text, Escape::Raw, &context).unwrap();
    let found = preview(source, m).unwrap_or_else(|e| panic!("{text}: {e}"));
    (found.scanned, found.lines)
}

#[test]
fn a_spreadsheet_has_a_sheet_for_each_kind_of_data_and_goes_by_letters() {
    let sheet = |sheet: usize| Source::Table {
        path: demo().join("data/customers.xlsx"),
        sheet,
        table: None,
        filter: None,
        unpacked: u64::MAX,
    };
    let (scanned, lines) = first_lines(&sheet(0), "%{B}|%{C}");
    assert_eq!(scanned, CUSTOMERS.len() as u64 + 1, "its header is a row");
    assert_eq!(lines[0], "code|name");
    assert_eq!(lines[1], "007|Ana Souza");

    let (scanned, lines) = first_lines(&sheet(1), "%{C}|%{D}");
    assert_eq!(scanned, ORDERS.len() as u64 + 1);
    assert_eq!(lines[0], "item|price");
    assert_eq!(lines[1], "Keyboard|149.9");
}

#[test]
fn a_database_has_a_table_for_each_kind_of_data() {
    let table = |table: &str| Source::Table {
        path: demo().join("data/customers.db"),
        sheet: 0,
        table: Some(table.to_string()),
        filter: None,
        unpacked: u64::MAX,
    };
    let (scanned, lines) = first_lines(&table("customers"), "%{code}");
    assert_eq!(scanned, CUSTOMERS.len() as u64);
    assert_eq!(lines[0], "007");

    let (scanned, lines) = first_lines(&table("orders"), "%{item}|%{price}");
    assert_eq!(scanned, ORDERS.len() as u64);
    assert_eq!(lines[0], "Keyboard|149.9");
}

#[test]
fn the_text_files_are_up_to_date() {
    let data = demo().join("data");
    let tmp = crate::testing::tempdir();
    write_json_files(tmp.path());
    write_separated(&tmp.path().join("customers.csv"), b',');
    write_separated(&tmp.path().join("customers.tsv"), b'\t');
    write_xml(&tmp.path().join("customers.xml"));
    for name in [
        "customers.json",
        "customers.jsonl",
        "customers.csv",
        "customers.tsv",
        "customers.xml",
    ] {
        let wanted = fs::read_to_string(tmp.path().join(name)).unwrap();
        assert_eq!(
            fs::read_to_string(data.join(name)).unwrap(),
            wanted,
            "{name}"
        );
    }
    assert_eq!(
        fs::read_to_string(data.join("access.log")).unwrap(),
        ACCESS_LOG
    );
}
