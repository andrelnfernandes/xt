# Duplicates (Alt+Y)

Files with the same contents, whatever their names and folders, and the space
the copies waste. **Alt+Y** looks in the branch of the current directory; **D**
in the Usage view looks in the folder listed there.

```
── 3 × 1.2 GiB (2.4 GiB to free) ──────────────────────
     1.2 GiB  2023-08-14  Backups/phone-2023.zip
 T   1.2 GiB  2023-08-14  Downloads/phone-2023.zip
 T   1.2 GiB  2023-08-14  Old/phone-2023 (1).zip
── 2 × 850.0 MiB (850.0 MiB to free) ──────────────────
     850.0 MiB  2021-12-25  Videos/christmas.mp4
 T   850.0 MiB  2022-01-03  WhatsApp/VID-20211225.mp4
```

What frees the most comes first. Nothing is tagged until you say so, and one
copy of each group always stays: Xylem does not let you tag the last one.

## Choosing what goes

| Key | What it does |
|---|---|
| ↑ ↓ PgUp PgDn Home End | move |
| `T` or Space | tag or untag the copy |
| `K` | keep this copy: tags the others of the group |
| `A` | in every group, tag all but one; `A` again changes which stays: the oldest, the newest, the shortest path |
| `F` | in every group with a copy in this copy's folder, keep that one |
| `U` | untag all |
| `D`, Del or Ctrl+D | delete the tagged ones, as the Delete command does (to the trash if `delete_to_trash` is on) |
| `M` or Ctrl+M | move the tagged files |
| `G` | files or whole folders |
| `V` or Enter | view the file |
| `E` | write the list to a file (never over one that exists): size, date, tag and path, a blank line between groups |
| `\` | close the view, with the tree on that folder |
| F3 | read the folders again and compare again |
| Esc or Alt+Y | close |

Before deleting or moving, each group is looked at again: if a copy changed
since the analysis, or the one that was to stay is gone, nothing of that group
is touched, and the footer says so.

## Whole folders

Backups repeated whole take the most space, and a long list of files hides
that. **G** shows the folders with exactly the same contents: the same names,
the same contents of each file, large or small, and the same links pointing to
the same place. Only the topmost ones are listed, not the folders inside them.

A folder is never called a copy when something in it was not compared: a folder
that could not be read, another disk mounted inside, a special file, or a
folder that is ignored (below). Two clones of a repository have the same files,
but their `.git` may hold different work.

## How files are compared

Sizes come from memory. Only files of a size shared with another are read:
first both ends, then the middle, part by part, and a group is split as soon as
its files differ, so different files are not read to the end. The comparison is
by SHA-256. It takes place in the background: the footer shows the progress and
Esc cancels.

Left out of the list:

- files smaller than `duplicates_min_size` (1 KiB, by default) and empty ones;
- links and special files, which are not followed;
- other hard links of the same file: they already share the space, and deleting
  one frees nothing;
- other disks mounted inside the branch;
- folders named in `duplicates_ignore` (`.git` and `node_modules`, by default).

A file that changed since the tree was read, or that cannot be read, is not
compared; the line at the bottom says how many. F3 reads everything again.

On a network folder (NFS, SMB, sshfs...), the total to read is asked first. On a
remote machine or inside an archive, Duplicates is not available yet.

## In the cloud (rclone)

In a folder mounted by `rclone mount`, reading a file means downloading it. So
Xylem asks rclone for the hashes the service already keeps (`rclone lsjson
--hash`), and nothing is downloaded: a whole Drive takes minutes. It uses the
strongest hash the service gives (SHA-256, SHA-1, MD5...), together with the
size, and rclone must be in the PATH. If it is not, or the remote gives no
hashes (an encrypted one, say), Xylem says why and asks before reading the files
through the mount, with the total it would download.

Whole folders are compared on local disks only.

On Google Drive:

- A file in two folders at once, or a shortcut, is one file: it is never offered
  against itself.
- Google Docs, Sheets and Slides have no hash: the line at the bottom says how
  many were left out.
- What others shared with you shows with `s`, and tagging all but one leaves it
  alone: deleting it frees nothing of your storage.
- Two files with one name in a folder cannot be told apart through the mount;
  the line at the bottom suggests `rclone dedupe`.
- Deleting sends to the Drive's trash, which still counts until it is emptied.
  Xylem does not empty it, because that takes everything in it; it shows the
  command: `rclone cleanup gdrive:`.

A hash from the service is taken on trust. MD5 can be forged on purpose: for
files others could have put there, compare before deleting.
