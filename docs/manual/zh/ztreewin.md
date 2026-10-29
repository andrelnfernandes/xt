# 与 ZTreeWin 的比较

## 首先，致谢

我从 1990 年代起就一直在用各种版本的 XTree，自 2013 年起拥有一份付费的 ZTreeWin 授权。在我看来，ZTreeWin 至今仍是最好的文本模式文件管理器：快速、可预期、完全由键盘操作，并围绕一种其他文件管理器都未能企及的工作方式而构建（载入目录树、在整个分支中标记文件、对已标记的文件执行操作）。Kim Henkel 数十年来一直让 XTree 的工作方式在 Windows 上延续下来，没有这份工作就不会有 Xylem。

Xylem 并不打算取代 ZTreeWin。它之所以存在，是因为我想在 Linux 上拥有同样的工作方式，而 ZTreeWin 无法在 Linux 上原生运行。由于 Rust 让跨平台编译变得容易，Xylem 也能在 Windows 上编译，但这并不是我的目标：在 Windows 上，ZTreeWin 完全值得购买：[ztree.com](https://www.ztree.com/)，以及其用户[论坛](https://groups.io/g/ZTreeWin)。

本页基于 ZTreeWin v2.2 官方手册（2011 年的 [ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf)，特别是其中的“Quick Command Table”以及关于配置的第 3.7 节）逐条比较两者。ZTreeWin 的后续版本可能在某些细节上有所变动；如果这里有内容已经过时，欢迎提交 issue。

## 相同的工作方式

按需载入的目录树（`+`、`*`）、文件窗口、Filespec、Branch、Showall（也可只显示已标记文件，Ctrl+S）、标记（T、U、I、Ctrl+T/U）、包含 Ctrl（已标记）与 Alt（已标记且保留路径）版本的复制/移动/删除/重命名、新建目录、删除目录、Prune、权限属性、查看器、编辑、执行 (eXecute)、比较（目录与分支）、排序、Log、分屏窗口（F8 与 Tab）、重新载入（F3）、打开（`O`）、自动刷新、浏览与解压归档文件、提示行历史记录、书签、帮助（F1）以及配置界面。按键尽可能与 ZTreeWin 保持一致；例外情况见文末的“按键差异”。

## Xylem 尚未具备的功能

- **目录树：** Global (G)、Graft (Alt+G)、Mirror (Alt+M)、从内存中释放/卸载（Alt+R、Ctrl+Backspace）、隐藏 (Alt+H)、清理空闲空间 Wash (Alt+W)、卷标 Volume (Ctrl+V)、F5/F6（隐藏子目录层级）、按属性标记（Alt+T/U）、编辑模式下的 Treespec（F5、通配符）与特殊文件夹（F6）、带提示符的先行输入搜索（`|`）与活动模式、盘符切换（`<` 与 `>`）。
- **文件：** 在新终端窗口中打开 (Alt+O)、当前文件的批处理 (Alt+B)、JFC（J，比较两个文件的内容）、创建快捷方式 sHortcut (H)、打印 Print (P)、修改所有者/组 (chown)、Megaview（文件窗口中的 Ctrl+F7）、在两个窗口间合并已标记文件 (Ctrl+F6)、部分取消标记 Partial Untag (Ctrl+F8)、Showall 以外的 Tags-Only 模式（Ctrl+Enter 与 Ctrl+F4，目录树中的 Ctrl+B）。
- **通用：** 宏 (F12)、应用程序菜单 (F9)、退出到临时 shell (Ctrl+Q)、保存与加载已载入目录列表（Zlist，Ctrl/Alt+Z）、按名称与属性着色（ZCOLORS.INI）、鼠标支持、ZTreeWin 命令行选项、视频模式命令（Alt+F6–F9）。

其中一些可以用其他程序代替；参见下文的“Xylem 交给其他程序的部分”。

## Xylem 更简单的地方

| 功能 | 在 ZTreeWin 中 | 在 Xylem 中 |
|---|---|---|
| Filespec | 具有独立主文件名与扩展名规则的名称过滤、日期、大小、属性、反向过滤（Ctrl+I）、标签 | 名称使用通配符 (glob)，日期和大小与 ZTreeWin 一致（支持 K/M/G）；无属性过滤、反向切换或标签 |
| 查看器 | Alpha、Dump、Hex、Junk 与 Wrap 模式；字符集切换；十六进制编辑；书签；自动滚动 | 文本或十六进制模式，具备 ZTreeWin 风格的搜索（F、B、/、\\、Space、+ 和 -）、软换行（W）、行号（#）、跳转行（g / :）；最大显示到大小限制（`viewer_max_size`，16 MiB 到 1 GiB）；有意不内置十六进制编辑（参见“Xylem 交给其他程序的部分”） |
| Ctrl+S | 文本、十六进制或 Unicode；仅搜索 ZIP 索引 | 文本或十六进制 |
| 属性 | Windows 属性（R、A、S、H） | 八进制 Unix 权限（如 `644`）；不含所有者/组修改。在 Windows 上与 ZTreeWin 一致，为 R、H、S 和 A |
| 排序 | 名称、扩展名、日期、时间、长度、字母、数字、大小、数值、不排序；字符集；分屏下相同/不同项 | 名称、扩展名、日期、大小、升序/降序、路径优先；每个窗口拥有各自独立的排序顺序 |
| 比较 | 相同、独有、更新、更旧、大小、二进制、大小写选项；显示过滤器 | 标记独有、更新、更旧、相同、不同或“独有+更新”；无二进制逐字节比较 |
| Prune | 仅删除子目录或连同当前目录一起删除；仅删除空目录；支持回收站 | 始终永久删除整个目录及其内容；确认方式可配置 |
| Log | 多个已载入盘符、Global 视图、盘符切换 | 每个窗口一个根目录；按 `L` 打开另一个目录 |
| eXecute | 批处理参数 `%1`–`%8`、新窗口运行 (NTW) | 使用 `sh` 运行，支持 `%f`、`%t` 和 `%d`；始终在 Xylem 所在终端运行 |
| 历史记录 | 带书签的历史记录窗口 | 在提示行中按 ↑/↓ 调出，每种提示行独立保存，跨会话持久化 |
| 配置 | 7 页配置、逐项颜色编辑器（F4）、Alt-Export | 4 页适用于 Linux 的配置项；按 F4 从 12 种预设主题中选择；可在 `config.toml` 中单独定制颜色 |
| 分屏 | 每侧可选显示统计面板 | 分屏时不显示右侧统计面板 |
| 菜单 | 按住 Ctrl/Alt 键时显示对应菜单 | 在支持 kitty 协议的终端中同样按住即显示；在任何终端中均可按 F9 切换 |

由于属于 Windows 特有概念而未移植的 ZTreeWin 配置项：RSH 属性、目录联接点 (junctions)、OEM/ANSI 编码、EBCDIC、窗口尺寸、快捷方式 (.lnk)。

## Xylem 更进一步的地方

以下功能在 ZTreeWin v2.2 手册中没有对应：

- **内置终端**（Alt+X）：在使用目录树时 shell 一直保持运行；在 shell 中 `cd`，目录树随之跳转，回到 shell 时它也会跟随目录树。Alt+O 将已标记的文件输入到命令行。参见[内置终端](terminal.md)。
- **通过 SSH 访问远程机器**（`xy 用户@主机:`）：目录树、Branch、查看器和复制都可以在另一台机器上使用，那台机器也有自己的终端。参见[远程机器](remote.md)。
- **Git**（可选）：文件列表中的状态字母、Git 菜单（Ctrl+G）以及查看器中的 diff。参见 [Git](git.md)。
- **磁盘用量**（Alt+W）：将一个分支中的文件夹和文件放在一起，从大到小排列，一直找到占满磁盘的内容；Alt+K 在目录树中显示每个文件夹所占的比例。参见[磁盘用量](usage.md)。
- **读得更多的查看器：** 语法高亮、自动识别文本编码（UTF-8、UTF-16、CP850、Windows-1252、CP437）、直接显示 `.gz`、`.xz`、`.zst` 和 `.bz2` 文件解压后的内容，并将隐藏的文本方向控制字符（Trojan Source）显示出来而不是执行它们。参见[查看器](viewer.md)。
- **无需外部归档工具的归档文件：** ZIP、7z 和 TAR（未压缩及 gzip、xz、bzip2、zstd）可以像文件夹一样打开，并能创建 ZIP 和 TAR，全部由 Xylem 自身完成。参见[归档文件](archives.md)。
- **批量重命名**，支持查找替换（`s/foo/bar/`）、计数器和大小写转换。参见[重命名](rename.md)。
- **Oops：** O（或 Ctrl+Z）将最后放入系统回收站的内容恢复到原处。参见[删除](delete.md)。
- **随处可用的剪贴板：** Y 复制路径，即使通过 SSH 或在 tmux 中也可以（OSC 52）。
- **文件是什么：** 一种显示模式（Alt+F），以 `file` 命令的方式说明每个文件的内容，另一种则显示所有者和权限。
- **外观：** 12 种预设主题和可选的 Nerd Fonts 图标。所有按键都可以在 `config.toml` 的 `[keys]` 中重新绑定。参见[配置](configuration.md)。
- **恶意文件：** 归档文件、文件名和内容都被视为潜在恶意，读取和解压的一切都有限制（参见 README 中的安全声明）。
- **六种语言的帮助**（F1，然后按 L），与本手册及 man 手册页内容相同。

## Xylem 交给其他程序的部分

有些事情 Xylem 有意不自己做：专门的程序做得更好，Xylem 把它们集成进来。

- **十六进制编辑：** ZTreeWin 在自己的查看器中编辑字节。Xylem 没有自己的十六进制编辑器：编写和维护一个需要大量工作而用处不大，而且手工编辑字节很容易损坏文件。取而代之的是，在二进制文件上按 E 会在 Xylem 的框架和配色中打开 `hexedit`、`heh`、`bvi` 或 `dhex`（已安装的那个，或你选择的那个），也可以打开 ImHex 或 GHex 这样的图形编辑器。参见[编辑](execute.md)。
- **编辑文本：** 你自己的编辑器（`$VISUAL`、`$EDITOR` 或在 F12 中设置的编辑器），如有需要还可以为每种文件类型指定不同的编辑器（`[editors]`）。
- **SSH：** 使用系统的 `ssh`，因此你的密钥、agent、`~/.ssh/config` 和跳板机都和在其他地方一样可用。
- **云存储：** 通过 `rclone mount` 使用 Google Drive、S3 等；Xylem 会识别挂载点并相应调整。参见[云存储](cloud.md)。
- **Git：** 直接使用 `git` 程序。
- **打开文件**（O）：桌面环境为每种类型指定的程序（`xdg-open`）。
- **比较两个文件的内容**（ZTreeWin 中的 JFC）：在内置之前，可以标记这两个文件，然后用 eXecute（X）运行 `diff %t`，或者使用终端。
- **打印**（ZTreeWin 中的 P）：用 eXecute 运行 `lp %t`。
- **其他归档格式**（ZTreeWin 的 ZAAP）：使用你自己的归档工具，通过 eXecute 或在终端中运行。

## 按键差异

Xylem 的按键基本遵循 ZTreeWin 的设计。例外情况如下：

| 功能 | ZTreeWin | Xylem |
|---|---|---|
| 帮助 | F1 | F1 或 Alt+H（在 GNOME Terminal 中，F1 会打开终端自身的帮助） |
| 配置 | Alt+F10 | F12 或 Alt+F10（在 GNOME 中，Alt+F10 会最大化窗口而无法传给 Xylem） |
| 书签 | F11 | F11、Alt+B 或单引号 `'`（在 GNOME Terminal 中，F11 会切换全屏） |
| 移动已标记文件 | Ctrl+M | 在支持 kitty 协议的终端中为 Ctrl+M；其他终端中按 `F9` `M`（或按 Alt+M 后按 F6） |
| F2 | （无功能） | 重新读取目录，与 F3 相同 |
| F9 | 应用程序菜单（未实现） | 切换 Ctrl/Alt 菜单 |
| F12 | 宏（未实现） | 打开配置界面 |
| `/` | 与 `?`（统计信息）相同 | 打开 Treespec，与 `\` 相同（`?` 仍用于显示统计信息） |
| 目录树中的 Ctrl+C、Ctrl+M | （无功能） | 复制 / 移动当前目录中已标记的文件 |
| 复制路径到剪贴板 | Ctrl+Insert | Y 或 Ctrl+Insert（Ctrl+Y 复制已标记文件；支持 OSC 52 与系统剪贴板） |

所有按键均可在 `config.toml` 的 `[keys]` 下重新绑定。
