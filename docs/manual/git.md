# Git

Git is not in XTree or ZTreeWin, so it starts off: turn it on with F12 (page 1,
"Git commands") or `git = true` under `[options]` in the
[configuration](configuration.md). It needs the `git` program.

## The Git menu (Ctrl+G)

Ctrl+G opens the Git menu in the footer; the next letter chooses:

- **S**tatus, **L**og, **D**iff (against HEAD: staged and unstaged together,
  with colors) and **B**lame open in the [viewer](viewer.md), with search;
- **G**raph: `git log --graph --decorate --oneline --all`,
  the last 1000 commits of every branch, in Git's own colors (one per branch
  line), also in the viewer;
- **A**dd and **U**nstage;
- **C**ommit opens Git's own editor, as `git commit` does.

The commands act on the tagged files of the list, otherwise on the current
file; in the tree, on the current directory. Status and Graph always cover the
whole repository. For push, pull and other commands that use the network, use
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

## Repositories you did not create

A repository's `.git` can name commands for Git to run, and a downloaded or extracted folder comes with its `.git` (a `git clone` does not bring it). So the file markers and Status, Log, Graph, Diff and Blame run none of them: no hooks, `core.fsmonitor`, external diffs, text conversions or signature checks. What this costs:

- A repository that defines its own filters, as `git-crypt` does, gets no markers, Status, Diff or Blame here, with a message saying why. Log and Graph work. Filters in your global configuration, such as Git LFS, are trusted.
- A submodule with changed files but no new commit does not show as changed.

Add, Unstage and Commit are things you ask for, and follow the repository's configuration: they can run what it names.

### Trusted folders

Nothing is trusted by default: every repository is treated as hostile. To let the repositories of a folder work with all of their configuration, such as `git-crypt` or a diff tool of your own, add the folder under **Trusted Git folders** (**K** on page 1 of **F12**), with `;` between folders:

```
~/projects; /work/code
```

In the file, it is a list: `git_trusted = ["~/projects", "/work/code"]`. A folder is a full path or one under `~`, and it covers everything below it. A link pointing out of it does not carry the trust along.

**Danger:** inside a trusted folder, just walking into a repository lets Git run what it names, with your permissions. Never extract or download anything from others into one, and do not trust `~` or `/`, which hold what you download.
