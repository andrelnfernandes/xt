# Demo files

Files to try the [viewer](../docs/manual/viewer.md) and
[Batch in the viewer](../docs/manual/execute.md) (Ctrl+B) with: the same nine made-up
customers in every format, and templates that turn them into other files.

> Português: [README.pt-BR.md](README.pt-BR.md)

## The data

| File | How it opens |
|---|---|
| `data/customers.csv`, `.tsv` | table |
| `data/customers.jsonl` | table |
| `data/customers.dbf` | table |
| `data/customers.db` (SQLite, with the tables `customers` and `orders`; `[` `]` change) | table, with `V` |
| `data/customers.parquet` | table, with `V` |
| `data/customers.xlsx` (with the sheets `customers` and `orders`; `[` `]` change) | table, with columns A, B, C; the first row is a row like any other |
| `data/customers.json`, `.xml` | text, with colors |
| `data/access.log` | text, for the line filter |

The last four customers are **hostile on purpose**: a name like
`<script>alert(1)</script>` or `Robert'); DROP TABLE customers;--`. They are there to
show each escape at work, in a page, a script or SQL. Nothing runs by opening them.

## The templates

Copy them to where Xylem looks for templates:

```bash
mkdir -p ~/.config/xylem/batch && cp demo/templates/*.tpl ~/.config/xylem/batch/
```

(On Windows, `%APPDATA%\xylem\batch\`.)

Then open the data, not the template: a table from `demo/data` with `V`. Press **Ctrl+B**, give the name of the file to write, and type
`:name` as the mask, or choose it with **↑↓** (**F3** shows it whole). The extension of that file chooses the escape, so use the one below.
The preview shows the first lines: **Enter** writes and **Esc** gives up.

| Mask | Write to | What you get |
|---|---|---|
| `:report` | `report.html` | a page with a table and the total |
| `:table` | `customers.md` | a Markdown table |
| `:update` | `update.sql` | one `UPDATE` per record, in a transaction left open |
| `:bulk_fix` | `bulk_fix.sql` | many fixes as a single `UPDATE`, through a temporary table |
| `:undo` | `undo.sql` | the way back of a fix, generated before it |
| `:missing` | `missing.sql` | `SELECT`s that list what a database does not have |
| `:requests` | `requests.sh` | a `curl` per record, with escapes for the address |
| `:one_file_each` | `split.sh` | a script that writes one file per record |
| `:contacts` | `contacts.vcf` | contacts for a phone or an e-mail program |
| `:elastic_bulk` | `bulk.ndjson` | two lines per record, for Elasticsearch |
| `:log_to_sql` | `log.sql` | the log as a database: see below |

Try the same template with `customers.csv` and `customers.parquet`: the text is the same,
but `id` is text in one and a number in the other, so `.sql` gets `'1'` or `1`.

These templates use column names, so they do not work with `customers.xlsx`, which has
letters: there, try the mask `%{B};%{C}` in a `.txt`.

## Other things to try

**Masks, with no template.** In `out.txt`: `%{#} %{name:url}`. In `out.sh`:
`echo %{name}`. In `out.json`, an empty mask writes whole records, and `@id, name` only
those fields.

**The SQL filter.** In a table, `&` filters the rows, and Ctrl+B writes only those:
`plan = 'pro'`, or `balance > 100`.

**Filtering lines.** In a text, `&` keeps only the lines that match: open
`data/access.log` and type `500`. A file that opens as a table has the SQL filter on `&`
instead: press `T` for its text first.

**A log as a table.** Open `data/access.log`, press `&`, then **F4** for a regular
expression, and filter with:

```
^(?P<ip>\S+) .*"(?P<method>[A-Z]+) (?P<path>.*) HTTP/[\d.]+" (?P<status>\d{3})
```

Then Ctrl+B with the mask `:log_to_sql` into `log.sql`, and:

```bash
sqlite3 log.db < log.sql
```

```bash
sqlite3 log.db 'SELECT status, count(*) FROM hits GROUP BY status'
```

**The SQL scripts.** They are meant for a table named `customers`, like the one in
`customers.db`. To try one, work on a copy:

```bash
cp demo/data/customers.db copy.db
```

```bash
sqlite3 copy.db < update.sql
```

Scripts come out without permission to run: read them first. `requests.tpl` needs
`API_URL` set when it runs, and there is no real server behind the demo.

## Writing them again

They are written by [batch_demo.rs](../src/viewer/batch_demo.rs), whose tests also run every
template against every table, and run the SQL and the scripts for real:

```bash
cargo test --bin xy write_demo_files -- --ignored
```
