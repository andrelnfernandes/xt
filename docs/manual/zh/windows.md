# Windows

Xylem 也可以在 Windows（Windows Terminal 或经典控制台）上编译运行。它在 Windows 上通过了自动化测试，但尚未经过大量日常使用检验，因此可能还有一些细节不够完善。主要差异如下：

- **属性**（`A`）像在 ZTreeWin 中一样编辑 R、H、S 和 A 属性：`RA` 精确设置为这些属性，`+H -A` 单独开启或关闭某些属性。文件列表中会将它们显示为 `r-sa`。
- **隐藏** 文件和目录是指带有 H 属性的条目（以点 `.` 开头的文件名在 Windows 上视为普通名称）。
- **执行 (eXecute)** 和 **编辑 (Edit)** 通过 `cmd` 运行，文件名会用双引号括起。文件名中包含的 `%NAME%` 仍会被 `cmd` 作为环境变量展开。
- 默认编辑器为 `notepad`，`O` 使用 `start` 打开文件，Ctrl+B 默认建议生成 `xy-batch.bat`（以 `@echo off` 开头）。
- 删除只读文件时会直接删除，行为与 Linux 上的 `rm` 一致。
- 复制符号链接需要开启“开发者模式”或具备管理员权限。
- 所有者显示模式（Alt+F）显示为 `-`。
- 配置文件保存在 `%APPDATA%\xylem\config.toml`，历史记录保存在 `%LOCALAPPDATA%\xylem\`。
- 内置终端（Alt+X）运行 `cmd`（`%COMSPEC%`）。进入终端时仍会将 shell 切换到目录树所在的目录，但目录树不会跟随 shell 的 `cd` 移动，也没有 `$XY_TAGGED` 变量。

关于在 PowerShell 中使用 Alt+Q，请参阅 [退出](quit.md)。
