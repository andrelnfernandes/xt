# Plano: `-` libera o ramo da memória (release), como no ZTreeWin

Origem: conversa de 2026-10-29. O autor notou que, depois de `L` em um
diretório, `+` e `*` numa subpasta, o Ctrl+T marcava arquivos de pastas
irmãs. Conferido no manual do ZTreeWin v2.2 (ztwhlp.pdf):

- **Ctrl-Tag** (árvore): "Tag all matching files logged on the current disk".
  O Xylem faz o mesmo: não é bug.
- **Ctrl-F7 Tag**: "Tag all matching files logged in the current branch". O
  Xylem também tem (Ctrl+F7/F8/F9).
- **`-`**: "Collapse the entire branch. All file and directory information
  for the branch is unlogged (released) from memory". No Xylem, `-` só
  recolhia: os arquivos continuavam logados e marcados, e entravam no Ctrl+T.
- **`+`**: "Log the files in the current directory, and display the next
  level subdirectories of the current directory, without logging their
  files". No Xylem, `+` (e o Log) leem também os arquivos das subpastas.

A diferença está no que fica logado, não no Ctrl+T.

## Etapa 1: `-` faz release (aprovada)

- Novo comando `release` ("release the branch from memory"), na tecla `-` da
  árvore. `←` continua sendo `collapse` (recolhe ou vai para o pai).
- `Tree::release(id)`: tira da memória tudo o que estava abaixo de `id` (os
  nós ficam soltos na arena, como no `rescan`, mas sem arquivos nem filhos,
  para liberar a memória) e relê `id` do disco, como logo depois de um Log:
  os arquivos de `id` voltam sem marca e as subpastas voltam como não
  logadas, recolhidas. Devolve quantas marcas se perderam.
- O ZTreeWin também descarrega os arquivos do próprio diretório; no Xylem a
  janela de arquivos sempre mostra o diretório sob o cursor, então ele é
  relido na hora (equivale ao `-` seguido do log implícito ao parar nele).
- Árvores virtuais (arquivos compactados) não têm de onde reler: lá `-` só
  recolhe.
- Mensagem no rodapé: `Released <caminho>`, com o número de marcas
  descartadas quando houver.
- Seletor de diretórios (picker): `-` continua recolhendo.
- Cada janela do split tem a sua árvore: o release só afeta a janela ativa
  (como no ZTreeWin, onde as marcas de cada lado são independentes).
- Manual (en, pt, es, fr, de, zh): tabela de teclas em `tree.md`, a linha do
  `-` em "Different behavior" e o item de "What Xylem does not have yet" em
  `ztreewin.md` (fica só Alt+R e Ctrl+Backspace).

## Etapa 2: `+` sem logar os arquivos das subpastas (não aprovada ainda)

Para o indicador `├─+` o Xylem lê as subpastas de cada filho ao expandir, e
com isso guarda também os arquivos delas. Separar "subpastas conhecidas" de
"arquivos logados" no `DirNode` (por exemplo, um estado intermediário entre
não lido e logado), para que `+` e o Log loguem só os arquivos do próprio
diretório, como no ZTreeWin. Mexe em `scan`, `expand`, `rescan`, nos totais
e no que conta como logado para Showall/Ctrl+T.
