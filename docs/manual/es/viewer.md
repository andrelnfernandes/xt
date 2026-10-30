# El visor

`V` abre el archivo en el visor (Autoview, F7, lo muestra junto a la lista). Los
archivos binarios se abren en hexadecimal. Los archivos que superan el límite de tamaño
muestran solo el principio.
Las hojas de cálculo, CSV, DBF, SQLite y Parquet se abren como tabla (véase Modo tabla).

| Tecla | Qué hace |
|---|---|
| ↑ ↓ PgUp PgDn Home End | desplazar |
| ← → | desplazar lateralmente |
| `H` | texto o hexadecimal |
| `O` | siguiente codificación (UTF-8, CP850, 1252, CP437, UTF-16) |
| `C` | activar o desactivar colores de sintaxis |
| `W` | activar o desactivar el ajuste de línea |
| `#` o `L` | activar o desactivar números de línea (en modo texto) |
| `g` o `:` | saltar a una línea (un desplazamiento de byte en hexadecimal, una fila en modo tabla) |
| `&` | muestra solo las líneas que coinciden, texto simple o regex ([filtrar líneas](filter.md)); en modo tabla, el filtro SQL |
| Ctrl+B | una línea de salida por registro o línea mostrada ([Batch en el visor](execute.md)) |
| `E` | editar el archivo; al volver a Xylem, el visor muestra el contenido nuevo en el mismo lugar (archivos binarios solo con un editor en [`[editors]`](execute.md)) |
| `D` | con Git activado: qué cambió desde el último commit, y volver ([Git](git.md)) |
| `T` | tabla o texto, en un archivo que se abrió como tabla |
| `U` | en la vista de los archivos marcados (Ctrl+V): desmarcar el archivo |
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
fondo del tema actual, y solo se colorean las primeras 20.000 líneas, y ninguna
línea de más de 10.000 caracteres, para que los archivos enormes nunca
ralenticen el visor.

## Caracteres de control de dirección

Algunos caracteres invisibles cambian la dirección del texto y pueden hacer que un código parezca distinto de lo que realmente dice (Trojan Source, CVE-2021-42574). El visor nunca los oculta: cada uno aparece como un `·` rojo, y el encabezado indica cuántos hay.

## Codificación del texto

Xylem descubre la codificación solo, en este orden: primero descomprime el archivo, luego busca un BOM (UTF-8, UTF-16 LE o BE), luego UTF-16 sin BOM, luego UTF-8. Un archivo que no es nada de eso es texto antiguo: Xylem elige CP850 (DOS, Europa occidental) o Windows-1252 contando las letras con tilde. Los archivos `.reg` de Windows son UTF-16 y se abren como texto normal.

**O** muestra el archivo en la siguiente codificación: UTF-8, CP850, Windows-1252, CP437, UTF-16 LE, UTF-16 BE y de nuevo UTF-8. El pie dice cuál, y la cabecera muestra su nombre cuando no es UTF-8. Si elige UTF-16 en un archivo que se abrió en hexadecimal, pasa a texto. Su elección se mantiene después de editar el archivo y volver.

Para usar siempre una sola codificación en los textos antiguos, en lugar de la suposición, ponga `legacy_charset` en `cp850`, `cp1252` o `cp437` (**K** en la página 2 de la [configuración](configuration.md)). El valor por defecto es `auto`.

## Límite de tamaño

El visor lee solo el principio de un archivo grande, y el encabezado dice `(showing first N MiB)`. Cuánto lee lo decide la opción `viewer_max_size` (**L** en la página 2 de la [configuración](configuration.md)): `auto`, `16`, `64`, `256` o `1024` (MiB). Con `auto`, el valor por defecto, el límite es 1/32 de la RAM de la máquina, nunca menos de 16 MiB ni más de 256 MiB (son 16 MiB si no se puede leer la RAM). La pantalla de configuración muestra el valor en uso, por ejemplo `auto (256 MiB)`.

Dos casos no siguen esta regla. Autoview siempre lee 1 MiB. Un archivo en una máquina remota (SFTP) se lee como máximo hasta 16 MiB, porque Esc no puede cancelar esa lectura.

Un límite mayor cuesta memoria y tiempo: con `1024`, un archivo de texto normal de ese tamaño ocupa unos 2,5 GiB de RAM y tarda unos 5 s en abrirse. Las líneas muy cortas cuestan más que sus bytes, porque cada línea tiene su propia gestión, y lo mismo pasa con los tabuladores y los caracteres de control de dirección ocultos: el visor guarda líneas hasta cerca del doble del límite de tamaño en memoria y, pasado eso, el encabezado dice `(showing first N lines)`. Un log de líneas normales (unos 100 bytes) nunca se corta por esto.

## Archivos comprimidos

Los archivos que terminan en `.gz`, `.xz`, `.zst` o `.bz2` se abren descomprimidos, como texto. Xylem mira los primeros bytes del archivo, no el nombre, y abre solo un nivel: un `.gz` dentro de otro `.gz` sigue comprimido. El encabezado muestra `gzip → N bytes` (el formato y el tamaño tras descomprimir).

La descompresión tiene límites. El visor lee hasta el límite de tamaño, tarda como máximo 3 s y usa como máximo 64 MiB de memoria para el diccionario de xz o la ventana de zstd. En Autoview los límites son 1 MiB, 100 ms y 16 MiB. Cuando se alcanza el límite de salida o el de tiempo, el encabezado dice `(showing first N bytes decompressed)`. Una nota explica lo que pasó:

- `decompression stopped after 3 s` (100 ms en Autoview)
- `compressed data truncated or corrupt` (datos comprimidos truncados o dañados)
- `needs more than 64 MiB of memory to decompress` (16 MiB en Autoview, con ` (V to view)` cuando los 64 MiB de V pueden bastar)

La nota desaparece con la primera tecla, pero el encabezado conserva el problema: `(truncated or corrupt)`, `(incomplete: …)` cuando el límite de memoria detuvo la descompresión a medias, o `(not decompressed: …)` cuando no se pudo descomprimir nada (entonces los bytes sin procesar se muestran en hexadecimal).

Autoview muestra la nota en su título. En modo hexadecimal (`H`) se ven los bytes descomprimidos. Los colores siguen el nombre sin la extensión de compresión: `app.rs.gz` se colorea como Rust.

Para un `.tar.gz` (o `.tar.xz`...), la nota dice `TAR archive: Alt+F5 opens it as a folder` (Alt+F5 lo abre como una carpeta).

Para ver los bytes comprimidos, desactive `viewer_decompress` (**J** en la página 2 de la [configuración](configuration.md)).

## Modo tabla

Algunos archivos se abren como una cuadrícula en vez de texto. Xylem decide por la extensión, y solo para archivos sin comprimir en este disco: un archivo de una máquina remota o dentro de un paquete se muestra como contenido normal, con la nota `Tables are shown only for files on this disk`.

| Formato | Extensiones | Nombres de las columnas |
|---|---|---|
| CSV, TSV | `.csv` `.tsv` `.tab` | la primera fila, cuando al menos la mitad no son números; si no, A, B, C... |
| JSON Lines | `.jsonl` `.ndjson` | las claves de los objetos |
| Hojas de cálculo | `.xlsx` `.xls` `.xlsb` `.ods` | A, B, C... (la primera fila es una fila más) |
| dBase, Clipper, FoxPro | `.dbf`, con su archivo memo `.dbt` | los nombres de los campos |
| SQLite | `.db` `.sqlite` `.sqlite3` | las columnas de la tabla |
| Parquet | `.parquet` | las columnas del archivo |

El separador del CSV (coma, punto y coma, tabulación o barra vertical) se detecta por el principio del archivo.

| Tecla | Qué hace |
|---|---|
| ↑ ↓ | fila anterior o siguiente |
| ← → | columna anterior o siguiente |
| PgUp PgDn | una página de filas hacia arriba o hacia abajo |
| Home End | primera o última columna |
| `[` `]` | hoja anterior o siguiente de una hoja de cálculo, o tabla de una base SQLite, dando la vuelta; esto borra el filtro |
| `g` o `:` | saltar a una fila, contando solo las que el filtro dejó |
| `\` o `&` | el filtro SQL |
| `T` | ir al texto del archivo y volver |
| `H` | de la tabla al texto, luego hexadecimal, luego de vuelta a la tabla |

El marco muestra `TABLE [2/5]` cuando hay más de una hoja o tabla, y la línea inferior dónde está el cursor: `Cell B7 (Row 7 of 120)`. La última línea de la cuadrícula trae el contenido completo de la celda y el filtro en uso. La búsqueda (`F`, `B`, `/`) no mira dentro de la cuadrícula y no encuentra nada ahí; funciona en modo texto.

### El filtro SQL

`\` o `&` pide una condición, como el `WHERE` de SQL (la palabra `WHERE` es opcional), y muestra solo las filas que la cumplen. El prompt empieza con el filtro actual; déjelo vacío para volver a ver todas las filas. Una nota dice `Filter matched N rows` o `SQL error: ...`; tras un error, el filtro anterior sigue activo.

```
age >= 30 AND name LIKE 'Ali%'
WHERE status IN ('open', 'late') OR total > 1000.50
NOT (city = 'Rio' OR city IS NULL)
B ILIKE '%silva%'
```

| Operador | Qué hace |
|---|---|
| `=` `!=` `<>` `<` `<=` `>` `>=` | comparan |
| `AND` `OR` `NOT` y paréntesis | combinan condiciones (`AND` va antes que `OR`) |
| `LIKE` `ILIKE` | coinciden con un patrón, `%` para cualquier texto y `_` para un carácter; `ILIKE` no distingue mayúsculas |
| `IN (...)` `NOT IN (...)` | está, o no está, en la lista |
| `IS NULL` `IS NOT NULL` | la celda está, o no está, vacía |

Las palabras clave pueden ir en mayúsculas o minúsculas.

- **Columnas:** el nombre de la columna, en mayúsculas o minúsculas y sin comillas, o su letra, de `A` a `Z`. La letra sirve para llegar a un nombre con espacios o símbolos. Una columna que no existe cuenta como vacía, sin error.
- **Valores:** texto entre comillas simples o dobles (`\'` para una comilla dentro de él), números, `TRUE`, `FALSE` y `NULL`. Lo que va entre comillas es siempre texto, nunca una columna.
- **Celdas:** el tipo viene del aspecto de la celda. Vacía es `NULL`, `true` y `false` son booleanos, los números son números y el resto es texto. Por eso escriba `code = 7`, no `code = '7'`: un texto y un número nunca son iguales, y una celda con `007` es el número 7. Las comparaciones de texto distinguen mayúsculas de minúsculas.
- **Tamaño:** un filtro tiene como máximo 1024 bytes y 128 términos.

### Límites

Una tabla guarda como máximo 50 000 filas y 256 columnas, y tarda como máximo lo que permiten los demás límites del visor: 3 s (100 ms en Autoview), y una hoja de cálculo que se desempaqueta a más del límite de tamaño (1 MiB en Autoview) no se abre como tabla. Lo que pasa de ahí no aparece y el filtro no lo ve; la cabecera lo avisa: `(showing first 50,000 rows)` o `(stopped by the time limit: showing N rows)`. Una celda muestra como máximo 2048 caracteres y luego `…`. Los caracteres de control y de dirección del texto aparecen como `·`.

## Tablas en bibliotecas C

Las bases SQLite (`.db`, `.sqlite`, `.sqlite3`) y los archivos Parquet se leen con bibliotecas escritas en C, que no tienen la seguridad de memoria de Rust. Por eso solo se abren como tabla con `V`. Autoview lee todo aquello por donde pasa el cursor y muestra estos archivos como contenido normal, con la nota `Parquet or SQLite: V opens the table`. Abre una base de origen no confiable solo si aceptas este riesgo.

Véase también: [buscar en los archivos marcados](search.md).
