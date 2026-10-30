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

6. **The table loaders ignore the viewer's limits.** They open the file by its
   path, so Autoview's 1 MiB and its time limit do not apply: passing the cursor
   over a big spreadsheet or CSV can hold the screen up. The row and column
   limits do apply. A SQLite query cannot be cancelled either.
7. **Tables of remote files and archive entries.** `Viewer::from_file` gives the
   loaders the path it received, which is opened on the local disk, so a remote
   `foo.csv` may show a local file with the same path, or fail. Seen in the code,
   not tested. Should load tables only from files on disk.
8. **SQLite and Parquet's zstd are C libraries.** They run only with V, never in
   Autoview. Running them in a separate, isolated process would contain a bug
   in them.

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
  charsets and compressed files, table mode (DBF, CSV, SDF), windowed
  viewer, logs, SQLite, external converters; six subprojects, security
  first. Subproject 1 (encodings, compressed files, the size limit) is done
  (PRs #44 to #49); next is subproject 2, the table mode.
- **Batch over files and records** ([docs/plans/batch.md](docs/plans/batch.md)):
  Ctrl+B also on what the viewer read (tables with or without a filter, text
  lines), with escaping by context and structured and binary outputs. The line
  filter in the text viewer (`&`) is done; next are raw values kept apart from
  the display and a record iterator without the 50,000-row limit.
