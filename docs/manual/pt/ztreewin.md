# Comparação com o ZTreeWin

Um levantamento comando por comando baseado no manual oficial do ZTreeWin v2.2
([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf), de 2011, em
especial a "Quick Command Table" e a seção 3.7 sobre configuração). Versões
mais recentes do ZTreeWin podem ter mudado alguns detalhes.

## Implementado

Árvore com leitura sob demanda (`+`, `*`), janela de arquivos, Filespec,
Branch, Showall, marcações (T, U, I, Ctrl+T/U), Copiar/Mover/Excluir/Renomear
com as versões Ctrl (marcados) e Alt (marcados com caminhos), Criar diretório,
Excluir diretório, Prune, Atributos, Visualizador, Edit, eXecute, Comparar
(diretório e ramo), Ordenar, Log, tela dividida (F8 e Tab), reler (F3), Abrir
(`O`), atualização automática, navegação e extração de arquivos compactados,
histórico nos prompts, ajuda (F1) e tela de configuração (Alt+F10).

## Simplificado

| Recurso | No ZTreeWin | No Xylem |
|---|---|---|
| Filespec | nome com regras próprias de nome e extensão, data, tamanho, atributo, filespec invertido (Ctrl+I), rótulos | glob para nomes, data e tamanho como no ZTreeWin (com K/M/G); sem atributos, inversão ou rótulos |
| Visualizador | modos Alpha, Dump, Hex, Junk e Wrap; conjuntos de caracteres; edição hexadecimal; favoritos; rolagem automática | texto (UTF-8, UTF-16, CP850, Windows-1252 e CP437, detectados sozinhos, O muda) ou hexadecimal, com a busca do ZTreeWin (F, B, /, \\, Space, + e -), cores de sintaxe (que o ZTreeWin não tem), quebra de linha (W), números de linha (#), ir para linha (g / :); até o limite de tamanho (`viewer_max_size`, de 16 MiB a 1 GiB); sem edição hexadecimal, por opção (um editor próprio para arquivos binários em [`[editors]`](execute.md)) |
| Ctrl+S | texto, hexadecimal ou Unicode; busca apenas no índice do ZIP | texto ou hexadecimal |
| Atributos | atributos do Windows (R, A, S, H) | permissões Unix em octal (`644`); sem dono/grupo. No Windows, R, H, S e A, como no ZTreeWin |
| Ordenar | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; igual/diferente na tela dividida | Name, Ext, Date, Size, crescente/decrescente, Path primeiro; cada janela tem sua própria ordem |
| Comparar | opções Identical, Unique, Newer, Older, Size, Binary, Case; filtro de exibição | marca únicos, mais novos, mais antigos, idênticos, diferentes ou únicos+mais novos; sem comparação binária |
| Prune | exclui apenas os subdiretórios ou também o atual; apenas diretórios vazios; lixeira | sempre exclui o diretório inteiro, permanentemente; confirmação configurável |
| Excluir | lixeira opcional | lixeira do sistema opcional (delete_to_trash, alternância T/P no prompt, Shift+Del permanente); Oops (O/Ctrl+Z) para restaurar |
| Log | várias unidades lidas, Global, troca de unidade | uma raiz por janela; `L` abre outro diretório |
| eXecute | parâmetros de lote `%1`–`%8`, nova janela (NTW) | `sh` com `%f`, `%t` e `%d`; sempre no terminal do Xylem; além de um terminal integrado (Alt+X) |
| Histórico | janela de histórico com favoritos | ↑/↓ no prompt, um histórico por tipo, salvo entre sessões |
| Configuração | 7 páginas, editor de cores por elemento (F4), Alt-Export | 4 páginas com as opções que fazem sentido no Linux; F4 escolhe entre 12 temas prontos; cores individuais em `config.toml` |
| Compactados | qualquer formato com compactador externo, criar (F5), extrair (Alt-F5) | ZIP/JAR/WAR/APK, 7z, TAR (puro, gzip, xz, bzip2, zstd): abrir (Alt+F5), extrair com Copiar, criar com F5 (ZIP, TAR e TAR.GZ) |
| Tela dividida | painéis opcionais de estatísticas por lado | sem painel de estatísticas |
| Menus | menu Ctrl/Alt ao segurar a tecla | igual em terminais com o protocolo do kitty; F9 em qualquer terminal |

Opções de configuração do ZTreeWin que não foram trazidas por serem específicas
do Windows: atributos RSH, junções (*junctions*), OEM/ANSI, EBCDIC, tamanho da
janela, atalhos (.lnk). As demais dependem de recursos ainda não implementados
(abaixo).

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

## Não implementado

- **Árvore:** Global (G), Graft (Alt+G), Mirror (Alt+M), Release/descarregar da
  memória (Alt+R, `-`, Ctrl+Backspace), Hide (Alt+H), Wash (Alt+W), Volume
  (Ctrl+V), F5/F6 (ocultar níveis de subdiretórios), marcar por atributo
  (Alt+T/U), Treespec em modo de edição (F5, curingas) e pastas especiais (F6),
  busca ao digitar com prompt (`|`) e modo ativo, troca de unidade (`<` e `>`).
- **Arquivos:** Abrir em nova janela de terminal (Alt+O), Batch do arquivo
  atual (Alt+B), JFC (J, comparar o conteúdo de dois arquivos), ZAAP
  (compactador externo), sHortcut (H), Print (P), dono/grupo (chown), Megaview
  (Ctrl+F7 na janela de arquivos), mesclar arquivos marcados entre janelas
  (Ctrl+F6), Partial Untag (Ctrl+F8).
- **Geral:** macros (F12), menu de aplicações (F9), Sair e abrir shell
  (Ctrl+Q), salvar e carregar o log (Zlist, Ctrl/Alt+Z), cores por nome e
  atributo (ZCOLORS.INI), mouse, opções de linha de comando do ZTreeWin,
  comandos de vídeo (Alt+F6–F9).
