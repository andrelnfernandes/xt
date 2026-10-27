# Sair e mudar o diretório do shell

**Q** ou F10 sai do programa (perguntando antes, a menos que `confirm_quit`
esteja desativado). Se um programa estiver rodando no
[terminal integrado](terminal.md), o Xylem sempre pergunta.

## Alt+Q

Um programa não pode alterar diretamente o diretório do shell que o iniciou.
Por isso, como em outros gerenciadores de arquivos, **Alt+Q** grava o diretório
atual em um arquivo (`--cwd-file`) e uma pequena função do shell faz o `cd`.
Coloque isto no seu `~/.bashrc` (ou `~/.zshrc`):

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

Com isso, `xy` funciona como antes; somente quando você sai com **Alt+Q** o
shell termina no diretório em que você estava. Q e F10 saem sem alterar nada.

No PowerShell, coloque isto no seu `$PROFILE`:

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
