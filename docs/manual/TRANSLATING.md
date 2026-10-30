# Translating the Xylem Manual

Xylem's built-in help (**F1** / **Alt+H**) and manual page (`xy --man`) are generated directly from the Markdown files in `docs/manual/`.

Contributors can translate single topics or add a new language **without writing any Rust code**.

## Reviews Wanted

The Spanish, French, German and Chinese versions have not been reviewed by native speakers. If you read one of these languages, a review is as welcome as a new translation: fix what is wrong or sounds odd in a Pull Request, or describe it in an [issue](https://github.com/andrelnfernandes/xylem/issues). You do not need to review a whole language at once; a single topic helps.

## Directory Structure

- `docs/manual/*.md` — Canonical English (`en`) topics (29 files).
- `docs/manual/<lang>/*.md` — Translated topics for ISO 639-1 language code `<lang>` (for example: `pt`, `es`, `fr`, `de`, `it`, `ru`, `zh`, `ja`).

```text
docs/manual/
├── index.md              # English (canonical fallback)
├── files.md
├── ...
├── pt/                   # Portuguese
│   ├── index.md
│   └── files.md
├── es/                   # Spanish
│   ├── index.md
│   └── ...
```

## Rules for Translators

1. **Keep the exact same file names**:
   A topic's file name (`index.md`, `tree.md`, `files.md`, `copy-move.md`, etc.) is its identifier. Do not translate file names.
2. **Keep link targets unchanged**:
   Translate the visible link label inside `[...]`, but keep the `(target.md)` part in English:
   - Good: `[A janela de arquivos](files.md)`
   - Bad: `[A janela de arquivos](arquivos.md)`
3. **Start every file with a `# ` title**:
   The first line must be `# Translated Title`. Xylem uses this heading as the title in the Help header and as the section header in `xy --man`.
4. **Supported Markdown subset**:
   Xylem's help layout engine supports:
   - `# Title`, `## Section`, `### Subsection`
   - Paragraphs and `- ` bullet lists (including nested lists indented by 2 spaces)
   - Pipe tables (`| Col 1 | Col 2 |`)
   - Fenced code blocks (` ``` `)
   - Inline `` `code` ``, `**bold**`, and `[label](topic.md)` links
5. **Incremental translations are welcome**:
   You do not need to translate all 29 files at once. If a topic is missing in `docs/manual/<lang>/`, Xylem automatically falls back to the English version for that topic.

## Live Preview Without Compiling Rust

You can preview your translation live inside an existing `xy` binary before opening a Pull Request:

1. Place your `.md` files in `~/.config/xylem/manual/<lang>/` (or `$XDG_CONFIG_HOME/xylem/manual/<lang>/`; `%APPDATA%\xylem\manual\<lang>\` on Windows).
2. Open `xy`, press **F1** (or **Alt+H**), and press **L** to cycle to `<lang>`.
3. Edit any `.md` file and reopen **F1** to see the updated layout immediately.

## Validating with Cargo (Optional)

When building from source, `build.rs` automatically discovers all `docs/manual/<lang>/*.md` files and `cargo test` validates every translation:

```bash
cargo test --bins help::tests
```

This checks that:
- Every file starts with `# Title`.
- Every `[...](topic.md)` link points to a valid topic.
- `index.md` links to all topics.
- Every topic lays out cleanly at narrow and wide terminal widths (20 to 200 columns).
