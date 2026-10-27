# A janela da árvore

A árvore mostra os diretórios que foram lidos para a memória (*logged*). A
janela de arquivos logo abaixo lista os arquivos do diretório sob o cursor.

## Navegação

| Tecla | O que faz |
|---|---|
| ↑ ↓ PgUp PgDn Home End | move o cursor |
| → ou `+` | expande o diretório |
| ← ou `-` | recolhe o diretório (ou vai para o diretório pai) |
| Backspace | vai para o diretório pai |
| Space | desce uma linha |
| Shift+letra, 0–9 | próximo diretório começando com essa letra ou dígito |
| `\` ou `/` | [Treespec](treespec.md): vai a um diretório digitando seu caminho |
| Enter ou Tab | vai para a janela de arquivos (Tab alterna janelas na [tela dividida](split.md)) |

Na árvore, um `+` no conector (`├─+`) indica um diretório recolhido que possui
subdiretórios.

## Leitura de diretórios (Log)

Os diretórios são lidos conforme você os abre. `*` lê e expande todo o ramo sob
o cursor; durante uma leitura longa, o progresso aparece no rodapé e `Esc`
cancela. **F3** (ou F2) relê o diretório do disco. O Xylem também monitora o
diretório atual de cada janela e o relê sozinho quando outro programa o altera
(no máximo uma vez a cada meio segundo, após qualquer operação em andamento);
outros diretórios ainda precisam de F3.

`L` ([Log](treespec.md)) abre outro diretório na janela.

## Listas de arquivos

- `B` (Branch): os arquivos de todo o ramo, lendo-o primeiro.
- `S` (Showall): os arquivos de tudo o que já foi lido.
- `F` ([Filespec](filespec.md)): quais arquivos são exibidos, por nome, data e tamanho.

Em Branch e Showall, `\` vai para o diretório do arquivo selecionado.

## Comandos na árvore

| Tecla | O que faz |
|---|---|
| `T` / `U` / `I` | marca / desmarca / inverte os arquivos do diretório ([marcações](tagging.md)) |
| `M` | cria um subdiretório |
| `R` | renomeia o diretório |
| `D` ou Del | exclui o diretório, se estiver vazio ([excluir](delete.md)) |
| Alt+P | Prune: exclui o diretório com tudo o que há nele |
| `O` ou Ctrl+Z | Oops: restaura o que foi enviado para a lixeira |
| `N` | altera a data do diretório ([datas](attributes.md)) |
| `C` / Alt+C | [Compara](compare.md) com a outra janela / todo o ramo |
| `A` | Avail: sistemas de arquivos montados ([informações](info.md)) |
| Alt+I | Info: detalhes do diretório |
| Alt+K | tamanhos, contagens de arquivos ou porcentagens ao lado de cada diretório |
| `Y` | copia o caminho do diretório para a área de transferência |
| `X` | [eXecute](execute.md): executa um comando do shell |
| Alt+X | o [terminal integrado](terminal.md) |
| F8 | [tela dividida](split.md) |
| F11 / Alt+B | [favoritos](bookmarks.md); Ctrl+B salva um |

Veja também: [a janela de arquivos](files.md), [as teclas em vigor](keys.md).
