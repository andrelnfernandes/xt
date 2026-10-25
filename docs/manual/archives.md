# Archives

## Opening an archive (Alt+F5)

Alt+F5 in the file window opens, as a tree and read-only:

- `.zip`, also `.jar`, `.war` and `.apk`;
- `.7z` (encrypted 7z archives are not supported);
- `.tar`, plain or compressed with gzip (`.tar.gz`, `.tgz`), xz (`.tar.xz`,
  `.txz`), bzip2 (`.tar.bz2`, `.tbz2`) or zstd (`.tar.zst`).

Navigation, [Filespec](filespec.md), Branch, Showall, tags,
[View](viewer.md), [Ctrl+S](search.md) and [Compare](compare.md) work inside
it, and [Copy](copy-move.md) (Ctrl+C and Alt+C with paths too) extracts the
files. `Esc` in the tree closes the archive and returns to where you were.

For safety, entries with `..` or an absolute path are ignored, and symbolic
links inside archives are never extracted.

## Creating an archive (F5 / Ctrl+F5)

Creates a `.zip`, `.tar`, `.tar.gz` or `.tgz` (according to the name you type)
with the current file or the tagged files; F2 [picks the folder](picker.md).
The paths inside are relative to the list's root, as with Alt+C: in a Branch,
subfolders are kept. It shows progress and `Esc` cancels; the file is written
to a temporary file and only gets its name at the end. Symbolic links are
stored as links. An existing file is not overwritten.
