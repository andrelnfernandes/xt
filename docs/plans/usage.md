# Plano: uso do disco (Usage)

Objetivo: descobrir **onde está o espaço**. Para cada pasta, quanto ela ocupa
em relação à pasta de cima, ao espaço usado e ao disco inteiro, entrando nas
subpastas até chegar no que mais ocupa. É um recurso "além do XTree", como o
terminal, o SSH e o Git.

## O que já existe e será aproveitado

- **A árvore em memória** (`src/model/tree.rs`): `LogJob` lê um ramo inteiro
  em passos, com progresso e Esc. O Branch (`B`) e o Prune já usam isso com
  `AfterLog::OpenBranch` e `AfterLog::Prune`.
- **Alt+K** (`Kbyte` em `src/pane.rs`, `Tree::dir_totals`, desenho em
  `src/ui.rs:214`): já soma tamanho e contagem por diretório ou por ramo.
- **Espaço do disco**: `platform::disk_space` (total, livre, disponível) e
  `info::mount_of`, usados pelo painel DISK.
- **Identidade de arquivo** (`platform`, `(dev, ino)`) e o **dispositivo** de
  cada arquivo, que o Prune já usa para não atravessar pontos de montagem.
- **Ações** que a visão vai chamar: Delete (lixeira ou permanente), Prune,
  Tag, viewer, `\` para ir ao diretório.

## Decisões

1. **Visão própria**, no estilo do ncdu e do TreeSize, e não só números na
   árvore. Ela ocupa a janela (árvore e lista de arquivos), e o painel da
   direita continua. Em split screen (F8), cada lado tem a sua.
2. **Três medidas:** cada linha mostra, em número, a porcentagem em relação à
   **pasta atual**. O rodapé mostra quanto a pasta atual é do **espaço usado**
   e do **disco inteiro**.
3. **A barra é relativa ao maior item** da pasta, como no ncdu: o maior enche
   a barra e os outros ficam proporcionais a ele. Se a barra fosse a
   porcentagem da pasta, um item dominante deixaria todas as outras barras
   vazias. A barra compara os itens; o número diz quanto cada um pesa.
4. **Espaço em disco por padrão** (o que `du` e ncdu mostram: blocos
   alocados; nenhum arquivo ocupa menos que um bloco, 4 KiB), com `a` para
   alternar as linhas para o **tamanho aparente**. O rodapé mostra **os dois
   sempre**, como o ncdu. Faz diferença em arquivos esparsos (imagens de VM),
   em sistemas com compressão (btrfs, ZFS) e em arquivos pequenos.
5. **Hard links contados uma vez**, pelo `(dev, ino)`. As outras cópias
   aparecem com a marca `H` e tamanho 0 na soma.
6. **Um disco só**: a análise não atravessa pontos de montagem. Um ponto de
   montagem dentro do ramo aparece como linha marcada (`>`, "outro disco"),
   fora da soma; Enter nele começa uma análise daquele disco. Melhor que o
   ncdu, que mostra essas linhas com 0 B: o Xylem mostra, apagado e fora da
   soma, o **espaço usado daquele outro sistema de arquivos** (do statvfs) e o
   tipo dele, por exemplo `> /boot  154 MiB  ext4` ou `> /tmp  4.6 MiB  tmpfs`.
   Assim dá para saber se vale entrar.
7. **Ocultos sempre contam**, mesmo com a exibição deles desligada (só ficam
   fora da lista). Senão a porcentagem não fecha.
8. **Tecla:** Alt+W, configurável ("where did the space go"). As teclas livres
   hoje são Alt+E, J, L, N, V, W e Y. Antes de fixar, conferir que não
   conflita com o ZTreeWin (`docs/manual/ztreewin.md`).
9. **Arquivos e pastas misturados**, pelo tamanho. O maior item costuma ser um
   arquivo solto (um backup `.tar` de 33 GiB na raiz da home, no caso real
   que motivou isto); listar pastas primeiro o esconderia.
10. **Detalhes no painel da direita**, para a lista ficar limpa como a do
    ncdu (tamanho, barra, nome): do item sob o cursor, o caminho, o espaço em
    disco e o tamanho aparente, o número de arquivos, a **data de
    modificação** (para decidir se algo "antigo" pode ir embora) e a
    porcentagem do usado e do disco.
11. **Total incompleto avisado:** uma pasta com alguma subpasta que não pôde
    ser lida (permissão) leva a marca `.`, que sobe até a raiz, como no ncdu:
    o tamanho dela pode ser maior do que o mostrado.
12. **Marcas de uma letra**, as mesmas do ncdu: `>` outro disco, `H` hard
    link, `!` erro ao ler a própria pasta, `.` erro em alguma subpasta, `e`
    pasta vazia, `@` link simbólico ou outro tipo especial (contado pelo
    tamanho do próprio link, sem segui-lo, como o Xylem já faz). Pastas com
    `/` antes do nome, arquivos com espaço.
13. **Sistemas de arquivos virtuais nunca entram**, nem com "um disco só"
    desligado: `/proc`, `/sys`, `/dev`, `/run` e afins (proc, sysfs,
    devtmpfs, cgroup...). No ncdu rodado em `/` sem `-x`, o `/proc/kcore`
    sozinho fez o tamanho aparente somar **128,1 TiB** num disco de 466 GiB.
14. **Cobertura comparada com o disco:** o rodapé da raiz da análise compara
    o total encontrado com o espaço usado do sistema de arquivos (o que o
    `df` diz). Muito abaixo: há pastas que não puderam ser lidas (o rodapé
    diz quantas; analisar `/` como usuário comum sempre dá isso, com `!` em
    `/root` e `/lost+found`). Acima: algo foi contado duas vezes, por exemplo
    pontos de montagem atravessados. No caso real, o ncdu sem `-x` somou
    123,1 GiB num disco com 114 GiB usados, porque entrou nos 23 snaps
    montados em `/snap` (squashfs), cujo conteúdo já está nos arquivos
    `.snap` de `/var/lib/snapd`. Com `-x`, o mesmo disco deu 104,9 GiB:
    92% dos 114 GiB usados. A diferença nem sempre é só permissão: entram
    também os metadados do sistema de arquivos (journal, tabelas de inodes) e
    arquivos apagados que algum programa ainda mantém abertos. Por isso o
    rodapé mostra a cobertura ("104,9 GiB encontrados, 92% do usado") e o
    número de pastas sem permissão, sem afirmar que a diferença está nelas.
15. **"Do usado" é o usado do disco**, o que o `df` informa, e não o total
    encontrado na análise, que pode estar incompleto. Na home do caso real:
    72,4 GiB são 63% dos 114 GiB usados e 16% dos 466 GiB do disco.

## Como fica a tela

```
USAGE  /home/andre
    32.9 GiB  ████████████████████  45.4%   wsl-ubuntu-backup.tar
    18.9 GiB  ███████████▌          26.1%  /projetos
 .   5.5 GiB  ███▍                   7.6%  /.local
     3.8 GiB  ██▎                    5.2%  /.config
     3.2 GiB  █▉                     4.4%  /.cache
 >   1.2 GiB                          -    /backup          (outro disco)
     ...
 72.4 GiB em disco · 71.7 GiB aparente · 381.182 itens · 62% do usado · 15% do disco
```

- Ordem: do maior para o menor; `S` alterna entre tamanho, nome, número de
  arquivos e data.
- `c` mostra ou esconde a coluna com o número de arquivos (como no ncdu).
- As barras usam caracteres de bloco com oitavos (`▏▎▍▌▋▊▉█`), para mostrar
  diferenças pequenas, e as cores do tema.
- Os arquivos soltos da pasta aparecem misturados com as subpastas, pelo
  tamanho. Uma linha resumo no fim junta os que não cabem na tela.

## Teclas dentro da visão

| Tecla | O que faz |
|---|---|
| ↑ ↓ PgUp PgDn Home End | move o cursor |
| Enter ou → | entra na pasta |
| Backspace ou ← | volta para a pasta de cima (até a raiz da análise, e além dela, analisando a de cima) |
| `\` | sai da visão com o cursor da árvore nessa pasta |
| `a` | alterna espaço em disco e tamanho aparente nas linhas |
| `c` | mostra ou esconde o número de arquivos |
| `S` | ordem: tamanho, nome, arquivos, data |
| Alt+I | Info do item, como no resto do Xylem |
| Alt+X | terminal nessa pasta |
| `T` / `U` | marca / desmarca o arquivo, ou todos os do ramo |
| `D`, Del, Shift+Del | apaga, como na janela de arquivos |
| Alt+P | Prune da pasta sob o cursor |
| `V` | viewer do arquivo |
| F3 | relê o ramo da pasta atual |
| Esc ou Alt+W | sai da visão |

## Mudanças no modelo

- `FileEntry` ganha `blocks: u64` (espaço em disco: `st_blocks * 512` no
  Unix; `GetCompressedFileSizeW` no Windows, ou o tamanho arredondado para o
  cluster) e `link: Option<u64>` (o `ino` quando `nlink > 1`). O custo é de
  uns 16 bytes por arquivo. O SFTP e os arquivos compactados não têm blocos:
  ali `blocks` = tamanho, e a tecla `a` fica sem efeito.
- `DirNode` ganha `dev: u64`, para saber onde começa outro disco sem reler.
- `Tree::usage(root, measure) -> Usage`: um passe de baixo para cima, como
  `dir_totals`, mas sobre todos os arquivos (ocultos incluídos), com um
  `HashSet<(dev, ino)>` para os hard links e parando nos pontos de montagem.
  O resultado é guardado e só recalculado quando a árvore muda (a árvore já
  tem um contador de versão para as linhas visíveis; conferir).
- Cada pasta do resultado guarda também se o total está incompleto (erro em
  alguma subpasta), para a marca `.`; `DirNode.error` já existe.
- Diretórios em si (os blocos do próprio diretório) entram na soma, como no
  `du`. É pouco, mas faz o número bater com o `du`.

## Fases

### Fase 1: porcentagem no Alt+K (pequena) — feita (branch usage-percent)
- Nova opção **%** no Alt+K: a porcentagem do ramo em relação à pasta de cima,
  com uma barra curta ao lado do nome na árvore.
- Testes: `dir_totals` com porcentagens; desenho num terminal estreito.

### Fase 2: a visão Usage, só leitura — feita (branch usage-view)
- `Pane` ganha `usage: Option<UsageView>` (raiz, pasta atual, cursor, ordem,
  medida). O desenho fica em `src/ui.rs` no lugar da árvore e da lista.
- Alt+W loga o ramo (`LogJob` com `AfterLog::Usage`) e abre a visão. Com o
  ramo já logado, abre na hora.
- `blocks`, `link` e `dev` no modelo; hard links e um disco só.
- Navegação, `a`, `c`, `S`, `\`, F3, Esc; marcas; rodapé; detalhes no
  painel da direita.
- Ajuda (F1): página nova `docs/manual/usage.md`, ligada ao índice e à seção
  "Beyond XTree"; READMEs e site.
- Testes: soma com hard link, com ponto de montagem (simulado pelo `dev`),
  com ocultos; teste e2e abrindo a visão numa árvore de exemplo.

### Fase 3: ações dentro da visão
- Tag, Delete, Prune e viewer, com as mesmas perguntas da janela de
  arquivos. Depois de apagar, a soma se refaz sozinha.
- Testes: apagar um arquivo e ver a porcentagem mudar; Prune pela visão.

### Fase 4 (opcional)
- Treemap em caracteres de bloco, alternado com a lista.
- Agrupar por tipo ("vídeos: 40%"), usando o modo Type.
- Exportar o resultado (texto ou JSON), como o `ncdu -o`.

## Além do ncdu

- **Marcar e agir em grupo:** vários itens grandes apagados ou movidos de uma
  vez (o ncdu apaga um por vez).
- **Ir para a árvore** (`\`) e usar Copy, Move, Compare, o viewer.
- **Ao vivo:** o ncdu tira uma foto e só atualiza com `r`; o Xylem já relê a
  pasta que muda, e os números acompanham.
- **Dois lados** com F8: o disco antes e depois de uma limpeza, ou o local
  contra o remoto.

## Riscos e cuidados

- **Fotos que envelhecem:** duas rodadas do ncdu na mesma home, minutos
  depois, já davam números diferentes (o `.claude` cresceu, o total aparente
  mudou). O Xylem relê sozinho a pasta que muda; as somas precisam acompanhar
  sem recalcular tudo a cada evento (recalcular só o caminho até a raiz).
- **Discos grandes:** analisar `/` pode carregar milhões de arquivos (algumas
  centenas de MB de memória). Mostrar o progresso, deixar cancelar e avisar
  na ajuda. Analisar a partir de `/` sem o `-x` seria pior: por isso o
  "um disco só".
- **Remoto (SSH) e rclone:** funciona, mas cada pasta é uma ida à rede. O
  espaço "do disco" do remoto pode não existir (no SFTP, só se o servidor
  tiver a extensão statvfs). Nesse caso o cabeçalho mostra só a porcentagem
  da pasta atual.
- **Windows:** blocos via `GetCompressedFileSizeW`, hard links via o índice
  do arquivo. Conferir com clippy para `x86_64-pc-windows-gnu`.
- **Consistência com o `du`:** escrever um teste que compara a soma do Xylem
  com `du -sx --block-size=1` numa árvore de exemplo (Linux).

# Duplicados

Objetivo: achar **arquivos com o mesmo conteúdo**, mesmo com nomes e pastas
diferentes, e liberar o espaço das cópias. É o complemento natural da visão
Usage: ela mostra *onde* está o espaço, os duplicados mostram o que dá para
tirar sem perder nada.

Hoje não existe: o Compare (`C`) só casa arquivos pelo **caminho** entre os
dois lados da tela dividida, e o "Identical" dele é mesmo tamanho e data, não
o conteúdo.

## Disco local (e SSH)

O Xylem já tem o tamanho de todos os arquivos logados na memória, então a
maior parte do trabalho sai de graça:

1. **Agrupar por tamanho** (memória, sem ler nada). Tamanho único não tem
   duplicado. Ficam de fora:
   - arquivos vazios e os menores que um mínimo (padrão 1 KiB, configurável);
   - links simbólicos e arquivos especiais;
   - **hard links** do mesmo arquivo (`dev`, `ino` iguais): já dividem o
     espaço, e apagar um deles não libera nada.
2. **Ler o começo e o fim** (64 KiB de cada lado) dos que sobraram e separar
   os grupos pelo que leu.
3. **Comparar o conteúdo inteiro** só dos candidatos que restarem, lendo os
   arquivos de um grupo em paralelo, bloco a bloco, e separando o grupo
   assim que um bloco difere. Isso dispensa uma biblioteca de hash (nenhuma
   dependência nova, como prefere o projeto) e para cedo em arquivos
   diferentes. Com progresso (bytes lidos de quantos) e Esc.

No SSH funciona igual, pelo SFTP, mas cada byte vem pela rede: avisar e
mostrar o total a ler antes de começar.

## Google Drive (e outros pelo rclone): MD5 sem baixar

Numa pasta montada pelo rclone, ler o conteúdo significa **baixar** cada
arquivo, o que é inviável num Drive grande. Mas o Drive guarda um **MD5 de
cada arquivo**, e o rclone informa sem baixar nada:

```bash
rclone lsjson -R --files-only --hash-type md5 --fast-list gdrive:
```

Cada item vem com `Path`, `Size`, `ModTime`, `ID` e `Hashes.md5`. Num Drive
inteiro, isso leva minutos, não horas.

- **Reconhecer a pasta:** `rclone::of` (já existe, `src/rclone.rs`) dá o
  remoto (`gdrive:`) e o ponto de montagem; o caminho remoto de um arquivo é
  o remoto mais o caminho relativo ao ponto de montagem. Rodar o `rclone`
  com o mesmo `RCLONE_CONFIG` que o Xylem vê.
- **Sem rclone no PATH**, ou remoto sem MD5: cair no método local (avisando
  quanto teria de baixar) ou oferecer o `--download` do `rclone md5sum`, só
  com confirmação.
- **Agrupar por MD5 e tamanho**; o tamanho é a confirmação barata contra
  colisão.

### Peculiaridades do Drive

- **Mesmo `ID` não é duplicado.** No Drive antigo, um arquivo podia estar em
  mais de uma pasta ao mesmo tempo; ele aparece duas vezes na listagem com o
  mesmo `ID`. É **um** arquivo: apagar "a cópia" apaga o único. Agrupar por
  `ID` primeiro e nunca oferecer um contra o outro.
- **Atalhos** (shortcuts) apontam para outro arquivo: ficam de fora
  (`--drive-skip-shortcuts`).
- **Docs, Planilhas e Apresentações** não têm MD5 (são exportados na hora):
  ficam de fora, e a tela diz quantos.
- **Compartilhados comigo** não contam no armazenamento de quem recebeu:
  apagar a sua cópia de algo que outra pessoa compartilhou não libera nada do
  seu plano. Marcar esses itens (o rclone mostra o dono com
  `--drive-shared-with-me` numa listagem à parte) e não sugerir apagá-los.
- **Nomes repetidos na mesma pasta** são permitidos no Drive, e o rclone
  avisa ("Duplicate object found"). Para esses, o próprio `rclone dedupe`
  resolve; a tela pode sugerir o comando.
- **Apagar manda para a lixeira do Drive**, que continua contando no
  armazenamento até ser esvaziada. Depois de apagar, lembrar de
  `rclone cleanup gdrive:` (esvazia a lixeira), com confirmação.
- **Limites da API:** usar `--fast-list` e um único `lsjson` recursivo, em
  vez de uma chamada por pasta.

Outros remotos: S3 tem MD5 no ETag só para arquivos enviados de uma vez (não
em partes); Dropbox, OneDrive e outros usam hashes próprios, que o
`rclone lsjson --hash` também traz. O agrupamento funciona com qualquer hash
que os dois lados de uma comparação tenham.

## A tela

Uma lista como a do Branch, com os grupos separados:

```
DUPLICATES  /home/andre/gdrive      38 grupos · 112 arquivos · 6,4 GiB a liberar
── 3 × 1,2 GiB (2,4 GiB a liberar) ─────────────────────────────
   1,2 GiB  2023-08-14  Backups/celular-2023.zip
 T 1,2 GiB  2023-08-14  Downloads/celular-2023.zip
 T 1,2 GiB  2023-08-14  Antigo/celular-2023 (1).zip
── 2 × 850 MiB (850 MiB a liberar) ─────────────────────────────
   850 MiB  2021-12-25  Vídeos/natal.mp4
 T 850 MiB  2022-01-03  WhatsApp/VID-20211225.mp4
```

- Ordem: grupos pelo espaço a liberar, do maior para o menor.
- **Marcar todos menos um** de cada grupo, escolhendo qual fica: o mais
  antigo, o mais novo, o de caminho mais curto, ou o que está numa pasta
  escolhida ("manter o que estiver em `Backups/`"). Depois, as operações de
  sempre: Ctrl+D apaga os marcados, Ctrl+M move, `\` vai à pasta.
- **Antes de apagar, conferir** que tamanho e data não mudaram desde a
  análise; se mudaram, refazer aquele grupo.
- O painel da direita mostra o caminho inteiro e as datas do item.

## Fases

- **D1:** duplicados no disco local (tamanho, começo e fim, conteúdo), tela,
  marcar todos menos um, apagar com conferência. Testes com hard links,
  arquivos que só diferem no fim, grupos de 3 que se dividem em 2 + 1.
- **D2:** rclone: MD5 pelo `lsjson`, com as peculiaridades do Drive (ID,
  atalhos, Docs, compartilhados, lixeira). Testes com uma saída de `lsjson`
  gravada, sem rede.
- **D3 (opcional):** ignorar por padrão pastas como `.git` e `node_modules`;
  exportar a lista; achar pastas inteiras duplicadas (mesmo conjunto de
  arquivos), que é onde backups repetidos mais ocupam.
