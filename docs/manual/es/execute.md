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

Con una tabla o un texto abierto en el [visor](viewer.md), **Ctrl+B** escribe una línea por registro, a partir del archivo entero: más allá de las 50 000 filas de la pantalla, con los valores enteros y tal como están en el archivo. Respeta el filtro en uso (el filtro SQL en la tabla, el [filtro de líneas](filter.md) en el texto). La tabla se vuelve a leer en ese momento; el texto sale como lo leyó el visor, hasta el límite de tamaño, y la vista previa avisa cuando eso no es todo el archivo. Pide el archivo, luego la máscara, y muestra las primeras líneas antes de escribir: **Enter** escribe, **Esc** desiste. Durante la escritura, Esc cancela y elimina el archivo incompleto. Un archivo existente no se sobrescribe.

Mientras escribe la máscara, un recuadro encima muestra una máscara que funciona en este archivo, sus campos y las plantillas que tiene, cada una con la primera frase de su comentario inicial (`%%#`). **↑↓** eligen una plantilla, **F3** la muestra entera antes de usarla y **PgUp**/**PgDn** recorren el historial. La descripción la escribió quien hizo la plantilla: lea con F3 una plantilla de otra persona antes de usarla. Un error en la máscara mantiene el prompt abierto con lo que escribió e indica qué está mal.

| Campo | Valor |
|---|---|
| `%{nombre}` `%{A}` `%{2}` | una columna de la tabla, por nombre, letra o posición |
| `%{line}` | la línea entera del texto, con sus tabulaciones |
| `%{ip}` `%{1}` `%{0}` | un grupo de la expresión regular del filtro de líneas, por nombre o número; 0 es todo lo que encontró |
| `%{#}` | el número del registro entre los escritos: 1, 2, 3... |
| `%{#line}` | el número de la línea en el archivo (solo en texto) |
| `%{#path}` `%{#dir}` `%{#name}` `%{#base}` `%{#ext}` `%{#size}` | el archivo abierto, con escape como cualquier campo; `%N` y los otros de arriba no lo tienen, así que prefiere estos en una página o un script |
| `%{$NOMBRE}` | una variable de entorno, como `%{$PATH}`, con el valor de ahora; la vista previa lista las leídas, y una que no existe es un error |
| `%{#date}` `%{#time}` `%{#datetime}` `%{#utc}` | ahora, local (`#datetime` con la zona horaria) o UTC: igual en todos los registros |
| `%{#year}` `%{#month}` `%{#day}` `%{#weekday}` `%{#weekday_name}` `%{#epoch}` | partes de ello: el día de la semana 1 es lunes, el nombre está en inglés, epoch en segundos desde 1970 |
| `%{#user}` `%{#host}` `%{#template}` `%{#output}` | quién lo genera, en qué máquina, con qué plantilla, y el archivo escrito |
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
| `:mysql` | lo mismo para MySQL y MariaDB, donde la barra invertida también escapa | |
| `:csv` | un campo, entre comillas cuando hace falta | `.csv` `.tsv` |
| `:csvsafe` | lo mismo, con un `'` delante de lo que una hoja de cálculo tomaría por fórmula | |
| `:line` | el valor, con los caracteres de control como espacios, para que nunca abra una línea nueva | los demás |
| `:raw` | el valor tal cual: **no lo uses en un script** | |

- Un blob sale en hexadecimal, salvo en `:raw` y en `:sql`, que tiene su propia forma.
- Para MySQL y MariaDB, usa `:mysql`, en un archivo leído como `utf8mb4`: en ellos, una barra invertida en un literal `:sql` puede cerrarlo, y la vista previa avisa de esos valores. Con `NO_BACKSLASH_ESCAPES` en el servidor, lo correcto es `:sql`.
- En SQL, `%f %d %n %b %e %s` dentro de una cadena se rechazan: `LIKE '%foo%'` llevaría la ruta del archivo abierto. Escribe `%%` para tener un `%`, como en `LIKE '%%foo%%'`, o un campo como `%{#name}` para tener el valor.
- El filtro de una tabla vale para todo el archivo, con las reglas de la pantalla: `code = 7` también encuentra una celda con `007`, que sale como `007`. Por eso el archivo puede tener filas que la pantalla, limitada a 50 000, no muestra.
- Un `.sh` empieza con `#!/bin/sh`, pero **no recibe permiso de ejecución**: léelo antes de ejecutarlo. `.bat` y `.cmd` se rechazan, porque `cmd` expande `%VAR%` incluso entre comillas.
- La vista previa y el mensaje final avisan de valores con caracteres de control o de dirección del texto y, en un CSV, de valores que empiezan como una fórmula. La vista previa cuenta solo lo que miró.

### Registros enteros

En un archivo `.json`, `.jsonl`, `.ndjson`, `.csv`, `.tsv`, `.xml`, `.html` o `.htm`, deja la máscara **vacía** para escribir registros enteros, con las reglas del formato: un array JSON de objetos, un objeto por línea, una tabla con fila de cabecera, un `<record>` por registro o una página con una tabla. Para escribir solo algunos campos, empieza la máscara con `@`: `@id, nombre, #`. El texto da `line_number`, `line` y los grupos del filtro. Los campos se descubren en los primeros 1000 registros: una clave de JSON Lines que solo aparezca después queda fuera, y el mensaje final lo avisa.

### Plantillas

Los archivos para probar todo esto están en [demo/](https://github.com/andrelnfernandes/xylem/tree/main/demo) en el repositorio: los mismos clientes en todos los formatos, y plantillas listas.

Para tener texto alrededor de los registros, guarda una plantilla en `~/.config/xylem/batch/nombre.tpl` y escribe `:nombre` como máscara; el prompt lista las que encuentre (las cuatro primeras).

```
%%# un comentario
%%header
<h1>%{#name}</h1>
<ul>
%%record
<li><a href="https://x/?id=%{id:url}">%{name}</a></li>
%%footer
</ul>
```

`%%header` y `%%footer` son opcionales y solo admiten el texto de la plantilla y los campos del archivo abierto. `%%record` se escribe una vez por registro y puede tener varias líneas; sin secciones, todo el archivo es el registro. Los escapes siguen al archivo de salida, como en una máscara, así que una plantilla de página debe escribirse en un `.html`. Una sección que no existe, una repetida o un campo de registro en la cabecera o el pie es un error en la vista previa. La vista previa muestra las últimas 3 líneas de la cabecera. Sin registros, no se escribe nada.

En el pie, `%{count}` es cuántos registros se escribieron. La cabecera no puede tenerlo, porque se escribe antes de leerlos.

En un `.sh`, `%{$HOME}` escribe el valor de ahora, mientras que `$HOME` se lee cuando el script se ejecuta. Una plantilla de otra persona es un script suyo: podría poner `%{$SECRETO:url}` en un comando, así que mira lo que la vista previa dice que lee.

## Historial

En cada entrada, ↑ y ↓ recuperan entradas anteriores. Cada tipo de entrada
(Filespec, destino, Log, eJecutar...) tiene su propio historial, guardado en
`~/.local/state/xylem/history.toml`, legible solo por ti, ya que puede contener
comandos escritos.
