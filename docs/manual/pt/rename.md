# Renomear e renomeação em lote

`R` renomeia o arquivo (na árvore, o diretório) sob o cursor: digite o novo
nome. Um nome que já exista é recusado.

## Renomeação em lote (Ctrl+R ou Alt+R)

Renomeia todos os arquivos marcados com uma máscara, numerados na ordem da
lista:

- **Curingas:** `*.jpg` (muda a extensão), `novo_*.*`, `*_antigo.*`, `???.*`
  (mantém essa quantidade de caracteres). `*.*` mantém os nomes como estão;
  `*.` remove a extensão.
- **Buscar e substituir:** `s/foo/bar/` substitui cada `foo` por `bar`;
  `s/foo/bar/i` ignora maiúsculas e minúsculas.
- **Contadores:** `%d`, `%02d`, `%03d`, `%04d`, `<#>`, `<01>`, `<001>`, `<0001>`
  numeram os arquivos (1, 01, 001...): `foto_<001>.*` gera `foto_001.jpg`,
  `foto_002.jpg`...
- **Caixa (maiúsculas/minúsculas):** `<lower>`, `<upper>`, `<title>` (também na
  parte do nome ou da extensão: `*.<lower>`), ou **F2** no prompt para alternar
  entre minúsculas, MAIÚSCULAS e Iniciais Maiúsculas no resultado.

Se a máscara der o mesmo nome a dois arquivos, nada é renomeado. Quando um novo
nome pertence a um arquivo que ainda será renomeado, o Xylem pergunta antes de
sobrescrevê-lo, como em [Copiar e Mover](copy-move.md).

Veja também: [marcações](tagging.md).
