# The tree window

The tree shows the directories that have been logged (read into memory). The
file window below it lists the files of the directory under the cursor.

## Moving around

| Key | What it does |
|---|---|
| ↑ ↓ PgUp PgDn Home End | move the cursor |
| → or `+` | expand the directory |
| ← or `-` | collapse it (or go to the parent) |
| Backspace | go to the parent directory |
| Space | down one line |
| Shift+letter, 0–9 | next directory starting with it |
| `\` or `/` | [Treespec](treespec.md): go to a directory by typing its path |
| Enter or Tab | go to the file window (Tab switches windows in [split screen](split.md)) |

In the tree, a `+` on the connector (`├─+`) marks a collapsed directory that has
subdirectories.

## Logging

Directories are read as you open them. `*` logs and expands the whole branch
under the cursor; during a long log, progress shows in the footer and `Esc`
cancels. **F3** (or F2) rereads the directory from disk. Xylem also watches the
current directory of each window and rereads it on its own when another
program changes it (at most once every half second, after any operation in
progress); other directories still need F3.

`L` ([Log](treespec.md)) opens another directory in the window.

## Lists of files

- `B` (Branch): the files of the whole branch, logging it first.
- `S` (Showall): the files of everything that has been logged.
- `F` ([Filespec](filespec.md)): which files show, by name, date and size.

In Branch and Showall, `\` goes to the directory of the selected file.

## Commands in the tree

| Key | What it does |
|---|---|
| `T` / `U` / `I` | tag / untag / invert the files of the directory ([tags](tagging.md)) |
| `M` | make a subdirectory |
| `R` | rename the directory |
| `D` or Del | delete the directory, if empty ([delete](delete.md)) |
| Alt+P | Prune: delete the directory with everything in it |
| `O` or Ctrl+Z | Oops: restore what went to the trash |
| `N` | change the directory's date ([dates](attributes.md)) |
| `C` / Alt+C | [Compare](compare.md) with the other window / the whole branch |
| `A` | Avail: mounted file systems ([information](info.md)) |
| Alt+I | Info: details of the directory |
| Alt+K | sizes or file counts next to each directory |
| `Y` | copy the directory's path to the clipboard |
| `X` | [eXecute](execute.md) a shell command |
| Alt+X | the [built-in terminal](terminal.md) |
| F8 | [split screen](split.md) |
| F11 / Alt+B | [bookmarks](bookmarks.md); Ctrl+B saves one |

See also: [the file window](files.md), [the keys in effect](keys.md).
