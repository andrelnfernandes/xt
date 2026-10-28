# La ventana de archivos

La ventana de archivos lista los archivos del directorio bajo el cursor del
árbol, o de toda una rama (`B`) o de todo lo registrado (`S`). **Enter** vuelve
al árbol, y **Esc** también (en el directorio del archivo seleccionado).

## Moverse

| Tecla | Qué hace |
|---|---|
| ↑ ↓ PgUp PgDn Home End | mover el cursor |
| ← → | página arriba y abajo (en los modos de columnas: archivo anterior y siguiente) |
| Backspace | mostrar los archivos del directorio padre |
| Shift+letra, 0–9 | siguiente archivo que empieza por ese carácter |
| Space | bajar una línea |

## Modos de visualización (Alt+F)

Alt+F alterna, en el orden de ZTreeWin, entre **nombres en varias columnas**,
**nombres y tamaños en columnas**, **detalles** (una columna con tamaño, fecha
y permisos, el modo predeterminado) y **nombres largos** (una columna, solo
nombres completos).

Antes de nombres largos vienen dos modos que ZTreeWin no tiene:

- **propietario**: tamaño, `usuario:grupo` y permisos (no se muestra en Windows
  ni dentro de archivos comprimidos);
- **tipo**: qué es cada archivo, al estilo del comando `file` ("PNG image, 800 x
  600", "ELF 64-bit executable", "python3 script", "Rust, UTF-8 text"...), a
  partir de los primeros bytes de los archivos visibles.

En los modos de columnas, los archivos van de izquierda a derecha, ← → mueven un
archivo y ↑ ↓ una fila; los nombres de más de 20 caracteres se cortan con `…`.
El modo inicial es `file_display` en la [configuración](configuration.md).

## Autoview (F7)

Divide la pantalla entre la lista y el contenido del archivo seleccionado, que
sigue al cursor. Las teclas de la ventana de archivos siguen funcionando (`T`
marca, por ejemplo); Shift+↑ ↓ PgUp PgDn Home End desplazan el contenido; `W`
alterna el ajuste de línea, `#` alterna los números de línea; F7, Esc o Enter
salen. Muestra el primer 1 MiB de cada archivo (`V` muestra hasta 16 MiB).

## Abrir archivos

- `V`: el [visor](viewer.md) (texto o hexadecimal, con búsqueda).
- `E`: el editor ([Editar](execute.md)).
- `O`: el programa predeterminado del sistema (`xdg-open`: imágenes en el visor
  de imágenes, PDF en el lector de PDF...), en segundo plano. Ctrl+O abre todos
  los archivos marcados. Dentro de un archivo comprimido, la entrada se extrae
  primero a una carpeta temporal que se elimina al salir de Xylem. Se puede
  configurar otro programa en `opener`.
- Alt+F5: abrir un [archivo comprimido](archives.md) como directorio.

## Caracteres de dirección ocultos

Algunos caracteres invisibles cambian la dirección del texto, así que un nombre puede parecer distinto de lo que realmente es (Trojan Source, CVE-2021-42574; por ejemplo, `fatura\u{202E}fdp.sh` puede parecer un PDF inofensivo). Xylem nunca los envía al terminal: en el nombre aparecen marcados en rojo, el pie avisa "Name has hidden bidirectional characters" cuando el cursor está sobre uno, y `O` pide confirmación antes de abrirlo, mostrando el nombre con los caracteres escritos como `\u{XXXX}`.

## El portapapeles

`Y` o Ctrl+Insert copia la ruta del archivo (en el árbol, del directorio) al
portapapeles; Ctrl+Y copia las rutas de los archivos marcados, una por línea;
`y` en el visor copia la ruta del archivo que se está viendo. Funciona por SSH y
tmux (OSC 52) y conserva el portapapeles tras salir de Xylem, en X11 y Wayland.

## Otros comandos

| Tecla | Qué hace |
|---|---|
| `T` / `U` | marcar / desmarcar el archivo y bajar ([marcas](tagging.md)) |
| `C` / `M` | [copiar / mover](copy-move.md) el archivo |
| `D` o Del | [eliminarlo](delete.md) (Shift+Del: permanentemente) |
| `R` | [renombrarlo](rename.md); Ctrl+R renombra los archivos marcados con una máscara |
| `A` | [permisos](attributes.md) |
| `N` | [cambiar la fecha](attributes.md) |
| Alt+S | [ordenar](sort.md) |
| Ctrl+S / Ctrl+V | [buscar en los archivos marcados / verlos](search.md) |
| Ctrl+B | [Batch](execute.md): un script con una línea por archivo marcado |
| F5 / Ctrl+F5 | crear un [archivo comprimido](archives.md) con el archivo / los archivos marcados |
| Alt+I | Info: detalles del archivo ([información](info.md)) |

Véase también: [la ventana del árbol](tree.md), [las teclas activas](keys.md).
