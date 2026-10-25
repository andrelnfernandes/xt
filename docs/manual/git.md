# Git

Git is not in XTree or ZTreeWin, so it starts off: turn it on with F12 (page 1,
"Git commands") or `git = true` under `[options]` in the
[configuration](configuration.md). It needs the `git` program.

## The Git menu (Ctrl+G)

Ctrl+G opens the Git menu in the footer; the next letter chooses:

- **S**tatus, **L**og, **D**iff (against HEAD: staged and unstaged together,
  with colors) and **B**lame open in the [viewer](viewer.md), with search;
- **A**dd and **U**nstage;
- **C**ommit opens Git's own editor, as `git commit` does.

The commands act on the tagged files of the list, otherwise on the current
file; in the tree, on the current directory. Status always covers the whole
repository. For push, pull and other commands that use the network, use
[eXecute](execute.md) (`X`), e.g. `git push`.

## Status in the file list

The file list shows each file's status before its name, with the two letters
of `git status --short`: `M ` staged, ` M` changed but not staged, `A `
added, `??` untracked, `R ` renamed. Unchanged files show blanks. The status is
read again after Xylem's own operations, after F3, and every 10 seconds (to
catch commits made in another terminal).

## Diff in the viewer

In the viewer, **D** shows what changed in the file since the last commit (its
`git diff HEAD`, with colors) and **D** again goes back to the file, at the
same place. "Diff" appears in the footer only for files with changes; **E**
still edits the file, and the diff is updated on return.
