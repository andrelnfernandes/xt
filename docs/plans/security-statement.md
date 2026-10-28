# Plano: declaração de segurança no README e no manual (PR 8)

Origem: conversa de 2026-10-27. O autor gostou da franqueza do texto
"Defense Against The Dark Arts" do SQLite ("SQLite should never crash... Nevertheless,
bugs happen...") e pediu algo parecido para o Xylem, no README e no início do manual.
O texto em inglês abaixo foi **aprovado pelo autor**; falta traduzir e montar o PR.

## Texto aprovado (inglês, usar como está)

```markdown
## Security: hostile files

Xylem should never crash, hang, run code, write outside the destination or
exhaust memory because of what a file contains or how it is named, even when
the file was crafted to do so. Files you download, archives you open, names a
remote server sends and repositories you clone are all treated as hostile. Any
such malfunction is a serious bug, and it will be fixed promptly once reported.

Nevertheless, bugs happen. Here is what Xylem does to keep the risk small:

- Everything Xylem reads on its own (text, hex, encodings, compressed files,
  archives) goes through memory-safe Rust parsers. No external program runs on
  a file unless you ask for it with E, O or X.
- Everything has a limit: how much is read, how much is decompressed, how long
  it takes and how much memory a decompressor may use. Only one level of
  compression is undone, and nothing is written to disk to show a file.
- Control characters and bidirectional controls (Trojan Source) never reach
  the terminal. Names with hidden bidi characters are marked, and O asks
  before opening them.
- When extracting archives or copying from a remote machine, paths with `..` or
  absolute paths are refused, symbolic links in archives are not extracted,
  setuid bits are dropped and names sent by an SFTP server are checked.
- The automatic Git status ignores a repository's `core.fsmonitor` and skips
  repositories that define their own filters.
- The dependencies are checked against the RustSec database in CI.

You can make the attack surface smaller still:

- Turn off decompression in the viewer (`viewer_decompress = false`, F12)
  if you browse untrusted folders with Autoview, or keep Autoview (F7) closed
  there.
- Leave Git integration off (`git = false`, the default) for repositories you
  did not create.
- Do not run Xylem as root.
- Open untrusted files with O only when you trust the program that will open
  them: Xylem hands the file over to it.

Found a problem? Please report it privately [CHANNEL], not in a public issue.
```

## Decisões tomadas

- **`[CHANNEL]`:** canal de denúncia privada definido como o Private vulnerability
  reporting do GitHub (`https://github.com/andrelnfernandes/xylem/security/advisories/new`).
- **Manual:** parágrafo curto no início de `index.md` nas 6 línguas apontando para o README.
- **Status da implementação:** arquivos modificados (`README.md`, `README.pt-BR.md`,
  `docs/manual/**/index.md`) e testes validados (`cargo test help`,
  `cargo test default_config_text_means_defaults`). Pronto para commit e PR mediante
  autorização do autor.

## O que fazer (roteiro para o agente que continuar)

Branch: `security-statement`, criado a partir de `ci-audit`. Será o PR 8, empilhado:
base `ci-audit`. O texto cita proteções que só existem na pilha (descompressão com
limites, guarda bidi), então não pode ir direto na `main` antes dos PRs 1–7.

1. **README.md:** inserir a seção acima logo depois de `## Why Xylem?` e antes de
   `## Disclaimer`.
2. **README.pt-BR.md:** a mesma seção traduzida para o português, na posição
   equivalente. Mantenha em inglês os nomes de opção, teclas e valores
   (`viewer_decompress = false`, `git = false`, F12, F7, E, O, X).
3. **docs/manual/index.md:** um parágrafo curto logo depois do parágrafo de abertura
   (antes do parágrafo do F1), com a promessa e o "Nevertheless, bugs happen", e um
   link para o README ou a lista completa. O manual é a ajuda F1: siga o subconjunto
   de Markdown de `docs/manual/TRANSLATING.md` (títulos `##`, parágrafos, listas
   com `- `). Alternativa aceitável: incluir a lista inteira. Escolha uma e use igual
   nas 6 línguas.
4. **Traduções do manual:** `docs/manual/{pt,es,de,fr,zh}/index.md`, na mesma posição,
   escritas de forma natural em cada língua, conforme `TRANSLATING.md`. Termos já
   usados no manual francês: "la visionneuse".
5. **Verificar:** o texto continua verdadeiro diante do código. Confira, por exemplo,
   que o `git = false` é mesmo o padrão em `src/config.rs` e que `viewer_decompress`
   existe (F12, página 2, J). Rode `cargo test` uma vez: o teste
   `default_config_text_means_defaults` e os testes da ajuda leem os arquivos do
   manual (`cargo test help`).
6. **Commit** em `security-statement`, terminando com o trailer de coautoria do agente.
   **Push** (`git push -u origin security-statement`) só com autorização do autor.
7. **PR 8:** base `ci-audit`, título `Docs: security statement for hostile files`.
   Descrição curta: inspirado no "Defense Against The Dark Arts" do SQLite; README nos
   2 idiomas; manual nas 6 línguas; "stacked on `ci-audit` (PR 7)".
8. Ao fim, atualizar o status em [viewers.md](viewers.md) ("Como retomar").

## Contexto útil

- A pilha de PRs 1–7 já está no GitHub (`viewer-plans` → `bidi-guard` → `decompress` →
  `viewer-decompress` → `viewer-charset` → `viewer-max-size` → `ci-audit`). Os links
  pré-preenchidos dos PRs estão em `~/.cache/xt-dev/pr/pr-links.md`, gerados por
  `make_links.py` na mesma pasta.
- O repositório mudou de nome para `andrelnfernandes/xylem`. O remoto local ainda
  aponta para `xt` e funciona por redirecionamento.
- Preferências do autor: segurança primeiro; arquivos temporários só em
  `~/.cache/xt-dev/`, nunca em `/tmp`; commit e push só quando ele pedir; registrar
  planos em `docs/plans/` a cada passo.
