# Edit, eXecute and Batch

## Edit (E)

Opens the file in the editor: the one set on page 1 of the
[configuration](configuration.md), or else `$VISUAL`, `$EDITOR`, Debian and
Ubuntu's `editor`, or `vi`.

By default, the editor opens **inside Xylem's frame** (`editor_in_frame = true`
on page 1 of **F12**) and uses **Xylem's color theme** (`editor_theme = true`),
including syntax and UI colors matching the active theme when the editor is
`vi`, `vim` or `nvim`. Set `editor_in_frame = false` to hand the whole terminal
over to the editor instead, or `editor_theme = false` to keep the editor's own
color scheme.

### Binary files and editors by file type

A text editor only mangles an image or a program, so `E` never opens **binary
files** (those with a null byte near the start, which the viewer shows in hex)
in the text editor.

Instead, when **Edit binary files with E** is enabled (`edit_binary = true`, key
**I** on page 1 of **F12**), `E` opens binary files in a hex/binary editor:

1. A matching pattern in `[editors]` (if any), or
2. The **Binary editor** set on page 1 of **F12** (`binary_editor`, key **J**) or
   `binary` in `[editors]`, or
3. If left empty, the first terminal hex editor found on `PATH`:
   - **Linux**: `hexedit`, `heh`, `bvi`, `dhex`
   - **Windows**: `heh` (`winget install ndd7xv.heh`), `hexedit`, `bvi`

Terminal hex editors run the same way as the text editor: **inside Xylem's
frame** by default (`editor_in_frame = true`) and with **Xylem's color theme**
(`editor_theme = true`). For a graphical hex editor (`ImHex`, `GHex`, `Okteta`
on Linux; `ImHex`, `HxD` on Windows) that you want to leave running in its own
window, add `&` at the end of the command (for example, `imhex %f &` or
`ghex %f &`): Xylem launches it directly without opening the terminal frame.

If you prefer `E` not to open binary files at all (and only suggest `O` to open
them in the system's program), set **Edit binary files with E** to `no`
(`edit_binary = false` on page 1 of **F12**).

To give specific file types an editor of their own, list them in the `[editors]`
table of `config.toml`:

```toml
[editors]
"*.png *.jpg *.jpeg *.gif" = "gimp %f &"
"*.md" = "typora %f &"
"*.svg" = ""            # empty: the usual editor, even though it matched
binary = "hexedit"      # fallback for any other binary file
```

Each key is a list of name patterns, as in [Filespec](filespec.md) (case does
not matter); each value a command line, with `%f` for the file (or else the
file goes at the end). The first key that matches wins, in the file's order.
`binary` is for the binary files that no pattern took when `binary_editor` is
empty; the other files open in the usual text editor.

Xylem has no built-in hex editor of its own, and that is a deliberate design
choice: writing and maintaining one would be a lot of work for little use, and
editing raw bytes by hand is an easy way to corrupt a file. Instead, Xylem
integrates dedicated hex editors right inside its frame and color theme — and
lets you turn binary editing off with a single option in **F12** if you do not
want it.

## eXecute (X)

Asks for a command line and runs it with `sh` in the current directory. In it:

| | |
|---|---|
| `%f` | the file under the cursor |
| `%t` | the tagged files |
| `%d` | the current directory |
| `%%` | a literal `%` |

All of them come out already quoted: `tar czf backup.tgz %t`. At the end,
Xylem waits for Enter so you can read the output, and the open directories are
reread. A command starting with `@` is not kept in the history (with
`history_skip_at`).

For long sessions, the [built-in terminal](terminal.md) (Alt+X) is often
handier.

## Batch (Ctrl+B)

Asks for the name of the file to create (F2 [picks the folder](picker.md)) and
then a line mask, and writes one line per tagged file. Parameters:

| | |
|---|---|
| `%f` | path |
| `%d` | directory |
| `%n` | name |
| `%b` | name without extension |
| `%e` | extension |
| `%s` | size |

In lowercase they come out quoted, safe for the shell (`convert %f %b.png`); in
uppercase, as plain text, for lists (`%N;%S`). A `.sh` file gets `#!/bin/sh`
and execute permission (run it with `X`). An existing file is not overwritten.

## History

At every prompt, ↑ and ↓ bring up earlier entries. Each kind of prompt
(Filespec, destination, Log, eXecute...) has its own history, saved in
`~/.local/state/xylem/history.toml`, readable only by you, since it may contain
typed commands.
