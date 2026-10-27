# The viewer

`V` opens the file in the viewer (Autoview, F7, shows it next to the list).
Binary files open in hex. Files over 16 MiB show only the beginning.

| Key | What it does |
|---|---|
| ↑ ↓ PgUp PgDn Home End | scroll |
| ← → | scroll sideways |
| `H` | text or hex |
| `C` | syntax colors on or off |
| `W` | soft wrap on or off |
| `#` or `L` | line numbers on or off (in text mode) |
| `g` or `:` | jump to a line (or byte offset in hex) |
| `E` | edit the file; back in Xylem, the viewer shows the new contents at the same place (binary files only with an editor in [`[editors]`](execute.md)) |
| `D` | with Git on: what changed since the last commit, and back ([Git](git.md)) |
| `T` / `U` | tag / untag the file |
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
lines are colored, so huge files never slow the viewer down.

## Text direction controls

Some invisible characters change the direction of the text and can make code look different from what it really says (Trojan Source, CVE-2021-42574). The viewer never hides them: each one appears as a red `·`, and the header says how many there are.

## Compressed files

Files ending in `.gz`, `.xz`, `.zst` or `.bz2` open decompressed, as text. Xylem looks at the first bytes of the file, not at its name, and opens one level only: a `.gz` inside a `.gz` stays compressed. The header says `gzip → N bytes` (the format and the size after decompression).

Decompression has limits. The viewer reads up to 16 MiB, takes at most 3 s and uses at most 64 MiB of memory for the xz dictionary or the zstd window. In Autoview the limits are 1 MiB, 100 ms and 16 MiB. When a limit is hit, the header says `(showing first N bytes decompressed)` and a note tells why:

- `compressed data truncated or corrupt`
- `decompression stopped after 3 s` (100 ms in Autoview)
- `needs more than 64 MiB of memory to decompress` (16 MiB in Autoview, with ` (V to view)`)

Autoview shows the note in its title. In hex mode (`H`) you see the decompressed bytes. Colors follow the name without the compression extension: `app.rs.gz` is colored as Rust.

For a `.tar.gz` (or `.tar.xz`...) the note says `TAR archive: Alt+F5 opens it as a folder`.

To see the raw compressed bytes, turn off `viewer_decompress` (**J** on page 2 of the [configuration](configuration.md)).

See also: [searching the tagged files](search.md).
