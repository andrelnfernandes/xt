# El visor

`V` abre el archivo en el visor (Autoview, F7, lo muestra junto a la lista). Los
archivos binarios se abren en hexadecimal. Los archivos de más de 16 MiB
muestran solo el principio.

| Tecla | Qué hace |
|---|---|
| ↑ ↓ PgUp PgDn Home End | desplazar |
| ← → | desplazar lateralmente |
| `H` | texto o hexadecimal |
| `C` | activar o desactivar colores de sintaxis |
| `W` | activar o desactivar el ajuste de línea |
| `#` o `L` | activar o desactivar números de línea (en modo texto) |
| `g` o `:` | saltar a una línea (o desplazamiento de byte en hexadecimal) |
| `E` | editar el archivo; al volver a Xylem, el visor muestra el contenido nuevo en el mismo lugar (archivos binarios solo con un editor en [`[editors]`](execute.md)) |
| `D` | con Git activado: qué cambió desde el último commit, y volver ([Git](git.md)) |
| `T` / `U` | marcar / desmarcar el archivo |
| `y` o Ctrl+Insert | copiar la ruta del archivo al portapapeles |
| Esc | cerrar |

## Búsqueda

Como en ZTreeWin: **F** busca desde el principio, **B** hacia atrás desde el
final, **/** (o `S`) desde la página actual y **\\** hacia atrás desde ella;
**Space** continúa en la misma dirección, **+** y **-** eligen la dirección. La
coincidencia aparece resaltada. En la entrada de búsqueda, **F2** alterna la
distinción de mayúsculas y minúsculas (al ignorarlas, los acentos también se
equiparan: `AÇÃO` encuentra `ação`) y **F4** alterna entre texto y hexadecimal
(pares de bytes como `4d 5a`). Un `*` en mitad del término coincide con
cualquier cosa en la misma línea (`error*disk`).

## Colores de sintaxis

El código fuente y los scripts se colorean en unos 150 lenguajes (Rust, Python,
shell, C, JavaScript, SQL, YAML, Markdown...), reconocidos por extensión, por
nombre (`Makefile`) o por la primera línea (`#!/bin/bash`). El valor
predeterminado es `syntax_highlight` (página 2 de la
[configuración](configuration.md)). Los colores se mantienen legibles sobre el
fondo del tema actual, y solo se colorean las primeras 20.000 líneas, para que
los archivos enormes nunca ralenticen el visor.

## Caracteres de control de dirección

Algunos caracteres invisibles cambian la dirección del texto y pueden hacer que un código parezca distinto de lo que realmente dice (Trojan Source, CVE-2021-42574). El visor nunca los oculta: cada uno aparece como un `·` rojo, y el encabezado indica cuántos hay.

## Archivos comprimidos

Los archivos que terminan en `.gz`, `.xz`, `.zst` o `.bz2` se abren descomprimidos, como texto. Xylem mira los primeros bytes del archivo, no el nombre, y abre solo un nivel: un `.gz` dentro de otro `.gz` sigue comprimido. El encabezado muestra `gzip → N bytes` (el formato y el tamaño tras descomprimir).

La descompresión tiene límites. El visor lee hasta 16 MiB, tarda como máximo 3 s y usa como máximo 64 MiB de memoria para el diccionario de xz o la ventana de zstd. En Autoview los límites son 1 MiB, 100 ms y 16 MiB. Cuando se alcanza un límite, el encabezado dice `(showing first N bytes decompressed)` y una nota explica el motivo:

- `compressed data truncated or corrupt` (datos comprimidos truncados o dañados)
- `decompression stopped after 3 s` (100 ms en Autoview)
- `needs more than 64 MiB of memory to decompress` (16 MiB en Autoview, con ` (V to view)`)

Autoview muestra la nota en su título. En modo hexadecimal (`H`) se ven los bytes descomprimidos. Los colores siguen el nombre sin la extensión de compresión: `app.rs.gz` se colorea como Rust.

Para un `.tar.gz` (o `.tar.xz`...), la nota dice `TAR archive: Alt+F5 opens it as a folder` (Alt+F5 lo abre como una carpeta).

Para ver los bytes comprimidos, desactive `viewer_decompress` (**J** en la página 2 de la [configuración](configuration.md)).

Véase también: [buscar en los archivos marcados](search.md).
