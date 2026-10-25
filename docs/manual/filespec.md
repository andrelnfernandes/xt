# Filespec: the file filter

`F` sets which files show in the file window (each window has its own). The
current filespec appears in the window's title. Specifications are separated
by spaces, `;` or `,`, of three kinds, as in ZTreeWin:

- **name:** a glob (`*.rs`, `a?c*`, `[abc]*`), case-insensitive; in quotes, a
  name can contain spaces (`"my file*"`);
- **date:** `<`, `=`, `>`, `<=`, `>=` or `<>` and a date in the order of the
  configured format, with `/`, `.` or `-` and a 2- or 4-digit year
  (`>01/05/2026`; `2026-05-01` always works), or `TODAY` and `TODAY-n`
  (`>TODAY-7`: the last 7 days);
- **size:** the same with an `s` after the operator (`>=s200000`); `K`, `M`
  and `G` also work (`>s100M`).

A leading `-` negates (`-*.bak`; `-<s1K` is the same as `>=s1K`). Within each
kind, positive specifications are ORed together, except `>a` immediately
followed by `<b`, which is the range between the two
(`>=01/05/2026,<=31/05/2026`: May); `<>` excludes. Between kinds, AND applies:
`*.txt,=TODAY,<s1000` are the `.txt` files from today smaller than 1000 bytes.

`*` (or an empty prompt) shows everything. **↑** brings up earlier filespecs.

See also: [the file window](files.md), [tags](tagging.md) (Ctrl+T in the tree
tags what matches the filespec).
