# Delete, trash and Prune

## Delete

`D` or Del deletes the file under the cursor (in the tree, the directory, only
if empty); Ctrl+D the tagged files. Xylem always asks first.

With `delete_to_trash = true` in the [configuration](configuration.md), files
and empty directories go to the system trash (FreeDesktop Trash on Linux, the
Recycle Bin on Windows) instead of being removed. At the question, **T** sends
them to the trash and **P** deletes permanently, whatever the setting.
**Shift+Del** always deletes permanently.

## Oops

**Oops** (`O` in the tree, or Ctrl+Z in both windows) restores what was last
sent to the trash to where it was. With nothing trashed in this session, it
offers the most recent item in the system trash.

## Prune (Alt+P in the tree)

Deletes the directory with everything in it. Xylem logs the whole branch,
shows how many files, directories and bytes will go, and asks you to **type
PRUNE** (or the directory's name, or Y, depending on `prune_confirmation`). It
deletes the files and then the directories, from the deepest up, with progress
and `Esc` to cancel. Symbolic links are removed without following them, and
**other mount points inside the branch are skipped** (a USB drive mounted in
there is not deleted). The tree's root cannot be pruned. Prune always deletes
permanently.
