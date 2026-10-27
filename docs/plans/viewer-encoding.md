# Spec: codificações, descompressão e limite de tamanho do viewer

Subprojeto 1 do [plano de visualizadores](viewers.md).

**Status (2026-10-27):** PR 1 (guarda bidi, Tarefas 1 a 4) implementado no
branch `bidi-guard`; PR 2 (módulo `decompress` e correção do Alt+F5,
Tarefas 5 a 7) implementado no branch `decompress`; o próximo é o PR 3
(Tarefa 8). Spec aprovado pelo
autor. O plano de implementação
está em [viewer-encoding-plan.md](viewer-encoding-plan.md). Ao escrevê-lo,
quatro pontos foram corrigidos aqui: a janela do zstd, a conta do xz, o
panic hook e as letras do F12.

O autor pediu cuidado especial com o CVE-2021-42574.

## Escopo

Entra:

- **Codificações:**
  - UTF-8 (com e sem BOM), UTF-16 LE/BE, CP437, CP850 e Windows-1252.
  - Latin-1 aparece certo como 1252.
  - UTF-16 sem BOM, que hoje cai no hex por causa dos zeros, passa a abrir
    como texto.
- **Descompressão:** `.gz`, `.xz`, `.zst` e `.bz2` soltos, reconhecidos pelos
  primeiros bytes (não pela extensão) e mostrados já descomprimidos. Um
  nível só.
- **Limite de tamanho do viewer configurável** (`viewer_max_size`), no lugar
  dos 16 MiB fixos.
- **Correção da brecha atual:** `archive.rs` passa a usar os leitores de xz e
  zstd com limite de memória. Hoje o `.tar.xz` usa `XzReader::new`, sem
  limite. O zstd já tinha o teto padrão do `ruzstd`, de 128 MiB, que desce
  para 64 MiB.
- **Trojan Source (CVE-2021-42574) na tela inteira:** os 12 caracteres bidi
  nunca chegam ao terminal, e o viewer e os nomes de arquivo os mostram
  marcados e com aviso (seção 2b). Isso vale para o Xylem todo, não só para
  o viewer.
- **`cargo audit` no CI**, num PR separado (seção 2).

Fica de fora:

- Ctrl+S (busca nos arquivos marcados) com outras codificações ou dentro de
  comprimidos.
- Qualquer edição.
- Ir para o fim do arquivo e janelas (subprojetos 3 e 4).
- Coluna ASCII do hex em outras codificações.

## Decisões tomadas

| Decisão | Escolha | Por quê |
|---|---|---|
| Arquivo que não é UTF-8 válido | Heurística entre CP850 e 1252; a tecla `O` troca; o F12 pode fixar uma | Os arquivos do autor vêm de Clipper (850) e de Windows (1252) |
| Autoview descomprime? | Sim, com limites menores; uma opção no F12 desliga a descompressão inteira | Os descompressores são Rust e já leem archives desconhecidos no Alt+F5 |
| Onde descomprimir | Sobre os bytes que o viewer já carrega (abordagem A) | Um lugar só para disco, archive e remoto. Descomprimir em fluxo direto da origem (B) exigiria adaptar três origens sem ganho, porque o teto de saída é o mesmo |
| Limite de 16 MiB | Configurável, com `auto` = 1/32 da RAM física, entre 16 e 256 MiB | Medido: 16 MiB custam ~40 MiB de RAM e ~75 ms. O gargalo é a carga síncrona, não a memória |
| Arquivo maior que o limite | Mostra o começo, com aviso correto no cabeçalho | O fim do arquivo fica para os subprojetos 3 e 4 |
| Tecla para trocar a codificação no viewer | `O` ("cOdepage") | `A` e `J` são modos do viewer do ZTreeWin (Alpha, Junk) e devem ficar livres. No viewer, `O` não tem outro uso hoje |
| Caracteres bidi | Filtro único no fim de `ui::draw`, mais aviso no viewer e nos nomes | Medido: hoje o U+202E de um nome chega ao terminal. O autor pediu cuidado especial |

## Seção 1 — Componentes e fluxo (aprovada)

```
bytes brutos (disco / archive / remoto, até o limite)
  │
  ├─ decompress::sniff   gzip / xz / zstd / bzip2 pelos primeiros bytes?
  │     └─ decompress(limites)        um nível só; o resultado não é reexaminado
  │
  ├─ charset::detect     BOM → UTF-16 sem BOM → binário (hex) → UTF-8 válido
  │                      → senão: legacy_charset ("auto" = heurística 850/1252)
  │
  └─ Viewer              data = bytes descomprimidos (hex e busca hex)
                         lines = texto decodificado (texto, busca, cores)
```

**Módulos novos, sem nenhuma dependência nova:**

- **`src/charset.rs`**:
  - `enum Charset { Utf8, Utf16Le, Utf16Be, Cp437, Cp850, Cp1252 }`.
  - `detect(bytes, fallback)` devolve a codificação e o tamanho do BOM.
  - `decode(bytes, charset)` devolve uma `String` e nunca falha. Byte que a
    codificação não define vira `�` (U+FFFD).
  - As tabelas são `[char; 128]` escritas à mão, para os bytes `0x80`–`0xFF`.
    Os bytes `0x00`–`0x7F` são ASCII em todas. Nos códigos do DOS, os bytes
    `0x01`–`0x1F` não viram os símbolos ☺☻♥: continuam sendo caracteres de
    controle e aparecem como `·`.
  - UTF-16 usa `char::decode_utf16` da std. Um byte solto no fim, de um
    corte no meio, é ignorado.
- **`src/decompress.rs`**:
  - `enum Compression { Gzip, Xz, Zstd, Bzip2 }`.
  - `sniff(bytes)` reconhece pelas assinaturas: `1F 8B` (gzip),
    `FD 37 7A 58 5A 00` (xz), `28 B5 2F FD` (zstd) e `42 5A 68` (bzip2).
  - `decompress(bytes, tipo, limites)` devolve os bytes, se a saída foi
    cortada, e um aviso quando a entrada estava truncada ou corrompida.
  - Os construtores `xz_reader(r, limites)` e `zstd_reader(r, limites)` são
    públicos, para o `archive.rs` usar também.

**Mudanças em módulos que já existem:**

- **`viewer.rs`**:
  - `from_bytes` continua igual, para texto gerado pelo próprio Xylem
    (ajuda, git, info): sempre UTF-8, sem examinar nada.
  - Um construtor novo, `from_file(path, size, bytes, &opções)`, roda o
    fluxo. `open_limited` e `App::load_viewer_limited` passam a usá-lo.
  - Campos novos no `Viewer`:
    - `charset`: a codificação em uso;
    - `charset_manual`: se ela foi escolhida com `O`;
    - `compression: Option<(Compression, tamanho comprimido)>`;
    - `cut: bool`: se a entrada ou a saída bateu no limite.
  - `truncated()` passa a usar o `cut`. A conta de hoje (`size >
    data.len()`) erra com arquivo comprimido: um `.gz` de 2 MiB que vira
    40 MiB fica com 16 MiB em memória, e o aviso não aparece.
  - `text_lines` recebe o texto já decodificado (`&str`).
  - Cores de sintaxe: usam o nome sem a extensão de compressão
    (`app.rs.gz` é colorido como Rust).
  - `reload` (depois do E) mantém a codificação se ela foi escolhida à mão;
    senão, detecta de novo.
- **`app/view.rs`**: `load_viewer_limited` passa as opções (`legacy_charset`,
  `viewer_decompress`) e os limites do contexto (V, Autoview ou remoto).
- **`archive.rs`**: passa a usar `decompress::xz_reader` e `zstd_reader` com os
  limites do V.

**Ordem da detecção:**

1. Se `viewer_decompress` estiver ligado e os primeiros bytes forem de
   compressão, descomprime. Os passos seguintes olham o resultado, e o
   resultado não é examinado de novo em busca de outra compressão.
2. BOM: `EF BB BF` é UTF-8, `FF FE` é UTF-16LE, `FE FF` é UTF-16BE.
3. UTF-16 sem BOM, olhando os primeiros 8 KiB (`BINARY_PROBE`), com pelo
   menos 16 bytes:
   - LE se pelo menos 70% dos bytes de posição ímpar forem `0x00` e no
     máximo 5% dos de posição par;
   - BE no caso espelhado.

   Isso vem antes do teste de binário. Texto UTF-16 fora do alfabeto latino
   e sem BOM (chinês, por exemplo) não é detectado e continua no hex. Isso é
   aceito, porque arquivos UTF-16 do Windows quase sempre têm BOM. Os
   limiares podem ser ajustados no plano se os testes pedirem.
4. Byte nulo no começo: hex, como hoje.
5. UTF-8 válido, aceitando uma sequência cortada no fim do bloco: UTF-8.
   ASCII puro também entra aqui.
6. Se nada disso se aplicar, vale `legacy_charset`: `cp850`, `cp1252` ou
   `cp437` fixam a codificação; `auto` usa a heurística:
   - cada byte `≥ 0x80` vizinho de letra ASCII conta ponto para a
     codificação em que ele vira letra com a mesma caixa das vizinhas.
     Exemplo: `0x87` entre minúsculas é `ç` no 850 e `‡` no 1252;
   - caracteres de caixa do DOS em sequência (`0xB3`–`0xDA`) somam para o
     850;
   - bytes que o 1252 não define (`0x81`, `0x8D`, `0x8F`, `0x90`, `0x9D`)
     somam para o 850;
   - a contagem usa os primeiros 64 KiB. Ganha quem fizer mais pontos; no
     empate, 1252.

   O CP437 não entra no `auto`: só pela tecla ou fixado no F12.

**Tela e teclas:**

- **Cabeçalho**, depois do tamanho:
  - `gzip → 9,876,543 bytes` quando o arquivo é comprimido;
  - o nome da codificação (`CP850`, `UTF-16LE`...) quando não é UTF-8;
  - quando o arquivo foi cortado: `(showing first 64 MiB)` ou
    `(showing first 64 MiB decompressed)`, com o limite real do contexto.
    Hoje esse aviso imprime sempre `MAX_BYTES`.
- **Tecla `O` ("cOdepage"), dentro do viewer** (na lista de arquivos, `O`
  continua sendo "abrir no programa do sistema"), troca em ciclo: UTF-8, CP850, 1252, CP437,
  UTF-16LE, UTF-16BE. O rodapé mostra o nome, por exemplo
  `Codepage: CP850 (DOS, Western Europe)`. As linhas e as cores são
  refeitas a partir de `data`. Escolhi `O` porque `A` e `J` são modos do
  viewer do ZTreeWin (Alpha, Junk) e devem ficar livres.
- **Modo hex** mostra os bytes descomprimidos, e a coluna ASCII continua
  como está.
- **Busca de texto** já trabalha sobre as linhas decodificadas, então `F`
  procurando `ação` encontra o texto num arquivo CP850. A busca hex
  trabalha sobre os bytes descomprimidos.
- **`.tar.gz` aberto com V:** o resultado é um TAR, que aparece em hex, com
  o aviso `TAR archive: Alt+F5 opens it as a folder` (a assinatura `ustar`
  fica no byte 257).

**Limite de tamanho (`viewer_max_size`):**

- Valores: `auto`, 16, 64, 256 e 1024 MiB.
- `auto` = 1/32 da RAM física, entre 16 e 256 MiB, sem arredondar para os
  valores fixos. Dá 256 MiB numa máquina de 8 GB, 192 MiB numa de 6 GB e
  16 MiB num servidor de 512 MB. O F12 mostra o valor calculado:
  `auto (256 MiB)`.
- Fonte da RAM: no Linux, `MemTotal` de `/proc/meminfo`. No Windows,
  `GlobalMemoryStatusEx` (a feature `Win32_System_SystemInformation` já está
  no `windows-sys`). Se a leitura falhar, 16 MiB.
- O Autoview continua fixo em 1 MiB (`AUTOVIEW_BYTES`), porque recarrega a
  cada movimento do cursor.
- O remoto (SFTP) fica em `min(viewer_max_size, 16 MiB)` enquanto a carga for
  síncrona e o Esc não cancelar. A 10 MB/s, 256 MiB seriam uns 25 s de
  tela parada.

Medições (binário release, log de 16 MiB com linhas de ~100 bytes): cerca de
+40 MiB de RAM e cerca de 75 ms de CPU. A conta linear dá 64 MiB → ~160 MiB
e ~0,3 s; 256 MiB → ~640 MiB e ~1,2 s; 1 GiB → ~2,5 GiB e ~5 s.

## Seção 2 — Modelo de ameaça e limites (aprovada)

**O que se protege:**
- a conta do usuário: execução de código;
- o Xylem: travamento e esgotamento de memória;
- o terminal: sequências de escape;
- o que o usuário vê: texto disfarçado.

**Quem ataca:** quem produziu o arquivo. Controla os bytes, o nome e o
tamanho. No remoto, também controla as respostas do servidor.

**Superfícies novas:**

1. **Descompressores:** `flate2` (miniz_oxide/zlib-rs), `lzma-rust2`,
   `ruzstd` e `bzip2` (libbz2-rs-sys).
   - São todos em Rust, sem C.
   - zlib-rs e libbz2-rs-sys são portes de C e usam `unsafe` por dentro.
   - Já leem archives de origem desconhecida no Alt+F5, então o risco não é
     novo.
   - O que muda é que o Autoview passa a rodá-los sozinho. Daí os limites
     menores no Autoview e a opção `viewer_decompress` para desligar tudo.
   - Nenhuma dependência nova.
2. **Decodificação e heurística:** código nosso, em Rust seguro, sem
   `unsafe`. As tabelas são indexadas por `b - 0x80` só quando `b ≥ 0x80`.
   O risco que sobra é `panic`, coberto pelos testes de propriedade.
3. **A tela:**
   - Controles C0, DEL e C1 continuam virando `·`. Isso inclui o ESC
     (`0x1B`), que no CP437 seria `←`.
   - Caracteres bidi: é o "Trojan Source" (CVE-2021-42574), em que o texto
     aparece numa ordem diferente da real. O risco é visual, não de
     execução, mas pesa em nomes de arquivo (extensão falsa). O tratamento
     completo, com 12 caracteres em três camadas e valendo para o Xylem
     todo, está na seção 2b.

**Limites:**

| | V | Autoview | V no remoto (SFTP) |
|---|---|---|---|
| bytes lidos | `viewer_max_size` (`auto` = 16–256 MiB) | 1 MiB | `min(viewer_max_size, 16 MiB)` |
| saída descomprimida | igual aos bytes lidos | 1 MiB | igual aos bytes lidos |
| tempo de descompressão | 3 s | 100 ms | 3 s |
| dicionário do xz | 64 MiB | 16 MiB | 64 MiB |
| janela do zstd | 64 MiB | 16 MiB | 64 MiB |

- **xz:** `XzReader::new_mem_limit(r, true, kib)`, com
  `kib = lzma_rust2::lzma2_get_memory_usage(limite do dicionário)`. Essa é
  a mesma conta que o `XzReader` faz em cada cabeçalho de bloco:
  `40 + 64 + dict / 1024` KiB, em que os 64 são o buffer de entrada
  (`COMPRESSED_SIZE_MAX`). Com um limite de 64 MiB, `xz -9` e `-9e` são
  aceitos. No
  Autoview, 16 MiB aceitam até `xz -7`; `-8` e `-9` mostram
  `large dictionary: V to view`.
- **zstd:** usa `StreamingDecoder::new_with_max_window_size(r, limite)`. O
  `ruzstd` 0.9.1 já recusa janelas acima do limite antes de alocar
  (`FrameDecoder::set_max_window_size`; o padrão, `DEFAULT_MAX_WINDOW_SIZE`,
  é 128 MiB). O máximo do formato, ~3,75 TB (`MAX_WINDOW_SIZE`), não é o
  teto do decoder. Arquivos `zstd --long` e `--ultra -22` (128 MiB) são
  recusados no V, com aviso.
  - O `StreamingDecoder` lê um frame só. Para arquivos com vários frames
    (`pzstd`, `zstd -T`), cria-se um decoder novo para cada frame até a
    entrada acabar. O erro `ReadFrameHeaderError::SkipFrame` pula `length`
    bytes (frames "skippable").
- **gzip e bzip2:** memória fixa (janela de 32 KiB; blocos de no máximo
  900 KB). Usam `MultiGzDecoder` e `MultiBzDecoder`, por causa de logs
  concatenados.
- **Tempo:** a descompressão lê de 64 KiB em 64 KiB e confere o relógio
  entre uma leitura e outra. Uma leitura sozinha é limitada pela entrada.
  Ao bater no tempo ou na saída, o viewer mostra o que já tem e avisa.
- **Memória total:** o buffer comprimido é liberado logo depois de
  descomprimir, porque o hex mostra os bytes descomprimidos e o E abre o
  original no disco. O pico fica perto de 2,5× o limite, mais o dicionário.
  Com `auto`, isso dá cerca de 8% da RAM, mais o dicionário (64 MiB no
  máximo).
- **Arquivo truncado ou corrompido:** mostra o que conseguiu descomprimir e
  o aviso `compressed data truncated or corrupt`.
- **Defesa extra:** a chamada aos descompressores roda dentro de
  `std::panic::catch_unwind`. Um bug de crate que cause `panic` vira
  "arquivo corrompido" em vez de derrubar o Xylem. O projeto usa `unwind`
  (não há `panic = "abort"` no `Cargo.toml`).
  - **Cuidado (encontrado ao escrever o plano):** o hook instalado pelo
    `ratatui::init` restaura o terminal, e o hook roda **antes** de o
    `catch_unwind` capturar o panic. Sem tratamento, o Xylem se recuperaria
    com a tela fora do modo raw.
  - **Correção:** um módulo `src/panic_guard.rs` com uma flag
    `thread_local`. `panic_guard::catch(f)` liga a flag, roda `f` dentro do
    `catch_unwind` e desliga a flag. Um hook instalado por último no
    `main.rs` (depois de `Keyboard::detect`) não chama os hooks anteriores
    quando a flag está ligada.
  - Um panic fora da área protegida continua restaurando o terminal, como
    hoje.
- **Sem recursão:** um nível só de descompressão.
- **Nada no disco:** nenhum arquivo temporário. Tudo acontece em memória.
- **Remoto que mente o tamanho:** `remote.read(path, limite)` já corta pelo
  limite, não pelo tamanho que o servidor informa.
- **Carga síncrona:** com V, o pior caso no disco local é de 1 a 3 s com
  256 MiB. Isso é aceito e documentado. O subprojeto 3 resolve.

**Fora do código, barato:** o CI (`.github/workflows/ci.yml`) roda clippy e
testes, mas não audita dependências. Proposta: um passo `cargo audit`
(RustSec) para os descompressores e o resto da árvore. Pode ser um PR
separado.

## Seção 2b — Trojan Source (CVE-2021-42574) na tela inteira (aprovada)

**O que existe hoje.** Medido em 2026-10-27, com o binário release num pty
(`~/.cache/xt-dev/viewer-mem/scripts/capture.py`, pasta de teste
`~/.cache/xt-dev/viewer-mem/bidi/`):

- **Lista de arquivos:** no nome `fatura\u{202E}fdp.sh`, o U+202E (RLO)
  **chega ao terminal**. Um terminal com BiDi (VTE/GNOME Terminal, Konsole,
  mlterm) pode mostrar o nome invertido, `faturahs.pdf`.
- **Viewer:** o mesmo caractere no conteúdo é **descartado sem aviso**.
  Saída capturada: `linha com  invertido` e depois `fim`; os dois espaços
  são onde ele estava. O texto aparece na ordem real, mas o usuário não
  fica sabendo que havia algo escondido.
- **Por que isso acontece:**
  - o ratatui filtra `char::is_control` (categoria Cc) em
    `Span::styled_graphemes` e `Buffer::set_stringn`, então ESC e C1 já não
    passam;
  - os caracteres bidi são da categoria **Cf**, não Cc;
  - `set_stringn` descarta grafemas de largura 0 (caso do viewer);
  - `Span::render` gruda o grafema de largura 0 na célula anterior, que
    então é enviada ao terminal (caso da lista);
  - o `tui-term`, usado pelo terminal embutido, escreve com `set_symbol`,
    que não filtra nada.

**Quais caracteres (12):**
- os explícitos do UAX #9: U+202A LRE, U+202B RLE, U+202C PDF, U+202D LRO,
  U+202E RLO, U+2066 LRI, U+2067 RLI, U+2068 FSI e U+2069 PDI. São os nove
  do CVE e do lint `text_direction_codepoint_in_literal` do rustc;
- as marcas implícitas U+200E LRM, U+200F RLM e U+061C ALM.

O terminal não precisa de nenhum deles: os terminais sem BiDi já mostram
árabe e hebraico na ordem lógica.

**Camada 1 — um ponto único, no fim de `ui::draw`.** Só há uma chamada de
desenho em produção (`main.rs:379`), e os testes de UI também passam por
`ui::draw`.

- Um módulo novo, `src/bidi.rs`, tem `is_bidi(char)` e
  `sanitize(&mut Buffer, Style)`.
- `sanitize` percorre as células do frame. Se o símbolo de uma célula
  contém um dos 12, ou um Cc que tenha escapado (por exemplo, do `tui-term`):
  - esses caracteres são retirados do símbolo;
  - a célula ganha a cor de erro do tema;
  - se o símbolo ficar vazio, vira `·`.
- Resultado: nenhum deles chega ao terminal, venha de onde vier. Isso cobre
  árvore, lista de arquivos, nomes dentro de archives e do SFTP, saída do
  git, prompts, mensagens, Alt+X e o editor dentro do frame.
- Custo, medido com um micro-benchmark (`rustc -O`,
  `~/.cache/xt-dev/viewer-mem/bench/bench.rs`, tela com ASCII, bordas e
  acentos):
  - 120×40 (4.800 células): **~10 µs por frame**;
  - 300×100 (30 mil células): ~63 µs por frame.

  O Xylem só redesenha numa tecla, ou a cada 50 ms quando há trabalho em
  andamento. No pior caso, isso dá 0,02% de um núcleo. Para comparar: um
  frame a 60 fps tem 16.700 µs, e o próprio diff do ratatui já percorre
  todas as células a cada frame. Para o caso comum, uma célula com um único
  byte ASCII imprimível sai antes de qualquer outra verificação.

**Camada 2 — viewer** (`text_lines`):
- os 12 viram `·`, como os controles, cada um na sua célula e na cor de
  erro;
- o `Viewer` guarda as posições deles (normalmente nenhuma) para o
  `styled()` colorir;
- o cabeçalho avisa, como o GitHub faz:
  `3 bidirectional control characters (shown as ·)`, na cor de erro (sem
  o `⚠`, que tem largura ambígua em alguns terminais);
- o hex mostra os bytes (`E2 80 AE`).

**Camada 3 — nomes de arquivo:**
- com o cursor num arquivo cujo nome tem um dos 12, o rodapé avisa
  `Name has hidden bidirectional characters: it may not be what it looks like`;
- `O` na lista de arquivos (abrir no programa do sistema), num arquivo
  assim, pede confirmação e
  mostra o nome com escapes: `fatura\u{202E}fdp.sh`. É onde a extensão
  falsa mais pesa: no Windows, `start` executa um `.exe`.

**Fora do escopo:**
- homóglifos (CVE-2021-42694, como o `а` cirílico no lugar do `a`): não há
  como tratar sem quebrar texto legítimo;
- caracteres de largura zero (U+200B, ZWJ, ZWNJ): ZWJ e ZWNJ são
  necessários em emoji e em várias escritas.

**Ordem:** este reforço é o **primeiro PR do subprojeto 1**. É pequeno,
independente e corrige um problema que já existe hoje.

**Testes:**
- `bidi::sanitize`:
  - a célula `a\u{202E}` vira `a` na cor de erro;
  - a célula só com `\u{202E}` vira `·`;
  - células sem nenhum dos 12 não mudam.
- UI (`TestBackend`, como os testes de `ui.rs`): uma pasta com
  `fatura\u{202E}fdp.sh` não deixa nenhum dos 12 no buffer, e o rodapé
  mostra o aviso.
- Viewer: numa linha com RLO e PDF, o `·` aparece nas posições certas, a
  contagem é 2 e o cabeçalho mostra o aviso.
- `O` num nome com bidi pede confirmação.
- Conferência manual: `capture.py` passa a contar 0 nos dois casos.

## Seção 3 — F12, documentação e testes (aprovada)

**Opções novas no F12, página 2, depois de `I`:**
- `J` — `viewer_decompress`: `true` ou `false`. Rótulo
  `Show .gz/.xz/.zst/.bz2 decompressed in the viewer`.
- `K` — `legacy_charset`: `auto`, `cp850`, `cp1252` ou `cp437`. Rótulo
  `Charset of non-UTF-8 files (O in the viewer)`.

As letras seguem a ordem dos PRs: o 3 traz o `J`, o 4 o `K` e o 5 o `L`.
- `L` — `viewer_max_size`: `auto`, 16, 64, 256 ou 1024. Rótulo
  `Largest part of a file the viewer reads`.

As três ficam também no `config.toml` padrão (`--default-config`), com
comentários.

**Documentação, nas seis línguas** (`docs/manual/` em, pt, es, de, fr, zh):
- `viewer.md`: tecla `O`, codificações, comprimidos, limite.
- `configuration.md`: as três opções.
- `ztreewin.md`: o ZTreeWin tem "character sets"; agora há codificações.
- Onde a documentação fala da lista de arquivos (`files.md`): nomes com
  caracteres bidi aparecem marcados e geram aviso (seção 2b).
- Texto da ajuda (F1) do viewer.

**Testes:**
- **`charset`:**
  - `0x00`–`0x7F` mapeiam para ASCII em todas as tabelas, e cada byte
    `0x80`–`0xFF` mapeia para um `char`;
  - amostras em português em CP850 e em 1252 detectadas corretamente;
  - UTF-16 LE/BE com e sem BOM;
  - UTF-8 cortado no meio de uma sequência continua UTF-8;
  - empate cai em 1252;
  - controles e bidi viram `·`.
- **`decompress`:**
  - ida e volta para cada formato. Os codificadores já existem: flate2,
    `ruzstd::encoding`, o `bzip2` e o `lzma-rust2` com `encoder` nas
    dev-deps;
  - entrada truncada mostra o parcial e o aviso;
  - bomba (muitos MiB de zeros) para no limite de saída;
  - xz com dicionário acima do limite é recusado sem alocar;
  - zstd com janela acima do limite (criado com `windowLog` alto) é
    recusado com aviso;
  - zstd com dois frames seguidos mostra os dois;
  - gzip com vários membros;
  - um nível só: `.gz` dentro de `.gz` mostra o `.gz` interno em hex.
- **proptest:** bytes aleatórios em `detect`, `decode`, `sniff` e
  `decompress` nunca causam `panic`.
- **App:**
  - V num `log.gz` mostra o texto;
  - o Autoview descomprime;
  - `O` troca e avisa no rodapé;
  - a escolha manual sobrevive ao `reload`;
  - o cabeçalho mostra o corte com o limite certo;
  - `viewer_decompress = false` mostra o `.gz` em hex;
  - `viewer_max_size` é respeitado.
- **`archive.rs`:** `.tar.xz` com dicionário acima de 64 MiB dá erro claro
  no Alt+F5.

## Ordem dos PRs

Cada PR traz a sua documentação nas seis línguas e os seus testes, e passa
em `cargo clippy --all-targets -- -D warnings` e `cargo test`, como o CI
exige.

1. **Bidi na tela inteira** (seção 2b): `src/bidi.rs`, o filtro no fim de
   `ui::draw`, o aviso nos nomes e a confirmação no `O`. O viewer já ganha o
   `·` marcado e o aviso no cabeçalho, ainda em UTF-8. Independente do
   resto.
2. **`src/decompress.rs` e a correção no `archive.rs`:** leitores com limite,
   `sniff` e `decompress`, ainda sem ligação com o viewer. Fecha a brecha do
   Alt+F5.
3. **Viewer descomprime:** `from_file`, o campo `cut`, o cabeçalho, as cores
   pelo nome sem a extensão de compressão, o Autoview e a opção
   `viewer_decompress`.
4. **`src/charset.rs` e a tecla `O`:** detecção, heurística, tabelas, o
   `legacy_charset` e a escolha manual que sobrevive ao `reload`.
5. **`viewer_max_size`:** o `auto` pela RAM e o teto de 16 MiB no remoto.
6. **`cargo audit` no CI:** separado e pode ir a qualquer momento.
