# Disk usage (Alt+W)

Where the space goes. **Alt+W** logs the whole branch of the current
directory and lists its folders and files by the space they take, largest
first, with a bar and their share of the directory. Enter goes into a
folder; folder by folder, you get down to what fills the disk.

```
       32.9 GiB ████████████████   45%   wsl-ubuntu-backup.tar
       18.9 GiB █████████▏         26%  /projetos
 .      5.5 GiB ██▋               7.6%  /.local
 >    154.6 MiB                      -  /boot
```

- **The bar** is drawn against the largest item of the directory, so the
  differences between the items show, on a track as wide as the largest;
  **the percentage** is the item's share of the directory. The color says how
  much that share is: up to 10% (green, in most themes), up to 40% (amber),
  more (red). Each theme has its own (`bar_low`, `bar_mid` and `bar_high` in
  `[colors]`).
- **Folders and files are mixed**, by size: a single large file (an old
  backup, an ISO image) shows among the folders. Folders start with `/`.
- **The line at the bottom** has the totals of the directory: on the disk,
  apparent, the number of files, and how much they are of the disk's used
  space and of the whole disk. At the top of a disk, it says how much of the
  used space was found: what is missing is in folders that could not be read,
  in the file system's own data, or in deleted files still open.
- **The panel on the right** shows the item under the cursor: the space on
  the disk and apparent, files, date, and its share of the used space and of
  the disk.

## On the disk or apparent

By default, sizes are **the space on the disk**, as `du` counts it: a file
takes whole blocks (4 KiB, usually), so a small file takes more than its
size, and a sparse file (a virtual machine's disk) or a compressed one less.
**A** switches to the **apparent** size, the files' own sizes. The line at
the bottom always shows both.

## What the analysis counts

- **One disk at a time.** Another file system mounted inside the branch
  (another disk, `/boot`, a USB drive, a folder mounted by rclone, `/proc`)
  is not logged or counted: it shows with `>` and the space that file system
  uses, and Enter starts an analysis of it.
- **Hard links count once.** Two names for the same file take its space
  once: the first found counts, the others show with `H` and no size.
- **Hidden files count**, even when the tree does not show them, so that the
  totals add up.
- Links (`@`) count for the link itself; they are not followed.

## Marks

| Mark | Meaning |
|---|---|
| `>` | another disk, not counted |
| `!` | the folder could not be read (usually permission) |
| `.` | something inside could not be read: it may take more |
| `H` | another hard link of a file already counted |
| `@` | a symbolic link or special file |
| `e` | an empty folder |

## Keys

| Key | What it does |
|---|---|
| ↑ ↓ PgUp PgDn Home End | move |
| Enter or → | go into the folder (on `>`: analyze that disk) |
| Backspace or ← | back to the folder above |
| `\` | close the view, with the tree on the folder listed |
| `A` | space on the disk or apparent |
| `C` | show or hide the number of files |
| `S` | sort by size, name, number of files or date |
| F3 | read the folder listed again |
| Esc or Alt+W | close |

Tab, F8, Alt+X (the terminal) and F12 work as usual with the view open.

Analyzing a large disk logs every folder into memory: it takes a while (the
footer shows the progress, and Esc cancels) and some memory, about a few
hundred megabytes for millions of files. As an ordinary user, some folders
of the system cannot be read (`!`), so the totals of `/` are short: the
line at the bottom says how many.
