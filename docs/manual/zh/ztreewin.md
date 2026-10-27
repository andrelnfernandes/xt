# 与 ZTreeWin 的比较

以下是基于 ZTreeWin v2.2 官方手册（2011 年的 [ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf)，特别是其中的“Quick Command Table”以及关于配置的第 3.7 节）逐条对比的概览。ZTreeWin 的后续版本可能在某些细节上有所变动。

## 已实现的功能

按需载入的目录树（`+`、`*`）、文件窗口、Filespec、Branch、Showall、标记（T、U、I、Ctrl+T/U）、包含 Ctrl（已标记）与 Alt（已标记且保留路径）版本的复制/移动/删除/重命名、新建目录、删除目录、Prune、权限属性、查看器、编辑、执行 (eXecute)、比较（目录与分支）、排序、Log、分屏窗口（F8 与 Tab）、重新载入（F3）、打开（`O`）、自动刷新、浏览与解压归档文件、提示行历史记录、帮助（F1）以及配置界面（Alt+F10）。

## 简化的功能

| 功能 | 在 ZTreeWin 中 | 在 Xylem 中 |
|---|---|---|
| Filespec | 具有独立主文件名与扩展名规则的名称过滤、日期、大小、属性、反向过滤（Ctrl+I）、标签 | 名称使用通配符 (glob)，日期和大小与 ZTreeWin 一致（支持 K/M/G）；无属性过滤、反向切换或标签 |
| 查看器 | Alpha、Dump、Hex、Junk 与 Wrap 模式；字符集切换；十六进制编辑；书签；自动滚动 | 文本（UTF-8、UTF-16、CP850、Windows-1252 和 CP437，自动识别，按 O 切换）或十六进制模式，具备 ZTreeWin 风格的搜索（F、B、/、\\、Space、+ 和 -）、语法高亮（ZTreeWin 无此功能）、软换行（W）、行号（#）、跳转行（g / :）；最大显示到大小限制（`viewer_max_size`，16 MiB 到 1 GiB）；有意不内置十六进制编辑（可在 [`[editors]`](execute.md) 中为二进制文件配置外部编辑器） |
| Ctrl+S | 文本、十六进制或 Unicode；仅搜索 ZIP 索引 | 文本或十六进制 |
| 属性 | Windows 属性（R、A、S、H） | 八进制 Unix 权限（如 `644`）；不含所有者/组修改。在 Windows 上与 ZTreeWin 一致，为 R、H、S 和 A |
| 排序 | 名称、扩展名、日期、时间、长度、字母、数字、大小、数值、不排序；字符集；分屏下相同/不同项 | 名称、扩展名、日期、大小、升序/降序、路径优先；每个窗口拥有各自独立的排序顺序 |
| 比较 | 相同、独有、更新、更旧、大小、二进制、大小写选项；显示过滤器 | 标记独有、更新、更旧、相同、不同或“独有+更新”；无二进制逐字节比较 |
| Prune | 仅删除子目录或连同当前目录一起删除；仅删除空目录；支持回收站 | 始终永久删除整个目录及其内容；确认方式可配置 |
| 删除 | 可选放入回收站 | 可选放入系统回收站（delete_to_trash，提示时按 T/P 切换，Shift+Del 永久删除）；支持 Oops（O/Ctrl+Z）恢复 |
| Log | 多个已载入盘符、Global 视图、盘符切换 | 每个窗口一个根目录；按 `L` 打开另一个目录 |
| eXecute | 批处理参数 `%1`–`%8`、新窗口运行 (NTW) | 使用 `sh` 运行，支持 `%f`、`%t` 和 `%d`；始终在 Xylem 所在终端运行；另提供内置终端（Alt+X） |
| 历史记录 | 带书签的历史记录窗口 | 在提示行中按 ↑/↓ 调出，每种提示行独立保存，跨会话持久化 |
| 配置 | 7 页配置、逐项颜色编辑器（F4）、Alt-Export | 4 页适用于 Linux 的配置项；按 F4 从 12 种预设主题中选择；可在 `config.toml` 中单独定制颜色 |
| 归档文件 | 调用外部归档工具支持任意格式，创建（F5），解压（Alt-F5） | 内置支持 ZIP/JAR/WAR/APK、7z、TAR（未压缩及 gzip、xz、bzip2、zstd）：打开（Alt+F5）、用复制命令解压、按 F5 创建（ZIP、TAR 与 TAR.GZ） |
| 分屏 | 每侧可选显示统计面板 | 分屏时不显示右侧统计面板 |
| 菜单 | 按住 Ctrl/Alt 键时显示对应菜单 | 在支持 kitty 协议的终端中同样按住即显示；在任何终端中均可按 F9 切换 |

由于属于 Windows 特有概念而未移植的 ZTreeWin 配置项：RSH 属性、目录联接点 (junctions)、OEM/ANSI 编码、EBCDIC、窗口尺寸、快捷方式 (.lnk)。其余未移植选项则依赖于下方尚未实现的功能。

## 按键差异

Xylem 的按键基本遵循 ZTreeWin 的设计。例外情况如下：

| 功能 | ZTreeWin | Xylem |
|---|---|---|
| 帮助 | F1 | F1 或 Alt+H（在 GNOME Terminal 中，F1 会打开终端自身的帮助） |
| 配置 | Alt+F10 | F12 或 Alt+F10（在 GNOME 中，Alt+F10 会最大化窗口而无法传给 Xylem） |
| 书签 | F11 | F11、Alt+B 或单引号 `'`（在 GNOME Terminal 中，F11 会切换全屏） |
| 移动已标记文件 | Ctrl+M | 在支持 kitty 协议的终端中为 Ctrl+M；其他终端中按 `F9` `M`（或按 Alt+M 后按 F6） |
| 目录树中的 `-` | 从内存中卸载该分支 | 仅折叠目录（仍保留在内存中） |
| F2 | （无功能） | 重新读取目录，与 F3 相同 |
| F9 | 应用程序菜单（未实现） | 切换 Ctrl/Alt 菜单 |
| F12 | 宏（未实现） | 打开配置界面 |
| `/` | 与 `?`（统计信息）相同 | 打开 Treespec，与 `\` 相同（`?` 仍用于显示统计信息） |
| 目录树中的 Ctrl+C、Ctrl+M | （无功能） | 复制 / 移动当前目录中已标记的文件 |
| 复制路径到剪贴板 | Ctrl+Insert | Y 或 Ctrl+Insert（Ctrl+Y 复制已标记文件；支持 OSC 52 与系统剪贴板） |

所有按键均可在 `config.toml` 的 `[keys]` 下重新绑定。

## 未实现的功能

- **目录树：** Global (G)、Graft (Alt+G)、Mirror (Alt+M)、从内存中释放/卸载（Alt+R、`-`、Ctrl+Backspace）、隐藏 (Alt+H)、清理空闲空间 Wash (Alt+W)、卷标 Volume (Ctrl+V)、F5/F6（隐藏子目录层级）、按属性标记（Alt+T/U）、编辑模式下的 Treespec（F5、通配符）与特殊文件夹（F6）、带提示符的先行输入搜索（`|`）与活动模式、盘符切换（`<` 与 `>`）。
- **文件：** 在新终端窗口中打开 (Alt+O)、当前文件的批处理 (Alt+B)、JFC（J，比较两个文件的内容）、ZAAP（外部归档工具插件）、创建快捷方式 sHortcut (H)、打印 Print (P)、修改所有者/组 (chown)、Megaview（文件窗口中的 Ctrl+F7）、在两个窗口间合并已标记文件 (Ctrl+F6)、部分取消标记 Partial Untag (Ctrl+F8)。
- **通用：** 宏 (F12)、应用程序菜单 (F9)、退出到临时 shell (Ctrl+Q)、保存与加载已载入目录列表（Zlist，Ctrl/Alt+Z）、按名称与属性着色（ZCOLORS.INI）、鼠标支持、ZTreeWin 命令行选项、视频模式命令（Alt+F6–F9）。
