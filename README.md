<img src="docs/icons/xylem.svg" width="96" align="right" alt="Xylem icon">

# Xylem

[Leia em português](README.pt-BR.md)

Text-mode file manager for Linux, inspired by XTree and ZTreeWin.

Xylem was called **xt** until version 0.1; the command is now `xy`. On its
first run it moves xt's configuration, history and bookmarks to its own
folders. The name: the xylem is the tissue that carries water through a
tree, and it starts with an X, like XTree.

![Xylem in use: the built-in terminal with the tree following a cd, tagging files in a Branch and typing them into a command with Alt+O](docs/demo.gif)

## Why Xylem?

I have used some flavor of XTree since the 1990s, and ZTreeWin is still, to
me, the best text-mode file manager there is. I wrote Xylem because I missed
something as good as it running on Linux: the existing alternatives seem
abandoned or have drifted too far from the classic look and feel.

Xylem also builds for Windows, because Rust makes that easy, but a Windows
version is not a priority: on Windows there is ZTreeWin.

Xylem follows ZTreeWin's keys and way of working whenever possible; the
differences are listed in [Comparison with ZTreeWin](docs/manual/ztreewin.md).

## Security: hostile files

Xylem should never crash, hang, run code, write outside the destination or
exhaust memory because of what a file contains or how it is named, even when
the file was crafted to do so. Files you download, archives you open, names a
remote server sends and repositories you clone are all treated as hostile. Any
such malfunction is a serious bug, and it will be fixed promptly once reported.

Nevertheless, bugs happen. Here is what Xylem does to keep the risk small:

- Everything Xylem reads on its own (text, hex, encodings, compressed files,
  archives, spreadsheets, CSV, DBF) goes through memory-safe Rust parsers. The
  exception is SQLite databases and Parquet files, read by libraries in C:
  they run only when you open such a file with V, never in Autoview. No
  external program runs on a file unless you ask for it with E, O or X.
- Everything has a limit: how much is read, how much is decompressed, how long
  it takes and how much memory a decompressor may use. Only one level of
  compression is undone, and nothing is written to disk to show a file.
- Control characters and bidirectional controls (Trojan Source) never reach
  the terminal. Names with hidden bidi characters are marked, and O asks
  before opening them.
- When extracting archives or copying from a remote machine, paths with `..` or
  absolute paths are refused, symbolic links in archives are not extracted,
  setuid bits are dropped and names sent by an SFTP server are checked.
- The automatic Git status ignores a repository's `core.fsmonitor` and skips
  repositories that define their own filters.
- The dependencies are checked against the RustSec database in CI.

You can make the attack surface smaller still:

- Turn off decompression in the viewer (`viewer_decompress = false`, F12)
  if you browse untrusted folders with Autoview, or keep Autoview (F7) closed
  there.
- Leave Git integration off (`git = false`, the default) for repositories you
  did not create.
- Do not run Xylem as root.
- Open untrusted files with O only when you trust the program that will open
  them: Xylem hands the file over to it.

Found a problem? Please report it privately [via GitHub Security Advisories](https://github.com/andrelnfernandes/xylem/security/advisories/new), not in a public issue.

## Disclaimer

Xylem is an independent project, written from scratch. It is not affiliated
with, endorsed by or connected to Zedtek, Inc. (ZTreeWin) or to the owners of
XTree. It contains no code from those programs; their names are mentioned only
to describe what inspired Xylem and how it compares. XTree, ZTree and ZTreeWin
are trademarks of their respective owners.

## Screenshots

The directory tree, the file window and the statistics, in the classic theme:

![Xylem: directory tree, file window and statistics, in the classic theme](docs/screenshots/tree.png)

**Branch** (`B`) lists the files of a whole branch; tagged files (`T`) are
highlighted and counted on the right:

![Branch view with three tagged files](docs/screenshots/branch-tagged.png)

**Split window** (F8), each side with its own directory:

![Split window: photos on the left, source code on the right](docs/screenshots/split.png)

**Viewer** (`V`), with syntax colors:

![Viewer showing Rust code with syntax colors](docs/screenshots/viewer.png)

**Built-in terminal** (Alt+X): a `cd` in the shell takes the tree along, and
Alt+O types the tagged files at the prompt:

![Three tagged photos typed into wc -c with Alt+O](docs/screenshots/terminal-tagged.png)

**Git** (optional): status letters in the file list and the Git menu (Ctrl+G):

![File list with Git status letters and the GIT menu in the footer](docs/screenshots/git-menu.png)

**Usage** (Alt+W): where the space goes, folders and files together by the
space they take (here in the nord theme):

![The Usage view of a home folder: an old backup file at the top, then the projects, videos and downloads folders](docs/screenshots/usage-nord.png)

**Themes**: besides the classic blue, `dark`, `light`, `mono`, `norton`,
`solarized`, `gruvbox` (below), `nord`, and three that look at home on today's
desktops: `slate` (in the style of Windows Terminal), `graphite` and `daylight`
(macOS in dark and light mode), and `vivid`, with strong colors for
presentations:

![The gruvbox theme](docs/screenshots/theme-gruvbox.png)

## Installation

**Ready-made binary:** download the archive for your system from the
[Releases](https://github.com/andrelnfernandes/xylem/releases) page: Linux
x86_64 or ARM64 (static, runs on any distribution) or Windows x86_64.
Extract it and put `xy` (`xy.exe`) somewhere on your `PATH`, e.g.:

```bash
tar xzf xy-v0.5.0-x86_64-unknown-linux-musl.tar.gz
install -m 755 xy-v0.5.0-x86_64-unknown-linux-musl/xy ~/.local/bin/
```

Each archive has a `.sha256` next to it, to check with `sha256sum -c`.

**Package (.deb or .rpm):** the releases from 0.5.0 on also have packages for
Debian, Ubuntu and their derivatives (`.deb`) and for Fedora, RHEL, openSUSE
and theirs (`.rpm`), for x86_64 and ARM64 (`arm64`/`aarch64` in the name).
Besides `xy`, they install the menu entry, the icons and the man page:

```bash
sudo apt install ./xylem-fm_*_amd64.deb     # Debian, Ubuntu
sudo dnf install ./xylem-fm-*.x86_64.rpm    # Fedora, RHEL (zypper on openSUSE)
```

The package is called `xylem-fm`, as on crates.io. It comes from the
Releases page, not from a repository, so it is not updated by itself: install
the next one the same way. To remove it, `sudo apt remove xylem-fm` or
`sudo dnf remove xylem-fm`.

**With Cargo** (needs Rust, via [rustup](https://rustup.rs), and a C compiler,
for SQLite and zstd), from
[crates.io](https://crates.io/crates/xylem-fm), where Xylem is called
`xylem-fm` (the name `xylem` was taken); the command is `xy`:

```bash
cargo install xylem-fm
```

Or the latest code, straight from GitHub, or from a clone of the
repository (in the project folder):

```bash
cargo install --git https://github.com/andrelnfernandes/xylem
cargo install --path .
```

The binary goes to `~/.cargo/bin/xy` (which rustup already puts on your `PATH`).
To update, run the same command again; to remove it, `cargo uninstall xylem-fm`.

**Menu entry (Linux, optional; the packages already have it):** to have
Xylem in your desktop's application menu, opening in a terminal, run this in
a clone of the repository. `xy` must be on the `PATH` your desktop sees
(`~/.local/bin` usually is; `~/.cargo/bin` not always).

```bash
for s in 16 32 48 64 128 256 512; do
    install -Dm644 docs/icons/xylem-$s.png ~/.local/share/icons/hicolor/${s}x$s/apps/xylem.png
done
install -Dm644 docs/icons/xylem.svg ~/.local/share/icons/hicolor/scalable/apps/xylem.svg
install -Dm644 packaging/linux/xylem.desktop ~/.local/share/applications/xylem.desktop
```

**Man page (optional; the packages already have it):** in a terminal,
`xy --man` shows it; redirected, it prints the source to install (the
release archives for Linux also include it as `xy.1`):

```bash
mkdir -p ~/.local/share/man/man1 && xy --man > ~/.local/share/man/man1/xy.1
```

## What it does

- **The XTree way:** a directory tree logged into memory, the file window
  below it, Branch and Showall to list the files of a whole branch or of
  everything logged, tags that group operations act on, and ZTreeWin's keys.
- **File operations:** copy and move (with or without the paths, with progress
  and conflict details), delete to the trash with Oops to restore, Prune,
  rename with masks, permissions and dates, Compare between two windows.
- **Finding things:** Filespec by name, date and size, Treespec to type a path,
  bookmarks, and Ctrl+S to search the tagged files for text.
- **Looking inside:** a text and hex viewer with search, a line filter with
  regular expressions and syntax colors,
  Autoview next to the list, ZIP, 7z and TAR archives opened as directories,
  and display modes that show owners or what each file is.
- **Where the space goes:** the Usage view (Alt+W) lists a branch's folders
  and files by the space they take, largest first, down to what fills the
  disk; Alt+K Percent shows each folder's share right in the tree.
- **Beyond XTree:** a built-in terminal whose tree follows the shell, remote
  machines over SSH (browse and copy from them, with their own terminal),
  Google Drive, S3 and other cloud storage through rclone, an optional Git
  menu, the clipboard, a configuration screen and themes.

## Usage

```bash
xy [directory]
```

With no argument, it opens the current directory. `xy --help` shows the
options. To have the shell end up in the directory you were in when you quit
with Alt+Q, add the small shell function from
[Quitting and changing the shell's directory](docs/manual/quit.md).

## Documentation

**F1** (or **Alt+H**, in terminals that keep F1 for themselves, like GNOME
Terminal) opens help about what you are doing: inside a prompt, the help for
that prompt; in a window, the help for that window, with links to the rest.

The same help is [the manual](docs/manual/index.md), readable here on GitHub,
and the man page (`xy --man`). Some topics:
[the tree window](docs/manual/tree.md), [the file window](docs/manual/files.md),
[tags and group operations](docs/manual/tagging.md),
[Copy and Move](docs/manual/copy-move.md), [Filespec](docs/manual/filespec.md),
[the built-in terminal](docs/manual/terminal.md),
[cloud storage (rclone)](docs/manual/cloud.md), [Git](docs/manual/git.md),
[configuration](docs/manual/configuration.md),
[Windows](docs/manual/windows.md) (experimental) and the
[comparison with ZTreeWin](docs/manual/ztreewin.md).

The help also comes in Portuguese, Spanish, French, German and Chinese. The
Spanish, French, German and Chinese translations have not been reviewed by
native speakers: if you read one of these languages, [suggestions are welcome](docs/manual/TRANSLATING.md#reviews-wanted).

## Structure

- `src/model/` — in-memory directory tree (arena), independent of the UI
- `src/app/` — global state (prompts, operations) and command execution
- `src/pane.rs` — one window: tree, file list, cursors and filespec
- `src/input.rs` — commands and the prompts' text field
- `src/keymap.rs` — default keys and configuration overrides
- `src/help.rs` — the help (F1): topics, links, and the man page
- `docs/manual/` — the manual: the help's topics, in Markdown
- `src/filetype.rs` — what a file is, for the Type display mode
- `src/ui.rs` — screen drawing (ratatui)
- `src/viewer.rs` — text/hex viewer
- `src/archive.rs` — ZIP, 7z and TAR reading; ZIP and TAR creation
- `src/watch.rs` — notifications of changes on disk (inotify)
- `src/shell.rs` — editor and shell commands (with safe quoting)
- `src/editors.rs` — which editor E uses for each file (`[editors]`)
- `src/term.rs` — built-in terminal (pseudo-terminal, key codes, colors)
- `src/git.rs` — Git menu (Ctrl+G) and status markers, through the `git` program
- `src/rclone.rs` — which cloud service a folder mounted by rclone is
- `src/config.rs` — configuration file, themes and formats
- `src/settings.rs` — configuration screen (F12)
- `src/state.rs` — prompt history between runs
- `src/model/compare.rs` — Compare rules
- `src/model/filespec.rs` — file filter
- `src/ops/` — copy, move and delete, in steps (progress, conflicts, cancellation)
- `docs/screenshots/tools/` — regenerates the README screenshots (`run.sh`, Linux)
- `tests/e2e.rs` — end-to-end tests: the real `xy` in a pseudo-terminal, driven by keys (Linux); `cargo test` also runs property tests (proptest) on the Filespec and the rename masks
- `docs/icons/` — the icon, in SVG, PNG (16 to 512) and ICO
- `build.rs` — embeds the icon in `xy.exe` (Windows)
- `packaging/linux/` — menu entry (`xylem.desktop`)

## Roadmap

### Done

- The XTree core (phases 0 to 4): the classic screen, the tree and the file
  window, Filespec, Showall and Branch, tags and group operations, the split
  window, Compare, archives, Edit and eXecute, configuration, themes,
  configurable keys, the help and persistent history.
- Beyond it: the built-in terminal, remote machines over SSH (browsing,
  copying from them, their terminal), cloud storage through rclone, Git, the
  Usage view, bookmarks, Oops, binary files in a hex editor of your choice,
  Nerd Fonts icons, the help in six languages and an experimental Windows
  build.
- The viewer: encodings found by itself, compressed files, a configurable
  size limit, soft wrap, line numbers, goto line, and protection against
  hidden text direction characters.

### Next

- **Remote machines (SSH):** today the remote side is read-only. Next comes
  writing to it (copy to it, move, delete, rename, make directories,
  permissions, dates), then Edit and Open (through a temporary copy that
  goes back when saved), eXecute and Git over ssh, and periodic refresh.
- **Viewer:** a table mode for DBF (with memo files) and CSV, and SDF as an
  extra; a viewer that reads in windows, like `less`, with no size limit for
  uncompressed files; logs (follow, jump to the end, filter lines, colors by
  level); SQLite; and, optional and isolated, external converters such as
  `pdftotext` or `exiftool`.
- **Disk usage:** actions inside the Usage view (tag, delete, prune, view),
  then an optional treemap, grouping by type and export.
- **Duplicate files:** on this machine (by size, then by content), on Google
  Drive through the checksums it already keeps, and whole duplicated folders.
- **From ZTreeWin:** the items in "What Xylem does not have yet" in the
  [comparison with ZTreeWin](docs/manual/ztreewin.md), such as JFC, Global,
  the mouse and saving the log.
- **Hardening:** the leftovers of the security pass, listed in
  [BACKLOG.md](BACKLOG.md).

### For later

- Windows: it builds and passes the tests (see
  [Windows](docs/manual/windows.md)), but it is not a priority while
  ZTreeWin exists. Maybe macOS.
- Postponed by choice: macros, chown and Graft.
- **Xylem in its own window**, further ahead: the same screen drawn in a
  window of its own instead of a terminal (a ratatui backend that renders
  to a window, e.g. on wgpu), with its own font and icon. It would need no
  terminal, which matters most on Windows and macOS; it would always see
  Ctrl and Alt pressed and released, so the menus would follow them in any
  system; and it would make room for the mouse. The work: a new event loop
  and key mapping (the rest of Xylem only receives keys and draws on a
  ratatui frame), E and eXecute running in the built-in terminal, and the
  packaging. A full graphical interface (egui, GTK...) is not the plan: it
  would be another program, without the XTree look.

The full list of what is left, including the developers' notes, is in
[BACKLOG.md](BACKLOG.md).

## License

Xylem is released under the [MIT License](LICENSE).
