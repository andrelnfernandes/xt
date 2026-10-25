# Rename and batch rename

`R` renames the file (in the tree, the directory) under the cursor: type the
new name. A name that already exists is refused.

## Batch rename (Ctrl+R or Alt+R)

Renames all the tagged files with a mask, numbered in the order of the list:

- **Wildcards:** `*.jpg` (changes the extension), `new_*.*`, `*_old.*`, `???.*`
  (keeps that many characters). `*.*` keeps names as they are; `*.` removes
  the extension.
- **Search and replace:** `s/foo/bar/` replaces every `foo` with `bar`;
  `s/foo/bar/i` ignores case.
- **Counters:** `%d`, `%02d`, `%03d`, `%04d`, `<#>`, `<01>`, `<001>`, `<0001>`
  number the files (1, 01, 001...): `photo_<001>.*` gives `photo_001.jpg`,
  `photo_002.jpg`...
- **Case:** `<lower>`, `<upper>`, `<title>` (also as the name or extension
  part: `*.<lower>`), or **F2** at the prompt to cycle lowercase, UPPERCASE and
  Title Case over the result.

If the mask would give two files the same name, nothing is renamed. When a new
name belongs to a file that is still to be renamed, Xylem asks before
overwriting it, as in [Copy and Move](copy-move.md).

See also: [tags](tagging.md).
