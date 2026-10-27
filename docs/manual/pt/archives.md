# Arquivos compactados

## Abrir um arquivo compactado (Alt+F5)

Alt+F5 na janela de arquivos abre, como uma árvore e somente leitura:

- `.zip`, além de `.jar`, `.war` e `.apk`;
- `.7z` (arquivos 7z criptografados não são suportados);
- `.tar`, puro ou comprimido com gzip (`.tar.gz`, `.tgz`), xz (`.tar.xz`,
  `.txz`), bzip2 (`.tar.bz2`, `.tbz2`) ou zstd (`.tar.zst`).

Navegação, [Filespec](filespec.md), Branch, Showall, marcações,
[Visualizador](viewer.md), [Ctrl+S](search.md) e [Comparar](compare.md)
funcionam dentro dele, e [Copiar](copy-move.md) (Ctrl+C e Alt+C com caminhos
também) extrai os arquivos. `Esc` na árvore fecha o arquivo compactado e
retorna para onde você estava.

Por segurança, entradas com `..` ou caminho absoluto são ignoradas, e links
simbólicos dentro de arquivos compactados nunca são extraídos.

## Criar um arquivo compactado (F5 / Ctrl+F5)

Cria um `.zip`, `.tar`, `.tar.gz` ou `.tgz` (conforme o nome que você digitar)
com o arquivo atual ou os arquivos marcados; F2 [escolhe a pasta](picker.md).
Os caminhos internos são relativos à raiz da lista, como no Alt+C: em um
Branch, as subpastas são mantidas. Mostra o progresso e `Esc` cancela; o
arquivo é gravado em um arquivo temporário e só recebe seu nome final ao
terminar. Links simbólicos são armazenados como links. Um arquivo existente não
é sobrescrito.
