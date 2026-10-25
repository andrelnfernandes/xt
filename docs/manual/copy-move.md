# Copy and Move

`C` copies and `M` moves the file under the cursor; Ctrl+C and Ctrl+M the
tagged files; Alt+C and Alt+M the tagged files recreating their paths. The
prompt asks for the destination.

## The destination

A directory (relative to the directory under the tree cursor, absolute, or with
`~`) or, for a single file, a new name. With the screen
[split](split.md), the other window's directory is suggested. **F2** opens a
tree to [pick the folder](picker.md); **↑** brings up earlier destinations.

A destination folder that does not exist yet is created after you confirm.

## With or without paths

Ctrl+C / Ctrl+M put all the tagged files straight into the destination. Alt+C /
Alt+M recreate each file's path there; where that path starts depends on the
mode. For `/home/me/proj/src/ops/mod.rs`, listed from `src/ops` (the list's
root: the current directory, the Branch root, or the root of the tree in
Showall), copied to `dest`:

| Mode | Result | |
|---|---|---|
| Full | `dest/home/me/proj/src/ops/mod.rs` | from the root of the disk, as in ZTreeWin (default) |
| Tree | `dest/src/ops/mod.rs` | from the root of Xylem's tree (here `proj`) |
| Current | `dest/ops/mod.rs` | from the list's root, its name included |
| Relative | `dest/mod.rs` | only what is below the list's root |

At the prompt, **F6** switches between with and without paths and **F7** cycles
the modes; both show in the prompt itself. The starting mode is `copy_paths`
(page 4 of the [configuration](configuration.md)). Inside an archive, paths
start at most at the archive's root.

## When the file already exists

Xylem asks **Yes** / **No** / **All** / **Skip all**, showing the size and date
of the new file and of the existing one, and whether the existing one is newer
or older, larger or smaller. `Esc` cancels the whole operation.

## Safety

The copy is made to a temporary file that only replaces the destination at the
end: cancelling or an error never leaves a half-written file. Permissions and
modification time are preserved, and symbolic links are copied as links.
Moving across disks copies and then deletes.

See also: [tags](tagging.md), [archives](archives.md) (Copy extracts from them).
