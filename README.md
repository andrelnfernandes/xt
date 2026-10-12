# xt

Gerenciador de arquivos em modo texto para Linux, inspirado no XTree e no ZTreeWin.

## Instalação

Requer Rust (via [rustup](https://rustup.rs)). Na pasta do projeto:

```bash
cargo install --path .
```

O binário vai para `~/.cargo/bin/xt` (que o rustup já coloca no `PATH`).
Para atualizar depois de mudanças no código, rode o mesmo comando de novo;
para remover, `cargo uninstall xt`.

## Uso

```bash
xt [diretório]
```

Sem argumento, abre o diretório atual. `xt --help` mostra as opções.

## Configuração

A forma mais fácil é a **tela de configuração: F12** (ou Alt+F10, como no
ZTreeWin; no Ubuntu/GNOME o Alt+F10 costuma maximizar a janela e não chega ao
xt). Ela tem quatro páginas:

| Página | Opções |
|---|---|
| 1. Programas e formatos | editor, formato de data, formato de número |
| 2. Exibição e ordenação | mostrar ocultos, ordenação inicial e sentido, o que o Enter faz na janela de arquivos |
| 3. Confirmações e histórico | confirmar ao sair, confirmação do Prune, não guardar entradas com `@` |
| 4. Compare e atualização | ignorar 2 s, ignorar 1 h, atualização automática |

A letra da opção (ou Enter) troca o valor, PgUp/PgDn ou 1–4 mudam de página,
**F4** escolhe o tema numa lista (com prévia na hora), **F3** grava e aplica,
**Esc** desiste (pergunta se houver mudanças).

Tudo fica em `~/.config/xt/config.toml` (ou `$XDG_CONFIG_HOME/xt/`), que
também pode ser editado à mão; a tela mantém os seus comentários. Para começar
de um arquivo comentado com todos os padrões:

```bash
mkdir -p ~/.config/xt && xt --default-config > ~/.config/xt/config.toml
```

- `theme`: `classic` (azul do XTree), `dark`, `light`, `mono` (cores do
  próprio terminal), `norton`, `solarized`, `gruvbox` ou `nord`.
- `[options]`: as opções da tela de configuração.
- `[colors]`: troca cores individuais por cima do tema, com nomes
  (`"yellow"`), `"#rrggbb"` ou índices de 0 a 255.
- `[keys.any]`, `[keys.tree]`, `[keys.files]`: troca teclas, nas duas
  janelas ou só numa (esta tem prioridade). Ex.: `"f3" = "view"`,
  `"ctrl+r" = "relog"`; `"none"` desliga uma tecla. **F1** dentro do xt
  mostra as teclas em vigor, já com as suas trocas.

Um erro no arquivo não impede o xt de abrir: ele usa os padrões e mostra o
erro no rodapé (e a tela de configuração não grava até o arquivo ser
corrigido, para não apagar o que você escreveu).

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
| Alt+S | ordena os arquivos: nome, extensão, data, tamanho; ordem | idem |
| `F2` | relê o diretório do disco | idem |
| `V` | | visualiza o arquivo (texto/hex) |
| `O` | | abre um ZIP/TAR como diretório |
| `E` | | abre o arquivo no editor |
| `X` | executa um comando do shell | idem |
| `L` | Log: abre outro diretório na janela | |
| `C` | Compare: compara com a outra janela e marca | |
| F8 | divide a tela em duas janelas (ou volta a uma) | idem |
| Tab | troca de janela (tela dividida) | idem |
| Enter | abre a janela de arquivos | volta para a árvore |
| Esc | | volta para a árvore (no diretório do arquivo) |
| F1 | ajuda: lista das teclas em vigor | idem |
| F12 / Alt+F10 | tela de configuração | idem |
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
| Alt+P | Prune: apaga o diretório com tudo dentro | |
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

**Ordenação (Alt+S):** **N**ame, **E**xt, **D**ate ou **S**ize; **O**rder
alterna crescente/decrescente e **P**ath first agrupa por diretório no
Branch/Showall. A ordem atual aparece no título da janela de arquivos (ex.:
`Size↓`) e o cursor fica no mesmo arquivo. A ordem inicial vem de `sort` e
`sort_descending` na configuração; novas janelas (F8, Log) herdam a atual.

**Edit e eXecute:** `E` abre o arquivo no editor (`$VISUAL`, `$EDITOR`, o
`editor` do Debian/Ubuntu ou `vi`). `X` pede uma linha de comando e a roda com
`sh` no diretório atual; `%f` vira o arquivo sob o cursor, `%t` os arquivos
marcados e `%d` o diretório atual, todos já entre aspas (`%%` é um `%`).
Exemplo: `tar czf backup.tgz %t`. No fim, o xt espera Enter para você ler a
saída. Ao voltar, os diretórios abertos são relidos.

**Histórico:** nos prompts, ↑/↓ trazem as entradas anteriores. Cada tipo de
prompt (Filespec, destino do Copy/Move, Log, eXecute...) tem o seu histórico,
guardado em `~/.local/state/xt/history.toml` (ou `$XDG_STATE_HOME/xt/`), com
permissão só para você, já que pode conter comandos digitados.

**Arquivos compactados:** `O` na janela de arquivos abre `.zip`, `.jar`,
`.war`, `.apk`, `.tar`, `.tar.gz` ou `.tgz` como uma árvore, somente leitura:
navegação, Filespec, Branch, Showall, tags, View e Compare funcionam dentro
dele, e Copy (inclusive Ctrl+C e Alt+C com caminhos) extrai os arquivos.
`Esc` na árvore fecha o arquivo e volta para onde estava. Por segurança,
entradas com `..` ou caminho absoluto são ignoradas, e links simbólicos
dentro de arquivos compactados nunca são extraídos.

**Atualização automática:** o xt observa (via inotify) o diretório atual de
cada janela e relê sozinho quando outro programa cria, apaga, renomeia ou
altera arquivos ali, mantendo o cursor no mesmo arquivo. As mudanças são
agrupadas (no máximo uma releitura a cada meio segundo) e esperam o fim de
uma operação em andamento. Outros diretórios continuam com `F2`.

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

**Prune (Alt+P na árvore):** loga o ramo inteiro, mostra quantos arquivos,
diretórios e bytes serão apagados e pede que você **digite PRUNE** (ou o nome
do diretório, ou Y, conforme `prune_confirmation`). Apaga os arquivos e depois
os diretórios, do mais fundo para cima, com progresso e `Esc` para cancelar.
Links simbólicos são removidos sem seguir o destino, e **outros pontos de
montagem dentro do ramo são pulados** (um pendrive montado ali dentro não é
apagado). A raiz da árvore não pode sofrer Prune.

**Visualizador:** setas/PgUp/PgDn/Home/End rolam, ←→ deslocam na horizontal,
`H` alterna texto/hex, `Esc` fecha. Arquivos binários abrem em hex.
Arquivos acima de 16 MiB mostram só o começo.

Na árvore, `+` no conector (`├─+`) indica um diretório recolhido que tem subdiretórios.

## Estrutura

- `src/model/` — árvore de diretórios em memória (arena), sem dependência da UI
- `src/app.rs` — estado global (prompts, operações) e execução de comandos
- `src/pane.rs` — uma janela: árvore, lista de arquivos, cursores e filespec
- `src/input.rs` — comandos e o campo de texto dos prompts
- `src/keymap.rs` — teclas padrão, trocas da configuração e ajuda (F1)
- `src/ui.rs` — desenho da tela (ratatui)
- `src/viewer.rs` — visualizador texto/hex
- `src/archive.rs` — leitura de ZIP/TAR
- `src/watch.rs` — avisos de mudanças no disco (inotify)
- `src/shell.rs` — editor e comandos do shell (com aspas seguras)
- `src/config.rs` — arquivo de configuração, temas e formatos
- `src/settings.rs` — tela de configuração (F12)
- `src/state.rs` — histórico dos prompts entre execuções
- `src/model/compare.rs` — regras do Compare
- `src/model/filespec.rs` — filtro de arquivos
- `src/ops/` — copiar, mover e apagar, em passos (progresso, conflitos, cancelamento)

## Roteiro

- [x] Fase 0 — tela clássica, árvore navegável, janela de arquivos
- [x] Fase 1 — Filespec, Showall/Branch, viewer texto/hex, log com progresso
- [x] Fase 2 — tagging, copy/move/delete/rename, makedir, permissões, re-log
- [x] Fase 3 — janela dividida (F8), Log, Compare, ZIP/TAR, inotify, Edit,
  eXecute, histórico nos prompts
- [x] Fase 4 — configuração, temas, teclas configuráveis, ajuda (F1),
  histórico persistente, build de release e instalação

### Para depois

- pacote `.deb` (falta definir licença e mantenedor)
- versões para macOS e Windows
- os itens de "Não implementado" abaixo

## Comparação com o ZTreeWin

Levantamento feito comando a comando a partir do manual oficial do ZTreeWin
v2.2 ([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf), de 2011, em
especial a "Quick Command Table" e a seção 3.7 de configuração). Versões mais
novas do ZTreeWin podem ter mudado detalhes.

### Implementado

Árvore com log sob demanda (`+`, `*`), janela de arquivos, Filespec, Branch,
Showall, tags (T, U, I, Ctrl+T/U), Copy/Move/Delete/Rename com as versões
Ctrl (marcados) e Alt (marcados com caminhos), Make dir, Delete dir, Prune,
Attributes, View, Edit, eXecute, Compare (diretório e ramo), Sort, Log,
janela dividida (F8 e Tab), relog (F2), atualização automática, navegação e
extração de arquivos compactados, histórico nos prompts, ajuda (F1) e tela de
configuração (Alt+F10).

### Simplificado

| Recurso | No ZTreeWin | No xt |
|---|---|---|
| Filespec | regras próprias, filtros por data, tamanho e atributo, filespec invertido | glob (`*`, `?`, `[abc]`), exclusão com `-`, sem filtros por data/tamanho/atributo |
| Visualizador | modos Alpha, Dump, Hex, Junk e Wrap; busca; conjuntos de caracteres; edição hex | texto (UTF-8) ou hex, sem busca nem quebra de linha; até 16 MiB |
| Attributes | atributos do Windows (R, A, S, H) | permissões do Unix em octal (`644`); sem dono/grupo |
| Sort | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; same/diff na tela dividida | Name, Ext, Date, Size, crescente/decrescente, Path first; cada janela tem a sua ordem |
| Compare | opções Identical, Unique, Newer, Older, Size, Binary, Case; filtro de exibição | marca únicos, mais novos, mais antigos, idênticos, diferentes ou únicos+mais novos; sem comparação binária |
| Prune | apaga só os subdiretórios ou também o atual; só diretórios vazios; lixeira | apaga sempre o diretório inteiro, permanentemente; confirmação configurável |
| Delete | lixeira opcional | apaga de vez |
| Log | vários drives logados, Global, troca de drive | uma raiz por janela; `L` abre outro diretório |
| eXecute | parâmetros de batch `%1`–`%8`, nova janela (NTW) | `sh` com `%f`, `%t` e `%d`; sempre no terminal do xt |
| Histórico | janela de histórico com marcadores | ↑/↓ no prompt, um histórico por tipo, salvo entre execuções |
| Configuração | 7 páginas, editor de cores (F4) por elemento, Alt-Export | 4 páginas com as opções que fazem sentido no Linux; F4 escolhe entre 8 temas prontos; cores individuais no `config.toml` |
| Arquivos compactados | qualquer formato com archiver externo, criar (F5), extrair (Alt-F5) | só leitura de ZIP/JAR/WAR/APK/TAR/TGZ; extrair com Copy |
| Tela dividida | painéis de estatísticas opcionais por lado | sem painel de estatísticas |
| Menus | menu de Ctrl/Alt ao segurar a tecla | igual em terminais com o protocolo do kitty; F9 em qualquer terminal |

Opções de configuração do ZTreeWin que não foram trazidas por serem do
Windows: atributos RSH, lixeira, junctions, OEM/ANSI, EBCDIC, tamanho da
janela, atalhos (.lnk). As demais dependem de recursos ainda não feitos
(abaixo).

### Teclas diferentes

| Função | ZTreeWin | xt |
|---|---|---|
| Reler o diretório | F3 | F2 |
| Configuração | Alt+F10 | F12 ou Alt+F10 (no GNOME, Alt+F10 maximiza a janela) |
| Abrir arquivo compactado | Alt+F5 | `O` |
| Compare do ramo | Alt+C na árvore | `C` e depois `B` |
| Alt+C na árvore | Compare do ramo | Copy dos marcados com caminhos |
| Alt+M na árvore | Mirror | Move dos marcados com caminhos |
| Mover marcados | Ctrl+M | Ctrl+M (terminais com o protocolo do kitty), `F9` `M`, ou Alt+M e F6 |
| `-` na árvore | descarrega o ramo da memória | só recolhe (continua logado) |
| ← na janela de arquivos | página acima | volta para a árvore |
| F9 | menu de aplicativos | menus de Ctrl/Alt |
| F12 | macros | configuração |

Todas as teclas podem ser trocadas em `[keys]` no `config.toml`.

### Não implementado

- **Árvore:** Avail (A), Global (G), Graft (Alt+G), Mirror (Alt+M), Oops
  (O, recuperar apagados), Release/descarregar da memória (Alt+R, `-`,
  Ctrl+Backspace), Hide (Alt+H), Info (Alt+I), Wash (Alt+W), Volume
  (Ctrl+V), KByte (Alt+K), F5/F6 (esconder níveis de subdiretórios),
  marcar/desmarcar o ramo (Ctrl+F7/F8), marcar por atributo (Alt+T/U),
  navegação por Treespec (`\`), busca ao digitar (spell search), janela de
  estatísticas (`?`), troca de drive (`<` e `>`).
- **Arquivos:** Open com o programa associado (O; no Linux seria
  `xdg-open`), New date (N), Batch (Ctrl+B/Alt+B), busca no conteúdo dos
  marcados (Ctrl+S), JFC (J, comparar o conteúdo de dois arquivos), ZAAP
  (Y), sHortcut (H), Print (P), modos de exibição (Alt+F: várias colunas,
  só nomes), Rename com máscara, localizar e trocar e mudança de maiúsculas,
  dono/grupo (chown), Megaview e Autoview (F7), juntar marcados entre as
  janelas (Ctrl+F6), Partial Untag (Ctrl+F8), copiar caminho para a área de
  transferência (Ctrl+Insert).
- **Geral:** macros (F12), menu de aplicativos (F9), marcadores (F11, `[`
  e `]`), Quit mudando o diretório do shell (Ctrl+Q/Alt+Q), salvar e
  carregar o log (Zlist, Ctrl/Alt+Z), lixeira, cores por nome e atributo
  (ZCOLORS.INI), mouse, opções de linha de comando do ZTreeWin, comandos de
  vídeo (Alt+F6–F9), criar arquivos compactados (F5).
