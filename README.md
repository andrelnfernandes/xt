# xt

Gerenciador de arquivos em modo texto para Linux, inspirado no XTree e no ZTreeWin.

## Uso

```bash
cargo run --release -- [diretório]
```

Sem argumento, abre o diretório atual.

## Teclas

### Navegação

| Tecla | Árvore | Janela de arquivos |
|---|---|---|
| ↑ ↓ PgUp PgDn Home End | move o cursor | move o cursor |
| → / `+` | expande o diretório | |
| ← / `-` | recolhe (ou vai para o pai) | volta para a árvore |
| `*` | loga e expande o ramo inteiro | |
| `B` | Branch: arquivos do ramo inteiro (loga antes) | idem |
| `S` | Showall: arquivos de tudo o que foi logado | idem |
| `F` | Filespec: filtro de arquivos | idem |
| `F2` | relê o diretório do disco | idem |
| `V` | | visualiza o arquivo (texto/hex) |
| `O` | | abre um ZIP/TAR como diretório |
| `L` | Log: abre outro diretório na janela | |
| `C` | Compare: compara com a outra janela e marca | |
| F8 | divide a tela em duas janelas (ou volta a uma) | idem |
| Tab | troca de janela (tela dividida) | idem |
| Enter | abre a janela de arquivos | volta para a árvore |
| Esc | | volta para a árvore (no diretório do arquivo) |
| `Q` / F10 | sair (pede confirmação) | sair |

### Tags e operações

| Tecla | Árvore | Janela de arquivos |
|---|---|---|
| `T` / `U` | marca / desmarca os arquivos do diretório | marca / desmarca o arquivo e desce |
| Ctrl+T / Ctrl+U | marca / desmarca tudo o que foi logado (filespec) | marca / desmarca a lista inteira |
| `I` | inverte as marcas do diretório | inverte as marcas da lista |
| `C` | | copia o arquivo |
| `M` | cria um subdiretório | move o arquivo |
| `D` | apaga o diretório (só se vazio) | apaga o arquivo |
| `R` | renomeia o diretório | renomeia o arquivo |
| `A` | | permissões (octal, ex.: `644`) |
| Ctrl + `C` `M` `D` `A` | idem, nos arquivos marcados | idem, nos arquivos marcados |
| Alt + `C` `M` | copia / move os marcados **recriando os caminhos** | idem |

As operações em grupo agem sobre os arquivos marcados **da lista atual**
(o diretório, ou o Branch/Showall aberto).

**Com ou sem caminhos:** Ctrl+C / Ctrl+M copiam ou movem os marcados todos
direto para o destino. Alt+C / Alt+M recriam no destino os subdiretórios,
relativos à raiz da lista (o diretório atual, a raiz do Branch, ou a raiz da
árvore no Showall): `proj/src/model/tree.rs` → `destino/src/model/tree.rs`.
No prompt do destino, **F6** troca entre os dois modos, e o modo atual aparece
no próprio prompt. Alt+D e Alt+A funcionam como Ctrl+D e Ctrl+A.

**Tela dividida:** F8 abre uma segunda janela, começando no mesmo diretório;
Tab alterna entre elas (sem a divisão, Tab faz o mesmo que Enter). Cada janela
tem a sua árvore, cursor e filespec; use `L` para abrir outro diretório nela.
Com a tela dividida, o Copy/Move sugere como destino o diretório da outra
janela, e as duas são atualizadas depois de qualquer operação.

**Compare (`C` na árvore, com a tela dividida):** compara os arquivos do
diretório atual com os do diretório da outra janela e marca, à escolha:
**U**nique (não existem do outro lado), **N**ewer, **O**lder, **I**dentical
(mesmo tamanho e data), **D**ifferent, ou **A**ll new (únicos + mais novos:
o que falta copiar para atualizar o outro lado). **B** troca o escopo para o
ramo inteiro: o xt loga o ramo, compara seguindo os caminhos relativos
(`src/a.rs` ↔ `<outro>/src/a.rs`) e abre o Branch com os arquivos marcados.
Datas com até 2 segundos de diferença contam como iguais (FAT/exFAT).

Para sincronizar: `C` `B` `A` e depois Alt+C (copiar com caminhos), que já
sugere a outra janela como destino.

**Arquivos compactados:** `O` na janela de arquivos abre `.zip`, `.jar`,
`.war`, `.apk`, `.tar`, `.tar.gz` ou `.tgz` como uma árvore, somente leitura:
navegação, Filespec, Branch, Showall, tags, View e Compare funcionam dentro
dele, e Copy (inclusive Ctrl+C e Alt+C com caminhos) extrai os arquivos.
`Esc` na árvore fecha o arquivo e volta para onde estava. Por segurança,
entradas com `..` ou caminho absoluto são ignoradas, e links simbólicos
dentro de arquivos compactados nunca são extraídos.

**Menus Ctrl/Alt:** como no ZTree, o rodapé mostra só os comandos de uma
tecla. **F9** alterna para o menu de Ctrl, depois o de Alt, e volta. Com um
desses menus aberto pelo F9, a próxima letra vale como Ctrl+letra ou
Alt+letra (ex.: `F9` `M` = Ctrl+M); `Esc` desiste. Nos terminais com o
protocolo de teclado do kitty, basta **segurar Ctrl ou Alt** para o menu
correspondente aparecer enquanto a tecla estiver pressionada.

**Ctrl+M:** na maioria dos terminais, Ctrl+M chega igual a Enter. O xt liga
automaticamente o protocolo de teclado do kitty quando o terminal o suporta
(kitty, WezTerm, foot, Ghostty, Alacritty recente, iTerm2), e aí Ctrl+M move
os arquivos marcados como no ZTree. Nos terminais baseados em VTE (GNOME
Terminal, Ptyxis, Terminator, Tilix) use **F9 M** (ou Alt+M e F6 no prompt).

Durante um log demorado, o progresso aparece no rodapé e `Esc` cancela.

**Filespec:** padrões glob separados por espaço, `;` ou `,`, sem diferenciar
maiúsculas. Um `-` na frente exclui: `*.rs *.toml -*.bak`.

**Copy/Move:** o destino pode ser um diretório (relativo ao diretório do
cursor da árvore, absoluto ou com `~`) ou, para um único arquivo, um novo nome. Se o destino já existe,
o xt pergunta: Yes / No / All / Skip all. A cópia é feita num temporário que
só substitui o destino no fim: cancelar (`Esc`) ou um erro nunca deixa um
arquivo pela metade. Permissões e data de modificação são preservadas, e links
simbólicos são copiados como links. Mover entre discos copia e depois apaga.

**Delete** apaga de verdade (não vai para a lixeira) e sempre pede confirmação.

**Visualizador:** setas/PgUp/PgDn/Home/End rolam, ←→ deslocam na horizontal,
`H` alterna texto/hex, `Esc` fecha. Arquivos binários abrem em hex.
Arquivos acima de 16 MiB mostram só o começo.

Na árvore, `+` no conector (`├─+`) indica um diretório recolhido que tem subdiretórios.

## Estrutura

- `src/model/` — árvore de diretórios em memória (arena), sem dependência da UI
- `src/app.rs` — estado global (prompts, operações) e execução de comandos
- `src/pane.rs` — uma janela: árvore, lista de arquivos, cursores e filespec
- `src/input.rs` — mapeamento tecla → comando
- `src/ui.rs` — desenho da tela (ratatui)
- `src/viewer.rs` — visualizador texto/hex
- `src/archive.rs` — leitura de ZIP/TAR
- `src/model/compare.rs` — regras do Compare
- `src/model/filespec.rs` — filtro de arquivos
- `src/ops/` — copiar, mover e apagar, em passos (progresso, conflitos, cancelamento)

## Roteiro

- [x] Fase 0 — tela clássica, árvore navegável, janela de arquivos
- [x] Fase 1 — Filespec, Showall/Branch, viewer texto/hex, log com progresso
- [x] Fase 2 — tagging, copy/move/delete/rename, makedir, permissões, re-log
- [ ] Fase 3 — janela dividida, compare, arquivos compactados, inotify, macros
- [ ] Fase 4 — temas, keymap configurável, empacotamento
