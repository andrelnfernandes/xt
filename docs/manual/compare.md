# Compare

`C` in the tree, with the screen [split](split.md), compares the files of the
current directory with those of the other window's directory and tags, as you
choose:

- **U**nique: not present on the other side;
- **N**ewer and **O**lder;
- **I**dentical: same size and date;
- **D**ifferent;
- **A**ll new: unique + newer, what is missing to bring the other side up to
  date.

**B** (or Alt+C in the tree, as in ZTreeWin) switches the scope to the whole
branch: Xylem logs the branch, compares following the relative paths
(`src/a.rs` ↔ `<other>/src/a.rs`) and opens the Branch with the tagged files.

Dates up to 2 seconds apart count as equal (FAT and exFAT keep times in steps
of 2 seconds); `compare_ignore_1h` also ignores differences of exactly one hour
(daylight saving time). Both are on page 4 of the
[configuration](configuration.md).

To synchronize: `C` `B` `A` and then Alt+C ([copy with paths](copy-move.md)),
which already suggests the other window as the destination.
