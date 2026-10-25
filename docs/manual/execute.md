# Edit, eXecute and Batch

## Edit (E)

Opens the file in the editor: the one set on page 1 of the
[configuration](configuration.md), or else `$VISUAL`, `$EDITOR`, Debian and
Ubuntu's `editor`, or `vi`.

### Editors by file type

A text editor only mangles an image or a program, so E does not open
**binary files** (those with a null byte near the start, which the viewer
shows in hex): it says so in the footer and suggests `O`, which opens the file
in the system's own program. To edit them, or to give some files an editor of
their own, list them in the `[editors]` table of `config.toml`:

```toml
[editors]
"*.png *.jpg *.jpeg *.gif" = "gimp %f"
"*.md" = "typora"
"*.svg" = ""            # empty: the usual editor, even though it matched
binary = "hexedit"      # any other binary file
```

Each key is a list of name patterns, as in [Filespec](filespec.md) (case does
not matter); each value a command line, with `%f` for the file (or else the
file goes at the end). The first key that matches wins, in the file's order.
`binary` is for the binary files that no pattern took; the other files open
in the usual editor. Commands run the same way as the editor: the terminal is
handed over and Xylem comes back when they end (add `&` for a graphical
program you want to leave running).

Xylem has no hex editor of its own, and that is a decision, not something
missing: it would be a lot of work for little use, and editing bytes by hand
is an easy way to corrupt a file. Whoever needs it can choose a tool made for
that (`hexedit`, `bvi`, `dhex`, `ImHex`...) in `binary`.

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
