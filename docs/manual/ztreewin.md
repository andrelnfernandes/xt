# Comparison with ZTreeWin

A command-by-command survey based on the official ZTreeWin v2.2 manual
([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf), from 2011, in
particular the "Quick Command Table" and section 3.7 on configuration). Newer
versions of ZTreeWin may have changed details.

## Implemented

Tree with on-demand log (`+`, `*`), file window, Filespec, Branch,
Showall, tags (T, U, I, Ctrl+T/U), Copy/Move/Delete/Rename with the
Ctrl (tagged) and Alt (tagged with paths) versions, Make dir, Delete dir, Prune,
Attributes, View, Edit, eXecute, Compare (directory and branch), Sort, Log,
split window (F8 and Tab), relog (F3), Open (`O`), automatic refresh, browsing and
extraction of archives, history in prompts, help (F1) and configuration
screen (Alt+F10).

## Simplified

| Feature | In ZTreeWin | In Xylem |
|---|---|---|
| Filespec | name with its own name and extension rules, date, size, attribute, inverted filespec (Ctrl+I), labels | glob for names, date and size as in ZTreeWin (with K/M/G); no attributes, inversion or labels |
| Viewer | Alpha, Dump, Hex, Junk and Wrap modes; character sets; hex editing; bookmarks; auto-scroll | text (UTF-8) or hex, with ZTreeWin's search (F, B, /, \\, Space, + and -), syntax colors (which ZTreeWin does not have), soft wrap (W), line numbers (#), goto line (g / :); up to 16 MiB; no hex editing, by choice (an editor of your own for binary files in [`[editors]`](execute.md)) |
| Ctrl+S | text, hex or Unicode; searches only the ZIP index | text or hex |
| Attributes | Windows attributes (R, A, S, H) | Unix permissions in octal (`644`); no owner/group. On Windows, R, H, S and A, as in ZTreeWin |
| Sort | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; same/diff in split screen | Name, Ext, Date, Size, ascending/descending, Path first; each window has its own order |
| Compare | options Identical, Unique, Newer, Older, Size, Binary, Case; display filter | tags unique, newer, older, identical, different or unique+newer; no binary comparison |
| Prune | deletes only the subdirectories or the current one too; only empty directories; recycle bin | always deletes the whole directory, permanently; configurable confirmation |
| Delete | optional recycle bin | optional system trash (delete_to_trash, T/P prompt toggle, Shift+Del permanent); Oops (O/Ctrl+Z) to restore |
| Log | several logged drives, Global, drive switching | one root per window; `L` opens another directory |
| eXecute | batch parameters `%1`–`%8`, new window (NTW) | `sh` with `%f`, `%t` and `%d`; always in Xylem's terminal; also a built-in terminal (Alt+X) |
| History | history window with bookmarks | ↑/↓ at the prompt, one history per type, saved between runs |
| Configuration | 7 pages, per-element color editor (F4), Alt-Export | 4 pages with the options that make sense on Linux; F4 chooses among 12 ready-made themes; individual colors in `config.toml` |
| Archives | any format with an external archiver, create (F5), extract (Alt-F5) | ZIP/JAR/WAR/APK, 7z, TAR (plain, gzip, xz, bzip2, zstd): open (Alt+F5), extract with Copy, create with F5 (ZIP, TAR and TAR.GZ) |
| Split screen | optional statistics panels per side | no statistics panel |
| Menus | Ctrl/Alt menu when holding the key | same in terminals with the kitty protocol; F9 in any terminal |

ZTreeWin configuration options that were not brought over because they are
Windows-specific: RSH attributes, junctions, OEM/ANSI, EBCDIC, window
size, shortcuts (.lnk). The rest depend on features not yet built
(below).

## Different keys

The keys follow ZTreeWin's. The exceptions:

| Function | ZTreeWin | Xylem |
|---|---|---|
| Help | F1 | F1 or Alt+H (on GNOME Terminal, F1 opens terminal help) |
| Configuration | Alt+F10 | F12 or Alt+F10 (on GNOME, Alt+F10 maximizes the window and never reaches Xylem) |
| Bookmarks | F11 | F11, Alt+B, or single quote `'` (on GNOME Terminal, F11 toggles fullscreen) |
| Move tagged | Ctrl+M | Ctrl+M in terminals with the kitty protocol; in others, `F9` `M` (or Alt+M and F6) |
| `-` in the tree | unloads the branch from memory | only collapses (stays logged) |
| F2 | (nothing) | rereads the directory, same as F3 |
| F9 | applications menu (not implemented) | Ctrl/Alt menus |
| F12 | macros (not implemented) | configuration |
| `/` | same as `?` (statistics) | Treespec, like `\` (`?` still shows the statistics) |
| Ctrl+C, Ctrl+M in the tree | (nothing) | Copy/Move of the current directory's tagged files |
| Copy path to clipboard | Ctrl+Insert | Y or Ctrl+Insert (Ctrl+Y for tagged; works via OSC 52 and system clipboard) |

All keys can be rebound under `[keys]` in `config.toml`.

## Not implemented

- **Tree:** Global (G), Graft (Alt+G), Mirror (Alt+M), Release/unload from memory (Alt+R, `-`,
  Ctrl+Backspace), Hide (Alt+H), Wash (Alt+W), Volume
  (Ctrl+V), F5/F6 (hide subdirectory levels),
  tag by attribute (Alt+T/U),
  Treespec in edit mode (F5, wildcards) and special folders (F6), type-ahead
  search with prompt (`|`) and active mode, drive switching (`<` and `>`).
- **Files:** Open in a new terminal window (Alt+O), Batch of the current file (Alt+B), JFC (J, compare the contents of two files), ZAAP
  (external archiver), sHortcut (H), Print (P),
  owner/group (chown), Megaview (Ctrl+F7 in the file window), merge tagged files between
  windows (Ctrl+F6), Partial Untag (Ctrl+F8).
- **General:** macros (F12), applications menu (F9), Quit and shell (Ctrl+Q), saving and
  loading the log (Zlist, Ctrl/Alt+Z), colors by name and attribute
  (ZCOLORS.INI), mouse, ZTreeWin command-line options, video
  commands (Alt+F6–F9).
