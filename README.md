# xt

Gerenciador de arquivos em modo texto para Linux, inspirado no XTree e no ZTreeWin.

## Uso

```bash
cargo run --release -- [diretório]
```

Sem argumento, abre o diretório atual.

## Teclas

| Tecla | Árvore | Janela de arquivos |
|---|---|---|
| ↑ ↓ PgUp PgDn Home End | move o cursor | move o cursor |
| → / `+` | expande o diretório | |
| ← / `-` | recolhe (ou vai para o pai) | volta para a árvore |
| `*` | loga e expande o ramo inteiro | |
| `B` | Branch: arquivos do ramo inteiro (loga antes) | |
| `S` | Showall: arquivos de tudo o que foi logado | |
| `F` | Filespec: filtro de arquivos | Filespec |
| `V` | | visualiza o arquivo (texto/hex) |
| Enter / Tab | abre a janela de arquivos | volta para a árvore |
| Esc | | volta para a árvore (no diretório do arquivo) |
| `Q` / F10 / Ctrl+C | sair (pede confirmação) | sair |

Durante um log demorado, o progresso aparece no rodapé e `Esc` cancela.

**Filespec:** padrões glob separados por espaço, `;` ou `,`, sem diferenciar
maiúsculas. Um `-` na frente exclui: `*.rs *.toml -*.bak`.

**Visualizador:** setas/PgUp/PgDn/Home/End rolam, ←→ deslocam na horizontal,
`H` alterna texto/hex, `Esc` fecha. Arquivos binários abrem em hex.
Arquivos acima de 16 MiB mostram só o começo.

Na árvore, `+` no conector (`├─+`) indica um diretório recolhido que tem subdiretórios.

## Estrutura

- `src/model/` — árvore de diretórios em memória (arena), sem dependência da UI
- `src/app.rs` — estado e execução de comandos
- `src/input.rs` — mapeamento tecla → comando
- `src/ui.rs` — desenho da tela (ratatui)
- `src/viewer.rs` — visualizador texto/hex
- `src/model/filespec.rs` — filtro de arquivos

## Roteiro

- [x] Fase 0 — tela clássica, árvore navegável, janela de arquivos
- [x] Fase 1 — Filespec, Showall/Branch, viewer texto/hex, log com progresso
- [ ] Fase 2 — tagging, copy/move/delete/rename, permissões
- [ ] Fase 3 — janela dividida, compare, arquivos compactados, inotify, macros
- [ ] Fase 4 — temas, keymap configurável, empacotamento
