# 退出并切换 shell 目录

按 **Q** 或 F10 退出程序（除非关闭了 `confirm_quit`，否则会先询问确认）。如果 [内置终端](terminal.md) 中有程序正在运行，Xylem 始终会先询问。

## Alt+Q

子进程无法直接改变启动它的父 shell 的工作目录。因此，与其他终端文件管理器一样，按 **Alt+Q** 会将当前目录写入一个临时文件（通过 `--cwd-file`），再由外层的 shell 函数读取并执行 `cd`。请将以下函数添加到您的 `~/.bashrc`（或 `~/.zshrc`）中：

```bash
xy() {
    local tmp dir
    tmp=$(mktemp) || return
    command xy --cwd-file "$tmp" "$@"
    dir=$(cat -- "$tmp")
    rm -f -- "$tmp"
    [ -n "$dir" ] && [ -d "$dir" ] && cd -- "$dir"
}
```

配置好后，`xy` 的日常使用完全不变；只有当您按 **Alt+Q** 退出时，外层 shell 才会自动切换到您最后所在的目录。按 Q 和 F10 退出则保持原目录不变。

在 PowerShell 中，请将以下内容添加到您的 `$PROFILE` 中：

```powershell
function xy {
    $tmp = New-TemporaryFile
    xy.exe --cwd-file $tmp @args
    $dir = Get-Content -Raw -Encoding UTF8 $tmp
    Remove-Item $tmp
    if ($dir -and (Test-Path -LiteralPath $dir -PathType Container)) {
        Set-Location -LiteralPath $dir
    }
}
```
