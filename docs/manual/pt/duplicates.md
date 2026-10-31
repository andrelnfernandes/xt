# Duplicados (Alt+Y)

Arquivos com o mesmo conteúdo, quaisquer que sejam os nomes e as pastas, e o
espaço que as cópias desperdiçam. **Alt+Y** procura no ramo do diretório atual;
**D** na tela de Uso procura na pasta listada ali.

```
── 3 × 1.2 GiB (2.4 GiB to free) ──────────────────────
     1.2 GiB  2023-08-14  Backups/celular-2023.zip
 T   1.2 GiB  2023-08-14  Downloads/celular-2023.zip
 T   1.2 GiB  2023-08-14  Antigo/celular-2023 (1).zip
── 2 × 850.0 MiB (850.0 MiB to free) ──────────────────
     850.0 MiB  2021-12-25  Videos/natal.mp4
 T   850.0 MiB  2022-01-03  WhatsApp/VID-20211225.mp4
```

O que libera mais vem primeiro. Nada fica marcado até você pedir, e uma cópia
de cada grupo sempre fica: o Xylem não deixa marcar a última.

## Escolhendo o que sai

| Tecla | O que faz |
|---|---|
| ↑ ↓ PgUp PgDn Home End | move |
| `T` ou Espaço | marca ou desmarca a cópia |
| `K` | fica com esta cópia: marca as outras do grupo |
| `A` | em todos os grupos, marca todas menos uma; `A` de novo muda qual fica: a mais antiga, a mais nova, o caminho mais curto |
| `F` | em todo grupo com uma cópia na pasta desta cópia, fica com aquela |
| `U` | desmarca tudo |
| `D`, Del ou Ctrl+D | apaga as marcadas, como o comando Delete (para a lixeira se `delete_to_trash` estiver ligado) |
| `M` ou Ctrl+M | move os arquivos marcados |
| `G` | arquivos ou pastas inteiras |
| `V` ou Enter | mostra o arquivo |
| `E` | grava a lista num arquivo (nunca por cima de um que exista): tamanho, data, marca e caminho, com uma linha em branco entre os grupos |
| `\` | fecha a tela, com a árvore naquela pasta |
| F3 | lê as pastas de novo e compara de novo |
| Esc ou Alt+Y | fecha |

Antes de apagar ou mover, cada grupo é conferido de novo: se uma cópia mudou
desde a análise, ou a que ia ficar sumiu, nada daquele grupo é tocado, e o
rodapé avisa.

## Pastas inteiras

Backups repetidos inteiros são o que mais ocupa, e uma lista longa de arquivos
esconde isso. **G** mostra as pastas com exatamente o mesmo conteúdo: os mesmos
nomes, o mesmo conteúdo de cada arquivo, grande ou pequeno, e os mesmos links
apontando para o mesmo lugar. Só as de cima aparecem, não as pastas dentro delas.

Uma pasta nunca é chamada de cópia quando algo nela não foi comparado: uma pasta
que não pôde ser lida, outro disco montado dentro, um arquivo especial, ou uma
pasta ignorada (abaixo). Dois clones de um repositório têm os mesmos arquivos,
mas o `.git` de cada um pode guardar trabalho diferente.

## Como os arquivos são comparados

Os tamanhos vêm da memória. Só os arquivos com um tamanho igual ao de outro são
lidos: primeiro as duas pontas, depois o meio, parte por parte, e o grupo se
divide assim que os arquivos diferem, de modo que arquivos diferentes não são
lidos até o fim. A comparação é por SHA-256. Ela corre em segundo plano: o rodapé
mostra o progresso e Esc cancela.

Ficam de fora da lista:

- arquivos menores que `duplicates_min_size` (1 KiB, por padrão) e os vazios;
- links e arquivos especiais, que não são seguidos;
- outros hard links do mesmo arquivo: já dividem o espaço, e apagar um não libera
  nada;
- outros discos montados dentro do ramo;
- pastas citadas em `duplicates_ignore` (`.git` e `node_modules`, por padrão).

Um arquivo que mudou depois que a árvore foi lida, ou que não pode ser lido, não
é comparado; a linha de baixo diz quantos. F3 lê tudo de novo.

Numa pasta de rede (NFS, SMB, sshfs...), o total a ler é perguntado antes. Numa
máquina remota ou dentro de um arquivo compactado, os Duplicados ainda não estão
disponíveis.

## Na nuvem (rclone)

Numa pasta montada pelo `rclone mount`, ler um arquivo significa baixá-lo. Por
isso o Xylem pede ao rclone os hashes que o serviço já guarda (`rclone lsjson
--hash`), e nada é baixado: um Drive inteiro leva minutos. Usa o hash mais forte
que o serviço der (SHA-256, SHA-1, MD5...), junto com o tamanho, e o rclone
precisa estar no PATH. Se não estiver, ou se o remoto não der hashes (um
criptografado, por exemplo), o Xylem diz o motivo e pergunta antes de ler os
arquivos pela montagem, com o total que baixaria.

Pastas inteiras só são comparadas em discos locais.

No Google Drive:

- Um arquivo em duas pastas ao mesmo tempo, ou um atalho, é um arquivo só: nunca
  é oferecido contra si mesmo.
- Docs, Planilhas e Apresentações não têm hash: a linha de baixo diz quantos
  ficaram de fora.
- O que outras pessoas compartilharam com você aparece com `s`, e marcar todas
  menos uma não mexe nele: apagar não libera nada do seu armazenamento.
- Dois arquivos com o mesmo nome numa pasta não se distinguem pela montagem; a
  linha de baixo sugere `rclone dedupe`.
- Apagar manda para a lixeira do Drive, que continua contando até ser esvaziada.
  O Xylem não a esvazia, porque isso leva tudo o que está lá; ele mostra o
  comando: `rclone cleanup gdrive:`.

O hash do serviço é aceito por confiança. MD5 pode ser forjado de propósito: em
arquivos que outras pessoas possam ter colocado lá, compare antes de apagar.
