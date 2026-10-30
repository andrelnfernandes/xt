# O visualizador

`V` abre o arquivo no visualizador (o Autoview, F7, o mostra ao lado da lista).
Arquivos binários abrem em hexadecimal. Arquivos acima do limite de tamanho mostram
apenas o início.
Planilhas, CSV, DBF, SQLite e Parquet abrem como tabela (veja Modo tabela).

| Tecla | O que faz |
|---|---|
| ↑ ↓ PgUp PgDn Home End | rola verticalmente |
| ← → | rola horizontalmente |
| `H` | texto ou hexadecimal |
| `O` | próxima codificação (UTF-8, CP850, 1252, CP437, UTF-16) |
| `C` | ativa ou desativa as cores de sintaxe |
| `W` | ativa ou desativa a quebra de linha automática |
| `#` ou `L` | ativa ou desativa os números de linha (no modo texto) |
| `g` ou `:` | pula para uma linha (um deslocamento de byte em hexadecimal, uma linha da grade no modo tabela) |
| `&` | mostra só as linhas que combinam, texto simples ou regex ([filtrar linhas](filter.md)); no modo tabela, o filtro SQL |
| `E` | edita o arquivo; de volta ao Xylem, o visualizador mostra o novo conteúdo na mesma posição (arquivos binários somente com um editor em [`[editors]`](execute.md)) |
| `D` | com Git ativo: mostra o que mudou desde o último commit, e volta ([Git](git.md)) |
| `T` | tabela ou texto, num arquivo que abriu como tabela |
| `U` | na visão dos arquivos marcados (Ctrl+V): desmarca o arquivo |
| `y` ou Ctrl+Insert | copia o caminho do arquivo para a área de transferência |
| Esc | fecha |

## Busca

Como no ZTreeWin: **F** busca desde o início, **B** de trás para frente a
partir do fim, **/** (ou `S`) a partir da página atual e **\\** para trás a
partir dela; **Space** continua na mesma direção, **+** e **-** escolhem a
direção. A ocorrência é destacada. No prompt de busca, **F2** alterna a
diferenciação de maiúsculas e minúsculas (ao ignorar maiúsculas/minúsculas, os
acentos também são considerados: `AÇÃO` encontra `ação`) e **F4** alterna entre
texto e hexadecimal (pares de bytes como `4d 5a`). Um `*` no meio do termo
corresponde a qualquer trecho na mesma linha (`error*disk`).

## Cores de sintaxe

Código-fonte e scripts são coloridos em cerca de 150 linguagens (Rust, Python,
shell, C, JavaScript, SQL, YAML, Markdown...), reconhecidas pela extensão, pelo
nome (`Makefile`) ou pela primeira linha (`#!/bin/bash`). O padrão é
`syntax_highlight` (página 2 da [configuração](configuration.md)). As cores
permanecem legíveis sobre o fundo do tema atual, e apenas as primeiras 20.000
linhas são coloridas, e nenhuma linha com mais de 10.000 caracteres, para que
arquivos enormes nunca deixem o visualizador lento.

## Caracteres de controle de direção

Alguns caracteres invisíveis mudam a direção do texto e podem fazer um código parecer diferente do que ele realmente diz (Trojan Source, CVE-2021-42574). O visualizador nunca os esconde: cada um aparece como um `·` vermelho, e o cabeçalho informa quantos são.

## Codificação do texto

O Xylem descobre a codificação sozinho, nesta ordem: primeiro descomprime o arquivo, depois procura um BOM (UTF-8, UTF-16 LE ou BE), depois UTF-16 sem BOM, depois UTF-8. Um arquivo que não é nada disso é texto antigo: o Xylem escolhe CP850 (DOS, Europa Ocidental) ou Windows-1252 contando as letras acentuadas. Os arquivos `.reg` do Windows são UTF-16 e abrem como texto normal.

**O** mostra o arquivo na próxima codificação: UTF-8, CP850, Windows-1252, CP437, UTF-16 LE, UTF-16 BE e de volta ao UTF-8. O rodapé diz qual, e o cabeçalho mostra o nome quando não é UTF-8. Se você escolher UTF-16 num arquivo que abriu em hexadecimal, ele passa para texto. Sua escolha continua depois de editar o arquivo e voltar.

Para usar sempre uma só codificação nos textos antigos, em vez do palpite, defina `legacy_charset` como `cp850`, `cp1252` ou `cp437` (**K** na página 2 da [configuração](configuration.md)). O padrão é `auto`.

## Limite de tamanho

O visualizador lê só o início de um arquivo grande, e o cabeçalho diz `(showing first N MiB)`. Quanto ele lê é a opção `viewer_max_size` (**L** na página 2 da [configuração](configuration.md)): `auto`, `16`, `64`, `256` ou `1024` (MiB). Com `auto`, o padrão, o limite é 1/32 da RAM da máquina, nunca menos de 16 MiB e nunca mais de 256 MiB (são 16 MiB quando não é possível ler a RAM). A tela de configuração mostra o valor em uso, por exemplo `auto (256 MiB)`.

Dois casos não seguem essa regra. O Autoview sempre lê 1 MiB. Um arquivo em uma máquina remota (SFTP) é lido até 16 MiB, no máximo, porque o Esc não consegue cancelar essa leitura.

Um limite maior custa memória e tempo: com `1024`, um arquivo de texto comum desse tamanho ocupa cerca de 2,5 GiB de RAM e leva uns 5 s para abrir. Linhas muito curtas custam mais do que os seus bytes, porque cada linha tem o seu próprio controle, e o mesmo vale para tabulações e caracteres de controle de direção escondidos: o visualizador guarda linhas até cerca do dobro do limite de tamanho na memória e, passado isso, o cabeçalho diz `(showing first N lines)`. Um log de linhas comuns (uns 100 bytes) nunca é cortado por isso.

## Arquivos comprimidos

Arquivos terminados em `.gz`, `.xz`, `.zst` ou `.bz2` abrem descomprimidos, como texto. O Xylem olha os primeiros bytes do arquivo, não o nome, e abre só um nível: um `.gz` dentro de um `.gz` continua comprimido. O cabeçalho mostra `gzip → N bytes` (o formato e o tamanho depois de descomprimir).

A descompressão tem limites. O visualizador lê até o limite de tamanho, leva no máximo 3 s e usa no máximo 64 MiB de memória para o dicionário do xz ou a janela do zstd. No Autoview os limites são 1 MiB, 100 ms e 16 MiB. Quando o limite de saída ou o de tempo é atingido, o cabeçalho diz `(showing first N bytes decompressed)`. Uma nota explica o que aconteceu:

- `decompression stopped after 3 s` (100 ms no Autoview)
- `compressed data truncated or corrupt` (dados comprimidos truncados ou corrompidos)
- `needs more than 64 MiB of memory to decompress` (16 MiB no Autoview, com ` (V to view)` quando os 64 MiB do V podem bastar)

A nota some na primeira tecla, mas o cabeçalho guarda o problema: `(truncated or corrupt)`, `(incomplete: …)` quando o limite de memória parou a descompressão no meio, ou `(not decompressed: …)` quando nada pôde ser descomprimido (aí os bytes brutos aparecem em hexadecimal).

O Autoview mostra a nota no título. No modo hexadecimal (`H`) você vê os bytes descomprimidos. As cores seguem o nome sem a extensão de compressão: `app.rs.gz` é colorido como Rust.

Para um `.tar.gz` (ou `.tar.xz`...), a nota diz `TAR archive: Alt+F5 opens it as a folder` (o Alt+F5 abre o arquivo como uma pasta).

Para ver os bytes comprimidos, desligue `viewer_decompress` (**J** na página 2 da [configuração](configuration.md)).

## Modo tabela

Alguns arquivos abrem como uma grade em vez de texto. O Xylem decide pela extensão, e só para arquivos sem compressão neste disco: um arquivo de máquina remota ou de dentro de um pacote aparece como conteúdo comum, com a nota `Tables are shown only for files on this disk`.

| Formato | Extensões | Nomes das colunas |
|---|---|---|
| CSV, TSV | `.csv` `.tsv` `.tab` | a primeira linha, quando pelo menos metade dela não é número; senão A, B, C... |
| JSON Lines | `.jsonl` `.ndjson` | as chaves dos objetos |
| Planilhas | `.xlsx` `.xls` `.xlsb` `.ods` | A, B, C... (a primeira linha é uma linha como as outras) |
| dBase, Clipper, FoxPro | `.dbf`, com seu arquivo memo `.dbt` | os nomes dos campos |
| SQLite | `.db` `.sqlite` `.sqlite3` | as colunas da tabela |
| Parquet | `.parquet` | as colunas do arquivo |

O separador do CSV (vírgula, ponto e vírgula, tabulação ou barra vertical) é detectado pelo início do arquivo.

| Tecla | O que faz |
|---|---|
| ↑ ↓ | linha anterior ou seguinte |
| ← → | coluna anterior ou seguinte |
| PgUp PgDn | uma página de linhas para cima ou para baixo |
| Home End | primeira ou última coluna |
| `[` `]` | aba anterior ou seguinte de uma planilha, ou tabela de um banco SQLite, dando a volta; isso limpa o filtro |
| `g` ou `:` | pula para uma linha, contando só as que o filtro manteve |
| `\` ou `&` | o filtro SQL |
| `T` | vai para o texto do arquivo e volta |
| `H` | da tabela para o texto, depois hexadecimal, depois de volta à tabela |

A moldura mostra `TABLE [2/5]` quando há mais de uma aba ou tabela, e a linha de baixo mostra onde está o cursor: `Cell B7 (Row 7 of 120)`. A última linha da grade traz o conteúdo inteiro da célula e o filtro em uso. A busca (`F`, `B`, `/`) não olha dentro da grade e nada encontra ali; ela funciona no modo texto.

### O filtro SQL

`\` ou `&` pede uma condição, como o `WHERE` do SQL (a palavra `WHERE` é opcional), e mostra só as linhas que a atendem. O prompt começa com o filtro atual; deixe-o vazio para ver todas as linhas de novo. Uma nota diz `Filter matched N rows` ou `SQL error: ...`; depois de um erro, o filtro anterior continua valendo.

```
age >= 30 AND name LIKE 'Ali%'
WHERE status IN ('open', 'late') OR total > 1000.50
NOT (city = 'Rio' OR city IS NULL)
B ILIKE '%silva%'
```

| Operador | O que faz |
|---|---|
| `=` `!=` `<>` `<` `<=` `>` `>=` | comparam |
| `AND` `OR` `NOT` e parênteses | combinam condições (`AND` vem antes de `OR`) |
| `LIKE` `ILIKE` | combinam com um padrão, `%` para qualquer texto e `_` para um caractere; `ILIKE` ignora maiúsculas e minúsculas |
| `IN (...)` `NOT IN (...)` | está, ou não está, na lista |
| `IS NULL` `IS NOT NULL` | a célula está, ou não está, vazia |

As palavras-chave podem estar em maiúsculas ou minúsculas.

- **Colunas:** o nome da coluna, em maiúsculas ou minúsculas e sem aspas, ou a sua letra, de `A` a `Z`. A letra é o jeito de chegar a um nome com espaços ou símbolos. Uma coluna que não existe conta como vazia, sem erro.
- **Valores:** texto entre aspas simples ou duplas (`\'` para uma aspa dentro dele), números, `TRUE`, `FALSE` e `NULL`. O que está entre aspas é sempre texto, nunca coluna.
- **Células:** o tipo vem da aparência da célula. Vazia é `NULL`, `true` e `false` são booleanos, números são números e o resto é texto. Por isso escreva `code = 7`, não `code = '7'`: texto e número nunca são iguais, e uma célula com `007` é o número 7. Comparações de texto distinguem maiúsculas de minúsculas.
- **Tamanho:** um filtro tem no máximo 1.024 bytes e 128 termos.

### Limites

Uma tabela guarda no máximo 50.000 linhas e 256 colunas, e leva no máximo o que os outros limites do visualizador permitem: 3 s (100 ms no Autoview), e uma planilha que desempacota para mais que o limite de tamanho (1 MiB no Autoview) não abre como tabela. O que passa disso não aparece e o filtro não vê; o cabeçalho avisa: `(showing first 50,000 rows)` ou `(stopped by the time limit: showing N rows)`. Uma célula mostra no máximo 2.048 caracteres e depois `…`. Caracteres de controle e de direção do texto aparecem como `·`.

## Tabelas em bibliotecas C

Bancos SQLite (`.db`, `.sqlite`, `.sqlite3`) e arquivos Parquet são lidos por bibliotecas escritas em C, que não têm a segurança de memória do Rust. Por isso, eles só abrem como tabela com `V`. O Autoview lê tudo por onde o cursor passa e mostra esses arquivos como conteúdo comum, com a nota `Parquet or SQLite: V opens the table`. Abra um banco de origem não confiável só se aceitar esse risco.

Veja também: [busca nos arquivos marcados](search.md).
