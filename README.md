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

### Sair mudando o diretório do shell (Alt+Q)

Um programa não consegue mudar o diretório do shell que o chamou. Por isso,
como nos outros gerenciadores de arquivos, o **Alt+Q** grava o diretório
atual num arquivo (`--cwd-file`) e uma pequena função do shell faz o `cd`.
Coloque no `~/.bashrc` (ou `~/.zshrc`):

```bash
xt() {
    local tmp dir
    tmp=$(mktemp) || return
    command xt --cwd-file "$tmp" "$@"
    dir=$(cat -- "$tmp")
    rm -f -- "$tmp"
    [ -n "$dir" ] && [ -d "$dir" ] && cd -- "$dir"
}
```

Com ela, `xt` funciona como antes; só ao sair com **Alt+Q** o shell fica no
diretório em que você estava no xt. `Q` e F10 saem sem mudar nada.

## Configuração

A forma mais fácil é a **tela de configuração: F12** (ou Alt+F10, como no
ZTreeWin; no Ubuntu/GNOME o Alt+F10 costuma maximizar a janela e não chega ao
xt). Ela tem quatro páginas:

| Página | Opções |
|---|---|
| 1. Programas e formatos | editor, formato de data, formato de número |
| 2. Exibição e ordenação | mostrar ocultos, ordenação inicial e sentido, o que o Enter faz na janela de arquivos, modo inicial da janela de arquivos |
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
| ← / `-` | recolhe (ou vai para o pai) | página acima (em colunas: arquivo anterior) |
| → | expande | página abaixo (em colunas: próximo arquivo) |
| Alt+F | | modo de exibição: colunas de nomes, colunas com tamanho, detalhes, nomes longos |
| Backspace | diretório pai | mostra os arquivos do diretório pai |
| Espaço | desce uma linha | idem |
| `*` | loga e expande o ramo inteiro | |
| `\` | Treespec: vai a um diretório digitando o caminho | idem (no Branch/Showall: vai ao diretório do arquivo) |
| Shift+letra, 0–9 | próximo diretório que começa com ela | próximo arquivo que começa com ela |
| Alt+K | mostra o tamanho ou o nº de arquivos de cada diretório na árvore | idem |
| `B` | Branch: arquivos do ramo inteiro (loga antes) | idem |
| `S` | Showall: arquivos de tudo o que foi logado | idem |
| `F` | Filespec: filtro de arquivos | idem |
| Alt+S | ordena os arquivos: nome, extensão, data, tamanho; ordem | idem |
| F3 (ou F2) | relê o diretório do disco | idem |
| `V` | | visualiza o arquivo (texto/hex, com busca) |
| F7 | Autoview: lista com o conteúdo do arquivo selecionado ao lado | idem |
| Ctrl+S | | procura um texto nos marcados e desmarca os que não o têm |
| Ctrl+V | | visualiza os marcados em sequência |
| `O` | | abre o arquivo no programa padrão (`xdg-open`) |
| Ctrl+O | | abre os arquivos marcados no programa padrão |
| Alt+F5 | | abre um ZIP/TAR como diretório |
| `E` | | abre o arquivo no editor |
| `X` | executa um comando do shell | idem |
| `L` | Log: abre outro diretório na janela | |
| `C` | Compare: compara com a outra janela e marca | |
| F8 | divide a tela em duas janelas (ou volta a uma) | idem |
| Tab | troca de janela (tela dividida) | idem |
| Enter | abre a janela de arquivos | volta para a árvore |
| Esc | | volta para a árvore (no diretório do arquivo) |
| Alt+I | Info: detalhes do diretório (e o que está logado nele) | Info: detalhes do arquivo |
| `A` | Avail: sistemas de arquivos montados e espaço livre | |
| `?` | estatísticas da janela | idem |
| F1 | ajuda: lista das teclas em vigor | idem |
| F12 / Alt+F10 | tela de configuração | idem |
| `Q` / F10 | sair (pede confirmação) | sair |
| Alt+Q | sai deixando o shell no diretório atual (veja acima) | idem |

### Tags e operações

| Tecla | Árvore | Janela de arquivos |
|---|---|---|
| `T` / `U` | marca / desmarca os arquivos do diretório | marca / desmarca o arquivo e desce |
| Ctrl+T / Ctrl+U | marca / desmarca tudo o que foi logado (filespec) | marca / desmarca a lista inteira |
| `I` | inverte as marcas do diretório | inverte as marcas da lista |
| Ctrl+F7 / F8 / F9 | marca / desmarca / inverte os arquivos logados do ramo | |
| `C` | | copia o arquivo |
| `M` | cria um subdiretório | move o arquivo |
| `D` | apaga o diretório (só se vazio) | apaga o arquivo |
| Alt+P | Prune: apaga o diretório com tudo dentro | |
| `R` | renomeia o diretório | renomeia o arquivo |
| `A` | | permissões (octal, ex.: `644`) |
| `N` | muda a data do diretório | muda a data do arquivo (Ctrl+N: dos marcados) |
| Ctrl+B | | Batch: cria um script ou lista com uma linha por marcado |
| F5 / Ctrl+F5 | | cria um ZIP/TAR com o arquivo / com os marcados |
| Ctrl + `C` `M` `D` `A` | idem, nos arquivos marcados | idem, nos arquivos marcados |
| Alt+C | Compare do ramo inteiro | copia os marcados **recriando os caminhos** |
| Alt+M | | move os marcados **recriando os caminhos** |

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
o que falta copiar para atualizar o outro lado). **B** (ou Alt+C na árvore,
como no ZTreeWin) troca o escopo para o
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

**Treespec (`\`):** abre um prompt com o caminho atual; enquanto você digita,
o cursor da árvore vai para o primeiro diretório que começa com o que foi
digitado (sem diferenciar maiúsculas). `/` ou `\` completam o nome e descem
um nível; Tab e Shift+Tab completam e depois passam para o próximo/anterior
diretório do mesmo nível; Backspace volta níveis; Enter confirma; Esc volta
para onde estava. ↑ traz o histórico (salvo entre execuções). Um caminho fora
da árvore vira um Log ao confirmar. No Branch/Showall, `\` vai direto ao
diretório do arquivo selecionado, com ele em destaque.

**Modos de exibição (Alt+F):** alterna, na ordem do ZTreeWin, entre
**nomes em várias colunas**, **nomes e tamanhos em colunas**, **detalhes**
(uma coluna com tamanho, data e permissões, o padrão) e **nomes longos** (uma
coluna, só os nomes inteiros). Nos modos de colunas, os arquivos seguem da
esquerda para a direita, ← → andam um arquivo e ↑ ↓ uma linha; nomes com mais
de 20 caracteres são cortados com `…`. O modo inicial fica em `file_display`
(página 2 da configuração).

**Autoview (F7):** divide a tela entre a lista de arquivos e o conteúdo do
arquivo selecionado, que acompanha o cursor. As teclas da janela de arquivos
continuam valendo (T marca, por exemplo); Shift+↑/↓/PgUp/PgDn/Home/End
rolam o conteúdo; F7, Esc ou Enter saem. Mostra o primeiro 1 MiB de cada
arquivo (o V mostra até 16 MiB).

**KByte (Alt+K):** mostra à direita de cada diretório da árvore o
**S**ize (total em K) ou o **C**ount (nº de arquivos); **N**one desliga.
Antes de escolher, **A**ll/**T**agged contam todos ou só os marcados, e
**B**ranch/**D**irectory somam o ramo inteiro ou só o diretório. Diretórios
ainda não logados mostram `?`. Cada janela tem a sua escolha.

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

**Open (`O`):** abre o arquivo no programa padrão do sistema (`xdg-open`:
imagens no visualizador de imagens, PDFs no leitor de PDF etc.), em
segundo plano, sem sair do xt. Ctrl+O abre todos os marcados. Dentro de um
arquivo compactado, a entrada é extraída antes para uma pasta temporária,
apagada ao sair do xt. Outro programa pode ser escolhido em `opener` (ou na
tela de configuração).

**Arquivos compactados:** Alt+F5 na janela de arquivos abre `.zip`, `.jar`,
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
uma operação em andamento. Outros diretórios continuam com F3.

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

**Filespec (`F`):** especificações separadas por espaço, `;` ou `,`, de três
tipos, como no ZTreeWin:

- **nome:** glob (`*.rs`, `a?c*`, `[abc]*`), sem diferenciar maiúsculas;
  entre aspas, o nome pode ter espaços (`"meu arquivo*"`);
- **data:** `<`, `=`, `>`, `<=`, `>=` ou `<>` e uma data na ordem do formato
  configurado, com `/`, `.` ou `-` e ano de 2 ou 4 dígitos (`>01/05/2026`;
  `2026-05-01` sempre vale), ou `TODAY` e `TODAY-n` (`>TODAY-7`: últimos 7
  dias);
- **tamanho:** o mesmo com `s` depois do operador (`>=s200000`); no xt
  também valem K, M e G (`>s100M`).

Um `-` na frente nega (`-*.bak`; `-<s1K` é o mesmo que `>=s1K`). Dentro de
cada tipo, as especificações positivas valem com "ou", exceto `>a` logo
seguido de `<b`, que é a faixa entre os dois (`>=01/05/2026,<=31/05/2026`:
maio); `<>` exclui. Entre os tipos vale "e": `*.txt,=TODAY,<s1000` são os
`.txt` de hoje com menos de 1000 bytes.

**Copy/Move:** o destino pode ser um diretório (relativo ao diretório do
cursor da árvore, absoluto ou com `~`) ou, para um único arquivo, um novo nome. Se o destino já existe,
o xt pergunta: Yes / No / All / Skip all. A cópia é feita num temporário que
só substitui o destino no fim: cancelar (`Esc`) ou um erro nunca deixa um
arquivo pela metade. Permissões e data de modificação são preservadas, e links
simbólicos são copiados como links. Mover entre discos copia e depois apaga.

**New date (`N`):** muda a data de modificação. O prompt já vem com a data
atual no seu formato; aceita também `now` e `today`. Só com a data (sem
`HH:MM`), cada arquivo mantém a sua hora. Links simbólicos são pulados (a
data mudaria a do arquivo para onde apontam).

**Batch (Ctrl+B):** pede o nome do arquivo a criar e uma máscara de linha,
e escreve uma linha por arquivo marcado. Parâmetros: `%f` caminho, `%d`
diretório, `%n` nome, `%b` nome sem extensão, `%e` extensão, `%s` tamanho.
Em minúsculas saem entre aspas, seguros para o shell (`convert %f %b.png`);
em maiúsculas, como texto puro, para listas (`%N;%S`). Um arquivo `.sh`
ganha `#!/bin/sh` e permissão de execução (rode-o com `X`). Um arquivo que
já existe não é sobrescrito.

**Criar compactado (F5 / Ctrl+F5):** cria um `.zip`, `.tar`, `.tar.gz` ou
`.tgz` (pelo nome digitado) com o arquivo atual ou com os marcados. Os
caminhos lá dentro são relativos à raiz da lista, como no Alt+C: num Branch,
as subpastas são mantidas. Tem progresso e `Esc` cancela; o arquivo é escrito
num temporário e só ganha o nome no fim, então nunca fica pela metade. Links
simbólicos entram como links. Um arquivo que já existe não é sobrescrito.

**Info, Avail e estatísticas:** Alt+I mostra tipo, tamanho, permissões,
dono e grupo, datas (modificação, acesso e mudança), inode, links e
dispositivo do arquivo ou diretório (de um link, para onde aponta; de um
diretório, também o que está logado nele e no ramo). `A` na árvore lista os
sistemas de arquivos montados com total, usado, livre e % de uso, como o
`df`. `?` resume a janela: o que está logado, a lista, os marcados e o disco.

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
Arquivos acima de 16 MiB mostram só o começo. Busca, como no ZTreeWin:
**F** procura do início, **B** para trás a partir do fim, **/** (ou S) a
partir da página atual e **\\** para trás a partir dela; **Espaço**
continua na mesma direção, **+** e **-** escolhem a direção. O trecho achado
fica destacado.

**Procurar nos marcados (Ctrl+S) e visualizá-los (Ctrl+V):** Ctrl+S procura
um texto em todos os arquivos marcados da lista e **desmarca os que não o
contêm**, com progresso e `Esc` para parar. Ctrl+V abre os que sobraram no
visualizador, um depois do outro: **N**ext e **P**rev trocam de arquivo
(Alt+Home/Alt+End: primeiro/último) e **U** desmarca o atual. O termo é o
mesmo nas duas buscas: depois de um Ctrl+S, o **Espaço** no Ctrl+V já vai
para o próximo trecho, passando para o próximo marcado quando o arquivo
acaba. Nos prompts de busca, **F2** liga/desliga diferenciar maiúsculas
(sem diferenciar, acentos também contam: `AÇÃO` acha `ação`) e **F4** troca
entre texto e hex (pares como `4d 5a`). Um `*` no meio do termo casa
qualquer coisa na mesma linha (`erro*disco`); um termo nunca atravessa uma
quebra de linha. Funciona também dentro de ZIP/TAR.

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
janela dividida (F8 e Tab), relog (F3), Open (`O`), atualização automática, navegação e
extração de arquivos compactados, histórico nos prompts, ajuda (F1) e tela de
configuração (Alt+F10).

### Simplificado

| Recurso | No ZTreeWin | No xt |
|---|---|---|
| Filespec | nome com regras próprias de nome e extensão, data, tamanho, atributo, filespec invertido (Ctrl+I), rótulos | glob para os nomes, data e tamanho como no ZTreeWin (com K/M/G); sem atributos, inversão e rótulos |
| Visualizador | modos Alpha, Dump, Hex, Junk e Wrap; conjuntos de caracteres; edição hex; marcadores; auto-scroll | texto (UTF-8) ou hex, com a busca do ZTreeWin (F, B, /, \\, Espaço, + e -), sem quebra de linha; até 16 MiB |
| Ctrl+S | texto, hex ou Unicode; busca só no índice de ZIPs | texto ou hex |
| Attributes | atributos do Windows (R, A, S, H) | permissões do Unix em octal (`644`); sem dono/grupo |
| Sort | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; same/diff na tela dividida | Name, Ext, Date, Size, crescente/decrescente, Path first; cada janela tem a sua ordem |
| Compare | opções Identical, Unique, Newer, Older, Size, Binary, Case; filtro de exibição | marca únicos, mais novos, mais antigos, idênticos, diferentes ou únicos+mais novos; sem comparação binária |
| Prune | apaga só os subdiretórios ou também o atual; só diretórios vazios; lixeira | apaga sempre o diretório inteiro, permanentemente; confirmação configurável |
| Delete | lixeira opcional | apaga de vez |
| Log | vários drives logados, Global, troca de drive | uma raiz por janela; `L` abre outro diretório |
| eXecute | parâmetros de batch `%1`–`%8`, nova janela (NTW) | `sh` com `%f`, `%t` e `%d`; sempre no terminal do xt |
| Histórico | janela de histórico com marcadores | ↑/↓ no prompt, um histórico por tipo, salvo entre execuções |
| Configuração | 7 páginas, editor de cores (F4) por elemento, Alt-Export | 4 páginas com as opções que fazem sentido no Linux; F4 escolhe entre 8 temas prontos; cores individuais no `config.toml` |
| Arquivos compactados | qualquer formato com archiver externo, criar (F5), extrair (Alt-F5) | ZIP/JAR/WAR/APK/TAR/TGZ: abrir (Alt+F5), extrair com Copy, criar com F5 (ZIP, TAR e TAR.GZ) |
| Tela dividida | painéis de estatísticas opcionais por lado | sem painel de estatísticas |
| Menus | menu de Ctrl/Alt ao segurar a tecla | igual em terminais com o protocolo do kitty; F9 em qualquer terminal |

Opções de configuração do ZTreeWin que não foram trazidas por serem do
Windows: atributos RSH, lixeira, junctions, OEM/ANSI, EBCDIC, tamanho da
janela, atalhos (.lnk). As demais dependem de recursos ainda não feitos
(abaixo).

### Teclas diferentes

As teclas seguem as do ZTreeWin. As exceções:

| Função | ZTreeWin | xt |
|---|---|---|
| Configuração | Alt+F10 | F12 ou Alt+F10 (no GNOME, Alt+F10 maximiza a janela e não chega ao xt) |
| Mover marcados | Ctrl+M | Ctrl+M em terminais com o protocolo do kitty; nos outros, `F9` `M` (ou Alt+M e F6) |
| `-` na árvore | descarrega o ramo da memória | só recolhe (continua logado) |
| F2 | (nada) | relê o diretório, igual ao F3 |
| F9 | menu de aplicativos (não implementado) | menus de Ctrl/Alt |
| F12 | macros (não implementadas) | configuração |
| Ctrl+C, Ctrl+M na árvore | (nada) | Copy/Move dos marcados do diretório atual |

Todas as teclas podem ser trocadas em `[keys]` no `config.toml`.

### Não implementado

- **Árvore:** Global (G), Graft (Alt+G), Mirror (Alt+M), Oops
  (O, recuperar apagados), Release/descarregar da memória (Alt+R, `-`,
  Ctrl+Backspace), Hide (Alt+H), Wash (Alt+W), Volume
  (Ctrl+V), F5/F6 (esconder níveis de subdiretórios),
  marcar por atributo (Alt+T/U),
  Treespec em modo de edição (F5, curingas) e pastas especiais (F6), busca
  ao digitar com prompt (`|`) e modo ativo, troca de drive (`<` e `>`).
- **Arquivos:** Open em nova janela de terminal (Alt+O), Batch do arquivo atual (Alt+B), JFC (J, comparar o conteúdo de dois arquivos), ZAAP
  (Y), sHortcut (H), Print (P), Rename com máscara, localizar e trocar e mudança de maiúsculas,
  dono/grupo (chown), Megaview (Ctrl+F7 na janela de arquivos), juntar marcados entre as
  janelas (Ctrl+F6), Partial Untag (Ctrl+F8), copiar caminho para a área de
  transferência (Ctrl+Insert).
- **Geral:** macros (F12), menu de aplicativos (F9), marcadores (F11, `[`
  e `]`), Quit and shell (Ctrl+Q), salvar e
  carregar o log (Zlist, Ctrl/Alt+Z), lixeira, cores por nome e atributo
  (ZCOLORS.INI), mouse, opções de linha de comando do ZTreeWin, comandos de
  vídeo (Alt+F6–F9).
