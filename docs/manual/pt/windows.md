# Windows

O Xylem também compila para Windows (Windows Terminal ou o console clássico).
Ele passa nos testes automatizados lá, mas ainda não foi usado no dia a dia,
então espere algumas arestas. O que muda:

- **Atributos** (`A`) edita R, H, S e A, como no ZTreeWin: `RA` define
  exatamente esses, `+H -A` ativa ou desativa alguns. A lista de arquivos os
  exibe como `r-sa`.
- Arquivos e diretórios **ocultos** são aqueles com o atributo H (um nome
  começando com ponto é um nome comum).
- **eXecute** e **Edit** passam pelo `cmd`, com os nomes entre aspas duplas. Um
  `%NOME%` dentro de um nome de arquivo ainda é expandido pelo `cmd`.
- O editor padrão é `notepad`, `O` usa `start` e Ctrl+B sugere `xy-batch.bat`
  (começando com `@echo off`).
- Excluir um arquivo somente leitura o exclui, assim como `rm` faz no Linux.
- Copiar links simbólicos exige o Modo de Desenvolvedor ou privilégios de
  administrador.
- O modo de exibição de dono (Alt+F) exibe `-`.
- Configuração em `%APPDATA%\xylem\config.toml`, histórico em
  `%LOCALAPPDATA%\xylem\`.
- O terminal integrado (Alt+X) executa `cmd` (`%COMSPEC%`). Entrar nele ainda
  leva o shell para o diretório da árvore, mas a árvore não acompanha os `cd`s
  do shell, e não há `$XY_TAGGED`.

Para usar Alt+Q no PowerShell, veja [sair](quit.md).
