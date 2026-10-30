# The viewer

`V` opens the file in the viewer (Autoview, F7, shows it next to the list).
Binary files open in hex. Files over the size limit show only the beginning.
Spreadsheets, CSV, DBF, SQLite and Parquet open as a table (see Table mode).

| Key | What it does |
|---|---|
| ↑ ↓ PgUp PgDn Home End | scroll |
| ← → | scroll sideways |
| `H` | text or hex |
| `O` | next encoding (UTF-8, CP850, 1252, CP437, UTF-16) |
| `C` | syntax colors on or off |
| `W` | soft wrap on or off |
| `#` or `L` | line numbers on or off (in text mode) |
| `g` or `:` | jump to a line (a byte offset in hex, a row in table mode) |
| `&` | show only the lines that match, plain text or regex ([filtering lines](filter.md)); in table mode, the SQL filter |
| `E` | edit the file; back in Xylem, the viewer shows the new contents at the same place (binary files only with an editor in [`[editors]`](execute.md)) |
| `D` | with Git on: what changed since the last commit, and back ([Git](git.md)) |
| `T` | table or text, in a file that opened as a table |
| `U` | in the view of the tagged files (Ctrl+V): untag the file |
| `y` or Ctrl+Insert | copy the file's path to the clipboard |
| Esc | close |

## Searching

As in ZTreeWin: **F** searches from the start, **B** backwards from the end,
**/** (or `S`) from the current page and **\\** backwards from it; **Space**
continues in the same direction, **+** and **-** choose the direction. The
match is highlighted. At the search prompt, **F2** toggles case sensitivity
(ignoring case, accents count too: `AÇÃO` finds `ação`) and **F4** switches
between text and hex (byte pairs such as `4d 5a`). A `*` in the middle of the
term matches anything on the same line (`error*disk`).

## Syntax colors

Source code and scripts are colored in about 150 languages (Rust, Python,
shell, C, JavaScript, SQL, YAML, Markdown...), recognized by extension, by name
(`Makefile`) or by the first line (`#!/bin/bash`). The default is
`syntax_highlight` (page 2 of the [configuration](configuration.md)). Colors
stay readable on the current theme's background, and only the first 20,000
lines are colored, and no line longer than 10,000 characters, so huge files
never slow the viewer down.

## Text direction controls

Some invisible characters change the direction of the text and can make code look different from what it really says (Trojan Source, CVE-2021-42574). The viewer never hides them: each one appears as a red `·`, and the header says how many there are.

## Text encoding

Xylem finds the encoding by itself, in this order: a decompressed file first, then a BOM (UTF-8, UTF-16 LE or BE), then UTF-16 without a BOM, then UTF-8. A file that is none of these is old text: Xylem guesses CP850 (DOS, Western Europe) or Windows-1252 by counting accented letters. Windows `.reg` exports are UTF-16 and open as plain text.

**O** shows the file in the next encoding: UTF-8, CP850, Windows-1252, CP437, UTF-16 LE, UTF-16 BE, and back to UTF-8. The footer says which one, and the header shows its name when it is not UTF-8. If you pick UTF-16 on a file that opened in hex, it switches to text. Your choice stays after you edit the file and come back.

To always use one encoding for old text files instead of the guess, set `legacy_charset` to `cp850`, `cp1252` or `cp437` (**K** on page 2 of the [configuration](configuration.md)). The default is `auto`.

## Size limit

The viewer reads only the beginning of a big file, and the header says `(showing first N MiB)`. How much it reads is the `viewer_max_size` option (**L** on page 2 of the [configuration](configuration.md)): `auto`, `16`, `64`, `256` or `1024` (MiB). With `auto`, the default, the limit is 1/32 of the machine's RAM, never less than 16 MiB and never more than 256 MiB (it is 16 MiB when the RAM cannot be read). The configuration screen shows the value in use, for example `auto (256 MiB)`.

Two cases do not follow it. Autoview always reads 1 MiB. A file on a remote machine (SFTP) is read at most up to 16 MiB, because Esc cannot cancel that read.

A bigger limit costs memory and time: with `1024`, an ordinary text file that big takes roughly 2.5 GiB of RAM and about 5 s to open. Very short lines cost more than their bytes, because each line has its own bookkeeping, and so do tabs and hidden direction controls: the viewer keeps lines up to about twice the size limit in memory, and past that the header says `(showing first N lines)`. A log of ordinary lines (about 100 bytes) is never cut by this.

## Compressed files

Files ending in `.gz`, `.xz`, `.zst` or `.bz2` open decompressed, as text. Xylem looks at the first bytes of the file, not at its name, and opens one level only: a `.gz` inside a `.gz` stays compressed. The header says `gzip → N bytes` (the format and the size after decompression).

Decompression has limits. The viewer reads up to the size limit, takes at most 3 s and uses at most 64 MiB of memory for the xz dictionary or the zstd window. In Autoview the limits are 1 MiB, 100 ms and 16 MiB. When the output or the time limit is hit, the header says `(showing first N bytes decompressed)`. A note tells what happened:

- `decompression stopped after 3 s` (100 ms in Autoview)
- `compressed data truncated or corrupt`
- `needs more than 64 MiB of memory to decompress` (16 MiB in Autoview, with ` (V to view)` when V's 64 MiB may be enough)

The note goes away at the first key, but the header keeps the problem: `(truncated or corrupt)`, `(incomplete: …)` when the memory limit stopped decompression partway, or `(not decompressed: …)` when nothing could be decompressed (the raw bytes are then shown in hex).

Autoview shows the note in its title. In hex mode (`H`) you see the decompressed bytes. Colors follow the name without the compression extension: `app.rs.gz` is colored as Rust.

For a `.tar.gz` (or `.tar.xz`...) the note says `TAR archive: Alt+F5 opens it as a folder`.

To see the raw compressed bytes, turn off `viewer_decompress` (**J** on page 2 of the [configuration](configuration.md)).

## Table mode

Some files open as a grid instead of text. Xylem goes by the extension, and only for uncompressed files on this disk: a file on a remote machine or inside an archive shows as ordinary contents, with the note `Tables are shown only for files on this disk`.

| Format | Extensions | Column names |
|---|---|---|
| CSV, TSV | `.csv` `.tsv` `.tab` | the first row, when at least half of it is not numbers; otherwise A, B, C... |
| JSON Lines | `.jsonl` `.ndjson` | the keys of the objects |
| Spreadsheets | `.xlsx` `.xls` `.xlsb` `.ods` | A, B, C... (the first row is a row like any other) |
| dBase, Clipper, FoxPro | `.dbf`, with its `.dbt` memo file | the field names |
| SQLite | `.db` `.sqlite` `.sqlite3` | the columns of the table |
| Parquet | `.parquet` | the columns of the file |

The CSV separator (comma, semicolon, tab or vertical bar) is detected from the start of the file.

| Key | What it does |
|---|---|
| ↑ ↓ | previous or next row |
| ← → | previous or next column |
| PgUp PgDn | a page of rows up or down |
| Home End | first or last column |
| `[` `]` | previous or next sheet of a spreadsheet, or table of a SQLite database, going around; this clears the filter |
| `g` or `:` | jump to a row, counting only the rows the filter kept |
| `\` or `&` | the SQL filter |
| `T` | to the file's text and back |
| `H` | from the table to text, then hex, then back to the table |

The frame shows `TABLE [2/5]` when there is more than one sheet or table, and the bottom line where the cursor is: `Cell B7 (Row 7 of 120)`. The last line of the grid has the whole contents of the cell and the filter in use. Searching (`F`, `B`, `/`) does not look inside the grid and finds nothing there; it works in text mode.

### The SQL filter

`\` or `&` asks for a condition, like the `WHERE` of SQL (the word `WHERE` itself is optional), and shows only the rows that meet it. The prompt starts with the current filter; leave it empty to see all the rows again. A note says `Filter matched N rows` or `SQL error: ...`; after an error, the previous filter stays.

```
age >= 30 AND name LIKE 'Ali%'
WHERE status IN ('open', 'late') OR total > 1000.50
NOT (city = 'Rio' OR city IS NULL)
B ILIKE '%silva%'
```

| Operator | What it does |
|---|---|
| `=` `!=` `<>` `<` `<=` `>` `>=` | compare |
| `AND` `OR` `NOT` and parentheses | combine conditions (`AND` comes before `OR`) |
| `LIKE` `ILIKE` | match a pattern, `%` for any text and `_` for one character; `ILIKE` ignores case |
| `IN (...)` `NOT IN (...)` | is, or is not, in the list |
| `IS NULL` `IS NOT NULL` | the cell is, or is not, empty |

Keywords can be in any case.

- **Columns:** the name of the column, in any case and without quotes, or its letter from `A` to `Z`. The letter is the way to reach a name with spaces or symbols. A column that does not exist counts as empty, without an error.
- **Values:** text in single or double quotes (`\'` for a quote inside it), numbers, `TRUE`, `FALSE` and `NULL`. Anything in quotes is text, never a column.
- **Cells:** the type comes from what the cell looks like. Empty is `NULL`, `true` and `false` are booleans, numbers are numbers, and the rest is text. So write `code = 7`, not `code = '7'`: text and a number are never equal, and a cell with `007` is the number 7. Text comparisons distinguish upper and lower case.
- **Size:** a filter has at most 1,024 bytes and 128 terms.

### Limits

A table holds at most 50,000 rows and 256 columns. What lies beyond them is not read: it does not show, the filter does not see it, and nothing on the screen warns about it. A cell shows at most 2,048 characters and then `…`. Control and text direction characters show as `·`.

## Tables in C libraries

SQLite databases (`.db`, `.sqlite`, `.sqlite3`) and Parquet files are read by libraries written in C, which do not have Rust's memory safety. So they open as tables only with `V`. Autoview reads whatever the cursor passes over, and shows these files as ordinary contents, with the note `Parquet or SQLite: V opens the table`. Open a database from an untrusted source only if you accept this risk.

See also: [searching the tagged files](search.md).
