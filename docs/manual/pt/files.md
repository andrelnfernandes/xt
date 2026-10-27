# A janela de arquivos

A janela de arquivos lista os arquivos do diretório sob o cursor da árvore, de
um ramo inteiro (`B`) ou de tudo o que foi lido (`S`). **Enter** volta para a
árvore, e **Esc** também (no diretório do arquivo selecionado).

## Navegação

| Tecla | O que faz |
|---|---|
| ↑ ↓ PgUp PgDn Home End | move o cursor |
| ← → | sobe e desce uma página (nos modos em colunas: arquivo anterior e próximo) |
| Backspace | mostra os arquivos do diretório pai |
| Shift+letra, 0–9 | próximo arquivo começando com essa letra ou dígito |
| Space | desce uma linha |

## Modos de exibição (Alt+F)

Alt+F alterna, na ordem do ZTreeWin, entre **nomes em várias colunas**, **nomes
e tamanhos em colunas**, **detalhes** (uma coluna com tamanho, data e
permissões, o padrão) e **nomes longos** (uma coluna, apenas nomes completos).

Antes dos nomes longos vêm dois modos que o ZTreeWin não tem:

- **owner** (dono): tamanho, `usuário:grupo` e permissões (não exibido no
  Windows nem dentro de arquivos compactados);
- **type** (tipo): o que é cada arquivo, no espírito do comando `file` ("PNG
  image, 800 x 600", "ELF 64-bit executable", "python3 script", "Rust, UTF-8
  text"...), a partir dos primeiros bytes dos arquivos visíveis.

Nos modos em colunas, os arquivos vão da esquerda para a direita, ← → movem um
arquivo e ↑ ↓ uma linha; nomes com mais de 20 caracteres são truncados com `…`.
O modo inicial é `file_display` na [configuração](configuration.md).

## Autoview (F7)

Divide a tela entre a lista e o conteúdo do arquivo selecionado, que acompanha
o cursor. As teclas da janela de arquivos continuam funcionando (`T` marca, por
exemplo); Shift+↑ ↓ PgUp PgDn Home End rolam o conteúdo; `W` alterna a quebra
de linha automática, `#` alterna os números de linha; F7, Esc ou Enter saem.
Exibe o primeiro 1 MiB de cada arquivo (`V` exibe até 16 MiB).

## Abrir arquivos

- `V`: o [visualizador](viewer.md) (texto ou hexadecimal, com busca).
- `E`: o editor ([Edit](execute.md)).
- `O`: o programa padrão do sistema (`xdg-open`: imagens no visualizador de
  imagens, PDFs no leitor de PDF...), em segundo plano. Ctrl+O abre todos os
  arquivos marcados. Dentro de um arquivo compactado, o item é primeiro
  extraído para uma pasta temporária, excluída quando o Xylem fecha. Outro
  programa pode ser definido em `opener`.
- Alt+F5: abre um [arquivo compactado](archives.md) como um diretório.

## Caracteres de direção escondidos

Alguns caracteres invisíveis mudam a direção do texto, e por isso um nome pode parecer diferente do que realmente é (Trojan Source, CVE-2021-42574; por exemplo, `fatura\u{202E}fdp.sh` pode parecer um PDF inofensivo). O Xylem nunca os envia ao terminal: no nome eles aparecem marcados em vermelho, o rodapé avisa "Name has hidden bidirectional characters" quando o cursor está sobre um deles, e o `O` pede confirmação antes de abrir, mostrando o nome com os caracteres escritos como `\u{XXXX}`.

## A área de transferência

`Y` ou Ctrl+Insert copia o caminho do arquivo (na árvore, do diretório) para a
área de transferência; Ctrl+Y copia os caminhos dos arquivos marcados, um por
linha; `y` no visualizador copia o caminho do arquivo visualizado. Funciona via
SSH e tmux (OSC 52) e mantém a área de transferência após o Xylem fechar, no
X11 e no Wayland.

## Outros comandos

| Tecla | O que faz |
|---|---|
| `T` / `U` | marca / desmarca o arquivo e desce ([marcações](tagging.md)) |
| `C` / `M` | [copia / move](copy-move.md) o arquivo |
| `D` ou Del | [exclui](delete.md) o arquivo (Shift+Del: permanentemente) |
| `R` | [renomeia](rename.md) o arquivo; Ctrl+R renomeia os marcados com máscara |
| `A` | [permissões](attributes.md) |
| `N` | [altera a data](attributes.md) |
| Alt+S | [ordena](sort.md) |
| Ctrl+S / Ctrl+V | [busca nos arquivos marcados / visualiza-os](search.md) |
| Ctrl+B | [Batch](execute.md): um script com uma linha por arquivo marcado |
| F5 / Ctrl+F5 | cria um [arquivo compactado](archives.md) com o arquivo / os marcados |
| Alt+I | Info: detalhes do arquivo ([informações](info.md)) |

Veja também: [a janela da árvore](tree.md), [as teclas em vigor](keys.md).
