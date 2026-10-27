# Plano de implementação: codificações, descompressão, limite e bidi no viewer

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Objetivo:** fazer o viewer mostrar codificações antigas e arquivos
comprimidos, com limite de tamanho configurável, e impedir que caracteres
bidi (Trojan Source) cheguem ao terminal em qualquer tela do Xylem.

**Arquitetura:**
- Uma etapa nova entre a leitura dos bytes (disco, archive, remoto) e o
  `Viewer`: primeiro `decompress`, depois `charset::detect`, depois
  `charset::decode`.
- Na tela, um filtro único no fim de `ui::draw` retira os caracteres bidi e
  os de controle de todas as células.
- Nenhuma dependência nova: os descompressores já estão no `Cargo.toml`, e as
  tabelas de codificação são escritas à mão.

**Stack:** Rust 2024, ratatui 0.30, `flate2`, `lzma-rust2` 0.23, `ruzstd` 0.9,
`bzip2` 0.6, proptest (dev).

**Spec:** [viewer-encoding.md](viewer-encoding.md). Leia o spec inteiro antes
de começar; este plano argumenta a partir dele.

## Restrições globais

- **Nenhuma dependência nova** de runtime. Dev-deps também não, salvo se o
  autor aprovar.
- **Nenhum `unsafe`** nos módulos novos (`bidi.rs`, `panic_guard.rs`,
  `decompress.rs`, `charset.rs`).
- **Limites, copiados do spec:**
  - V: lê `viewer_max_size`, com `auto` = RAM/32 entre 16 e 256 MiB;
    descomprime no máximo a mesma quantidade; tempo de 3 s; dicionário do
    xz e janela do zstd de 64 MiB.
  - Autoview: 1 MiB, 1 MiB, 100 ms e 16 MiB.
  - Remoto: lê no máximo 16 MiB.
- **Os 12 caracteres bidi** são U+202A–U+202E, U+2066–U+2069, U+200E, U+200F
  e U+061C. Nenhum deles chega ao terminal, nem um Cc (C0, DEL, C1).
- **Textos que aparecem na tela** ficam em inglês, como o resto da interface.
  Use os textos exatos deste plano.
- **Cada tarefa** termina com `cargo clippy --all-targets -- -D warnings` e
  `cargo test` passando, porque o CI roda os dois em Linux e Windows. Quando
  o código tiver `cfg(windows)`, confira também com
  `cargo clippy --target x86_64-pc-windows-gnu`, se o target estiver
  instalado.
- **Documentação do usuário** vai nas 6 línguas (`docs/manual/` e as pastas
  `pt`, `es`, `de`, `fr`, `zh`), seguindo `docs/manual/TRANSLATING.md`. A
  ajuda F1 e o `xy --man` são gerados desses arquivos.
- **Arquivos temporários** ficam só em `~/.cache/xt-dev/`, nunca em `/tmp`.
  Os testes usam `crate::testing::tempdir()`.
- **Git:**
  - um branch por PR, a partir da `main` (ou do branch do PR anterior, se
    ele ainda não tiver sido mergeado);
  - um commit por tarefa;
  - sem push nem PR sem autorização do autor.
- **Ao fim de cada PR,** atualize o status em [viewers.md](viewers.md)
  ("Como retomar") e no topo do spec.

## Foco da revisão

Casos que o spec implica, que os testes óbvios não cobrem e que mais
provavelmente vão incomodar alguém. Cada um tem um teste na tarefa indicada.

1. **Bidi grudado em letra acentuada** (`e\u{301}\u{202E}` numa célula): o
   acento combinante fica e só o bidi sai. *Tarefa 1.*
2. **Arquivo minúsculo que começa com a assinatura** (só `1F 8B`): nada de
   panic; o viewer mostra os bytes originais em hex com a nota de truncado.
   *Tarefas 5 e 8.*
3. **Arquivo `.gz` que é texto puro:** fica como texto, sem compressão, e as
   cores usam o nome inteiro. *Tarefa 8.*
4. **Exportação `.reg` do Windows** (UTF-16LE com BOM e CRLF): a primeira
   linha é exatamente `Windows Registry Editor Version 5.00`, sem `·` do
   `\r`. *Tarefa 10.*
5. **Tab antes de um bidi:** a posição marcada é a coluna depois de expandir
   o tab (coluna 8), senão a cor cai no caractere errado. *Tarefa 3.*

---

## PR 1 — Bidi na tela inteira (branch `bidi-guard`)

### Tarefa 1: `src/bidi.rs`

**Arquivos:**
- Criar: `src/bidi.rs`.
- Modificar: `src/main.rs` (`mod bidi;`, junto dos outros `mod`).

**Interfaces:**
- Produz:
  - `pub fn is_bidi(c: char) -> bool`
  - `pub fn has_bidi(s: &str) -> bool`
  - `pub fn escape(s: &str) -> String`: troca cada bidi e cada Cc por
    `\u{XXXX}`, em hexadecimal maiúsculo, sem zeros à esquerda;
  - `pub fn sanitize(buf: &mut ratatui::buffer::Buffer, mark: ratatui::style::Style)`.

- [x] **Passo 1: escrever os testes, que devem falhar**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{buffer::Buffer, layout::Rect, style::{Color, Style}};

    const TWELVE: [char; 12] = [
        '\u{061C}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}',
        '\u{202D}', '\u{202E}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
    ];

    #[test]
    fn knows_the_twelve() {
        for c in TWELVE { assert!(is_bidi(c), "{c:?}"); }
        for c in ['a', 'ç', '─', '\u{200B}', '\u{200D}', '\u{FEFF}'] { assert!(!is_bidi(c), "{c:?}"); }
        assert!(has_bidi("fatura\u{202E}fdp.sh"));
        assert!(!has_bidi("ação"));
    }

    #[test]
    fn strips_bidi_and_marks_the_cell() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf[(0, 0)].set_symbol("a\u{202E}");
        buf[(1, 0)].set_symbol("\u{202E}");
        buf[(2, 0)].set_symbol("e\u{301}\u{2066}");
        buf[(3, 0)].set_symbol("b");
        sanitize(&mut buf, Style::new().fg(Color::Red));
        assert_eq!(buf[(0, 0)].symbol(), "a");
        assert_eq!(buf[(0, 0)].fg, Color::Red);
        assert_eq!(buf[(1, 0)].symbol(), "·");
        assert_eq!(buf[(2, 0)].symbol(), "e\u{301}");
        assert_eq!(buf[(3, 0)].symbol(), "b");
        assert_eq!(buf[(3, 0)].fg, Color::Reset);
    }

    #[test]
    fn strips_control_chars_that_slipped_through() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        buf[(0, 0)].set_symbol("\u{1B}");
        buf[(1, 0)].set_symbol("x\u{9B}");
        sanitize(&mut buf, Style::new());
        assert_eq!(buf[(0, 0)].symbol(), "·");
        assert_eq!(buf[(1, 0)].symbol(), "x");
    }

    #[test]
    fn escapes_hidden_chars() {
        assert_eq!(escape("fatura\u{202E}fdp.sh"), "fatura\\u{202E}fdp.sh");
        assert_eq!(escape("a\u{1B}b"), "a\\u{1B}b");
        assert_eq!(escape("ação"), "ação");
    }
}
```

- [x] **Passo 2:** `cargo test bidi::` falha por não compilar (as funções
  ainda não existem).
- [x] **Passo 3: implementar as quatro funções.**
  - Em `sanitize`, percorra `buf.content`. O caminho rápido é a célula cujo
    símbolo é um único byte ASCII entre `0x20` e `0x7E`: siga para a
    próxima sem olhar mais nada.
  - Nas outras células, se algum caractere for bidi ou `char::is_control`,
    remonte o símbolo sem eles. Se ele ficar vazio, use `·`. Aplique `mark`
    com `cell.set_style(mark)`.
- [x] **Passo 4:** `cargo test bidi::` passa.
- [x] **Passo 5:** commit
  `Add bidi module: detect, escape and strip bidi controls from the screen`.

### Tarefa 2: o filtro no fim de `ui::draw`

**Arquivos:**
- Modificar: `src/ui.rs`. A função `draw`, linha 43, tem cinco `return`
  antecipados.

**Interfaces:**
- Consome `bidi::sanitize`.
- Produz `pub fn draw(frame, app)` com a mesma assinatura de hoje. O corpo
  atual vira `fn draw_screen(frame: &mut Frame, app: &mut App)`, e `draw`
  chama `draw_screen` e depois
  `crate::bidi::sanitize(frame.buffer_mut(), Style::new().fg(theme().error))`.

- [x] **Passo 1: escrever o teste em `ui.rs`, que deve falhar**

```rust
#[test]
fn bidi_in_names_never_reaches_the_terminal() {
    use ratatui::{Terminal, backend::TestBackend};
    let tmp = crate::testing::tempdir();
    std::fs::write(tmp.path().join("fatura\u{202E}fdp.sh"), "").unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    app.apply(crate::input::Command::SwitchWindow);
    let mut term = Terminal::new(TestBackend::new(80, 20)).unwrap();
    term.draw(|f| draw(f, &mut app)).unwrap();
    let buf = term.backend().buffer();
    assert!(buf.content.iter().all(|c| !crate::bidi::has_bidi(c.symbol())));
    let screen: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(screen.contains("faturafdp.sh"), "{screen}");
}
```

- [x] **Passo 2:** o teste falha, porque o U+202E está grudado no `a`.
- [x] **Passo 3:** fazer a separação `draw` / `draw_screen`.
- [x] **Passo 4:** `cargo test ui::` passa, e os testes de UI que já existiam
  continuam passando.
- [x] **Passo 5:** commit
  `Strip bidi and control characters from every frame before drawing`.

### Tarefa 3: viewer, camada 2 (`·` marcado e aviso no cabeçalho)

**Arquivos:**
- Modificar: `src/viewer.rs`. São `text_lines` (linha ~823), o struct
  `Viewer` e `styled()` (os dois caminhos, com e sem wrap).
- Modificar: `src/ui.rs`, em `draw_viewer` (cabeçalho, linha ~1962).
- Documentação: `docs/manual/viewer.md` e as 5 traduções.

**Interfaces:**
- Produz:
  - `fn text_lines(text: &str) -> (Vec<String>, Vec<(usize, usize)>)`: as
    linhas e as posições `(linha, coluna em caracteres depois de expandir
    os tabs)` de cada bidi trocado por `·`, em ordem;
  - o campo `bidi: Vec<(usize, usize)>` no `Viewer`;
  - `pub fn bidi_count(&self) -> usize`.
- Neste PR, `from_bytes` continua chamando `text_lines` com
  `&String::from_utf8_lossy(&data)`. O PR 4 troca isso pelo texto já
  decodificado.

- [x] **Passo 1: escrever os testes em `viewer.rs`, que devem falhar**

```rust
#[test]
fn bidi_controls_show_as_marked_dots() {
    let mut v = Viewer::from_bytes(PathBuf::from("x.txt"), 0, b"a\xE2\x80\xAEb\xE2\x80\xACc\n\t\xE2\x81\xA6x".to_vec());
    assert_eq!(v.visible_lines(2), ["a·b·c", "        ·x"]);
    assert_eq!(v.bidi_count(), 3);
    let error = crate::config::theme().error;
    let marked = |row: &StyledRow| row.iter().filter(|s| s.1 == Some(error)).map(|s| s.0.clone()).collect::<Vec<_>>();
    let rows = v.styled(2);
    assert_eq!(marked(&rows[0]), ["·", "·"]);
    assert_eq!(marked(&rows[1]), ["·"]);
    v.toggle_wrap();
    assert_eq!(marked(&v.styled(2)[0]), ["·", "·"]);
}
```

  E em `ui.rs`: um teste que desenha o viewer de um arquivo com dois bidi e
  confere que a linha 0 contém
  `2 bidirectional control characters (shown as ·)`.
- [x] **Passo 2:** os testes falham.
- [x] **Passo 3: implementar.**
  - Em `text_lines`, a mesma substituição que já é feita para
    `is_control` vale para `bidi::is_bidi`, guardando `(i, col)`.
  - Em `styled()`, a cor de um caractere numa posição de bidi é
    `Some(theme().error)`. Faça a busca com `binary_search` em `self.bidi`.
  - No cabeçalho, se `bidi_count() > 0`, acrescente o span
    `format!("  {n} bidirectional control characters (shown as ·)")` na cor
    `theme().error`.
- [x] **Passo 4:** os testes passam.
- [x] **Passo 5:** documentação. Em `viewer.md` (e nas traduções), um
  parágrafo: caracteres de controle de direção do texto (Trojan Source,
  CVE-2021-42574) aparecem como `·` vermelho, e o cabeçalho diz quantos são.
- [x] **Passo 6:** commit
  `Viewer: show bidi controls as marked dots and count them in the header`.

### Tarefa 4: nomes, camada 3 (aviso no rodapé e confirmação no `O`)

**Arquivos:**
- Modificar:
  - `src/ui.rs`: rodapé da janela de arquivos, o `match &app.message` na
    linha ~1162;
  - `src/app/actions.rs`: `open_with`, linha 366;
  - `src/app/prompts.rs`: `ConfirmKind` e `confirmed`.
- Testes: `src/app/tests.rs` e `src/ui.rs`.
- Documentação: `docs/manual/files.md` e as 5 traduções.

**Interfaces:**
- Produz:
  - `pub(crate) fn selected_name_has_bidi(&self) -> bool` em `App`: arquivo
    selecionado, `bidi::has_bidi(&name.to_string_lossy())`;
  - `ConfirmKind::OpenHidden { tagged: bool }`;
  - `pub(crate) fn open_sources(&mut self, tagged: bool)`: o corpo atual de
    `open_with`. O `open_with` passa a perguntar antes, quando preciso.
- Textos:
  - rodapé:
    `Name has hidden bidirectional characters: it may not be what it looks like`
    (na cor `theme().error`, só quando não há `app.message`);
  - pergunta:
    `format!("Hidden bidirectional characters in {}. Open anyway?", bidi::escape(&name))`,
    em que `name` é o primeiro nome com bidi entre as fontes.

- [x] **Passo 1: escrever o teste em `app/tests.rs`, que deve falhar**

```rust
#[test]
fn o_asks_before_opening_a_name_with_hidden_bidi() {
    let tmp = crate::testing::tempdir();
    std::fs::write(tmp.path().join("fatura\u{202E}fdp.sh"), "").unwrap();
    let mut app = App::new(tmp.path().to_path_buf());
    press(&mut app, KeyCode::Enter);
    assert!(app.selected_name_has_bidi());
    press(&mut app, KeyCode::Char('o'));
    let c = app.confirm.as_ref().expect("asks first");
    assert!(c.question.contains("fatura\\u{202E}fdp.sh"), "{}", c.question);
    press(&mut app, KeyCode::Char('n'));
    assert!(app.confirm.is_none());
}
```

  E em `ui.rs`: um teste que desenha a janela de arquivos com o cursor nesse
  nome e confere que a penúltima linha contém `Name has hidden bidirectional
  characters`.
- [x] **Passo 2:** os testes falham.
- [x] **Passo 3:** implementar. Em `confirmed`,
  `OpenHidden { tagged }` com `'y'` chama `open_sources(tagged)`.
- [x] **Passo 4:** os testes passam. Rode também o teste completo:
  `cargo test`.
- [x] **Passo 5:** documentação. Em `files.md` (e nas traduções): nomes com
  caracteres de direção escondidos aparecem marcados em vermelho, o rodapé
  avisa e o `O` pede confirmação.
- [x] **Passo 6: conferência manual.** Rode
  `cargo build --release`. Depois rode
  `python3 -I ~/.cache/xt-dev/viewer-mem/scripts/capture.py $PWD/target/release/xy ~/.cache/xt-dev/viewer-mem/bidi ~/.cache/xt-dev/viewer-mem/home $'\r'`.
  O esperado é `RLO U+202E bytes sent to terminal: 0`. Repita com as
  teclas `$'\r|\x1b[B|v'` e o mesmo resultado. Se os scripts não
  existirem, a seção 2b do spec diz o que eles fazem.
- [x] **Passo 7:** commit
  `Warn about file names with hidden bidi characters and confirm O on them`.
  Atualize o status em `viewers.md` e no spec. Fim do PR 1.

---

## PR 2 — Módulo de descompressão e correção no Alt+F5 (branch `decompress`)

### Tarefa 5: `src/panic_guard.rs` e `src/decompress.rs` (gzip e bzip2)

**Arquivos:**
- Criar: `src/panic_guard.rs` e `src/decompress.rs`.
- Modificar: `src/main.rs`. Acrescente `mod`. Depois de
  `Keyboard::detect()` (linha ~214), instale o hook mais externo:
  `let previous = take_hook(); set_hook(Box::new(move |info| if !panic_guard::active() { previous(info) }))`.

**Interfaces:**
- Produz, em `panic_guard`:
  - `pub fn catch<T>(f: impl FnOnce() -> T) -> Option<T>`: liga uma flag
    `thread_local`, roda `catch_unwind(AssertUnwindSafe(f))`, desliga a flag
    em qualquer caso e devolve `ok()`;
  - `pub fn active() -> bool`.
- Produz, em `decompress`:
  - `pub enum Compression { Gzip, Xz, Zstd, Bzip2 }`, com `Clone`, `Copy`,
    `Debug`, `PartialEq` e `Eq`;
  - `Compression::name(self) -> &'static str`: `"gzip"`, `"xz"`, `"zstd"` ou
    `"bzip2"`;
  - `Compression::extension(self) -> &'static str`: `"gz"`, `"xz"`, `"zst"`
    ou `"bz2"`;
  - `pub fn sniff(head: &[u8]) -> Option<Compression>`, com as assinaturas
    do spec;
  - as constantes:
    - `pub const CHUNK: usize = 64 << 10;`
    - `pub const VIEW_DICT: u32 = 64 << 20;`
    - `pub const AUTOVIEW_DICT: u32 = 16 << 20;`
    - `pub const VIEW_TIME: Duration = Duration::from_secs(3);`
    - `pub const AUTOVIEW_TIME: Duration = Duration::from_millis(100);`
    - `pub const TRUNCATED: &str = "compressed data truncated or corrupt";`
  - `#[derive(Clone, Copy, Debug, PartialEq)] pub struct Limits { pub output: usize, pub dict: u32, pub time: Duration }`;
  - `pub struct Decompressed { pub data: Vec<u8>, pub cut: bool, pub note: Option<String> }`;
  - `pub fn decompress(input: &[u8], kind: Compression, limits: Limits) -> Decompressed`.
- Comportamento:
  - lê do decoder em pedaços de `CHUNK`;
  - para em `limits.output` bytes (`cut = true`);
  - confere `Instant` depois de cada pedaço; se passar de `limits.time`,
    `cut = true` e a nota é `decompression stopped after {t}`, em que `t` é
    `"{n} ms"` abaixo de 1 s e `"{n} s"` a partir disso;
  - em erro do decoder, mantém o que já saiu e usa a nota `TRUNCATED`;
  - tudo dentro de `panic_guard::catch`; um panic vira dados vazios com a
    nota `TRUNCATED`;
  - gzip usa `flate2::read::MultiGzDecoder` e bzip2 usa
    `bzip2::read::MultiBzDecoder`.

- [ ] **Passo 1: escrever os testes, que devem falhar.** Use os helpers
  `gz(&[u8]) -> Vec<u8>` e `bz2(&[u8]) -> Vec<u8>` com os encoders, como em
  `archive.rs:755`, e `TEXT = b"linha de log\n"`.

```rust
fn limits(output: usize) -> Limits { Limits { output, dict: VIEW_DICT, time: VIEW_TIME } }
const MIB: usize = 1 << 20;

#[test] fn sniffs_by_signature() {
    assert_eq!(sniff(&gz(b"x")), Some(Compression::Gzip));
    assert_eq!(sniff(b"\xFD7zXZ\0..."), Some(Compression::Xz));
    assert_eq!(sniff(b"\x28\xB5\x2F\xFD..."), Some(Compression::Zstd));
    assert_eq!(sniff(b"BZh91AY"), Some(Compression::Bzip2));
    assert_eq!(sniff(b"plain"), None);
    assert_eq!(sniff(b"\x1F"), None);
}
#[test] fn gzip_and_bzip2_round_trip() {
    for (kind, packed) in [(Compression::Gzip, gz(TEXT)), (Compression::Bzip2, bz2(TEXT))] {
        let d = decompress(&packed, kind, limits(MIB));
        assert_eq!((d.data.as_slice(), d.cut, d.note), (TEXT, false, None));
    }
}
#[test] fn gzip_members_are_joined() {
    let mut two = gz(b"one\n"); two.extend(gz(b"two\n"));
    assert_eq!(decompress(&two, Compression::Gzip, limits(MIB)).data, b"one\ntwo\n");
}
#[test] fn bomb_stops_at_the_output_limit() {
    let d = decompress(&gz(&vec![0; 16 * MIB]), Compression::Gzip, limits(MIB));
    assert_eq!(d.data.len(), MIB); assert!(d.cut);
}
#[test] fn stops_at_the_time_limit() {
    let big = gz(&TEXT.repeat(50_000));
    let d = decompress(&big, Compression::Gzip, Limits { time: Duration::ZERO, ..limits(64 * MIB) });
    assert!(d.data.len() <= CHUNK && d.cut);
    assert_eq!(d.note.as_deref(), Some("decompression stopped after 0 ms"));
}
#[test] fn truncated_keeps_what_was_decoded() {
    let all = TEXT.repeat(10_000); let packed = gz(&all);
    let d = decompress(&packed[..packed.len() / 2], Compression::Gzip, limits(64 * MIB));
    assert!(!d.data.is_empty() && all.starts_with(&d.data));
    assert_eq!(d.note.as_deref(), Some(TRUNCATED));
}
#[test] fn only_the_signature_is_truncated_not_a_panic() {
    let d = decompress(b"\x1F\x8B", Compression::Gzip, limits(MIB));
    assert!(d.data.is_empty()); assert_eq!(d.note.as_deref(), Some(TRUNCATED));
}
#[test] fn one_level_only() {
    let inner = gz(b"hello");
    assert_eq!(decompress(&gz(&inner), Compression::Gzip, limits(MIB)).data, inner);
}
#[test] fn guard_catches_a_panic() {
    assert_eq!(crate::panic_guard::catch(|| -> u8 { panic!("boom") }), None);
    assert!(!crate::panic_guard::active());
}
proptest! {
    #[test] fn never_panics(bytes in prop::collection::vec(any::<u8>(), 0..4096), k in 0usize..4) {
        let kind = [Compression::Gzip, Compression::Xz, Compression::Zstd, Compression::Bzip2][k];
        let _ = sniff(&bytes);
        let _ = decompress(&bytes, kind, limits(MIB));
    }
}
```

  Enquanto a tarefa 6 não fizer xz e zstd, eles devolvem a nota `TRUNCATED`.
- [ ] **Passo 2:** `cargo test decompress:: panic_guard::` falha.
- [ ] **Passo 3:** implementar.
- [ ] **Passo 4:** os testes passam.
- [ ] **Passo 5:** commit
  `Add decompress module (gzip, bzip2) with output, time and panic guards`.

### Tarefa 6: xz e zstd com limite de memória

**Arquivos:**
- Modificar: `src/decompress.rs`.

**Interfaces:**
- Produz:
  - `pub fn xz_reader<R: Read>(r: R, dict: u32) -> lzma_rust2::XzReader<R>`:
    `XzReader::new_mem_limit(r, true, lzma_rust2::lzma2_get_memory_usage(dict))`;
  - `pub fn zstd_reader<R: Read>(r: R, window: u32) -> io::Result<ruzstd::decoding::StreamingDecoder<R, ruzstd::decoding::FrameDecoder>>`:
    `StreamingDecoder::new_with_max_window_size(r, window as u64)`. Lê um
    frame só, que é o que o `archive.rs` precisa;
  - `pub fn memory_note(limit: u32) -> String`:
    `format!("needs more than {} MiB of memory to decompress", limit >> 20)`.
- Em `decompress`:
  - **zstd:** percorre os frames sobre `let mut rest: &[u8] = input`,
    criando um `StreamingDecoder` por frame enquanto `rest` não estiver
    vazio.
    - `FrameDecoderError::WindowSizeTooBig { .. }` dá a nota
      `memory_note(limits.dict)`.
    - `SkipFrame { length, .. }` (dentro de `ReadFrameHeaderError`) pula
      `length` bytes de `rest`.
  - **xz:** um erro cuja mensagem contém `"mem_limit"` dá a nota
    `memory_note(limits.dict)`. É a mensagem do `lzma-rust2` 0.23
    (`needed memory too big for mem_limit_kb`), e o teste abaixo acusa se
    ela mudar.

- [ ] **Passo 1: escrever os testes, que devem falhar.**
  - O helper `xz_header_with_dict_props(props: u8) -> Vec<u8>` monta um xz
    só com cabeçalhos, que pede um dicionário enorme sem precisar de um
    encoder com esse dicionário. O CRC32 usa `flate2::Crc`, que é API
    pública do flate2. O código está abaixo porque nem a assinatura nem o
    teste determinam os bytes.

```rust
fn crc32(bytes: &[u8]) -> [u8; 4] { let mut c = flate2::Crc::new(); c.update(bytes); c.sum().to_le_bytes() }
/// Stream header (CRC32 check) + LZMA2 block header whose dictionary is `props` (40 = 4 GiB).
fn xz_header_with_dict_props(props: u8) -> Vec<u8> {
    let mut v = b"\xFD7zXZ\0".to_vec();
    v.extend([0x00, 0x01]); v.extend(crc32(&[0x00, 0x01]));
    let block = [0x02, 0x00, 0x21, 0x01, props, 0x00, 0x00, 0x00]; // size=(2+1)*4, flags, LZMA2, 1 prop byte, padding
    v.extend(block); v.extend(crc32(&block));
    v
}
#[test] fn xz_round_trip_and_dictionary_limit() {
    let mut enc = lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::with_preset(6)).unwrap();
    enc.write_all(TEXT).unwrap(); let packed = enc.finish().unwrap();
    assert_eq!(decompress(&packed, Compression::Xz, limits(MIB)).data, TEXT);
    let small = Limits { dict: 1 << 20, ..limits(MIB) }; // preset 6 = 8 MiB dictionary
    assert_eq!(decompress(&packed, Compression::Xz, small).note, Some(memory_note(1 << 20)));
    let huge = xz_header_with_dict_props(40);
    assert_eq!(decompress(&huge, Compression::Xz, limits(MIB)).note, Some(memory_note(VIEW_DICT)));
}
#[test] fn zstd_frames_and_window_limit() {
    let mut two = Vec::new();
    ruzstd::encoding::compress(&b"one\n"[..], &mut two, ruzstd::encoding::CompressionLevel::Fastest);
    ruzstd::encoding::compress(&b"two\n"[..], &mut two, ruzstd::encoding::CompressionLevel::Fastest);
    assert_eq!(decompress(&two, Compression::Zstd, limits(MIB)).data, b"one\ntwo\n");
    // Magic, descriptor 0x00, window descriptor 0x88 = 2^(10+17) = 128 MiB.
    let big_window = [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x88];
    assert_eq!(decompress(&big_window, Compression::Zstd, limits(MIB)).note, Some(memory_note(VIEW_DICT)));
}
```

- [ ] **Passo 2:** os testes falham.
- [ ] **Passo 3:** implementar. Se a forma de chamar o `lzma-rust2` ou o
  `ruzstd` for diferente da escrita acima, ajuste o helper, não a
  expectativa.
- [ ] **Passo 4:** `cargo test decompress::` passa, inclusive o proptest.
- [ ] **Passo 5:** commit `decompress: xz and zstd with memory limits`.

### Tarefa 7: `archive.rs` usa os leitores com limite

**Arquivos:**
- Modificar: `src/archive.rs`, em `each_tar_entry`, linhas ~314–321.

**Interfaces:**
- Consome `decompress::xz_reader(file, decompress::VIEW_DICT)` e
  `decompress::zstd_reader(file, decompress::VIEW_DICT)`.

- [ ] **Passo 1: escrever o teste em `archive.rs`, que deve falhar.** Um
  arquivo `big.tar.xz` com o conteúdo `xz_header_with_dict_props(40)`.
  Torne o helper `pub(crate)` dentro de `#[cfg(test)]` em `decompress.rs`.
  Listar ou extrair dá um `Err` cuja mensagem contém `mem_limit`. Hoje o
  `XzReader::new` sem limite tentaria alocar 4 GiB.
- [ ] **Passo 2:** o teste falha (ou aloca: rode-o sozinho primeiro).
- [ ] **Passo 3:** trocar os dois construtores.
- [ ] **Passo 4:** `cargo test archive::` passa, inclusive os testes de
  `.tar.xz` e `.tar.zst` que já existiam.
- [ ] **Passo 5:** commit
  `Archives: cap xz dictionary and zstd window at 64 MiB`. Atualize o status
  em `viewers.md`. Fim do PR 2.

---

## PR 3 — Viewer descomprime (branch `viewer-decompress`)

### Tarefa 8: `Viewer::from_file` e `LoadOptions`

**Arquivos:**
- Modificar: `src/viewer.rs` (construtores, `reload`, `truncated`).
- Modificar: `src/app/view.rs` (`load_viewer_limited`, `sync_autoview`).
- Modificar: `src/ui.rs` (cabeçalho do viewer, linhas ~1962–1975).

**Interfaces:**
- Consome `decompress::{sniff, decompress, Limits, Compression, ...}`.
- Produz:
  - `#[derive(Clone, Copy, Debug, PartialEq)] pub struct LoadOptions { pub decompress: bool, pub limits: decompress::Limits }`;
  - `LoadOptions::view(limit: u64) -> Self`: `decompress: true`,
    `output = limit`, `VIEW_DICT`, `VIEW_TIME`;
  - `LoadOptions::autoview() -> Self`: 1 MiB, `AUTOVIEW_DICT`,
    `AUTOVIEW_TIME`;
  - `pub fn from_file(path: PathBuf, size: u64, raw: Vec<u8>, load: LoadOptions) -> Viewer`;
  - `pub fn open(path: PathBuf, load: LoadOptions) -> io::Result<Viewer>`:
    lê até `load.limits.output` bytes e marca `on_disk`. O nome não é
    `open_with`, para não confundir com o `App::open_with` da tecla `O`;
  - `open_limited(path, limit)` continua, como
    `open(path, LoadOptions::view(limit))`;
  - campos novos: `load: LoadOptions`,
    `compression: Option<(Compression, u64)>` (u64 = tamanho do arquivo),
    `cut: bool`;
  - `pub fn truncated(&self) -> bool { self.cut }` e
    `pub fn limit(&self) -> u64`;
  - `App::load_viewer_limited(path, size, limit)` vira
    `load_viewer_with(path, size, load: LoadOptions)`. O `load_viewer` passa
    `LoadOptions::view(viewer::MAX_BYTES)` (o PR 5 troca pelo limite
    configurado), e o `sync_autoview` passa `LoadOptions::autoview()`.
- Regras de `from_file`:
  - `cut` começa como `size > raw.len()`;
  - com `load.decompress` e `sniff(&raw)` reconhecendo o formato:
    - se `decompress` devolver dados, eles viram `data`,
      `cut |= d.cut`, `compression = Some((kind, size))` e
      `note = d.note`;
    - se devolver vazio, `data` fica com o `raw` (hex, como hoje) e a nota
      vem junto;
    - se `load.limits.dict == AUTOVIEW_DICT` e a nota for a de memória,
      acrescente ` (V to view)`;
  - **cores:** só quando há compressão e o nome termina em
    `.{kind.extension()}`, o `Highlighter` recebe o nome sem essa extensão;
  - **TAR:** se `data[257..262] == *b"ustar"`, a nota é
    `TAR archive: Alt+F5 opens it as a folder`;
  - `reload` usa `Self::open(path, self.load)`.
- Cabeçalho:
  - `format!("  {} → {} bytes", kind.name(), fmt_num(data_len))`;
  - corte: `format!("  (showing first {} MiB)", limit >> 20)` ou
    `"... MiB decompressed)"` quando há compressão.

- [ ] **Passo 1: escrever os testes em `viewer.rs`, que devem falhar.** Use
  os helpers da tarefa 5 (`gz`) e um `LOAD = LoadOptions::view(MAX_BYTES)`.
  - `gz_shows_text`: `from_file("app.log.gz", n, gz(b"one\ntwo\n"), LOAD)`
    dá `visible_lines(2) == ["one", "two"]`,
    `compression == Some((Gzip, n))` e `!truncated()`.
  - `cut_counts_the_decompressed_side`: 4 MiB de texto comprimido com
    `LoadOptions::view(1 << 20)` dá `truncated()` mesmo com
    `size < data.len()`.
  - `plain_text_named_gz_stays_text`: `b"just text"` num `notes.txt.gz` dá
    `compression == None` e o texto como está (Foco 3).
  - `signature_only_shows_raw_hex`: `b"\x1F\x8B"` dá `mode == Hex` e
    `note == Some(TRUNCATED)` (Foco 2).
  - `decompressed_source_gets_its_colors`: `x.rs.gz` com `fn main() {}`
    dá `language() == Some("Rust")`.
  - `tar_gz_says_alt_f5`: um `.tar.gz` criado pelo helper de testes do
    `archive.rs` dá a nota do TAR.
  - `decompress_off_shows_hex`:
    `LoadOptions { decompress: false, ..LOAD }` dá `mode == Hex` e
    `compression == None`.
  - `reload_keeps_the_load_options`: depois de `reload()`, `load` é o
    mesmo.
  - Em `app/tests.rs`: com o Autoview aberto num `app.log.gz`, o
    `app.autoview` mostra o texto.
- [ ] **Passo 2:** os testes falham.
- [ ] **Passo 3:** implementar.
- [ ] **Passo 4:** `cargo test` passa.
- [ ] **Passo 5:** commit
  `Viewer: show .gz/.xz/.zst/.bz2 decompressed, with an honest cut notice`.

### Tarefa 9: opção `viewer_decompress` (F12, página 2, `J`)

**Arquivos:**
- Modificar `src/config.rs`:
  - o campo `pub viewer_decompress: bool`, com default `true`;
  - o comentário em `DEFAULT_CONFIG`, depois de `viewer_line_numbers`:
    `# Show .gz/.xz/.zst/.bz2 files decompressed in the viewer.`;
  - a lista de serialização, linha ~276.
- Modificar `src/settings.rs`:
  - `Opt::ViewerDecompress`;
  - `('J', "Show .gz/.xz/.zst/.bz2 decompressed in the viewer", Opt::ViewerDecompress)`
    na página 2;
  - `value` mostra `yes_no`;
  - `cycle` alterna.
- Modificar `src/app/view.rs`: `LoadOptions.decompress` vem de
  `self.options.viewer_decompress`.
- Documentação: `viewer.md` (seção sobre arquivos comprimidos: formatos,
  limites, a nota do TAR) e `configuration.md`, nas 6 línguas.

- [ ] **Passo 1: escrever os testes, que devem falhar.**
  - O `default_config_text_means_defaults`, que já existe, continua
    passando.
  - Um teste no estilo dos de `settings.rs`: a tecla `J` na página 2
    alterna `viewer_decompress`.
  - Em `app/tests.rs`: com `viewer_decompress = false`, o V num `.gz`
    mostra hex.
- [ ] **Passo 2:** os testes falham.
- [ ] **Passo 3:** implementar.
- [ ] **Passo 4:** os testes passam.
- [ ] **Passo 5:** escrever a documentação.
- [ ] **Passo 6:** commit `Add viewer_decompress option (F12 page 2, J)`.
  Atualize o status. Fim do PR 3.

---

## PR 4 — Codificações e a tecla `O` (branch `viewer-charset`)

### Tarefa 10: `src/charset.rs`

**Arquivos:**
- Criar: `src/charset.rs` e registrar o `mod` em `src/main.rs`.

**Interfaces:**
- Produz:
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Charset { Utf8, Utf16Le, Utf16Be, Cp437, Cp850, Cp1252 }`;
  - `name()`: `"UTF-8"`, `"UTF-16LE"`, `"UTF-16BE"`, `"CP437"`, `"CP850"`
    ou `"Windows-1252"`;
  - `description()`: `"Unicode"`, `"Unicode, little-endian"`,
    `"Unicode, big-endian"`, `"DOS, United States"`,
    `"DOS, Western Europe"` ou `"Windows, Western Europe"`;
  - `next()` na ordem UTF-8 → CP850 → 1252 → CP437 → UTF-16LE → UTF-16BE
    → UTF-8;
  - `pub struct Detected { pub charset: Charset, pub binary: bool }`;
  - `pub fn detect(data: &[u8], fallback: Option<Charset>) -> Detected`.
    `None` usa a heurística;
  - `pub fn decode(data: &[u8], charset: Charset) -> String`: tira o BOM da
    própria codificação, se houver; byte indefinido vira U+FFFD; byte
    ímpar no fim do UTF-16 é ignorado;
  - `pub fn guess_legacy(data: &[u8]) -> Charset`: devolve só `Cp850` ou
    `Cp1252`.
- **Tabelas:** `const CP437`, `CP850` e `CP1252: [char; 128]`, para os bytes
  `0x80`–`0xFF`. Gere-as com o Python, offline, e cole o resultado:
  `python3 -c "print(repr(bytes(range(0x80,0x100)).decode('cp850')))"`,
  `cp437` e `cp1252` com `errors='replace'`. Os bytes `0x00`–`0x7F` são
  sempre ASCII: nos códigos do DOS, os controles não viram ☺☻.
- **Detecção:** a ordem e os limiares estão no spec (seção 1, "Ordem da
  detecção", passos 2 a 6).
  - UTF-8 válido inclui o caso em que `from_utf8` falha com
    `error_len() == None`, que é uma sequência cortada no fim.
  - O binário usa o `looks_binary` que já existe.
- **Heurística** (`guess_legacy`), nos primeiros 64 KiB, para cada byte
  `b ≥ 0x80` na posição `i`:
  - **vizinho:** é `data[i-1]` se for letra ASCII; senão `data[i+1]`, se for
    letra ASCII;
  - **ponto por letra:** se houver vizinho, para cada candidata entre
    CP850 e 1252, some 1 quando `table[b - 0x80]` for alfabético e tiver a
    mesma caixa do vizinho (`is_lowercase` ou `is_uppercase`);
  - **caixa do DOS:** se `b` estiver em `0xB3..=0xDA` e um dos vizinhos
    imediatos estiver em `0xB0..=0xDF`, some 1 para o CP850;
  - **bytes indefinidos no 1252:** se `b` for `0x81`, `0x8D`, `0x8F`,
    `0x90` ou `0x9D`, some 1 para o CP850;
  - **resultado:** CP850 se tiver mais pontos que o 1252; senão 1252.

- [ ] **Passo 1: escrever os testes, que devem falhar.** Use um helper
  `encode(s: &str, table: &[char; 128]) -> Vec<u8>`, que faz a busca
  reversa na tabela.

```rust
const PT: &str = "Ação, coração e informação não são fáceis; café à vista.";
#[test] fn tables_are_ascii_below_0x80_and_known_above() {
    assert_eq!(decode(b"abc\t\n", Charset::Cp850), "abc\t\n");
    assert_eq!(decode(&[0x87, 0xA0, 0xC6, 0xE4, 0xB3, 0xDA], Charset::Cp850), "çáãõ│┌");
    assert_eq!(decode(&[0x87, 0xE0, 0xB3], Charset::Cp437), "çα│");
    assert_eq!(decode(&[0x80, 0x87, 0xE7, 0x81], Charset::Cp1252), "€‡ç\u{FFFD}");
}
#[test] fn guesses_cp850_and_1252_from_portuguese() {
    assert_eq!(guess_legacy(&encode(PT, &CP850)), Charset::Cp850);
    assert_eq!(guess_legacy(&encode(PT, &CP1252)), Charset::Cp1252);
    assert_eq!(guess_legacy(b"plain ascii"), Charset::Cp1252); // tie
}
#[test] fn detects_in_order() {
    assert_eq!(detect("ação".as_bytes(), None).charset, Charset::Utf8);
    assert_eq!(detect(&"ação".as_bytes()[..4], None).charset, Charset::Utf8); // cut mid-sequence
    assert_eq!(detect(&encode(PT, &CP850), None).charset, Charset::Cp850);
    assert_eq!(detect(&encode(PT, &CP1252), Some(Charset::Cp437)).charset, Charset::Cp437);
    let le: Vec<u8> = "plain text here".encode_utf16().flat_map(u16::to_le_bytes).collect();
    let be: Vec<u8> = "plain text here".encode_utf16().flat_map(u16::to_be_bytes).collect();
    assert_eq!((detect(&le, None).charset, detect(&le, None).binary), (Charset::Utf16Le, false));
    assert_eq!(detect(&be, None).charset, Charset::Utf16Be);
    assert!(detect(b"\x7FELF\0\x01\x02", None).binary);
}
#[test] fn utf16_with_bom_crlf_and_a_cut() {
    let reg = "\u{FEFF}Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER]\r\n";
    let mut bytes: Vec<u8> = reg.encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert_eq!(detect(&bytes, None).charset, Charset::Utf16Le);
    bytes.push(0x41); // odd byte at the end
    assert!(decode(&bytes, Charset::Utf16Le).starts_with("Windows Registry Editor Version 5.00\r\n"));
    assert_eq!(decode(&[0x00, 0xD8], Charset::Utf16Le), "\u{FFFD}"); // lone surrogate
}
proptest! {
    #[test] fn never_panics(bytes in prop::collection::vec(any::<u8>(), 0..4096), k in 0usize..6) {
        let cs = [Charset::Utf8, Charset::Utf16Le, Charset::Utf16Be, Charset::Cp437, Charset::Cp850, Charset::Cp1252][k];
        let _ = detect(&bytes, None); let _ = guess_legacy(&bytes); let _ = decode(&bytes, cs);
    }
}
```

  O teste da exportação `.reg` (Foco 4) também confere, na tarefa 11, que
  a primeira linha do viewer é exatamente
  `Windows Registry Editor Version 5.00`.
- [ ] **Passo 2:** `cargo test charset::` falha.
- [ ] **Passo 3:** implementar.
- [ ] **Passo 4:** os testes passam.
- [ ] **Passo 5:** commit
  `Add charset module: UTF-16, CP437, CP850, 1252 and a 850/1252 guess`.

### Tarefa 11: o viewer usa `charset`, a tecla `O` e a opção `legacy_charset` (F12 `K`)

**Arquivos:**
- Modificar:
  - `src/viewer.rs` (`from_file`, `handle_key`, `reload`, campos novos);
  - `src/ui.rs` (cabeçalho e palavra de menu);
  - `src/config.rs`: `pub enum LegacyCharset { Auto, Cp850, Cp1252, Cp437 }`
    com `#[serde(rename_all = "lowercase")]`, `impl Choice`,
    `pub fn fallback(self) -> Option<Charset>`, o campo
    `legacy_charset: LegacyCharset` (default `Auto`), `DEFAULT_CONFIG` e a
    serialização;
  - `src/settings.rs`: `('K', "Charset of non-UTF-8 files (O in the viewer)", Opt::LegacyCharset)`;
  - `src/app/view.rs`: `LoadOptions.fallback`.
- Documentação: `viewer.md`, `configuration.md` e `ztreewin.md` (a linha
  do viewer na tabela de comparação), nas 6 línguas.

**Interfaces:**
- Consome a tarefa 10 e o `LoadOptions` da tarefa 8, que ganha
  `pub fallback: Option<Charset>`. Em `view()` e `autoview()` ele é
  `None`.
- Produz:
  - os campos `charset: Charset` e `charset_manual: bool`;
  - `pub fn charset(&self) -> Charset`;
  - `pub fn cycle_charset(&mut self)`: troca para `next()`, refaz `lines`,
    `bidi` e o `Highlighter` a partir de `data`, marca a escolha como
    manual e põe a nota `format!("Codepage: {} ({})", name, description)`.
- `from_file`: `detect` sobre os dados já descomprimidos. Com `binary`,
  modo hex; senão, `text_lines(&decode(..))`.
- `handle_key`: `'o'` chama `cycle_charset`.
- Menu do viewer: a palavra `"cOdepage"` vem depois de `"Edit"`.
- Cabeçalho: `format!("  {}", charset.name())` quando a codificação não for
  `Utf8`.
- `reload`: se `charset_manual`, reaplica a codificação escolhida; senão,
  detecta de novo.

- [ ] **Passo 1: escrever os testes, que devem falhar.**
  - `cp850_file_reads_as_portuguese`: o `from_file` do `encode(PT, &CP850)`
    dá `visible_lines(1) == [PT]` e `charset() == Cp850`.
  - `o_cycles_and_says_so`: com o `handle_key('o')` em UTF-8, `charset()`
    vira `Cp850` e
    `note == Some("Codepage: CP850 (DOS, Western Europe)")`. Seis toques
    voltam a UTF-8.
  - `manual_choice_survives_reload`: arquivo em disco, `O` para CP437,
    depois `reload()`, e `charset()` continua `Cp437`.
  - `search_finds_accented_text_in_cp850`: `Pattern` de `ação` encontra o
    termo. Use o mesmo `SearchSpec` dos testes de busca do viewer.
  - `reg_export_first_line_is_clean`: o `.reg` da tarefa 10 dá
    `visible_lines(1) == ["Windows Registry Editor Version 5.00"]`
    (Foco 4).
  - Em `config.rs`: `legacy_charset = "cp850"` é lido, e um valor inválido
    dá erro.
  - Em `settings.rs`: `K` cicla auto → cp850 → cp1252 → cp437.
- [ ] **Passo 2:** os testes falham.
- [ ] **Passo 3:** implementar.
- [ ] **Passo 4:** `cargo test` passa.
- [ ] **Passo 5:** escrever a documentação.
- [ ] **Passo 6:** commit
  `Viewer: detect legacy charsets and UTF-16; O cycles the codepage`.
  Atualize o status. Fim do PR 4.

---

## PR 5 — Limite configurável (branch `viewer-max-size`)

### Tarefa 12: `viewer_max_size` e a RAM da máquina

**Arquivos:**
- Modificar `src/platform/unix.rs` e `src/platform/windows.rs`:
  `pub fn memory() -> Option<Memory>`, com a leitura de memória que hoje
  está dentro de `system()`. O `system()` passa a chamá-la.
- Modificar `src/config.rs`:
  - `pub enum ViewerMaxSize { Auto, Mib16, Mib64, Mib256, Mib1024 }`, com
    `#[serde(rename = "auto"|"16"|"64"|"256"|"1024")]`;
  - `impl Choice`;
  - `pub fn bytes(self, total_ram: Option<u64>) -> u64`;
  - o campo `viewer_max_size` (default `Auto`), `DEFAULT_CONFIG` e a
    serialização.
- Modificar `src/settings.rs`:
  `('L', "Largest part of a file the viewer reads", Opt::ViewerMaxSize)`.
  O valor mostra `auto (256 MiB)` ou `64 MiB`.
- Modificar `src/app/mod.rs` e `src/app/view.rs`:
  - o campo `viewer_limit: u64`, calculado em `App::new` e em
    `apply_config` com
    `options.viewer_max_size.bytes(platform::memory().map(|m| m.total))`;
  - o `load_viewer` usa `LoadOptions::view(self.viewer_limit)`;
  - no remoto, `LoadOptions::view(self.viewer_limit.min(viewer::MAX_BYTES))`.
- Modificar `src/viewer.rs`: o doc de `MAX_BYTES` passa a dizer "padrão,
  piso do `auto` e teto do remoto (16 MiB)". O valor não muda.
- Documentação: `viewer.md` (o parágrafo "Files over 16 MiB show only the
  beginning" vira a explicação do limite) e `configuration.md`, nas 6
  línguas.

- [ ] **Passo 1: escrever os testes, que devem falhar.**

```rust
#[test] fn auto_is_a_32nd_of_ram_between_16_and_256_mib() {
    const MIB: u64 = 1 << 20; const GIB: u64 = 1 << 30;
    assert_eq!(ViewerMaxSize::Auto.bytes(Some(8 * GIB)), 256 * MIB);
    assert_eq!(ViewerMaxSize::Auto.bytes(Some(6 * GIB)), 192 * MIB);
    assert_eq!(ViewerMaxSize::Auto.bytes(Some(512 * MIB)), 16 * MIB);
    assert_eq!(ViewerMaxSize::Auto.bytes(Some(64 * GIB)), 256 * MIB);
    assert_eq!(ViewerMaxSize::Auto.bytes(None), 16 * MIB);
    assert_eq!(ViewerMaxSize::Mib1024.bytes(Some(512 * MIB)), 1024 * MIB);
}
```

  Mais:
  - `viewer_max_size = "64"` é lido, e `"65"` dá erro;
  - `L` cicla os cinco valores;
  - em `app/tests.rs`, um arquivo de 2 MiB com `viewer_limit = 1 << 20`
    abre `truncated()` com `limit() == 1 << 20`;
  - o teste de `parse_meminfo`, que já existe, continua passando.
- [ ] **Passo 2:** os testes falham.
- [ ] **Passo 3:** implementar.
- [ ] **Passo 4:** os testes passam. Rode também o `clippy` com o target
  Windows, porque `windows.rs` mudou.
- [ ] **Passo 5:** escrever a documentação.
- [ ] **Passo 6:** commit
  `Make the viewer size limit configurable (auto = RAM/32, 16-256 MiB)`.
  Atualize o status. Fim do PR 5.

---

## PR 6 — `cargo audit` no CI (branch `ci-audit`)

### Tarefa 13: auditoria de dependências

**Arquivos:**
- Modificar: `.github/workflows/ci.yml`.

- [ ] **Passo 1:** acrescentar um job `audit` em `ubuntu-latest`, com
  checkout e o toolchain que o job `test` já usa, e o passo
  `cargo install cargo-audit --locked && cargo audit`. Nada de action de
  terceiros para a auditoria, porque seria mais um fornecedor no CI.
- [ ] **Passo 2:** rodar localmente
  `cargo install cargo-audit --locked && cargo audit`. O esperado é sair
  com código 0. Se ele apontar alertas, informe o autor antes de silenciar
  qualquer um.
- [ ] **Passo 3:** commit `CI: audit dependencies with cargo audit`.
  Atualize o status em `viewers.md`: subprojeto 1 concluído.
