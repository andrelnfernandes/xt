# Quitting and changing the shell's directory

**Q** or F10 quits (asking first, unless `confirm_quit` is off). If a program
is running in the [built-in terminal](terminal.md), Xylem always asks.

## Alt+Q

A program cannot change the directory of the shell that launched it. So, as in
other file managers, **Alt+Q** writes the current directory to a file
(`--cwd-file`) and a small shell function does the `cd`. Put this in your
`~/.bashrc` (or `~/.zshrc`):

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

With it, `xy` works as before; only when you quit with **Alt+Q** does the shell
end up in the directory you were in. Q and F10 quit without changing anything.

In PowerShell, put this in your `$PROFILE`:

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
