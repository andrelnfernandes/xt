# xt

[Leia em português](README.pt-BR.md)

Text-mode file manager for Linux, inspired by XTree and ZTreeWin.

## Why xt?

I have used some flavor of XTree since the 1990s, and ZTreeWin is still, to
me, the best text-mode file manager there is. I wrote xt because I missed
something as good as it running on Linux: the existing alternatives seem to be
abandoned.

As much as I like ZTreeWin, its development is extremely slow. That is why xt
will probably get a Windows version too.

xt follows ZTreeWin's keys and way of working whenever possible; the
differences are listed in [Comparison with ZTreeWin](#comparison-with-ztreewin).

## Disclaimer

xt is an independent project, written from scratch. It is not affiliated
with, endorsed by or connected to Zedtek, Inc. (ZTreeWin) or to the owners of
XTree. It contains no code from those programs; their names are mentioned only
to describe what inspired xt and how it compares. XTree, ZTree and ZTreeWin
are trademarks of their respective owners.

## Installation

Requires Rust (via [rustup](https://rustup.rs)). In the project folder:

```bash
cargo install --path .
```

The binary goes to `~/.cargo/bin/xt` (which rustup already puts on your `PATH`).
To update after code changes, run the same command again;
to remove it, `cargo uninstall xt`.

## Usage

```bash
xt [directory]
```

With no argument, it opens the current directory. `xt --help` shows the options.

### Quitting and changing the shell's directory (Alt+Q)

A program cannot change the directory of the shell that launched it. So,
as in other file managers, **Alt+Q** writes the current directory to a file
(`--cwd-file`) and a small shell function does the `cd`.
Put this in your `~/.bashrc` (or `~/.zshrc`):

```bash
xt() {
    local tmp dir
    tmp=$(mktemp) || return
    command xt --cwd-file "$tmp" "$@"
    dir=$(cat -- "$tmp")
    rm -f -- "$tmp"
    [ -n "$dir" ] && [ -d "$dir" ] && cd -- "$dir"
}
```

With it, `xt` works as before; only when you quit with **Alt+Q** does the shell
end up in the directory you were in inside xt. `Q` and F10 quit without changing anything.

## Configuration

The easiest way is the **configuration screen: F12** (or Alt+F10, as in
ZTreeWin; on Ubuntu/GNOME, Alt+F10 usually maximizes the window and never
reaches xt). It has four pages:

| Page | Options |
|---|---|
| 1. Programs and formats | editor, date format, number format |
| 2. Display and sorting | show hidden files, initial sort order and direction, what Enter does in the file window, initial mode of the file window, syntax colors in the viewer |
| 3. Confirmations and history | confirm on quit, Prune confirmation, don't keep entries containing `@` |
| 4. Compare and refresh | ignore 2 s, ignore 1 h, automatic refresh |

The option's letter (or Enter) changes the value, PgUp/PgDn or 1–4 switch pages,
**F4** picks the theme from a list (with an instant preview), **F3** saves and applies,
**Esc** cancels (it asks if there are changes).

Everything is stored in `~/.config/xt/config.toml` (or `$XDG_CONFIG_HOME/xt/`),
which can also be edited by hand; the screen preserves your comments. To start
from a commented file with all the defaults:

```bash
mkdir -p ~/.config/xt && xt --default-config > ~/.config/xt/config.toml
```

- `theme`: `classic` (XTree blue), `dark`, `light`, `mono` (the terminal's
  own colors), `norton`, `solarized`, `gruvbox` or `nord`.
- `[options]`: the options from the configuration screen.
- `[colors]`: overrides individual colors on top of the theme, using names
  (`"yellow"`), `"#rrggbb"` or indexes from 0 to 255.
- `[keys.any]`, `[keys.tree]`, `[keys.files]`: rebind keys, in both
  windows or in just one (this one takes precedence). E.g.: `"f3" = "view"`,
  `"ctrl+r" = "relog"`; `"none"` disables a key. **F1** inside xt
  shows the keys currently in effect, including your changes.

An error in the file does not stop xt from starting: it uses the defaults and shows the
error in the footer (and the configuration screen will not save until the file is
fixed, so it doesn't erase what you wrote).

## Keys

### Navigation

| Key | Tree | File window |
|---|---|---|
| ↑ ↓ PgUp PgDn Home End | moves the cursor | moves the cursor |
| → / `+` | expands the directory | |
| ← / `-` | collapses (or goes to the parent) | page up (in columns: previous file) |
| → | expands | page down (in columns: next file) |
| Alt+F | | display mode: name columns, columns with size, details, long names |
| Backspace | parent directory | shows the files of the parent directory |
| Space | down one line | same |
| `*` | logs and expands the whole branch | |
| `\` | Treespec: go to a directory by typing the path | same (in Branch/Showall: goes to the file's directory) |
| Shift+letter, 0–9 | next directory starting with it | next file starting with it |
| Alt+K | shows the size or file count of each directory in the tree | same |
| `B` | Branch: files of the whole branch (logs first) | same |
| `S` | Showall: files of everything that has been logged | same |
| `F` | Filespec: file filter | same |
| Alt+S | sorts the files: name, extension, date, size; order | same |
| F3 (or F2) | rereads the directory from disk | same |
| `V` | | views the file (text/hex, with search) |
| F7 | Autoview: list with the contents of the selected file alongside | same |
| Ctrl+S | | searches for text in the tagged files and untags those without it |
| Ctrl+V | | views the tagged files in sequence |
| `O` | | opens the file in the default program (`xdg-open`) |
| Ctrl+O | | opens the tagged files in the default program |
| Alt+F5 | | opens a ZIP/TAR as a directory |
| `E` | | opens the file in the editor |
| `X` | runs a shell command | same |
| `L` | Log: opens another directory in the window | |
| `C` | Compare: compares with the other window and tags | |
| F8 | splits the screen into two windows (or back to one) | same |
| Tab | switches window (split screen) | same |
| Enter | opens the file window | goes back to the tree |
| Esc | | goes back to the tree (at the file's directory) |
| Alt+I | Info: directory details (and what is logged in it) | Info: file details |
| `A` | Avail: mounted file systems and free space | |
| `?` | window statistics | same |
| F1 | help: list of the keys in effect | same |
| F12 / Alt+F10 | configuration screen | same |
| `Q` / F10 | quit (asks for confirmation) | quit |
| Alt+Q | quits, leaving the shell in the current directory (see above) | same |

### Tags and operations

| Key | Tree | File window |
|---|---|---|
| `T` / `U` | tags / untags the files in the directory | tags / untags the file and moves down |
| Ctrl+T / Ctrl+U | tags / untags everything logged (filespec) | tags / untags the whole list |
| `I` | inverts the directory's tags | inverts the list's tags |
| Ctrl+F7 / F8 / F9 | tags / untags / inverts the logged files of the branch | |
| `C` | | copies the file |
| `M` | creates a subdirectory | moves the file |
| `D` | deletes the directory (only if empty) | deletes the file |
| Alt+P | Prune: deletes the directory with everything in it | |
| `R` | renames the directory | renames the file |
| `A` | | permissions (octal, e.g. `644`) |
| `N` | changes the directory's date | changes the file's date (Ctrl+N: of the tagged files) |
| Ctrl+B | | Batch: creates a script or list with one line per tagged file |
| F5 / Ctrl+F5 | | creates a ZIP/TAR with the file / with the tagged files |
| Ctrl + `C` `M` `D` `A` | same, on the tagged files | same, on the tagged files |
| Alt+C | Compare of the whole branch | copies the tagged files, **recreating the paths** |
| Alt+M | | moves the tagged files, **recreating the paths** |

Group operations act on the tagged files **of the current list**
(the directory, or the open Branch/Showall).

**With or without paths:** Ctrl+C / Ctrl+M copy or move all the tagged files
straight into the destination. Alt+C / Alt+M recreate the subdirectories at the destination,
relative to the list's root (the current directory, the Branch root, or the root of the
tree in Showall): `proj/src/model/tree.rs` → `destino/src/model/tree.rs`.
At the destination prompt, **F6** switches between the two modes, and the current mode is shown
in the prompt itself. Alt+D and Alt+A work like Ctrl+D and Ctrl+A.

**Split screen:** F8 opens a second window, starting in the same directory;
Tab switches between them (without the split, Tab does the same as Enter). Each window
has its own tree, cursor and filespec; use `L` to open another directory in it.
With the screen split, Copy/Move suggests the other window's directory as the destination,
and both windows are refreshed after any operation.

**Compare (`C` in the tree, with the screen split):** compares the files of the
current directory with those of the other window's directory and tags, as you choose:
**U**nique (not present on the other side), **N**ewer, **O**lder, **I**dentical
(same size and date), **D**ifferent, or **A**ll new (unique + newer:
what is missing to bring the other side up to date). **B** (or Alt+C in the tree,
as in ZTreeWin) switches the scope to the whole branch: xt logs the branch,
compares following the relative paths
(`src/a.rs` ↔ `<other>/src/a.rs`) and opens the Branch with the tagged files.
Dates up to 2 seconds apart count as equal (FAT/exFAT).

To synchronize: `C` `B` `A` and then Alt+C (copy with paths), which
already suggests the other window as the destination.

**Sorting (Alt+S):** **N**ame, **E**xt, **D**ate or **S**ize; **O**rder
toggles ascending/descending and **P**ath first groups by directory in
Branch/Showall. The current order appears in the file window's title (e.g.
`Size↓`) and the cursor stays on the same file. The initial order comes from `sort` and
`sort_descending` in the configuration; new windows (F8, Log) inherit the current one.

**Treespec (`\`):** opens a prompt with the current path; as you type,
the tree cursor goes to the first directory starting with what you have
typed (case-insensitive). `/` or `\` complete the name and go down
one level; Tab and Shift+Tab complete and then move to the next/previous
directory at the same level; Backspace goes back up levels; Enter confirms; Esc returns
to where you were. ↑ brings up the history (saved between runs). A path outside
the tree becomes a Log when confirmed. In Branch/Showall, `\` goes straight to the
directory of the selected file, with it highlighted.

**Display modes (Alt+F):** cycles, in ZTreeWin's order, between
**names in several columns**, **names and sizes in columns**, **details**
(one column with size, date and permissions, the default) and **long names** (one
column, full names only). In the column modes, files run from
left to right, ← → move one file and ↑ ↓ one row; names longer than
20 characters are cut off with `…`. The initial mode is set in `file_display`
(page 2 of the configuration).

**Autoview (F7):** splits the screen between the file list and the contents of the
selected file, which follows the cursor. The file window's keys
keep working (T tags, for example); Shift+↑/↓/PgUp/PgDn/Home/End
scroll the contents; F7, Esc or Enter exit. It shows the first 1 MiB of each
file (V shows up to 16 MiB).

**KByte (Alt+K):** shows to the right of each directory in the tree the
**S**ize (total in K) or the **C**ount (number of files); **N**one turns it off.
Before choosing, **A**ll/**T**agged count all files or only the tagged ones, and
**B**ranch/**D**irectory add up the whole branch or just the directory. Directories
not yet logged show `?`. Each window has its own setting.

**Edit and eXecute:** `E` opens the file in the editor (`$VISUAL`, `$EDITOR`, Debian/Ubuntu's
`editor` or `vi`). `X` asks for a command line and runs it with
`sh` in the current directory; `%f` becomes the file under the cursor, `%t` the tagged
files and `%d` the current directory, all already quoted (`%%` is a literal `%`).
Example: `tar czf backup.tgz %t`. At the end, xt waits for Enter so you can read the
output. When it returns, the open directories are reread.

**History:** at prompts, ↑/↓ bring up previous entries. Each type of
prompt (Filespec, Copy/Move destination, Log, eXecute...) has its own history,
stored in `~/.local/state/xt/history.toml` (or `$XDG_STATE_HOME/xt/`), readable
only by you, since it may contain typed commands.

**Open (`O`):** opens the file in the system's default program (`xdg-open`:
images in the image viewer, PDFs in the PDF reader, etc.), in the
background, without leaving xt. Ctrl+O opens all the tagged files. Inside an
archive, the entry is first extracted to a temporary folder,
deleted when xt exits. Another program can be chosen in `opener` (or in the
configuration screen).

**Archives:** Alt+F5 in the file window opens `.zip`, `.jar`,
`.war`, `.apk`, `.tar`, `.tar.gz` or `.tgz` as a tree, read-only:
navigation, Filespec, Branch, Showall, tags, View and Compare work inside
it, and Copy (including Ctrl+C and Alt+C with paths) extracts the files.
`Esc` in the tree closes the archive and returns to where you were. For safety,
entries containing `..` or an absolute path are ignored, and symbolic links
inside archives are never extracted.

**Automatic refresh:** xt watches (via inotify) the current directory of
each window and rereads it on its own when another program creates, deletes, renames or
modifies files there, keeping the cursor on the same file. Changes are
batched (at most one reread every half second) and wait for the end of
any operation in progress. Other directories still need F3.

**Ctrl/Alt menus:** as in ZTree, the footer shows only the one-key commands. **F9**
switches to the Ctrl menu, then the Alt menu, and back. With one of
these menus opened by F9, the next letter counts as Ctrl+letter or
Alt+letter (e.g. `F9` `M` = Ctrl+M); `Esc` cancels. In terminals with the
kitty keyboard protocol, just **holding Ctrl or Alt** makes the
corresponding menu appear while the key is pressed.

**Ctrl+M:** in most terminals, Ctrl+M arrives the same as Enter. xt
automatically enables the kitty keyboard protocol when the terminal supports it
(kitty, WezTerm, foot, Ghostty, recent Alacritty, iTerm2), and then Ctrl+M moves
the tagged files as in ZTree. In VTE-based terminals (GNOME
Terminal, Ptyxis, Terminator, Tilix) use **F9 M** (or Alt+M and F6 at the prompt).

During a long log, progress appears in the footer and `Esc` cancels.

**Filespec (`F`):** specifications separated by spaces, `;` or `,`, of three
kinds, as in ZTreeWin:

- **name:** glob (`*.rs`, `a?c*`, `[abc]*`), case-insensitive;
  in quotes, the name can contain spaces (`"meu arquivo*"`);
- **date:** `<`, `=`, `>`, `<=`, `>=` or `<>` and a date in the order of the
  configured format, with `/`, `.` or `-` and a 2- or 4-digit year (`>01/05/2026`;
  `2026-05-01` always works), or `TODAY` and `TODAY-n` (`>TODAY-7`: last 7
  days);
- **size:** the same with an `s` after the operator (`>=s200000`); in xt
  K, M and G also work (`>s100M`).

A leading `-` negates (`-*.bak`; `-<s1K` is the same as `>=s1K`). Within
each kind, positive specifications are ORed together, except `>a` immediately
followed by `<b`, which is the range between the two (`>=01/05/2026,<=31/05/2026`:
May); `<>` excludes. Between kinds, AND applies: `*.txt,=TODAY,<s1000` are the
`.txt` files from today smaller than 1000 bytes.

**Copy/Move:** the destination can be a directory (relative to the directory under the
tree cursor, absolute, or with `~`) or, for a single file, a new name. If the destination already exists,
xt asks: Yes / No / All / Skip all. The copy is made to a temporary file that
only replaces the destination at the end: canceling (`Esc`) or an error never leaves a
half-written file. Permissions and modification time are preserved, and symbolic
links are copied as links. Moving across disks copies and then deletes.

**New date (`N`):** changes the modification date. The prompt comes pre-filled with the
current date in your format; it also accepts `now` and `today`. With only the date (no
`HH:MM`), each file keeps its own time of day. Symbolic links are skipped (the
date would change that of the file they point to).

**Batch (Ctrl+B):** asks for the name of the file to create and a line mask,
and writes one line per tagged file. Parameters: `%f` path, `%d`
directory, `%n` name, `%b` name without extension, `%e` extension, `%s` size.
In lowercase they come out quoted, safe for the shell (`convert %f %b.png`);
in uppercase, as plain text, for lists (`%N;%S`). A `.sh` file
gets `#!/bin/sh` and execute permission (run it with `X`). An existing file
is not overwritten.

**Create archive (F5 / Ctrl+F5):** creates a `.zip`, `.tar`, `.tar.gz` or
`.tgz` (according to the name you type) with the current file or the tagged files. The
paths inside are relative to the list's root, as with Alt+C: in a Branch,
subfolders are kept. It shows progress and `Esc` cancels; the file is written
to a temporary file and only gets its name at the end, so it is never left
half-written. Symbolic links are stored as links. An existing file is not overwritten.

**Info, Avail and statistics:** Alt+I shows type, size, permissions,
owner and group, dates (modification, access and change), inode, links and
device of the file or directory (for a link, where it points; for a
directory, also what is logged in it and in its branch). `A` in the tree lists the mounted
file systems with total, used, free and % used, like
`df`. `?` summarizes the window: what is logged, the list, the tagged files and the disk.

**Delete** really deletes (it does not go to the trash) and always asks for confirmation.

**Prune (Alt+P in the tree):** logs the whole branch, shows how many files,
directories and bytes will be deleted and asks you to **type PRUNE** (or the
directory's name, or Y, depending on `prune_confirmation`). It deletes the files and then
the directories, from the deepest up, with progress and `Esc` to cancel.
Symbolic links are removed without following the target, and **other mount
points inside the branch are skipped** (a USB drive mounted in there is not
deleted). The tree's root cannot be pruned.

**Syntax colors:** the viewer colors source code and scripts in about 150
languages (Rust, Python, shell, C, JavaScript, SQL, YAML, Markdown...),
recognized by extension, by name (`Makefile`) or by the first line
(`#!/bin/bash`). **C** turns colors on and off while viewing; the default is
`syntax_highlight` (page 2 of the configuration). Colors are adjusted to stay
readable on the current theme's background, the classic blue included. Only
the first 20,000 lines are colored, so huge files never slow the viewer down.

**Viewer:** arrows/PgUp/PgDn/Home/End scroll, ←→ scroll horizontally,
`H` toggles text/hex, `Esc` closes. Binary files open in hex.
Files over 16 MiB show only the beginning. Search, as in ZTreeWin:
**F** searches from the start, **B** backwards from the end, **/** (or S) from
the current page and **\\** backwards from it; **Space**
continues in the same direction, **+** and **-** choose the direction. The match
is highlighted.

**Searching the tagged files (Ctrl+S) and viewing them (Ctrl+V):** Ctrl+S searches
for text in all the tagged files of the list and **untags those that do not
contain it**, with progress and `Esc` to stop. Ctrl+V opens the remaining ones in the
viewer, one after another: **N**ext and **P**rev switch files
(Alt+Home/Alt+End: first/last) and **U** untags the current one. The term is the
same in both searches: after a Ctrl+S, **Space** in Ctrl+V goes
straight to the next match, moving on to the next tagged file when the file
ends. At the search prompts, **F2** toggles case sensitivity
(when case-insensitive, accents also count: `AÇÃO` finds `ação`) and **F4** switches
between text and hex (byte pairs such as `4d 5a`). A `*` in the middle of the term matches
anything on the same line (`erro*disco`); a term never crosses a
line break. It also works inside ZIP/TAR.

In the tree, a `+` on the connector (`├─+`) marks a collapsed directory that has subdirectories.

## Structure

- `src/model/` — in-memory directory tree (arena), independent of the UI
- `src/app.rs` — global state (prompts, operations) and command execution
- `src/pane.rs` — one window: tree, file list, cursors and filespec
- `src/input.rs` — commands and the prompts' text field
- `src/keymap.rs` — default keys, configuration overrides and help (F1)
- `src/ui.rs` — screen drawing (ratatui)
- `src/viewer.rs` — text/hex viewer
- `src/archive.rs` — ZIP/TAR reading
- `src/watch.rs` — notifications of changes on disk (inotify)
- `src/shell.rs` — editor and shell commands (with safe quoting)
- `src/config.rs` — configuration file, themes and formats
- `src/settings.rs` — configuration screen (F12)
- `src/state.rs` — prompt history between runs
- `src/model/compare.rs` — Compare rules
- `src/model/filespec.rs` — file filter
- `src/ops/` — copy, move and delete, in steps (progress, conflicts, cancellation)

## Roadmap

- [x] Phase 0 — classic screen, navigable tree, file window
- [x] Phase 1 — Filespec, Showall/Branch, text/hex viewer, log with progress
- [x] Phase 2 — tagging, copy/move/delete/rename, makedir, permissions, re-log
- [x] Phase 3 — split window (F8), Log, Compare, ZIP/TAR, inotify, Edit,
  eXecute, history in prompts
- [x] Phase 4 — configuration, themes, configurable keys, help (F1),
  persistent history, release build and installation

### For later

- `.deb` package (license and maintainer still to be decided)
- Windows version (and maybe macOS)
- the items under "Not implemented" below

## Comparison with ZTreeWin

A command-by-command survey based on the official ZTreeWin v2.2 manual
([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf), from 2011, in
particular the "Quick Command Table" and section 3.7 on configuration). Newer
versions of ZTreeWin may have changed details.

### Implemented

Tree with on-demand log (`+`, `*`), file window, Filespec, Branch,
Showall, tags (T, U, I, Ctrl+T/U), Copy/Move/Delete/Rename with the
Ctrl (tagged) and Alt (tagged with paths) versions, Make dir, Delete dir, Prune,
Attributes, View, Edit, eXecute, Compare (directory and branch), Sort, Log,
split window (F8 and Tab), relog (F3), Open (`O`), automatic refresh, browsing and
extraction of archives, history in prompts, help (F1) and configuration
screen (Alt+F10).

### Simplified

| Feature | In ZTreeWin | In xt |
|---|---|---|
| Filespec | name with its own name and extension rules, date, size, attribute, inverted filespec (Ctrl+I), labels | glob for names, date and size as in ZTreeWin (with K/M/G); no attributes, inversion or labels |
| Viewer | Alpha, Dump, Hex, Junk and Wrap modes; character sets; hex editing; bookmarks; auto-scroll | text (UTF-8) or hex, with ZTreeWin's search (F, B, /, \\, Space, + and -), syntax colors (which ZTreeWin does not have), no line wrapping; up to 16 MiB |
| Ctrl+S | text, hex or Unicode; searches only the ZIP index | text or hex |
| Attributes | Windows attributes (R, A, S, H) | Unix permissions in octal (`644`); no owner/group |
| Sort | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; same/diff in split screen | Name, Ext, Date, Size, ascending/descending, Path first; each window has its own order |
| Compare | options Identical, Unique, Newer, Older, Size, Binary, Case; display filter | tags unique, newer, older, identical, different or unique+newer; no binary comparison |
| Prune | deletes only the subdirectories or the current one too; only empty directories; recycle bin | always deletes the whole directory, permanently; configurable confirmation |
| Delete | optional recycle bin | deletes for good |
| Log | several logged drives, Global, drive switching | one root per window; `L` opens another directory |
| eXecute | batch parameters `%1`–`%8`, new window (NTW) | `sh` with `%f`, `%t` and `%d`; always in xt's terminal |
| History | history window with bookmarks | ↑/↓ at the prompt, one history per type, saved between runs |
| Configuration | 7 pages, per-element color editor (F4), Alt-Export | 4 pages with the options that make sense on Linux; F4 chooses among 8 ready-made themes; individual colors in `config.toml` |
| Archives | any format with an external archiver, create (F5), extract (Alt-F5) | ZIP/JAR/WAR/APK/TAR/TGZ: open (Alt+F5), extract with Copy, create with F5 (ZIP, TAR and TAR.GZ) |
| Split screen | optional statistics panels per side | no statistics panel |
| Menus | Ctrl/Alt menu when holding the key | same in terminals with the kitty protocol; F9 in any terminal |

ZTreeWin configuration options that were not brought over because they are
Windows-specific: RSH attributes, recycle bin, junctions, OEM/ANSI, EBCDIC, window
size, shortcuts (.lnk). The rest depend on features not yet built
(below).

### Different keys

The keys follow ZTreeWin's. The exceptions:

| Function | ZTreeWin | xt |
|---|---|---|
| Configuration | Alt+F10 | F12 or Alt+F10 (on GNOME, Alt+F10 maximizes the window and never reaches xt) |
| Move tagged | Ctrl+M | Ctrl+M in terminals with the kitty protocol; in others, `F9` `M` (or Alt+M and F6) |
| `-` in the tree | unloads the branch from memory | only collapses (stays logged) |
| F2 | (nothing) | rereads the directory, same as F3 |
| F9 | applications menu (not implemented) | Ctrl/Alt menus |
| F12 | macros (not implemented) | configuration |
| Ctrl+C, Ctrl+M in the tree | (nothing) | Copy/Move of the current directory's tagged files |

All keys can be rebound under `[keys]` in `config.toml`.

### Not implemented

- **Tree:** Global (G), Graft (Alt+G), Mirror (Alt+M), Oops
  (O, recover deleted files), Release/unload from memory (Alt+R, `-`,
  Ctrl+Backspace), Hide (Alt+H), Wash (Alt+W), Volume
  (Ctrl+V), F5/F6 (hide subdirectory levels),
  tag by attribute (Alt+T/U),
  Treespec in edit mode (F5, wildcards) and special folders (F6), type-ahead
  search with prompt (`|`) and active mode, drive switching (`<` and `>`).
- **Files:** Open in a new terminal window (Alt+O), Batch of the current file (Alt+B), JFC (J, compare the contents of two files), ZAAP
  (Y), sHortcut (H), Print (P), Rename with mask, find and replace and case change,
  owner/group (chown), Megaview (Ctrl+F7 in the file window), merge tagged files between
  windows (Ctrl+F6), Partial Untag (Ctrl+F8), copy path to the
  clipboard (Ctrl+Insert).
- **General:** macros (F12), applications menu (F9), bookmarks (F11, `[`
  and `]`), Quit and shell (Ctrl+Q), saving and
  loading the log (Zlist, Ctrl/Alt+Z), recycle bin, colors by name and attribute
  (ZCOLORS.INI), mouse, ZTreeWin command-line options, video
  commands (Alt+F6–F9).

## License

xt is released under the [MIT License](LICENSE).
