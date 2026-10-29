# Comparação com o ZTreeWin

## Antes de tudo, um agradecimento

Uso alguma variante do XTree desde os anos 1990 e tenho uma licença paga do
ZTreeWin desde 2013. Para mim, o ZTreeWin continua sendo o melhor gerenciador
de arquivos em modo texto que existe: rápido, previsível, todo no teclado, e
construído em torno de um jeito de trabalhar (ler a árvore, marcar arquivos em
ramos inteiros, agir sobre as marcações) que nenhum outro gerenciador de
arquivos igualou. Kim Henkel mantém vivo o jeito XTree no Windows há décadas,
e o Xylem não existiria sem esse trabalho.

O Xylem não pretende substituir o ZTreeWin. Ele existe porque eu queria o
mesmo jeito de trabalhar no Linux, onde o ZTreeWin não roda nativamente. O
Xylem também compila para Windows, porque o Rust torna isso fácil, mas essa
não é uma meta minha: no Windows, o ZTreeWin vale muito a licença:
[ztree.com](https://www.ztree.com/), e o [fórum](https://groups.io/g/ZTreeWin)
dos seus usuários.

Esta página compara os dois comando por comando, com base no manual oficial do
ZTreeWin v2.2 ([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf), de
2011, em especial a "Quick Command Table" e a seção 3.7 sobre configuração).
Versões mais recentes do ZTreeWin podem ter mudado alguns detalhes; se algo
aqui estiver desatualizado, abra uma issue.

## O mesmo jeito de trabalhar

Árvore com leitura sob demanda (`+`, `*`), janela de arquivos, Filespec,
Branch, Showall, marcações (T, U, I, Ctrl+T/U), Copiar/Mover/Excluir/Renomear
com as versões Ctrl (marcados) e Alt (marcados com caminhos), Criar diretório,
Excluir diretório, Prune, Atributos, Visualizador, Edit, eXecute, Comparar
(diretório e ramo), Ordenar, Log, tela dividida (F8 e Tab), reler (F3), Abrir
(`O`), atualização automática, navegação e extração de arquivos compactados,
histórico nos prompts, favoritos, ajuda (F1) e tela de configuração. As teclas
são as do ZTreeWin sempre que possível; as exceções estão em "Teclas
diferentes", no fim.

## O que o Xylem ainda não tem

- **Árvore:** Global (G), Graft (Alt+G), Mirror (Alt+M), Release/descarregar da
  memória (Alt+R, `-`, Ctrl+Backspace), Hide (Alt+H), Wash (Alt+W), Volume
  (Ctrl+V), F5/F6 (ocultar níveis de subdiretórios), marcar por atributo
  (Alt+T/U), Treespec em modo de edição (F5, curingas) e pastas especiais (F6),
  busca ao digitar com prompt (`|`) e modo ativo, troca de unidade (`<` e `>`).
- **Arquivos:** Abrir em nova janela de terminal (Alt+O), Batch do arquivo
  atual (Alt+B), JFC (J, comparar o conteúdo de dois arquivos), sHortcut (H),
  Print (P), dono/grupo (chown), Megaview (Ctrl+F7 na janela de arquivos),
  mesclar arquivos marcados entre janelas (Ctrl+F6), Partial Untag (Ctrl+F8).
- **Geral:** macros (F12), menu de aplicações (F9), Sair e abrir shell
  (Ctrl+Q), salvar e carregar o log (Zlist, Ctrl/Alt+Z), cores por nome e
  atributo (ZCOLORS.INI), mouse, opções de linha de comando do ZTreeWin,
  comandos de vídeo (Alt+F6–F9).

Para alguns deles há um substituto em outro programa; veja "O que o Xylem
deixa para outros programas", mais abaixo.

## Onde o Xylem é mais simples

| Recurso | No ZTreeWin | No Xylem |
|---|---|---|
| Filespec | nome com regras próprias de nome e extensão, data, tamanho, atributo, filespec invertido (Ctrl+I), rótulos | glob para nomes, data e tamanho como no ZTreeWin (com K/M/G); sem atributos, inversão ou rótulos |
| Visualizador | modos Alpha, Dump, Hex, Junk e Wrap; conjuntos de caracteres; edição hexadecimal; favoritos; rolagem automática | texto ou hexadecimal, com a busca do ZTreeWin (F, B, /, \\, Space, + e -), quebra de linha (W), números de linha (#), ir para linha (g / :); até o limite de tamanho (`viewer_max_size`, de 16 MiB a 1 GiB); sem edição hexadecimal, por opção (veja "O que o Xylem deixa para outros programas") |
| Ctrl+S | texto, hexadecimal ou Unicode; busca apenas no índice do ZIP | texto ou hexadecimal |
| Atributos | atributos do Windows (R, A, S, H) | permissões Unix em octal (`644`); sem dono/grupo. No Windows, R, H, S e A, como no ZTreeWin |
| Ordenar | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; igual/diferente na tela dividida | Name, Ext, Date, Size, crescente/decrescente, Path primeiro; cada janela tem sua própria ordem |
| Comparar | opções Identical, Unique, Newer, Older, Size, Binary, Case; filtro de exibição | marca únicos, mais novos, mais antigos, idênticos, diferentes ou únicos+mais novos; sem comparação binária |
| Prune | exclui apenas os subdiretórios ou também o atual; apenas diretórios vazios; lixeira | sempre exclui o diretório inteiro, permanentemente; confirmação configurável |
| Log | várias unidades lidas, Global, troca de unidade | uma raiz por janela; `L` abre outro diretório |
| eXecute | parâmetros de lote `%1`–`%8`, nova janela (NTW) | `sh` com `%f`, `%t` e `%d`; sempre no terminal do Xylem |
| Histórico | janela de histórico com favoritos | ↑/↓ no prompt, um histórico por tipo, salvo entre sessões |
| Configuração | 7 páginas, editor de cores por elemento (F4), Alt-Export | 4 páginas com as opções que fazem sentido no Linux; F4 escolhe entre 12 temas prontos; cores individuais em `config.toml` |
| Tela dividida | painéis opcionais de estatísticas por lado | sem painel de estatísticas |
| Menus | menu Ctrl/Alt ao segurar a tecla | igual em terminais com o protocolo do kitty; F9 em qualquer terminal |

Opções de configuração do ZTreeWin que não foram trazidas por serem específicas
do Windows: atributos RSH, junções (*junctions*), OEM/ANSI, EBCDIC, tamanho da
janela, atalhos (.lnk).

## Onde o Xylem vai além

Coisas para as quais o manual do ZTreeWin v2.2 não tem equivalente:

- **Um terminal integrado** (Alt+X): o shell continua rodando enquanto você usa
  a árvore; um `cd` no shell leva a árvore junto, e o shell acompanha a árvore
  quando você volta a ele. Alt+O digita os arquivos marcados no prompt. Veja
  [o terminal integrado](terminal.md).
- **Máquinas remotas via SSH** (`xy usuario@host:`): a árvore, o Branch, o
  visualizador e o Copiar funcionam em outra máquina, que também tem seu
  próprio terminal. Veja [máquinas remotas](remote.md).
- **Git** (opcional): letras de status na lista de arquivos, um menu Git
  (Ctrl+G) e diffs no visualizador. Veja [Git](git.md).
- **Uso do disco** (Alt+W): pastas e arquivos de um ramo juntos, os maiores
  primeiro, até chegar ao que enche o disco; Alt+K mostra a fatia de cada pasta
  na árvore. Veja [uso do disco](usage.md).
- **Um visualizador que lê mais:** cores de sintaxe, a codificação do texto
  detectada sozinha (UTF-8, UTF-16, CP850, Windows-1252, CP437), arquivos
  `.gz`, `.xz`, `.zst` e `.bz2` mostrados descompactados, e caracteres ocultos
  de direção do texto (Trojan Source) mostrados em vez de obedecidos. Veja
  [o visualizador](viewer.md).
- **Arquivos compactados sem compactador externo:** ZIP, 7z e TAR (puro, gzip,
  xz, bzip2, zstd) abrem como pastas, e ZIP e TAR são criados, tudo pelo
  próprio Xylem. Veja [arquivos compactados](archives.md).
- **Renomear em lote** com busca e substituição (`s/foo/bar/`), contadores e
  mudança de maiúsculas/minúsculas. Veja [renomear](rename.md).
- **Oops:** O (ou Ctrl+Z) devolve ao lugar de origem o que foi enviado por
  último para a lixeira do sistema. Veja [excluir](delete.md).
- **A área de transferência em todo lugar:** Y copia o caminho mesmo via SSH
  ou dentro do tmux (OSC 52).
- **O que é cada arquivo:** um modo de exibição (Alt+F) que diz, no espírito
  do comando `file`, o que cada arquivo contém, e outro com donos e permissões.
- **Aparência:** 12 temas prontos e ícones Nerd Fonts opcionais. Toda tecla
  pode ser redefinida em `[keys]` no `config.toml`. Veja
  [configuração](configuration.md).
- **Arquivos hostis:** arquivos compactados, nomes e conteúdos são tratados
  como potencialmente hostis, com limites em tudo o que é lido e descompactado
  (veja a declaração de segurança no README).
- **Ajuda em seis línguas** (F1, depois L), o mesmo texto deste manual e da
  página man.

## O que o Xylem deixa para outros programas

Algumas coisas o Xylem não faz sozinho, de propósito: um programa dedicado as
faz melhor, e o Xylem o traz para dentro.

- **Edição hexadecimal:** o ZTreeWin edita bytes no próprio visualizador. O
  Xylem não tem um editor hexadecimal próprio: escrever e manter um seria muito
  trabalho para pouco uso, e editar bytes à mão é um jeito fácil de corromper
  um arquivo. Em vez disso, E num arquivo binário abre o `hexedit`, `heh`,
  `bvi` ou `dhex` (o que estiver instalado, ou o que você escolher) dentro da
  moldura e das cores do Xylem, ou um gráfico como o ImHex ou o GHex. Veja
  [Edit](execute.md).
- **Editar texto:** o seu editor (`$VISUAL`, `$EDITOR` ou o definido no F12),
  e um editor diferente por tipo de arquivo, se você quiser (`[editors]`).
- **SSH:** o `ssh` do sistema, então suas chaves, o agente, o `~/.ssh/config`
  e os *jump hosts* funcionam como em qualquer outro lugar.
- **Armazenamento em nuvem:** Google Drive, S3 e outros via `rclone mount`; o
  Xylem reconhece a montagem e se adapta. Veja
  [armazenamento em nuvem](cloud.md).
- **Git:** o próprio programa `git`.
- **Abrir arquivos** (O): o programa do ambiente gráfico para cada tipo
  (`xdg-open`).
- **Comparar o conteúdo de dois arquivos** (JFC no ZTreeWin): enquanto não
  estiver embutido, marque os dois arquivos e rode `diff %t` com o eXecute (X),
  ou use o terminal.
- **Imprimir** (P no ZTreeWin): `lp %t` com o eXecute.
- **Outros formatos de arquivo compactado** (o ZAAP do ZTreeWin): o seu
  compactador, com o eXecute ou no terminal.

## Teclas diferentes

As teclas seguem as do ZTreeWin. As exceções:

| Função | ZTreeWin | Xylem |
|---|---|---|
| Ajuda | F1 | F1 ou Alt+H (no GNOME Terminal, F1 abre a ajuda do terminal) |
| Configuração | Alt+F10 | F12 ou Alt+F10 (no GNOME, Alt+F10 maximiza a janela e nunca chega ao Xylem) |
| Favoritos | F11 | F11, Alt+B ou aspas simples `'` (no GNOME Terminal, F11 alterna a tela cheia) |
| Mover marcados | Ctrl+M | Ctrl+M em terminais com o protocolo do kitty; nos demais, `F9` `M` (ou Alt+M e F6) |
| `-` na árvore | descarrega o ramo da memória | apenas recolhe (continua lido na memória) |
| F2 | (nada) | relê o diretório, igual a F3 |
| F9 | menu de aplicações (não implementado) | menus Ctrl/Alt |
| F12 | macros (não implementado) | configuração |
| `/` | igual a `?` (estatísticas) | Treespec, como `\` (`?` continua mostrando as estatísticas) |
| Ctrl+C, Ctrl+M na árvore | (nada) | Copiar/Mover os arquivos marcados do diretório atual |
| Copiar caminho para a área de transferência | Ctrl+Insert | Y ou Ctrl+Insert (Ctrl+Y para os marcados; funciona via OSC 52 e área de transferência do sistema) |

Todas as teclas podem ser redefinidas sob `[keys]` em `config.toml`.
