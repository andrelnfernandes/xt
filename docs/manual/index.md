# Xylem manual

Xylem is a text-mode file manager inspired by XTree and ZTreeWin: a directory
tree on top, the files of the current directory below, and statistics on the
right. It logs (reads) directories into memory, so you can tag files across a
whole branch and copy, move, delete, compare or rename them in one go.

Xylem treats untrusted files as hostile: it should never crash, hang, run
code, write outside the destination or exhaust memory because of what a file
contains or how it is named. Nevertheless, bugs happen. See the security
statement in the README for the full list of safeguards and recommendations.

**F1** (or **Alt+H**, for terminals that keep F1 for themselves) opens help
about what you are doing: inside a prompt, the help for that prompt; in the
tree or the file window, the help for that window. In the help, **Tab** and
**Shift+Tab** pick a link, **Enter** follows it, **Backspace** goes back,
**I** shows this index, **K** the keys in effect, **L** switches the language
and **Esc** closes.

## Windows and views

- [The tree window](tree.md): moving around, logging, Branch and Showall
- [The file window](files.md): display modes, Autoview, opening files
- [Split screen](split.md): two windows side by side
- [The viewer](viewer.md): text and hex, search, syntax colors
- [Searching tagged files](search.md): Ctrl+S and Ctrl+V
- [Information](info.md): Info, Avail, statistics and directory sizes
- [Disk usage](usage.md): where the space goes (Alt+W)

## Working with files

- [Tags and group operations](tagging.md)
- [Copy and Move](copy-move.md), [picking a folder](picker.md)
- [Delete, trash and Prune](delete.md)
- [Rename and batch rename](rename.md)
- [Permissions and dates](attributes.md)
- [Filespec: the file filter](filespec.md)
- [Treespec and Log](treespec.md)
- [Compare](compare.md) and [Sort](sort.md)
- [Edit, eXecute and Batch](execute.md)
- [Archives](archives.md): ZIP, 7z and TAR

## Beyond XTree

- [The built-in terminal](terminal.md)
- [Remote machines (SSH)](remote.md): browse another machine, its terminal
- [Cloud storage (rclone)](cloud.md): Google Drive, S3 and others
- [Git](git.md)
- [Bookmarks](bookmarks.md)

## Setting up

- [Configuration](configuration.md): the F12 screen, themes, keys
- [Quitting and changing the shell's directory](quit.md)
- [Windows](windows.md)
- [Comparison with ZTreeWin](ztreewin.md)
- The keys in effect: press **K** in the help.
