# Permissões e datas

## Permissões (A)

`A` altera as permissões do arquivo sob o cursor (Ctrl+A: dos arquivos
marcados), em octal: `644`, `755`. Alt+A funciona como Ctrl+A.

No Windows, `A` edita os atributos R, H, S e A, como no ZTreeWin: `RA` define
exatamente esses, `+H -A` ativa ou desativa alguns. A lista de arquivos os
exibe como `r-sa`. Veja [Windows](windows.md).

## Nova data (N)

`N` altera a data de modificação do arquivo ou diretório (Ctrl+N: dos arquivos
marcados). O prompt já vem preenchido com a data atual no seu formato; também
aceita `now` e `today`. Com apenas a data (sem `HH:MM`), cada arquivo mantém
seu próprio horário. Links simbólicos são ignorados (a data alteraria a do
arquivo para o qual apontam).

O formato de data é definido na página 1 da [configuração](configuration.md).
