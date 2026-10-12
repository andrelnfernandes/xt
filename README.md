# xt

Gerenciador de arquivos em modo texto para Linux, inspirado no XTree e no ZTreeWin.

## Uso

```bash
cargo run --release -- [diretório]
```

Sem argumento, abre o diretório atual.

## Teclas

### Navegação

| Tecla | Árvore | Janela de arquivos |
|---|---|---|
| ↑ ↓ PgUp PgDn Home End | move o cursor | move o cursor |
| → / `+` | expande o diretório | |
| ← / `-` | recolhe (ou vai para o pai) | volta para a árvore |
| `*` | loga e expande o ramo inteiro | |
| `B` | Branch: arquivos do ramo inteiro (loga antes) | idem |
| `S` | Showall: arquivos de tudo o que foi logado | idem |
| `F` | Filespec: filtro de arquivos | idem |
| `F2` | relê o diretório do disco | idem |
| `V` | | visualiza o arquivo (texto/hex) |
| Enter / Tab | abre a janela de arquivos | volta para a árvore |
| Esc | | volta para a árvore (no diretório do arquivo) |
| `Q` / F10 | sair (pede confirmação) | sair |

### Tags e operações

| Tecla | Árvore | Janela de arquivos |
|---|---|---|
| `T` / `U` | marca / desmarca os arquivos do diretório | marca / desmarca o arquivo e desce |
| Ctrl+T / Ctrl+U | marca / desmarca tudo o que foi logado (filespec) | marca / desmarca a lista inteira |
| `I` | inverte as marcas do diretório | inverte as marcas da lista |
| `C` | | copia o arquivo |
| `M` | cria um subdiretório | move o arquivo |
| `D` | apaga o diretório (só se vazio) | apaga o arquivo |
| `R` | renomeia o diretório | renomeia o arquivo |
| `A` | | permissões (octal, ex.: `644`) |
| Ctrl ou Alt + `C` `M` `D` `A` | idem, nos arquivos marcados | idem, nos arquivos marcados |

As operações em grupo agem sobre os arquivos marcados **da lista atual**
(o diretório, ou o Branch/Showall aberto). Toda operação em grupo também
aceita Alt+letra.

**Ctrl+M:** na maioria dos terminais, Ctrl+M chega igual a Enter. O xt liga
automaticamente o protocolo de teclado do kitty quando o terminal o suporta
(kitty, WezTerm, foot, Ghostty, Alacritty recente, iTerm2), e aí Ctrl+M move
os arquivos marcados como no ZTree. Nos terminais baseados em VTE (GNOME
Terminal, Ptyxis, Terminator, Tilix) use **Alt+M**.

Durante um log demorado, o progresso aparece no rodapé e `Esc` cancela.

**Filespec:** padrões glob separados por espaço, `;` ou `,`, sem diferenciar
maiúsculas. Um `-` na frente exclui: `*.rs *.toml -*.bak`.

**Copy/Move:** o destino pode ser um diretório (relativo ao atual, absoluto
ou com `~`) ou, para um único arquivo, um novo nome. Se o destino já existe,
o xt pergunta: Yes / No / All / Skip all. A cópia é feita num temporário que
só substitui o destino no fim: cancelar (`Esc`) ou um erro nunca deixa um
arquivo pela metade. Permissões e data de modificação são preservadas, e links
simbólicos são copiados como links. Mover entre discos copia e depois apaga.

**Delete** apaga de verdade (não vai para a lixeira) e sempre pede confirmação.

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
- `src/ops/` — copiar, mover e apagar, em passos (progresso, conflitos, cancelamento)

## Roteiro

- [x] Fase 0 — tela clássica, árvore navegável, janela de arquivos
- [x] Fase 1 — Filespec, Showall/Branch, viewer texto/hex, log com progresso
- [x] Fase 2 — tagging, copy/move/delete/rename, makedir, permissões, re-log
- [ ] Fase 3 — janela dividida, compare, arquivos compactados, inotify, macros
- [ ] Fase 4 — temas, keymap configurável, empacotamento
