# Configuração

## A tela de configuração (F12)

**F12** (ou Alt+F10, como no ZTreeWin; no GNOME, Alt+F10 geralmente maximiza a
janela e nunca chega ao Xylem) abre quatro páginas:

| Página | Opções |
|---|---|
| 1. Programas e formatos | editor, abrir editor dentro da moldura do Xylem, tema do Xylem no editor, idioma da ajuda, programa de abertura (`opener`), formato de data, formato numérico, comandos Git |
| 2. Exibição e ordenação | arquivos ocultos, ordem e direção iniciais de classificação, o que Enter faz na janela de arquivos, modo de exibição inicial, cores de sintaxe, quebra de linha, números de linha, arquivos comprimidos, a codificação de textos antigos, o limite de tamanho do que o visualizador lê, ícones e glifos (Nerd Fonts), cores do terminal integrado |
| 3. Confirmações e histórico | confirmar ao sair, excluir para a lixeira, confirmação do Prune, entradas com `@` não salvas, histórico próprio do terminal integrado |
| 4. Copiar, comparar e atualizar | ignorar 2 s, ignorar 1 h, atualização automática, onde começam os caminhos de Alt+C/Alt+M |

O limite de tamanho do visualizador (**L** na página 2) custa memória e tempo: com `1024` MiB, abrir um arquivo de texto comum desse tamanho ocupa cerca de 2,5 GiB de RAM e leva uns 5 s, e arquivos de linhas muito curtas custam mais por byte (veja [o visualizador](viewer.md)).

Ícones e glifos (**M** na página 2) exigem uma fonte com o patch do Nerd Fonts. Não é necessária uma fonte específica: qualquer fonte monoespaçada funciona (como Fira Code, JetBrains Mono, Hack, Meslo ou Cascadia Code), desde que seja a sua versão "Nerd Font". O Nerd Fonts injeta milhares de glifos padronizados na área privada do Unicode. Fontes comuns sem o patch (mesmo a Fira Code original) não contêm esses desenhos e exibirão retângulos vazios (`□`). Para utilizá-los:
- Baixe qualquer Nerd Font de sua preferência em nerdfonts.com ou instale um pacote Nerd Fonts pelo gerenciador de pacotes da sua distribuição.
- Selecione a fonte instalada nas preferências do seu emulador de terminal.

Se as colunas se desalinharem com os ícones ligados, o terminal provavelmente desenha os caracteres de "largura ambígua" como largos (ajuste comum com fontes chinesas, japonesas e coreanas), enquanto o Xylem conta cada ícone como uma célula. Ajuste essa opção para estreito ou desligue os ícones.

A letra da opção (ou Enter) altera o valor, PgUp/PgDn ou 1–4 trocam de página,
**F4** escolhe o tema em uma lista (com pré-visualização instantânea), **F3**
salva e aplica, **Esc** cancela (pergunta se houver alterações).

## O arquivo de configuração

Tudo fica armazenado em `~/.config/xylem/config.toml` (ou
`$XDG_CONFIG_HOME/xylem/`; `%APPDATA%\xylem\` no Windows), que também pode ser
editado manualmente: a tela preserva seus comentários. Para começar a partir de
um arquivo comentado com todos os padrões:

```bash
mkdir -p ~/.config/xylem && xy --default-config > ~/.config/xylem/config.toml
```

- `theme`: `classic` (azul XTree), `dark`, `light`, `mono` (as cores do próprio
  terminal), `norton`, `solarized`, `gruvbox`, `nord`, `slate` (no estilo do
  Windows Terminal), `graphite` (macOS no modo escuro), `daylight` (macOS no
  modo claro) ou `vivid` (cores fortes, para apresentações e capturas de tela).
  F4 mostra todos eles, com pré-visualização.
- `[options]`: as opções da tela de configuração.
- `[colors]`: cores individuais sobrepostas ao tema, como nomes (`"yellow"`),
  `"#rrggbb"` ou índices de 0 a 255.
- `[keys.any]`, `[keys.tree]`, `[keys.files]`: redefine teclas em ambas as
  janelas ou em apenas uma (que tem precedência). Por exemplo, `"f3" = "view"`,
  `"ctrl+r" = "relog"`; `"none"` desativa uma tecla. A lista de teclas da ajuda
  (**K**) mostra as teclas em vigor, com as suas alterações.
- `[editors]`: o editor para cada tipo de arquivo (`"*.png *.jpg" = "gimp %f"`,
  `binary = "hexedit"`); veja [Edit](execute.md).

Um erro no arquivo não impede o Xylem de iniciar: ele usa os padrões, mostra o
erro no rodapé e a tela de configuração não salvará até que o arquivo seja
corrigido, para não apagar o que você escreveu.

O Xylem chamava-se xt até a versão 0.1. Na primeira execução, ele move a
configuração, o histórico e os favoritos do xt para as suas próprias pastas.
