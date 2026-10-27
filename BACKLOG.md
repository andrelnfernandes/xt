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
- Postponed by choice: macros, chown, Graft, `.deb` package, macOS.
