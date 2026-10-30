# Editar, eJecutar y Batch

## Editar (E)

Abre el archivo en el editor: el configurado en la página 1 de la
[configuración](configuration.md), o bien `$VISUAL`, `$EDITOR`, `editor` de
Debian y Ubuntu, o `vi`.

Por defecto, el editor se abre **dentro del marco de Xylem**
(`editor_in_frame = true` en la página 1 de **F12**) y usa **el tema de colores
de Xylem** (`editor_theme = true`), incluidos los colores de sintaxis y de la
interfaz acordes al tema activo cuando el editor es `vi`, `vim` o `nvim`. Pon
`editor_in_frame = false` para ceder toda la terminal al editor, o
`editor_theme = false` para conservar el esquema de colores propio del editor.

### Archivos binarios y editores por tipo de archivo

Un editor de texto solo estropea una imagen o un programa, así que `E` nunca
abre **archivos binarios** (los que tienen un byte nulo cerca del principio, que
el visor muestra en hexadecimal) en el editor de texto.

En su lugar, cuando **Edit binary files with E** está activado
(`edit_binary = true`, tecla **I** en la página 1 de **F12**), `E` abre los
archivos binarios en un editor hexadecimal/binario:

1. Un patrón que coincida en `[editors]` (si lo hay), o
2. El **Binary editor** configurado en la página 1 de **F12** (`binary_editor`,
   tecla **J**) o `binary` en `[editors]`, o
3. Si se deja vacío, el primer editor hexadecimal de terminal encontrado en el
   `PATH`:
   - **Linux**: `hexedit`, `heh`, `bvi`, `dhex`
   - **Windows**: `heh` (`winget install ndd7xv.heh`), `hexedit`, `bvi`

Los editores hexadecimales de terminal se ejecutan igual que el editor de texto:
**dentro del marco de Xylem** por defecto (`editor_in_frame = true`) y con **el
tema de colores de Xylem** (`editor_theme = true`). Para un editor hexadecimal
gráfico (`ImHex`, `GHex`, `Okteta` en Linux; `ImHex`, `HxD` en Windows) que
quieras dejar abierto en su propia ventana, añade `&` al final del comando (por
ejemplo, `imhex %f &` o `ghex %f &`): Xylem lo lanza directamente sin abrir el
marco de terminal.

Si prefieres que `E` no abra archivos binarios (y solo lo indique en el pie de
página sugiriendo `O` para abrirlos con el programa del sistema), pon **Edit
binary files with E** en `no` (`edit_binary = false` en la página 1 de **F12**).

Para asignar un editor propio a determinados tipos de archivo, indícalos en la
tabla `[editors]` de `config.toml`:

```toml
[editors]
"*.png *.jpg *.jpeg *.gif" = "gimp %f &"
"*.md" = "typora %f &"
"*.svg" = ""            # vacío: el editor habitual, aunque haya coincidido
binary = "hexedit"      # respaldo para cualquier otro archivo binario
```

Cada clave es una lista de patrones de nombre, como en [Filespec](filespec.md)
(las mayúsculas y minúsculas no importan); cada valor es una línea de comando,
con `%f` para el archivo (si no, el archivo se añade al final). Gana la primera
clave que coincida, en el orden del archivo. `binary` es para los archivos
binarios que ningún patrón haya capturado cuando `binary_editor` está vacío; el
resto de archivos se abren en el editor de texto habitual.

Xylem no tiene un editor hexadecimal propio, y es una decisión deliberada:
desarrollar y mantener uno supondría mucho trabajo para poco uso, y editar bytes
a mano es una forma fácil de corromper un archivo. En su lugar, Xylem integra
editores hexadecimales dedicados directamente dentro de su marco y con su tema
de colores, y permite desactivar la edición de binarios con una sola opción en
**F12** si no la deseas.

## eJecutar (X)

Pide una línea de comando y la ejecuta con `sh` en el directorio actual. En
ella:

| | |
|---|---|
| `%f` | el archivo bajo el cursor |
| `%t` | los archivos marcados |
| `%d` | el directorio actual |
| `%%` | un `%` literal |

Todos salen ya entrecomillados: `tar czf backup.tgz %t`. Al terminar, Xylem
espera a que pulses Enter para que puedas leer la salida, y vuelve a leer los
directorios abiertos. Un comando que empiece por `@` no se guarda en el
historial (con `history_skip_at`).

Para sesiones largas, la [terminal integrada](terminal.md) (Alt+X) suele ser más
cómoda.

## Batch (Ctrl+B)

Pide el nombre del archivo que se va a crear (F2 [elige la carpeta](picker.md))
y luego una máscara de línea, y escribe una línea por cada archivo marcado.
Parámetros:

| | |
|---|---|
| `%f` | ruta |
| `%d` | directorio |
| `%n` | nombre |
| `%b` | nombre sin extensión |
| `%e` | extensión |
| `%s` | tamaño |

En minúsculas salen entrecomillados, seguros para la shell
(`convert %f %b.png`); en mayúsculas, como texto plano, para listas (`%N;%S`).
Un archivo `.sh` recibe `#!/bin/sh` y permiso de ejecución (ejecútalo con `X`).
Un archivo existente no se sobrescribe.

## Batch en el visor

Con una tabla o un texto abierto en el [visor](viewer.md), **Ctrl+B** escribe una línea por registro, a partir del archivo entero: más allá de las 50 000 filas de la pantalla, con los valores enteros y tal como están en el archivo. Respeta el filtro en uso (el filtro SQL en la tabla, el [filtro de líneas](filter.md) en el texto). La tabla se vuelve a leer en ese momento; el texto sale como lo leyó el visor, hasta el límite de tamaño. Pide el archivo, luego la máscara, y muestra las primeras líneas antes de escribir: **Enter** escribe, **Esc** desiste. Durante la escritura, Esc cancela y elimina el archivo incompleto. Un archivo existente no se sobrescribe.

| Campo | Valor |
|---|---|
| `%{nombre}` `%{A}` `%{2}` | una columna de la tabla, por nombre, letra o posición |
| `%{line}` | la línea entera del texto, con sus tabulaciones |
| `%{ip}` `%{1}` `%{0}` | un grupo de la expresión regular del filtro de líneas, por nombre o número; 0 es todo lo que encontró |
| `%{#}` | el número del registro entre los escritos: 1, 2, 3... |
| `%{#line}` | el número de la línea en el archivo (solo en texto) |
| `%f %d %n %b %e %s` | el archivo abierto, como en el Batch de arriba |
| `%%` | un `%` |

Un campo que no existe es un error en la vista previa, nunca un valor vacío. Como la vista previa mira 1000 registros como máximo, una clave de JSON Lines que solo aparezca después cuenta como inexistente.

Los valores vienen de un archivo que puede ser hostil y pueden ir a algo que vas a ejecutar, así que cada campo tiene un escape. Escríbelo tras el campo: `curl "https://api/x?id=%{id:url}"`.

| Escape | Para | Por defecto en |
|---|---|---|
| `:sh` | un argumento entre comillas de un comando de shell | `.sh` `.bash` `.zsh` |
| `:url` | una parte de una URL, con codificación por porcentaje | |
| `:json` | un valor JSON: números sin comillas, texto con comillas | `.json` `.jsonl` `.ndjson` |
| `:xml` o `:html` | texto o atributo | `.xml` `.html` `.htm` `.xhtml` `.svg` |
| `:sql` | un literal con comillas, `NULL`, `X'00ff'` | `.sql` |
| `:csv` | un campo, entre comillas cuando hace falta | `.csv` `.tsv` |
| `:csvsafe` | lo mismo, con un `'` delante de lo que una hoja de cálculo tomaría por fórmula | |
| `:line` | el valor, con los caracteres de control como espacios, para que nunca abra una línea nueva | los demás |
| `:raw` | el valor tal cual: **no lo uses en un script** | |

- Un blob sale en hexadecimal, salvo en `:raw` y en `:sql`, que tiene su propia forma.
- El filtro de una tabla vale para todo el archivo, con las reglas de la pantalla: `code = 7` también encuentra una celda con `007`, que sale como `007`. Por eso el archivo puede tener filas que la pantalla, limitada a 50 000, no muestra.
- Un `.sh` empieza con `#!/bin/sh`, pero **no recibe permiso de ejecución**: léelo antes de ejecutarlo. `.bat` y `.cmd` se rechazan, porque `cmd` expande `%VAR%` incluso entre comillas.
- La vista previa y el mensaje final avisan de valores con caracteres de control o de dirección del texto y, en un CSV, de valores que empiezan como una fórmula. La vista previa cuenta solo lo que miró.

## Historial

En cada entrada, ↑ y ↓ recuperan entradas anteriores. Cada tipo de entrada
(Filespec, destino, Log, eJecutar...) tiene su propio historial, guardado en
`~/.local/state/xylem/history.toml`, legible solo por ti, ya que puede contener
comandos escritos.
