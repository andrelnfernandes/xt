# Tags and group operations

Tagged files (marked with `■`) are what the group operations act on: the
tagged files **of the current list** (the directory, or the open Branch or
Showall).

## Tagging

| Key | Tree | File window |
|---|---|---|
| `T` / `U` | tag / untag the files of the directory | tag / untag the file and move down |
| Ctrl+T / Ctrl+U | tag / untag everything logged (that matches the Filespec) | tag / untag the whole list |
| `I` | invert the directory's tags | invert the list's tags |
| Ctrl+F7 / F8 / F9 | tag / untag / invert the logged files of the branch | |

## Operations on the tagged files

| Key | What it does |
|---|---|
| Ctrl+C / Ctrl+M | [copy / move](copy-move.md) them into the destination |
| Alt+C / Alt+M | copy / move them, **recreating their paths** |
| Ctrl+D | [delete](delete.md) them |
| Ctrl+A | change their [permissions](attributes.md) |
| Ctrl+N | change their [date](attributes.md) |
| Ctrl+R | [rename](rename.md) them with a mask |
| Ctrl+B | [Batch](execute.md): a script or list with one line per file |
| Ctrl+F5 | put them in an [archive](archives.md) |
| Ctrl+S / Ctrl+V | [search them for text / view them](search.md) |
| Ctrl+O | open them in the default program |
| Ctrl+Y | copy their paths to the clipboard |
| Alt+O | type them into the [built-in terminal](terminal.md) |

In eXecute, `%t` stands for the tagged files ([eXecute](execute.md)).

## Ctrl and Alt menus (F9)

As in ZTree, the footer shows only the one-key commands. **F9** switches to the
Ctrl menu, then the Alt menu, and back. With one of these menus open, the next
letter counts as Ctrl+letter or Alt+letter (`F9` `M` = Ctrl+M); `Esc` cancels.
In terminals with the kitty keyboard protocol, just **holding Ctrl or Alt**
shows the corresponding menu.

**Ctrl+M:** in most terminals, Ctrl+M arrives the same as Enter. Xylem enables
the kitty keyboard protocol when the terminal supports it (kitty, WezTerm,
foot, Ghostty, recent Alacritty, iTerm2), and then Ctrl+M moves the tagged
files as in ZTree. In VTE-based terminals (GNOME Terminal, Ptyxis, Terminator,
Tilix) use **F9 M** (or Alt+M and F6 at the prompt).
