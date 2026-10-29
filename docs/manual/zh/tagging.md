# 标记与批量操作

已标记的文件（标有 `■`）是批量操作的对象：操作作用于 **当前列表中** 已标记的文件（当前目录，或已打开的 Branch 或 Showall）。

## 标记文件

| 按键 | 目录树 | 文件窗口 |
|---|---|---|
| `T` / `U` | 标记 / 取消标记该目录中的文件 | 标记 / 取消标记当前文件并下移 |
| Ctrl+T / Ctrl+U | 标记 / 取消标记所有已载入且匹配 Filespec 的文件 | 标记 / 取消标记整个列表 |
| Ctrl+S | 只显示已标记文件的 Showall | |
| `I` | 反选该目录的标记 | 反选列表的标记 |
| Ctrl+F7 / F8 / F9 | 标记 / 取消标记 / 反选该分支中已载入的文件 | |

## 对已标记文件的操作

| 按键 | 功能 |
|---|---|
| Ctrl+C / Ctrl+M | 将它们 [复制 / 移动](copy-move.md) 到目标位置 |
| Alt+C / Alt+M | 复制 / 移动它们，并 **重建其目录路径** |
| Ctrl+D | [删除](delete.md) 它们 |
| Ctrl+A | 修改它们的 [权限](attributes.md) |
| Ctrl+N | 修改它们的 [日期](attributes.md) |
| Ctrl+R | 使用掩码批量 [重命名](rename.md) |
| Ctrl+B | [批处理](execute.md)：生成每个文件占一行的脚本或列表 |
| Ctrl+F5 | 将它们打包到 [归档文件](archives.md) 中 |
| Ctrl+S / Ctrl+V | [在其中搜索文本 / 逐个查看](search.md) |
| Ctrl+O | 用默认程序打开它们 |
| Ctrl+Y | 将它们的路径复制到剪贴板 |
| Alt+O | 将它们输入到 [内置终端](terminal.md) |

在 eXecute（执行）中，`%t` 代表所有已标记的文件（见 [执行](execute.md)）。

## Ctrl 与 Alt 菜单 (F9)

与 ZTree 一样，底部状态栏平时只显示单键命令。按 **F9** 会依次切换到 Ctrl 菜单、Alt 菜单，再回到普通菜单。当其中一个菜单打开时，接下来按下的字母将视为 Ctrl+字母 或 Alt+字母（如 `F9` `M` = Ctrl+M）；按 `Esc` 取消。在支持 kitty 键盘协议的终端中，只需 **按住 Ctrl 或 Alt** 即可直接显示对应菜单。

**Ctrl+M：** 在大多数终端中，Ctrl+M 发送的信号与 Enter 相同。当终端支持 kitty 键盘协议时（如 kitty、WezTerm、foot、Ghostty、新版 Alacritty、iTerm2），Xylem 会启用该协议，此时 Ctrl+M 可像在 ZTree 中一样移动已标记文件。在基于 VTE 的终端（GNOME Terminal、Ptyxis、Terminator、Tilix）中，请使用 **F9 M**（或按 Alt+M 后在提示行中按 F6）。
