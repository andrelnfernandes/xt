# Salir y cambiar el directorio de la shell

**Q** o F10 sale de Xylem (preguntando antes, a menos que `confirm_quit` esté
desactivado). Si hay un programa ejecutándose en la
[terminal integrada](terminal.md), Xylem pregunta siempre.

## Alt+Q

Un programa no puede cambiar por sí mismo el directorio de la shell que lo
lanzó. Por eso, como en otros gestores de archivos, **Alt+Q** escribe el
directorio actual en un archivo (`--cwd-file`) y una pequeña función de shell
hace el `cd`. Pon esto en tu `~/.bashrc` (o `~/.zshrc`):

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

Con ello, `xy` funciona como antes; solo cuando sales con **Alt+Q** la shell
termina en el directorio en el que estabas. Q y F10 salen sin cambiar nada.

En PowerShell, pon esto en tu `$PROFILE`:

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
