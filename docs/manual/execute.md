# Edit, eXecute and Batch

## Edit (E)

Opens the file in the editor: the one set on page 1 of the
[configuration](configuration.md), or else `$VISUAL`, `$EDITOR`, Debian and
Ubuntu's `editor`, or `vi`.

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
