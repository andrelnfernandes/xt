# Xylem 手册

Xylem 是一款受 XTree 与 ZTreeWin 启发的文本模式文件管理器：上方是目录树，下方是当前目录中的文件，右侧是统计信息。它会将目录载入（读取）到内存中，因此您可以在整个分支上标记文件，并一次性完成复制、移动、删除、比较或重命名。

**F1**（若终端占用了 F1，也可按 **Alt+H**）会打开与当前操作相关的帮助：在提示行中，打开该提示的帮助；在目录树或文件窗口中，打开该窗口的帮助。在帮助中，**Tab** 与 **Shift+Tab** 选择链接，**Enter** 跳转，**Backspace** 返回，**I** 显示本目录，**K** 显示当前生效的按键，**L** 切换语言，**Esc** 关闭。

## 窗口与视图

- [目录树窗口](tree.md)：移动、载入、Branch 与 Showall
- [文件窗口](files.md)：显示模式、Autoview、打开文件
- [分屏](split.md)：并排显示两个窗口
- [查看器](viewer.md)：文本与十六进制、搜索、语法高亮
- [搜索已标记的文件](search.md)：Ctrl+S 与 Ctrl+V
- [信息](info.md)：Info、Avail、统计信息与目录大小
- [磁盘用量](usage.md)：查看空间去向（Alt+W）

## 文件操作

- [标记与批量操作](tagging.md)
- [复制与移动](copy-move.md)、[选择文件夹](picker.md)
- [删除、回收站与 Prune](delete.md)
- [重命名与批量重命名](rename.md)
- [权限与日期](attributes.md)
- [Filespec：文件过滤器](filespec.md)
- [Treespec 与 Log](treespec.md)
- [比较](compare.md) 与 [排序](sort.md)
- [编辑、执行与批处理](execute.md)
- [归档文件](archives.md)：ZIP、7z 与 TAR

## 超越 XTree

- [内置终端](terminal.md)
- [远程主机 (SSH)](remote.md)：浏览另一台机器及其终端
- [云存储 (rclone)](cloud.md)：Google Drive、S3 等
- [Git](git.md)
- [书签](bookmarks.md)

## 设置

- [配置](configuration.md)：F12 界面、主题、按键
- [退出并切换 shell 目录](quit.md)
- [Windows](windows.md)
- [与 ZTreeWin 的比较](ztreewin.md)
- 当前生效的按键：在帮助中按 **K**。
