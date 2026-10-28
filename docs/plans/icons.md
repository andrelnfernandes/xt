# Spec: suporte a fontes com ícones (Nerd Fonts)

Origem: conversa de 2026-10-27. O autor perguntou sobre o uso de fontes com glifos
e ícones (como Fira Code / Nerd Fonts) para enriquecer graficamente a interface do
Xylem sem perder a fidelidade clássica do XTree / ZTreeWin. Revisada em 2026-10-28
(veja [Revisão](#revisão-2026-10-28)).

## Objetivo

Permitir que usuários que utilizam emuladores de terminal com **Nerd Fonts** possam
habilitar ícones visuais para arquivos, extensões, diretórios e títulos de janela,
de forma totalmente opcional e sem quebrar o alinhamento de colunas da interface.

## Princípios e Regras

1. **Opt-in estrito (desligado por padrão):** O padrão é `icons = "none"`. O visual
   clássico retrô (ASCII puro e box-drawing) permanece intocado para máxima
   compatibilidade com consoles virtuais (`/dev/tty1`), conexões SSH remotas e
   terminais sem fontes customizadas. Desligado, nada do caminho dos ícones roda
   (nem a verificação de repositório na árvore).
2. **Geometria e alinhamento sagrados:** O Xylem baseia-se em colunas rigorosamente
   alinhadas (especialmente nos modos multi-coluna `Alt+F` e estatísticas). Quando
   habilitado, o slot do ícone ocupa exatamente `icons::SLOT_WIDTH` = 2 células
   (1 glifo + 1 espaço, onde os glifos largos das variantes não-Mono transbordam).
   Toda conta de largura usa essa constante; um teste garante que todo ícone mede
   isso e outro que a coluna de detalhes não se move entre linhas.
3. **Desempenho:** mapeamento estático por `match`, sem tabela em memória. A busca
   aloca 2 a 3 `String` pequenas por arquivo visível a cada quadro (nome e extensão
   em minúsculas); só as linhas visíveis passam por ali, então o custo é desprezível
   e não cresce com o tamanho do diretório.
4. **Respeito aos temas:** Os ícones herdam as cores do tema ativo: na lista de
   arquivos, o estilo do nome; na árvore, `theme().key` (ou `theme().error` se o
   diretório não pôde ser lido).
5. **Status do Git em letras:** o marcador `XY` continua em letras com os ícones
   ligados. Ele separa o índice da árvore de trabalho (`M ` × ` M` × `MM`) e cobre
   todas as combinações (`AM`, `RM`, `UU`, `??`...); um ícone por status perdia essa
   informação e misturava ícones com letras na mesma coluna.
6. **O desenho nunca toca o disco:** o ícone de repositório sai do que a árvore já
   leu (um `.git` entre os subdiretórios ou arquivos do nó), nunca de um `stat`
   durante o desenho, como já faz o `StatusCache`.

## Configuração

### Arquivo `config.toml`
```toml
# Estilo de ícones na interface:
# "none"       - Clássico, sem ícones (padrão)
# "nerd-fonts" - Ícones modernos usando faixas PUA do Nerd Fonts (v3.x)
icons = "none"
```

### Tela de configurações (F12 / Alt+F10)
- **Localização:** Página 2 (*Display and sorting*).
- **Entrada:** Letra `M` (logo após `L` - `viewer_max_size`).
  ```text
  M  Icons and glyphs (Nerd Fonts)                 none / nerd-fonts
  ```
- **Comportamento:** Pressionar `M` alterna entre `none` e `nerd-fonts`, com efeito
  imediato na interface.

## Mapeamento de Glifos (Nerd Fonts v3.x PUA)

Os comentários em `src/icons.rs` trazem o **nome oficial** de cada glifo, como em
`glyphnames.json` do repositório `ryanoasis/nerd-fonts`, e não o glifo em si: sem
uma Nerd Font ele não aparece no editor, e foi assim que a troca de pastas passou
despercebida na primeira versão. Todos os nomes abaixo foram conferidos nesse
arquivo (branch `master`, 2026-10-28). A tabela completa de extensões está em
`src/icons.rs`; aqui ficam as escolhas que pedem justificativa.

### 1. Diretórios na árvore (`DirKind`)
- Fechado (`Closed`): `\u{f07b}` `fa-folder`
- Aberto (`Open`, nó expandido): `\u{f07c}` `fa-folder_open`
- Repositório Git (`Repo`, tem `.git` diretório ou arquivo de worktree/submódulo):
  `\u{e5fb}` `custom-folder_git`
- Raiz de pacote aberto como pasta (`Archive`): `\u{f410}` `oct-file_zip`
- Raiz de máquina remota (`Remote`, SFTP): `\u{f048b}` `md-server`

Prioridade: pacote/remoto (só na raiz) > repositório > aberto/fechado.

### 2. Título da janela de arquivos (`view_icon`)
- `Files`: `\u{f07c}` `fa-folder_open` (os arquivos do diretório sob o cursor)
- `Branch`: `\u{f0645}` `md-file_tree`. Branch é a visão do XTree com os arquivos
  de uma subárvore, **não** o ramo do Git; a primeira versão usava
  `dev-git_branch` (`\u{e725}`), que sugeria o contrário.
- `Showall`: `\u{f0349}` `md-magnify`

### 3. Arquivos (`file_icon`)
Ordem: link simbólico > nome exato > extensão > executável > genérico.
- Link simbólico: `\u{f0c1}` `fa-link`
- `LICENSE`, `LICENCE`, `LICENSE.md`, `LICENSE.txt`: `\u{f495}` `oct-law` (a
  primeira versão usava `\u{f48e}`, que é `oct-trash`, uma lixeira)
- Executável sem extensão conhecida: `\u{f013}` `fa-gear` (a primeira versão usava
  `\u{f0633}`, que é `md-apple_keyboard_command`, o símbolo ⌘)
- Genérico: `\u{f0214}` `md-file`
- Linguagens, dados, documentos, pacotes, mídia e fontes: veja `match_extension`.

### 4. Não implementado
- **Ponto de montagem / raiz de disco** (`\u{f02ca}` `md-harddisk`): saber se um
  diretório é ponto de montagem exige comparar o dispositivo com o do pai, o que a
  árvore não guarda; e a raiz da árvore costuma ser um diretório comum (o inicial),
  onde um disco seria enganoso.
- **Ícones de status do Git**: descartados (princípio 5).

## Roteiro de Implementação

1. **Módulo `src/icons.rs`:** [Concluído]
   - `SLOT_WIDTH`, enum `DirKind`.
   - `fn file_icon(name: &OsStr, kind: FileKind, is_exec: bool, style: IconStyle) -> Option<&'static str>`.
   - `fn dir_icon(kind: DirKind, style: IconStyle) -> Option<&'static str>`.
   - `fn view_icon(view: FileView, style: IconStyle) -> Option<&'static str>`.
   - O enum `IconStyle` fica em `src/config.rs`, como os demais `Choice`.
2. **Configuração em `src/config.rs` e `src/settings.rs`:** [Concluído]
   - Campo `icons: IconStyle` em `Options` (padrão `none`).
   - Opção `M` na página 2 de `PAGES` em `settings.rs`.
3. **Modelo em `src/model/tree.rs`:** [Concluído]
   - `Tree::has_git(id)`: `.git` entre os subdiretórios ou arquivos já lidos.
4. **Renderização em `src/ui.rs`:** [Concluído]
   - Lista de arquivos em modo detalhado (`file_line`).
   - Modo multi-coluna `Alt+F` com ajuste exato de largura (`draw_file_columns`).
   - Árvore de diretórios e picker de pastas (`draw_tree`, `dir_kind`).
   - Título da janela de arquivos (`draw_files`, via `icons::view_icon`).
5. **Documentação e Testes:** [Concluído]
   - Manuais nas 6 línguas (`docs/manual/**/configuration.md`), com a nota sobre
     terminais que desenham caracteres de largura ambígua como largos (comum em
     ambientes CJK): os ícones da área privada ficam com 2 células e desalinham.
   - `src/icons.rs`: `none_style_returns_none`, `nerd_fonts_returns_appropriate_icons`,
     `dir_icons_follow_state`, `every_icon_fills_the_slot`.
   - `src/config.rs`: `icons_is_read`.
   - `src/ui.rs`: `icons_switch_in_file_list`,
     `icons_keep_the_details_column_with_git_status`,
     `git_status_letters_stay_with_icons`, `branch_title_is_not_a_git_branch`,
     `tree_marks_git_repositories`.
   - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test`
     verdes (427 + 7 testes).

## Revisão (2026-10-28)

Revisão da primeira versão (commits `19449be`, `44bce39`, `ae41721`), aceita pelo
autor. Cada correção começou por um teste que falhou pelo motivo certo.

1. **Desalinhamento com status Git:** `file_line` somava `String::len` (bytes) do
   marcador; com o ícone de 3 bytes, as linhas modificadas ficavam 2 colunas mais
   curtas (78 em vez de 80) e a coluna de detalhes andava. Resolvido com a volta
   das letras (item 2) e coberto por `icons_keep_the_details_column_with_git_status`.
2. **Letras do Git de volta** (princípio 5): `git_icon` e `git_marker_text`
   removidos.
3. **Pastas trocadas:** aberta mostrava `fa-folder` e fechada `fa-folder_open`.
4. **Branch com ícone de ramo do Git:** trocado por `md-file_tree`.
5. **Ícone de repositório nunca aparecia** (`is_repo` fixo em `false`): agora vem
   de `Tree::has_git`; quatro `bool` posicionais viraram `DirKind`.
6. **Glifos errados achados ao conferir os nomes:** LICENSE (lixeira → `oct-law`)
   e executável (⌘ → `fa-gear`).
7. **Largura do slot centralizada** em `SLOT_WIDTH` (antes, `2` fixo em 3 lugares)
   e **glifos de título centralizados** em `icons::view_icon`.
8. **Plano corrigido:** a promessa de "zero alocação", o mapeamento das pastas e os
   itens marcados como concluídos sem estar (repositório, raiz de disco).
