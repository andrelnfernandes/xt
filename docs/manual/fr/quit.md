# Quitter et changer le répertoire du shell

**Q** ou F10 quitte (en demandant confirmation, sauf si `confirm_quit` est
désactivé). Si un programme tourne dans le [terminal intégré](terminal.md),
Xylem demande toujours confirmation.

## Alt+Q

Un programme ne peut pas changer le répertoire du shell qui l'a lancé. C'est
pourquoi, comme dans d'autres gestionnaires de fichiers, **Alt+Q** écrit le
répertoire courant dans un fichier (`--cwd-file`) et une petite fonction shell
effectue le `cd`. Ajoutez ceci à votre `~/.bashrc` (ou `~/.zshrc`) :

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

Avec cette fonction, `xy` fonctionne comme avant ; ce n'est que lorsque vous
quittez avec **Alt+Q** que le shell se retrouve dans le répertoire où vous
étiez. Q et F10 quittent sans rien changer.

Dans PowerShell, mettez ceci dans votre `$PROFILE` :

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
