# Information

- **Info (Alt+I):** type, size, permissions, owner and group, dates
  (modification, access and change), inode, links and device of the file or
  directory; for a link, where it points; for a directory, also what is logged
  in it and in its branch.
- **Avail (`A` in the tree):** the mounted file systems with total, used, free
  and % used, like `df`.
- **Statistics (`?`):** what the window has logged, the list, the tagged files
  and the disk (mount point, free and used); below it, the machine (system and
  kernel, CPU, uptime, load, memory and swap, shell, terminal), the network
  (addresses of each interface, gateway and DNS) and Xylem itself (version,
  PID, memory, executable and configuration file).

## Directory sizes (Alt+K)

Shows to the right of each directory in the tree the **S**ize (total in K),
the **C**ount of files or the **P**ercent; **N**one turns it off. Before
choosing, **A**ll or **T**agged count all files or only the tagged ones, and
**B**ranch or **D**irectory add up the whole branch or just the directory.
Directories not yet logged show `?`. Each window has its own setting.

**Percent** shows how much of the directory above each branch takes, and a
bar: the largest directory among its siblings fills it, and the others are
drawn in proportion, so the one taking the space stands out. It always adds
up whole branches. The files of the directory above count in its total, so
the shares of its subdirectories may add up to less than 100%. To find what
fills a disk, log the branch with `*` first: a directory not yet logged
counts as empty.
