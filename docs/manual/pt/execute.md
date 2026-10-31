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

## Batch no visualizador

Com uma tabela ou um texto aberto no [visualizador](viewer.md), **Ctrl+B** escreve uma linha por registro, a partir do arquivo inteiro: além das 50.000 linhas da tela, com os valores inteiros e exatamente como estão no arquivo. Ele respeita o filtro em uso (o filtro SQL na tabela, o [filtro de linhas](filter.md) no texto). A tabela é relida na hora; o texto sai como o visualizador leu, até o limite de tamanho, e a prévia avisa quando isso não é o arquivo inteiro. Ele pede o arquivo, depois a máscara, e mostra as primeiras linhas antes de gravar: **Enter** grava, **Esc** desiste. Durante a gravação, Esc cancela e remove o arquivo incompleto. Um arquivo existente não é sobrescrito.

Enquanto você digita a máscara, um quadro acima dela mostra uma máscara que funciona neste arquivo, os campos dele e os templates que você tem, cada um com a primeira frase do comentário inicial (`%%#`). **↑↓** escolhem um template, **F3** o mostra inteiro antes de usar e **PgUp**/**PgDn** percorrem o histórico. A descrição foi escrita por quem fez o template: leia com F3 um template de outra pessoa antes de usá-lo. Um erro na máscara mantém o prompt aberto com o que você digitou e diz o que está errado.

| Campo | Valor |
|---|---|
| `%{nome}` `%{A}` `%{2}` | uma coluna da tabela, por nome, letra ou posição |
| `%{line}` | a linha inteira do texto, com as tabulações |
| `%{ip}` `%{1}` `%{0}` | um grupo da expressão regular do filtro de linhas, por nome ou número; 0 é tudo o que ela encontrou |
| `%{#}` | o número do registro entre os escritos: 1, 2, 3... |
| `%{#line}` | o número da linha no arquivo (só no texto) |
| `%{#path}` `%{#dir}` `%{#name}` `%{#base}` `%{#ext}` `%{#size}` | o arquivo aberto, com escape como qualquer campo; `%N` e os outros acima não têm, então prefira estes em uma página ou um script |
| `%{$NOME}` | uma variável de ambiente, como `%{$PATH}`, com o valor de agora; a prévia lista as lidas, e uma que não existe é erro |
| `%{#date}` `%{#time}` `%{#datetime}` `%{#utc}` | agora, local (`#datetime` com o fuso) ou UTC: igual em todos os registros |
| `%{#year}` `%{#month}` `%{#day}` `%{#weekday}` `%{#weekday_name}` `%{#epoch}` | partes disso: o dia da semana 1 é segunda, o nome é em inglês, epoch em segundos desde 1970 |
| `%{#user}` `%{#host}` `%{#template}` `%{#output}` | quem gera, em qual máquina, com qual template, e o arquivo gravado |
| `%f %d %n %b %e %s` | o arquivo aberto, como no Batch acima |
| `%%` | um `%` |

Um campo que não existe é erro na prévia, nunca um valor vazio. Como a prévia olha no máximo 1.000 registros, uma chave de JSON Lines que só apareça depois deles conta como inexistente.

Os valores vêm de um arquivo que pode ser hostil e podem ir para algo que você vai executar, então cada campo tem um escape. Escreva-o depois do campo: `curl "https://api/x?id=%{id:url}"`.

| Escape | Para | Padrão em |
|---|---|---|
| `:sh` | um argumento entre aspas de um comando do shell | `.sh` `.bash` `.zsh` |
| `:url` | uma parte de uma URL, com percent-encoding | |
| `:json` | um valor JSON: número sem aspas, texto com aspas | `.json` `.jsonl` `.ndjson` |
| `:xml` ou `:html` | texto ou atributo | `.xml` `.html` `.htm` `.xhtml` `.svg` |
| `:sql` | um literal com aspas, `NULL`, `X'00ff'` | `.sql` |
| `:mysql` | o mesmo para MySQL e MariaDB, onde a barra invertida também escapa | |
| `:csv` | um campo, entre aspas quando precisa | `.csv` `.tsv` |
| `:csvsafe` | o mesmo, com um `'` antes do que a planilha tomaria por fórmula | |
| `:line` | o valor, com caracteres de controle como espaços, para nunca abrir uma linha nova | os demais |
| `:raw` | o valor como está: **não use em um script** | |

- Um blob sai em hexadecimal, menos em `:raw` e em `:sql`, que tem a sua forma.
- Para MySQL e MariaDB, use `:mysql`, em um arquivo lido como `utf8mb4`: neles, uma barra invertida em um literal `:sql` pode fechá-lo, e a prévia avisa sobre esses valores. Com `NO_BACKSLASH_ESCAPES` no servidor, o certo é `:sql`.
- Em SQL, `%f %d %n %b %e %s` dentro de uma string são recusados: `LIKE '%foo%'` ficaria com o caminho do arquivo aberto. Escreva `%%` para ter um `%`, como em `LIKE '%%foo%%'`, ou um campo como `%{#name}` para ter o valor.
- O filtro de uma tabela vale para o arquivo todo, com as regras da tela: `code = 7` também acha uma célula com `007`, que sai como `007`. Por isso o arquivo pode ter linhas que a tela, limitada a 50.000, não mostra.
- Um `.sh` começa com `#!/bin/sh`, mas **não recebe permissão de execução**: leia antes de rodar. `.bat` e `.cmd` são recusados, porque o `cmd` expande `%VAR%` até entre aspas.
- A prévia e a mensagem final avisam sobre valores com caracteres de controle ou de direção do texto e, em um CSV, sobre valores que começam como fórmula. A prévia conta só o que olhou.

### Registros inteiros

Em um arquivo `.json`, `.jsonl`, `.ndjson`, `.csv`, `.tsv`, `.xml`, `.html` ou `.htm`, deixe a máscara **vazia** para gravar registros inteiros, com as regras do formato: um array JSON de objetos, um objeto por linha, uma tabela com linha de cabeçalho, um `<record>` por registro ou uma página com uma tabela. Para gravar só alguns campos, comece a máscara com `@`: `@id, nome, #`. O texto dá `line_number`, `line` e os grupos do filtro. Os campos são descobertos nos primeiros 1.000 registros: uma chave de JSON Lines que só apareça depois fica de fora, e a mensagem final avisa.

### Templates

Arquivos para testar tudo isso estão em [demo/](https://github.com/andrelnfernandes/xylem/tree/main/demo) no repositório: os mesmos clientes em todos os formatos, e templates prontos.

Para ter texto em volta dos registros, salve um template em `~/.config/xylem/batch/nome.tpl` e digite `:nome` como máscara; o prompt lista os que encontrar (os quatro primeiros).

```
%%# um comentário
%%header
<h1>%{#name}</h1>
<ul>
%%record
<li><a href="https://x/?id=%{id:url}">%{name}</a></li>
%%footer
</ul>
```

`%%header` e `%%footer` são opcionais e aceitam só o texto do template e os campos do arquivo aberto. `%%record` é gravado uma vez por registro e pode ter várias linhas; sem seções, o arquivo inteiro é o registro. Os escapes seguem o arquivo de saída, como em uma máscara, então um template de página deve ser gravado em um `.html`. Uma seção que não existe, uma repetida ou um campo de registro no cabeçalho ou no rodapé é erro na prévia. A prévia mostra as últimas 3 linhas do cabeçalho. Sem registros, nada é gravado.

No rodapé, `%{count}` é quantos registros foram gravados. O cabeçalho não pode ter isso, pois é gravado antes de lê-los.

Em um `.sh`, `%{$HOME}` grava o valor de agora, enquanto `$HOME` é lido quando o script roda. Um template de outra pessoa é um script dela: ele poderia colocar `%{$SEGREDO:url}` em um comando, então olhe o que a prévia diz que ele lê.

## Histórico

Em todos os prompts, ↑ e ↓ trazem entradas anteriores. Cada tipo de prompt
(Filespec, destino, Log, eXecute...) tem seu próprio histórico, salvo em
`~/.local/state/xylem/history.toml`, legível apenas por você, já que pode
conter comandos digitados.
