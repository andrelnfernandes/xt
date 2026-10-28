# Spec: suporte a fontes com ícones (Nerd Fonts)

Origem: conversa de 2026-10-27. O autor perguntou sobre o uso de fontes com glifos
e ícones (como Fira Code / Nerd Fonts) para enriquecer graficamente a interface do
Xylem sem perder a fidelidade clássica do XTree / ZTreeWin.

## Objetivo

Permitir que usuários que utilizam emuladores de terminal com **Nerd Fonts** possam
habilitar ícones visuais para arquivos, extensões, diretórios e status do Git, de
forma totalmente opcional e sem quebrar o alinhamento de colunas da interface.

## Princípios e Regras

1. **Opt-in estrito (desligado por padrão):** O padrão é `icons = "none"`. O visual
   clássico retrô (ASCII puro e box-drawing) permanece intocado para máxima
   compatibilidade com consoles virtuais (`/dev/tty1`), conexões SSH remotas e
   terminais sem fontes customizadas.
2. **Geometria e alinhamento sagrados:** O Xylem baseia-se em colunas rigorosamente
   alinhadas (especialmente nos modos multi-coluna `Alt+F` e estatísticas). Quando
   habilitado, o slot do ícone ocupa exatamente 2 células (1 glifo + 1 espaço separador).
3. **Desempenho O(1):** Mapeamento estático e performático de extensões por tabela
   de correspondência rápida, com zero alocação de heap durante o loop de desenho
   da tela.
4. **Respeito aos temas:** Os ícones herdam as cores e estilos definidos pelo tema
   ativo (`theme().text`, `theme().dir`, `theme().dim`, etc.), evitando um visual
   excessivamente colorido que destoe da estética textual do Xylem.

## Configuração

### Arquivo `config.toml`
```toml
# Estilo de ícones na interface:
# "none"       - Clássico, sem ícones (padrão)
# "nerd-fonts" - Ícones modernos usando faixas PUA do Nerd Fonts (v3.x)
icons = "none"
```

### Tela de configurações (F12 / Alt+F10)
- **Localização:** Página 2 (*Display and sorting*).
- **Entrada:** Letra `M` (logo após `L` - `viewer_max_size`).
  ```text
  M  Icons and glyphs (Nerd Fonts)                 none / nerd-fonts
  ```
- **Comportamento:** Pressionar `M` alterna entre `none` e `nerd-fonts`, com efeito
  imediato na interface.

## Mapeamento de Glifos (Nerd Fonts v3.x PUA)

### 1. Diretórios e Estrutura
- Diretório fechado: ` ` (`\u{f07c}`)
- Diretório aberto / selecionado: ` ` (`\u{f07b}`)
- Ponto de montagem / Raiz: `󰋊 ` (`\u{f02ca}`)
- Repositório Git: ` ` (`\u{e5fb}`)
- Pacote aberto como pasta (ZIP/TAR/7z): ` ` (`\u{f410}`)
- Pasta remota (SFTP / Nuvem): `󰒋 ` (`\u{f048b}`)

### 2. Extensões e Tipos de Arquivo
- **Código-fonte:**
  - Rust (`.rs`): ` ` (`\u{e7a8}`)
  - Python (`.py`): ` ` (`\u{e73c}`)
  - C / C++ (`.c`, `.h`, `.cpp`, `.hpp`, `.cc`): ` ` (`\u{e61d}`)
  - Go (`.go`): ` ` (`\u{e627}`)
  - JavaScript / TypeScript (`.js`, `.jsx`, `.ts`, `.tsx`, `.mjs`): ` ` (`\u{e60c}`)
  - Shell script (`.sh`, `.bash`, `.zsh`, `.fish`): ` ` (`\u{e795}`)
  - HTML / CSS (`.html`, `.htm`, `.css`, `.scss`): ` ` (`\u{f13b}`) / ` ` (`\u{e749}`)
  - Configuração / Dados (`.toml`, `.yaml`, `.yml`, `.json`, `.ini`, `.xml`): ` ` (`\u{e615}`)
- **Documentos e Texto:**
  - Markdown (`.md`, `.markdown`): ` ` (`\u{f48a}`)
  - PDF (`.pdf`): ` ` (`\u{f1c1}`)
  - Texto simples (`.txt`, `.log`): ` ` (`\u{f15c}`)
- **Pacotes e Arquivos Comprimidos:**
  - `.zip`, `.tar`, `.gz`, `.xz`, `.zst`, `.bz2`, `.7z`, `.rar`: ` ` (`\u{f410}`)
- **Mídia:**
  - Imagens (`.png`, `.jpg`, `.jpeg`, `.gif`, `.svg`, `.webp`, `.bmp`, `.ico`): ` ` (`\u{f03e}`)
  - Vídeo (`.mp4`, `.mkv`, `.avi`, `.mov`, `.webm`): `󰿎 ` (`\u{f0fce}`)
  - Áudio (`.mp3`, `.flac`, `.wav`, `.ogg`): `󰎆 ` (`\u{f0386}`)
- **Especiais:**
  - Executáveis: `󰘳 ` (`\u{f0633}`)
  - Links simbólicos: ` ` (`\u{f0c1}`)

### 3. Git e Marcadores
- Marcador de modificado: ` ` (`\u{f459}`)
- Marcador de novo/não rastreado: ` ` (`\u{f457}`)
- Marcador de conflito: ` ` (`\u{f4db}`)
- Ramo no cabeçalho: ` ` (`\u{e725}`)

## Roteiro de Implementação

1. **Módulo `src/icons.rs`:** [Concluído]
   - Enum `IconStyle`: `None`, `NerdFonts`.
   - Função `fn file_icon(name: &OsStr, kind: FileKind, is_exec: bool, style: IconStyle) -> Option<&'static str>`.
   - Função `fn dir_icon(open: bool, is_repo: bool, is_archive: bool, is_remote: bool, style: IconStyle) -> Option<&'static str>`.
   - Função `fn git_icon(status: [u8; 2], style: IconStyle) -> Option<&'static str>`.
2. **Configuração em `src/config.rs` e `src/settings.rs`:** [Concluído]
   - Campo `icons: IconStyle` adicionado à `Config`/`Options` (padrão `none`).
   - Opção `M` adicionada na página 2 de `PAGES` em `settings.rs`.
3. **Renderização em `src/ui.rs`:** [Concluído]
   - Suporte a ícones na lista de arquivos em modo detalhado (`file_line`).
   - Suporte a ícones em modo multi-coluna `Alt+F` com ajuste exato de largura (`draw_file_columns`).
   - Ícones na árvore de diretórios (`draw_tree`) e picker de pastas.
   - Ícones de status do Git (`git_marker_text`).
4. **Documentação e Testes:** [Concluído]
   - Atualizados manuais nas 6 línguas (`docs/manual/**/configuration.md`).
   - Testes unitários em `src/icons.rs`, `src/config.rs` e `src/ui.rs` (`icons_switch_in_file_list`). Toda a suíte `cargo test` verde (425 testes).
