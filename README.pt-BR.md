<img src="docs/icons/xylem.svg" width="96" align="right" alt="Ícone do Xylem">

# Xylem

[Read in English](README.md)

Gerenciador de arquivos em modo texto para Linux, inspirado no XTree e no ZTreeWin.

O Xylem se chamava **xt** até a versão 0.1; o comando agora é `xy`. Na
primeira execução, ele move a configuração, o histórico e os bookmarks do xt
para as pastas novas. O nome: o xilema (*xylem*, em inglês) é o tecido que
leva a água pela árvore, e começa com X, como o XTree.

![O Xylem em uso: o terminal embutido com a árvore acompanhando um cd, arquivos marcados num Branch e digitados num comando com Alt+O](docs/demo.gif)

## Por que o Xylem?

Uso alguma variação do XTree desde a década de 1990, e o ZTreeWin continua
sendo, para mim, o melhor gerenciador de arquivos em modo texto. Desenvolvi o
Xylem por sentir falta de algo tão bom quanto ele rodando no Linux: as versões
que existem parecem estar abandonadas.

Apesar de gostar muito do ZTreeWin, a evolução dele é extremamente lenta. Por
isso, provavelmente o Xylem também terá uma versão para Windows.

O Xylem segue as teclas e o jeito de trabalhar do ZTreeWin sempre que possível;
as diferenças estão descritas em [Comparação com o ZTreeWin](docs/manual/ztreewin.md) (em inglês).

## Aviso

O Xylem é um projeto independente, escrito do zero. Não tem ligação com a
Zedtek, Inc. (ZTreeWin) nem com os donos do XTree, nem é endossado por eles.
Não contém código desses programas; os nomes deles aparecem só para descrever
o que inspirou o Xylem e como ele se compara. XTree, ZTree e ZTreeWin são marcas
de seus respectivos donos.

## Telas

A árvore de diretórios, a janela de arquivos e as estatísticas, no tema clássico:

![Xylem: árvore de diretórios, janela de arquivos e estatísticas, no tema clássico](docs/screenshots/tree.png)

**Branch** (`B`) lista os arquivos de um ramo inteiro; os arquivos marcados
(`T`) ficam destacados e são contados à direita:

![Branch com três arquivos marcados](docs/screenshots/branch-tagged.png)

**Janela dividida** (F8), cada lado com seu próprio diretório:

![Janela dividida: fotos à esquerda, código-fonte à direita](docs/screenshots/split.png)

**Visualizador** (`V`), com cores de sintaxe:

![Visualizador mostrando código Rust com cores de sintaxe](docs/screenshots/viewer.png)

**Terminal embutido** (Alt+X): um `cd` no shell leva a árvore junto, e o
Alt+O digita os arquivos marcados no prompt:

![Três fotos marcadas digitadas num wc -c com Alt+O](docs/screenshots/terminal-tagged.png)

**Git** (opcional): as letras de status na lista de arquivos e o menu Git
(Ctrl+G):

![Lista de arquivos com as letras de status do Git e o menu GIT no rodapé](docs/screenshots/git-menu.png)

**Temas**: além do azul clássico, `dark`, `light`, `mono`, `norton`,
`solarized`, `gruvbox` (abaixo) e `nord`:

![O tema gruvbox](docs/screenshots/theme-gruvbox.png)

## Instalação

**Binário pronto:** baixe o arquivo do seu sistema na página de
[Releases](https://github.com/andrelnfernandes/xylem/releases): Linux x86_64 ou
ARM64 (estático, roda em qualquer distribuição) ou Windows x86_64. Extraia e
coloque o `xy` (`xy.exe`) numa pasta do `PATH`, por exemplo:

```bash
tar xzf xy-v0.3.0-x86_64-unknown-linux-musl.tar.gz
install -m 755 xy-v0.3.0-x86_64-unknown-linux-musl/xy ~/.local/bin/
```

Cada arquivo tem um `.sha256` ao lado, para conferir com `sha256sum -c`.

**Com o Cargo** (requer Rust, via [rustup](https://rustup.rs)), a partir do
[crates.io](https://crates.io/crates/xylem-fm), onde o Xylem se chama
`xylem-fm` (o nome `xylem` já era de outro projeto); o comando é `xy`:

```bash
cargo install xylem-fm
```

Ou o código mais recente, direto do GitHub, ou a partir de um clone do
repositório (na pasta do projeto):

```bash
cargo install --git https://github.com/andrelnfernandes/xylem
cargo install --path .
```

O binário vai para `~/.cargo/bin/xy` (que o rustup já coloca no `PATH`).
Para atualizar, rode o mesmo comando de novo; para remover, `cargo uninstall xylem-fm`.

**Atalho no menu (Linux, opcional):** para ter o Xylem no menu de
aplicativos do seu desktop, abrindo num terminal, rode isto num clone do
repositório. O `xy` precisa estar no `PATH` que o desktop enxerga
(`~/.local/bin` costuma estar; `~/.cargo/bin` nem sempre).

```bash
for s in 16 32 48 64 128 256 512; do
    install -Dm644 docs/icons/xylem-$s.png ~/.local/share/icons/hicolor/${s}x$s/apps/xylem.png
done
install -Dm644 docs/icons/xylem.svg ~/.local/share/icons/hicolor/scalable/apps/xylem.svg
install -Dm644 packaging/linux/xylem.desktop ~/.local/share/applications/xylem.desktop
```

**Página de manual (opcional):** o `xy` imprime a sua (os arquivos das
releases para Linux também trazem o `xy.1`):

```bash
mkdir -p ~/.local/share/man/man1 && xy --man > ~/.local/share/man/man1/xy.1
```

## O que ele faz

- **O jeito XTree:** uma árvore de diretórios logada na memória, a janela de
  arquivos embaixo, Branch e Showall para listar os arquivos de um ramo
  inteiro ou de tudo o que foi logado, tags sobre as quais agem as operações
  em grupo, e as teclas do ZTreeWin.
- **Operações com arquivos:** copiar e mover (com ou sem os caminhos, com
  progresso e detalhes dos conflitos), apagar para a lixeira com Oops para
  restaurar, Prune, renomear com máscaras, permissões e datas, Compare entre
  duas janelas.
- **Achar coisas:** Filespec por nome, data e tamanho, Treespec para digitar um
  caminho, bookmarks, e Ctrl+S para buscar texto nos arquivos marcados.
- **Olhar dentro:** um visualizador de texto e hex com busca e cores de
  sintaxe, Autoview ao lado da lista, arquivos ZIP, 7z e TAR abertos como
  diretórios, e modos de exibição que mostram o dono ou o que cada arquivo é.
- **Além do XTree:** um terminal embutido cuja árvore acompanha o shell,
  máquinas remotas via SSH (navegar e copiar delas, com o terminal de lá), um
  menu Git opcional, a área de transferência, uma tela de configuração e
  temas.

## Uso

```bash
xy [diretório]
```

Sem argumento, abre o diretório atual. `xy --help` mostra as opções. Para o
shell ficar no diretório em que você estava ao sair com Alt+Q, acrescente a
pequena função de shell de
[Quitting and changing the shell's directory](docs/manual/quit.md).

## Documentação

**F1** (ou **Alt+H**, nos terminais que guardam o F1 para si, como o GNOME
Terminal) abre a ajuda sobre o que você está fazendo: dentro de um prompt, a
ajuda daquele prompt; numa janela, a ajuda daquela janela, com links para o
resto.

A mesma ajuda é [o manual](docs/manual/index.md), que dá para ler aqui no
GitHub, e a página de manual (`xy --man`); os dois estão em inglês. Alguns
tópicos: [a janela da árvore](docs/manual/tree.md),
[a janela de arquivos](docs/manual/files.md),
[tags e operações em grupo](docs/manual/tagging.md),
[Copy e Move](docs/manual/copy-move.md), [Filespec](docs/manual/filespec.md),
[o terminal embutido](docs/manual/terminal.md), [Git](docs/manual/git.md),
[configuração](docs/manual/configuration.md),
[Windows](docs/manual/windows.md) (experimental) e a
[comparação com o ZTreeWin](docs/manual/ztreewin.md).

## Estrutura

- `src/model/` — árvore de diretórios em memória (arena), sem dependência da UI
- `src/app/` — estado global (prompts, operações) e execução de comandos
- `src/pane.rs` — uma janela: árvore, lista de arquivos, cursores e filespec
- `src/input.rs` — comandos e o campo de texto dos prompts
- `src/keymap.rs` — teclas padrão e trocas da configuração
- `src/help.rs` — a ajuda (F1): tópicos, links e a página de manual
- `docs/manual/` — o manual: os tópicos da ajuda, em Markdown (em inglês)
- `src/filetype.rs` — o que um arquivo é, para o modo de exibição Tipo
- `src/ui.rs` — desenho da tela (ratatui)
- `src/viewer.rs` — visualizador texto/hex
- `src/archive.rs` — leitura de ZIP, 7z e TAR; criação de ZIP e TAR
- `src/watch.rs` — avisos de mudanças no disco (inotify)
- `src/shell.rs` — editor e comandos do shell (com aspas seguras)
- `src/editors.rs` — qual editor o E usa para cada arquivo (`[editors]`)
- `src/term.rs` — terminal embutido (pseudoterminal, códigos de teclas, cores)
- `src/git.rs` — menu Git (Ctrl+G) e marcadores de estado, pelo programa `git`
- `src/config.rs` — arquivo de configuração, temas e formatos
- `src/settings.rs` — tela de configuração (F12)
- `src/state.rs` — histórico dos prompts entre execuções
- `src/model/compare.rs` — regras do Compare
- `src/model/filespec.rs` — filtro de arquivos
- `src/ops/` — copiar, mover e apagar, em passos (progresso, conflitos, cancelamento)
- `docs/screenshots/tools/` — refaz as capturas de tela do README (`run.sh`, Linux)
- `tests/e2e.rs` — testes de ponta a ponta: o `xy` de verdade num pseudo-terminal, dirigido por teclas (Linux); o `cargo test` também roda testes de propriedades (proptest) do Filespec e das máscaras de renomeação
- `docs/icons/` — o ícone, em SVG, PNG (16 a 512) e ICO
- `build.rs` — embute o ícone no `xy.exe` (Windows)
- `packaging/linux/` — atalho para o menu (`xylem.desktop`)

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
- versão para Windows: compila e passa nos testes (veja
  [Windows](docs/manual/windows.md)); falta usar no dia a dia.
  Talvez macOS
- os itens de "Not implemented" da
  [comparação com o ZTreeWin](docs/manual/ztreewin.md)

## Licença

O Xylem é distribuído sob a [licença MIT](LICENSE).
