# 内置终端 (Alt+X)

这是 XTree 与 ZTreeWin 中没有的功能：您的 shell（`$SHELL`）直接运行在 Xylem 屏幕的下半部分，上方依然可以看到目录树和文件列表。

- **Alt+X** 打开终端，再次按下返回 Xylem。当终端隐藏时，shell 仍在后台继续运行，保留其命令历史和屏幕内容；输入 `exit` 可关闭该 shell。
- **Alt+Z** 以全屏模式打开终端；在终端内部按 Alt+Z 可在全屏与半屏面板之间切换。之后按 Alt+X 会按上次的模式重新打开它。
- **目录树跟随 shell：** 在终端中执行 `cd` 会将目录树光标同步移动到同一目录（仅限 Linux）。
- **shell 跟随目录树：** 当您从另一个目录进入终端时，shell 会自动切换到该目录。这仅在 shell 处于提示符等待输入时发生；如果终端内正在运行程序（如 vim、less 等），则不会自动输入任何内容。
- **`$XY_TAGGED`** 保存了已标记的文件，会在您进入终端时自动设置。它是一个数组：在 bash 中写为 `"${XY_TAGGED[@]}"`（如 `cp "${XY_TAGGED[@]}" ~/backup`）；在 zsh 和 fish 中写为 `$XY_TAGGED`。对于其他 shell（如 sh、dash），则仅自动切换目录（`cd`）。
- **Alt+O** 将已标记的文件路径输入到终端光标处，路径会自动加上引号并转换为相对于当前 shell 目录的相对路径（如 `wc -l 'a.txt' 'b c.txt'`）。在目录树或文件窗口中按 Alt+O 会直接打开终端并输入这些路径。
- **Shift+PgUp/PgDn** 向上 / 向下翻阅滚出屏幕的历史输出。
- 离开终端返回 Xylem 时，已打开的目录会自动重新读取。
- **退出 Xylem 会结束该 shell** 及其内部运行的所有程序。如果终端内仍有程序在运行（例如 `vim`），即使关闭了 `confirm_quit`，按 Q 或 Alt+Q 退出时也会先询问确认（Linux 与 macOS）。
- 占用整个屏幕的程序（如 vim、htop、less）也能在内置终端中正常运行；此时除了 Alt+X、Alt+Z 和 Alt+O 之外，所有按键都会直接传递给该程序。因此，在终端内部按 F1 和 Alt+H 不会打开本帮助。

## 独立的历史记录

在内置终端中输入的命令会保存在独立的 shell 历史记录文件中，与您日常使用的其他终端互不干扰：通过设置 `HISTFILE` 保存在 `~/.local/state/xylem/bash_history`（或 `zsh_history` 等）中；在 fish 中则使用 `xylem` 会话。如果您的 shell 配置文件无条件覆盖了 `HISTFILE`，则以您的配置为准。可通过 F12（第 3 页）或设置 `terminal_history = false` 关闭此特性。

## 按键与颜色

可在 `[keys.any]` 中修改相关按键（`terminal`、`terminal_full`、`terminal_tagged`）：例如在某些终端模拟器中，Alt+T 会被拦截用于打开“终端”菜单。颜色可通过 F12（第 2 页）或 `terminal_theme` 选择：`native`（使用运行 Xylem 的宿主终端颜色）、`xylem`（使用 Xylem 当前主题的背景与文字颜色）、`solarized`、`gruvbox`、`nord`、`dracula` 或 `one_dark`。

为了在进入终端时同步目录和变量，Xylem 会在自己的缓存文件夹（`~/.cache/xylem/`，仅对您本人可读）中写入一个小脚本，并在提示符处自动输入 ` . script`；开头的空格可避免该命令被记录到 bash（`ignorespace`）和 zsh（`HIST_IGNORE_SPACE`）的历史记录中。在内置终端内部会设置环境变量 `XY_TERMINAL=1`，供有需要的提示符或脚本判断；[远程主机](remote.md) 上的终端同样会设置该变量（例如可用于防止远程的 `~/.bashrc` 自动启动 tmux）。
