# Edit, eXecute e Batch

## Edit (E)

Abre o arquivo no editor: o definido na página 1 da
[configuração](configuration.md), ou então `$VISUAL`, `$EDITOR`, o `editor` do
Debian e do Ubuntu, ou `vi`.

Por padrão, o editor abre **dentro da moldura do Xylem** (`editor_in_frame = true`
na página 1 do **F12**) e usa o **tema de cores do Xylem** (`editor_theme = true`),
incluindo cores de sintaxe e interface compatíveis com o tema ativo quando o
editor é `vi`, `vim` ou `nvim`. Defina `editor_in_frame = false` para entregar
todo o terminal ao editor, ou `editor_theme = false` para manter o esquema de
cores próprio do editor.

### Arquivos binários e editores por tipo de arquivo

Um editor de texto só estraga uma imagem ou um programa, por isso o `E` nunca
abre **arquivos binários** (aqueles com um byte nulo próximo ao início, que o
visualizador mostra em hexadecimal) no editor de texto.

Em vez disso, quando **Edit binary files with E** está ativado
(`edit_binary = true`, tecla **I** na página 1 do **F12**), o `E` abre arquivos
binários em um editor hexadecimal/binário:

1. Um padrão correspondente em `[editors]` (se houver), ou
2. O **Binary editor** definido na página 1 do **F12** (`binary_editor`, tecla
   **J**) ou `binary` em `[editors]`, ou
3. Se deixado em branco, o primeiro editor hexadecimal de terminal encontrado no
   `PATH`:
   - **Linux**: `hexedit`, `heh`, `bvi`, `dhex`
   - **Windows**: `heh` (`winget install ndd7xv.heh`), `hexedit`, `bvi`

Editores hexadecimais de terminal rodam da mesma forma que o editor de texto:
**dentro da moldura do Xylem** por padrão (`editor_in_frame = true`) e com o
**tema de cores do Xylem** (`editor_theme = true`). Para um editor hexadecimal
gráfico (`ImHex`, `GHex`, `Okteta` no Linux; `ImHex`, `HxD` no Windows) que você
queira deixar aberto na própria janela dele, adicione `&` ao final do comando
(por exemplo, `imhex %f &` ou `ghex %f &`): o Xylem o inicia diretamente sem
abrir a moldura de terminal.

Se preferir que o `E` não abra arquivos binários (e apenas avise no rodapé
sugerindo `O` para abrir no programa do sistema), mude **Edit binary files with
E** para `no` (`edit_binary = false` na página 1 do **F12**).

Para dar um editor próprio a determinados tipos de arquivo, liste-os na tabela
`[editors]` do `config.toml`:

```toml
[editors]
"*.png *.jpg *.jpeg *.gif" = "gimp %f &"
"*.md" = "typora %f &"
"*.svg" = ""            # vazio: o editor habitual, mesmo tendo correspondido
binary = "hexedit"      # reserva para qualquer outro arquivo binário
```

Cada chave é uma lista de padrões de nomes, como no [Filespec](filespec.md)
(maiúsculas e minúsculas não importam); cada valor é uma linha de comando, com
`%f` para o arquivo (caso contrário, o arquivo vai ao final). A primeira chave
que corresponder vence, na ordem do arquivo. `binary` serve para os arquivos
binários que nenhum padrão capturou quando `binary_editor` está vazio; os demais
arquivos abrem no editor de texto habitual.

O Xylem não tem um editor hexadecimal próprio, e isso é uma decisão deliberada:
desenvolver e manter um daria muito trabalho para pouco uso, e editar bytes à
mão é um jeito fácil de corromper um arquivo. Em vez disso, o Xylem integra
editores hexadecimais dedicados direto dentro da sua moldura e com o seu tema de
cores — e permite desativar a edição de binários com uma única opção no **F12**
se você não quiser essa integração.

## eXecute (X)

Pede uma linha de comando e a executa com `sh` no diretório atual. Nela:

| | |
|---|---|
| `%f` | o arquivo sob o cursor |
| `%t` | os arquivos marcados |
| `%d` | o diretório atual |
| `%%` | um `%` literal |

Todos eles já saem entre aspas: `tar czf backup.tgz %t`. Ao final, o Xylem
aguarda Enter para que você possa ler a saída, e os diretórios abertos são
relidos. Um comando começando com `@` não é guardado no histórico (com
`history_skip_at`).

Para sessões longas, o [terminal integrado](terminal.md) (Alt+X) costuma ser
mais prático.

## Batch (Ctrl+B)

Pede o nome do arquivo a ser criado (F2 [escolhe a pasta](picker.md)) e depois
uma máscara de linha, e escreve uma linha por arquivo marcado. Parâmetros:

| | |
|---|---|
| `%f` | caminho |
| `%d` | diretório |
| `%n` | nome |
| `%b` | nome sem extensão |
| `%e` | extensão |
| `%s` | tamanho |

Em minúsculas, saem entre aspas, seguros para o shell (`convert %f %b.png`); em
maiúsculas, como texto puro, para listas (`%N;%S`). Um arquivo `.sh` recebe
`#!/bin/sh` e permissão de execução (execute-o com `X`). Um arquivo existente
não é sobrescrito.

## Histórico

Em todos os prompts, ↑ e ↓ trazem entradas anteriores. Cada tipo de prompt
(Filespec, destino, Log, eXecute...) tem seu próprio histórico, salvo em
`~/.local/state/xylem/history.toml`, legível apenas por você, já que pode
conter comandos digitados.
