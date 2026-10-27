# Marcações e operações em grupo

Os arquivos marcados (indicados com `■`) são o alvo das operações em grupo: os
arquivos marcados **da lista atual** (o diretório, ou o Branch ou Showall
aberto).

## Marcar arquivos

| Tecla | Árvore | Janela de arquivos |
|---|---|---|
| `T` / `U` | marca / desmarca os arquivos do diretório | marca / desmarca o arquivo e desce |
| Ctrl+T / Ctrl+U | marca / desmarca tudo o que foi lido (que corresponda ao Filespec) | marca / desmarca toda a lista |
| `I` | inverte as marcações do diretório | inverte as marcações da lista |
| Ctrl+F7 / F8 / F9 | marca / desmarca / inverte os arquivos lidos do ramo | |

## Operações nos arquivos marcados

| Tecla | O que faz |
|---|---|
| Ctrl+C / Ctrl+M | [copia / move](copy-move.md) os arquivos para o destino |
| Alt+C / Alt+M | copia / move os arquivos, **recriando seus caminhos** |
| Ctrl+D | [exclui](delete.md) os arquivos |
| Ctrl+A | altera suas [permissões](attributes.md) |
| Ctrl+N | altera sua [data](attributes.md) |
| Ctrl+R | [renomeia](rename.md) os arquivos com uma máscara |
| Ctrl+B | [Batch](execute.md): um script ou lista com uma linha por arquivo |
| Ctrl+F5 | coloca-os em um [arquivo compactado](archives.md) |
| Ctrl+S / Ctrl+V | [busca texto neles / visualiza-os](search.md) |
| Ctrl+O | abre-os no programa padrão |
| Ctrl+Y | copia seus caminhos para a área de transferência |
| Alt+O | digita-os no [terminal integrado](terminal.md) |

Em eXecute, `%t` representa os arquivos marcados ([eXecute](execute.md)).

## Menus Ctrl e Alt (F9)

Como no ZTree, o rodapé mostra apenas os comandos de uma tecla. **F9** alterna
para o menu Ctrl, depois para o menu Alt e de volta. Com um desses menus
aberto, a próxima letra vale como Ctrl+letra ou Alt+letra (`F9` `M` = Ctrl+M);
`Esc` cancela. Em terminais com o protocolo de teclado do kitty, basta
**segurar Ctrl ou Alt** para ver o menu correspondente.

**Ctrl+M:** na maioria dos terminais, Ctrl+M chega igual a Enter. O Xylem ativa
o protocolo de teclado do kitty quando o terminal o suporta (kitty, WezTerm,
foot, Ghostty, Alacritty recente, iTerm2), e então Ctrl+M move os arquivos
marcados como no ZTree. Em terminais baseados em VTE (GNOME Terminal, Ptyxis,
Terminator, Tilix) use **F9 M** (ou Alt+M e F6 no prompt).
