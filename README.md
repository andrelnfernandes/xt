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
| `*` | expande o ramo inteiro | |
| Enter / Tab | abre a janela de arquivos | volta para a árvore |
| Esc | | volta para a árvore |
| `Q` / F10 / Ctrl+C | sair (pede confirmação) | sair |

Na árvore, `+` no conector (`├─+`) indica um diretório recolhido que tem subdiretórios.

## Estrutura

- `src/model/` — árvore de diretórios em memória (arena), sem dependência da UI
- `src/app.rs` — estado e execução de comandos
- `src/input.rs` — mapeamento tecla → comando
- `src/ui.rs` — desenho da tela (ratatui)

## Roteiro

- [x] Fase 0 — tela clássica, árvore navegável, janela de arquivos
- [ ] Fase 1 — Filespec, Showall/Branch, viewer texto/hex, log completo com progresso
- [ ] Fase 2 — tagging, copy/move/delete/rename, permissões
- [ ] Fase 3 — janela dividida, compare, arquivos compactados, inotify, macros
- [ ] Fase 4 — temas, keymap configurável, empacotamento
