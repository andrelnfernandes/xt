# Beenden und Verzeichnis der Shell wechseln

**Q** oder F10 beendet das Programm (nach einer Sicherheitsabfrage, sofern
`confirm_quit` nicht ausgeschaltet ist). Falls im [integrierten
Terminal](terminal.md) noch ein Programm läuft, fragt Xylem immer nach.

## Alt+Q

Ein Prozess kann das Arbeitsverzeichnis der Shell, die ihn gestartet hat, nicht
von sich aus ändern. Wie bei anderen Dateimanagern schreibt **Alt+Q** daher das
aktuelle Verzeichnis in eine Datei (`--cwd-file`), und eine kleine
Shell-Funktion führt anschließend das `cd` aus. Fügen Sie Folgendes in Ihre
`~/.bashrc` (oder `~/.zshrc`) ein:

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

Damit funktioniert `xy` wie gewohnt; nur wenn Sie mit **Alt+Q** beenden, landet
die Shell in dem Verzeichnis, in dem Sie sich zuletzt befanden. Q und F10
beenden ohne Verzeichniswechsel.

In PowerShell tragen Sie Folgendes in Ihr `$PROFILE` ein:

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
