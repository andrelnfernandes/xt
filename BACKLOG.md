# Backlog

What is known to be left to do, kept here so it does not live only on one
machine. The user-facing summary is under "For later" in the
[README](README.md#for-later); detailed plans are in [docs/plans/](docs/plans/)
(in Portuguese).

## Security

Leftovers of the security pass that came from an external audit
([docs/plans/security.md](docs/plans/security.md); PRs #37 to #40 did the
rest).

1. **Ctrl+G still runs the repository's `core.fsmonitor`.** Only the
   automatic `git status` (file markers) runs with
   `-c core.fsmonitor=false` and skips repositories that define their own
   filters. Status, Log, Diff and Blame from the Ctrl+G menu still follow
   the repository's `.git/config`, so pressing Ctrl+G S in a downloaded or
   extracted repository can run a command it names. Small: add the same
   option to the commands that only read (`git::report`).
2. **Entering a compressed TAR still freezes the screen.** Listing a
   `.tar.gz`/`.xz`/`.bz2`/`.zst` decompresses all of it, synchronously, in
   `pane.rs` (`Reader::list`); a decompression bomb freezes Xylem there.
   Copying out of it lists it again (`Operation::reading_archive`), a
   second full pass. Medium: list once, in the background and cancellable
   (like `archive::Extractor`), and reuse the list for the copy.
3. **eXecute (`%f`, `%t`) on Windows.** `cmd` expands `%VAR%` even inside
   quotes, so a name with `%PATH%` in it changes. E and O already avoid it
   by passing the file in `XY_FILE`; commands typed with X still put the
   names in the line. Could pass each file in a variable of its own.
4. **ssh password through a file on Windows.** On Unix the askpass answer
   goes through a named pipe (mode 0600); on Windows it is written to a file
   under `%LOCALAPPDATA%` (private to the user) for a moment. A Windows
   named pipe would avoid the disk. Low priority; needs testing on Windows.
5. **Searching inside an archive reads each entry in one step.** Memory is
   bounded now (streamed), but a huge entry holds the screen up while it is
   searched, as a huge file on disk does. The search could keep the open
   reader between steps.

6. **What the table readers still cannot limit.** The time limit stops them
   between records, and SQLite midway, but a spreadsheet's reader loads the whole
   sheet first: its size is checked before that, by what the ZIP declares, so a
   ZIP that lies about its sizes gets through. A Parquet page is decompressed
   whole too. Esc cannot interrupt a table being read, only the time limit can.
7. **SQLite and Parquet's zstd are C libraries: replace them with Rust as soon
   as a usable option appears.** The author's decision. They run only with V,
   never in Autoview, and that is meant to be temporary. What to watch:
   - zstd: a way for the `parquet` crate to decode through a Rust codec; Xylem
     already has `ruzstd` for `.zst` files.
   - SQLite: a Rust implementation that reads real SQLite files reliably and is
     mature enough for hostile ones. Limbo is now Turso Database (crate `turso`).
     Checked on 2026-10-30, at 0.9.0-pre: not yet. Replace it when it has reached
     1.0, builds with no C (`default-features = false`, `pure-rust-crypto`, no
     `mimalloc`), and a fuzz of corrupt files no longer makes it panic or hang.
   Once one is replaced, its format can go back to Autoview. Until then, running
   them in a separate, isolated process would contain a bug in them. Also recheck
   the C dependencies whenever these crates are updated: `cargo tree | grep -- -sys`.

Decided against: wiping the password from memory (`zeroize`: ssh and the
pipe hold it too), and checking where remote symlinks point (copying a link
as a link is what `cp -a` and `rsync -l` do).

## Features

- **Remote machines, phases 3 and 4** (phases 1 and 2 are in 0.4.0):
  3. writing on the remote machine: upload, move, delete, rename, mkdir,
     chmod, dates; needs a filesystem layer under `ops` and the tree scan;
  4. finishing: Edit and Open via temporary download and upload, eXecute
     over ssh, Git over ssh, periodic refresh instead of inotify.
- **Disk usage, phases 3 and 4** ([docs/plans/usage.md](docs/plans/usage.md);
  phases 1 and 2 are in 0.4.0): actions inside the Usage view (tag, delete,
  prune, viewer); optional treemap, grouping by type, export.
- **Duplicate files** (same plan, "Duplicados"): D1 local (size, first and
  last bytes, then contents), D2 through rclone using the MD5 that Google
  Drive already has, D3 extras (whole duplicated folders).
- **Xylem in its own window**, further ahead: see "For later" in the README.
- Postponed by choice: macros, chown, Graft, macOS.
- **New file viewers** ([docs/plans/viewers.md](docs/plans/viewers.md)):
  six subprojects, security first. Done: subproject 1 (encodings, compressed
  files, the size limit; PRs #44 to #49), and the table mode with SQLite and the
  SQL filter (subprojects 2 and 5; PR #63), documented in
  [the manual](docs/manual/viewer.md). Still to do: SDF in the table mode, as an
  extra; the windowed viewer (3); logs (4); external converters (6).
- **Batch over files and records** ([docs/plans/batch.md](docs/plans/batch.md)):
  Ctrl+B also on what the viewer read (tables with or without a filter, text
  lines), with escaping by context and structured and binary outputs. Done: the
  line filter, the record stream, and Ctrl+B in the viewer with masks and with
  JSON, CSV, XML and HTML of whole records, and template files (steps 1 to 5).
  Next: binary outputs (6) and tagging records (7). Records into `.bat`/`.cmd` wait for item 3
  of Security; the filter on screen sees 50,000 rows, while Ctrl+B sees them all.
