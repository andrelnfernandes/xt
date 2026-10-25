# Windows

Xylem also builds for Windows (Windows Terminal or the classic console). It
passes the automated tests there, but has not been used day to day yet, so
expect rough edges. What changes:

- **Attributes** (`A`) edits R, H, S and A, as in ZTreeWin: `RA` sets exactly
  those, `+H -A` turns some on or off. The file list shows them as `r-sa`.
- **Hidden** files and directories are the ones with the H attribute (a name
  starting with a dot is an ordinary name).
- **eXecute** and **Edit** go through `cmd`, with names in double quotes. A
  `%NAME%` inside a file name is still expanded by `cmd`.
- The default editor is `notepad`, `O` uses `start`, and Ctrl+B suggests
  `xy-batch.bat` (starting with `@echo off`).
- Deleting a read-only file deletes it, as `rm` does on Linux.
- Copying symbolic links needs Developer Mode or administrator rights.
- The owner display mode (Alt+F) shows `-`.
- Configuration in `%APPDATA%\xylem\config.toml`, history in
  `%LOCALAPPDATA%\xylem\`.
- The built-in terminal (Alt+X) runs `cmd` (`%COMSPEC%`). Entering it still
  takes the shell to the tree's directory, but the tree does not follow the
  shell's `cd`s, and there is no `$XY_TAGGED`.

For Alt+Q in PowerShell, see [quitting](quit.md).
