# The file window

The file window lists the files of the directory under the tree cursor, or of
a whole branch (`B`) or of everything logged (`S`). **Enter** goes back to the
tree, **Esc** too (at the directory of the selected file).

## Moving around

| Key | What it does |
|---|---|
| ↑ ↓ PgUp PgDn Home End | move the cursor |
| ← → | page up and down (in the column modes: previous and next file) |
| Backspace | show the files of the parent directory |
| Shift+letter, 0–9 | next file starting with it |
| Space | down one line |

## Display modes (Alt+F)

Alt+F cycles, in ZTreeWin's order, between **names in several columns**,
**names and sizes in columns**, **details** (one column with size, date and
permissions, the default) and **long names** (one column, full names only).

Before long names come two modes ZTreeWin does not have:

- **owner**: size, `user:group` and permissions (not shown on Windows or
  inside archives);
- **type**: what each file is, in the spirit of the `file` command ("PNG
  image, 800 x 600", "ELF 64-bit executable", "python3 script", "Rust, UTF-8
  text"...), from the first bytes of the visible files.

In the column modes, files run from left to right, ← → move one file and ↑ ↓
one row; names longer than 20 characters are cut off with `…`. The initial mode
is `file_display` in the [configuration](configuration.md).

## Autoview (F7)

Splits the screen between the list and the contents of the selected file,
which follows the cursor. The file window's keys keep working (`T` tags, for
example); Shift+↑ ↓ PgUp PgDn Home End scroll the contents; `W` toggles soft
wrap, `#` toggles line numbers; F7, Esc or Enter leave. It shows the first
1 MiB of each file (`V` shows up to 16 MiB).

## Opening files

- `V`: the [viewer](viewer.md) (text or hex, with search).
- `E`: the editor ([Edit](execute.md)).
- `O`: the system's default program (`xdg-open`: images in the image viewer,
  PDFs in the PDF reader...), in the background. Ctrl+O opens all the tagged
  files. Inside an archive, the entry is first extracted to a temporary folder,
  deleted when Xylem exits. Another program can be set in `opener`.
- Alt+F5: open an [archive](archives.md) as a directory.

## The clipboard

`Y` or Ctrl+Insert copies the path of the file (in the tree, of the directory)
to the clipboard; Ctrl+Y copies the paths of the tagged files, one per line;
`y` in the viewer copies the path of the file being viewed. It works over SSH
and tmux (OSC 52) and keeps the clipboard after Xylem exits, on X11 and Wayland.

## Other commands

| Key | What it does |
|---|---|
| `T` / `U` | tag / untag the file and move down ([tags](tagging.md)) |
| `C` / `M` | [copy / move](copy-move.md) the file |
| `D` or Del | [delete](delete.md) it (Shift+Del: permanently) |
| `R` | [rename](rename.md) it; Ctrl+R renames the tagged files with a mask |
| `A` | [permissions](attributes.md) |
| `N` | [change the date](attributes.md) |
| Alt+S | [sort](sort.md) |
| Ctrl+S / Ctrl+V | [search the tagged files / view them](search.md) |
| Ctrl+B | [Batch](execute.md): a script with one line per tagged file |
| F5 / Ctrl+F5 | create an [archive](archives.md) with the file / the tagged files |
| Alt+I | Info: details of the file ([information](info.md)) |

See also: [the tree window](tree.md), [the keys in effect](keys.md).
