# Permissions and dates

## Permissions (A)

`A` changes the permissions of the file under the cursor (Ctrl+A: of the
tagged files), in octal: `644`, `755`. Alt+A works like Ctrl+A.

On Windows, `A` edits the attributes R, H, S and A, as in ZTreeWin: `RA` sets
exactly those, `+H -A` turns some on or off. The file list shows them as
`r-sa`. See [Windows](windows.md).

## New date (N)

`N` changes the modification date of the file or directory (Ctrl+N: of the
tagged files). The prompt comes filled with the current date in your format;
it also accepts `now` and `today`. With only the date (no `HH:MM`), each file
keeps its own time of day. Symbolic links are skipped (the date would change
that of the file they point to).

The date format is set on page 1 of the [configuration](configuration.md).
