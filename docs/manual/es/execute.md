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

### Editores por tipo de archivo

Un editor de texto solo estropea una imagen o un programa, así que E no abre
**archivos binarios** (los que tienen un byte nulo cerca del principio, que el
visor muestra en hexadecimal): lo indica en el pie de página y sugiere `O`, que
abre el archivo con el programa propio del sistema. Para editarlos, o para
asignar un editor propio a determinados archivos, indícalos en la tabla
`[editors]` de `config.toml`:

```toml
[editors]
"*.png *.jpg *.jpeg *.gif" = "gimp %f"
"*.md" = "typora"
"*.svg" = ""            # vacío: el editor habitual, aunque haya coincidido
binary = "hexedit"      # cualquier otro archivo binario
```

Cada clave es una lista de patrones de nombre, como en [Filespec](filespec.md)
(las mayúsculas y minúsculas no importan); cada valor es una línea de comando,
con `%f` para el archivo (si no, el archivo se añade al final). Gana la primera
clave que coincida, en el orden del archivo. `binary` es para los archivos
binarios que ningún patrón haya capturado; el resto de archivos se abren en el
editor habitual. Los comandos se ejecutan igual que el editor: dentro del marco
de Xylem por defecto, o cediendo la terminal cuando `editor_in_frame = false`
(para un programa gráfico que quieras dejar abierto, añade `&` y pon
`editor_in_frame = false`, o usa `O`).

Xylem no tiene un editor hexadecimal propio, y es una decisión deliberada, no
una carencia: supondría mucho trabajo para poco uso, y editar bytes a mano es
una forma fácil de corromper un archivo. Quien lo necesite puede elegir una
herramienta hecha para ello (`hexedit`, `bvi`, `dhex`, `ImHex`...) en `binary`.

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

## Historial

En cada entrada, ↑ y ↓ recuperan entradas anteriores. Cada tipo de entrada
(Filespec, destino, Log, eJecutar...) tiene su propio historial, guardado en
`~/.local/state/xylem/history.toml`, legible solo por ti, ya que puede contener
comandos escritos.
