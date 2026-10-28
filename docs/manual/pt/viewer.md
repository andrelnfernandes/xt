# O visualizador

`V` abre o arquivo no visualizador (o Autoview, F7, o mostra ao lado da lista).
Arquivos binários abrem em hexadecimal. Arquivos acima do limite de tamanho mostram
apenas o início.

| Tecla | O que faz |
|---|---|
| ↑ ↓ PgUp PgDn Home End | rola verticalmente |
| ← → | rola horizontalmente |
| `H` | texto ou hexadecimal |
| `O` | próxima codificação (UTF-8, CP850, 1252, CP437, UTF-16) |
| `C` | ativa ou desativa as cores de sintaxe |
| `W` | ativa ou desativa a quebra de linha automática |
| `#` ou `L` | ativa ou desativa os números de linha (no modo texto) |
| `g` ou `:` | pula para uma linha (ou deslocamento de byte em hexadecimal) |
| `E` | edita o arquivo; de volta ao Xylem, o visualizador mostra o novo conteúdo na mesma posição (arquivos binários somente com um editor em [`[editors]`](execute.md)) |
| `D` | com Git ativo: mostra o que mudou desde o último commit, e volta ([Git](git.md)) |
| `T` / `U` | marca / desmarca o arquivo |
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

Veja também: [busca nos arquivos marcados](search.md).
