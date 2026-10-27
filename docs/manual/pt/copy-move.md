# Copiar e Mover

`C` copia e `M` move o arquivo sob o cursor; Ctrl+C e Ctrl+M os arquivos
marcados; Alt+C e Alt+M os arquivos marcados recriando seus caminhos. O prompt
pede o destino.

## O destino

Um diretório (relativo ao diretório sob o cursor da árvore, absoluto ou com
`~`) ou, para um único arquivo, um novo nome. Com a tela
[dividida](split.md), o diretório da outra janela é sugerido. **F2** abre uma
árvore para [escolher a pasta](picker.md); **↑** traz destinos anteriores.

Uma pasta de destino que ainda não exista é criada após a confirmação.

## Com ou sem caminhos

Ctrl+C / Ctrl+M colocam todos os arquivos marcados diretamente no destino.
Alt+C / Alt+M recriam o caminho de cada arquivo lá; onde esse caminho começa
depende do modo. Para `/home/me/proj/src/ops/mod.rs`, listado a partir de
`src/ops` (a raiz da lista: o diretório atual, a raiz do Branch ou a raiz da
árvore no Showall), copiado para `dest`:

| Modo | Resultado | |
|---|---|---|
| Full | `dest/home/me/proj/src/ops/mod.rs` | desde a raiz do disco, como no ZTreeWin (padrão) |
| Tree | `dest/src/ops/mod.rs` | desde a raiz da árvore do Xylem (aqui `proj`) |
| Current | `dest/ops/mod.rs` | desde a raiz da lista, incluindo seu nome |
| Relative | `dest/mod.rs` | apenas o que está abaixo da raiz da lista |

No prompt, **F6** alterna entre com e sem caminhos e **F7** percorre os modos;
ambos aparecem no próprio prompt. O modo inicial é `copy_paths` (página 4 da
[configuração](configuration.md)). Dentro de um arquivo compactado, os caminhos
começam no máximo na raiz do arquivo compactado.

## Quando o arquivo já existe

O Xylem pergunta **Yes** / **No** / **All** / **Skip all**, mostrando o tamanho
e a data do novo arquivo e do existente, e se o existente é mais novo ou mais
antigo, maior ou menor. `Esc` cancela toda a operação.

## Segurança

A cópia é feita em um arquivo temporário que só substitui o destino ao final:
cancelar ou encontrar um erro nunca deixa um arquivo escrito pela metade. As
permissões e a data de modificação são preservadas, e links simbólicos são
copiados como links. Mover entre discos diferentes copia e depois exclui.

Veja também: [marcações](tagging.md), [arquivos compactados](archives.md)
(Copiar extrai deles).
