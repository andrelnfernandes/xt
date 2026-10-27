# La ventana del árbol

El árbol muestra los directorios que han sido registrados (leídos en memoria).
La ventana de archivos debajo lista los archivos del directorio bajo el cursor.

## Moverse

| Tecla | Qué hace |
|---|---|
| ↑ ↓ PgUp PgDn Home End | mover el cursor |
| → o `+` | expandir el directorio |
| ← o `-` | contraerlo (o ir al directorio padre) |
| Backspace | ir al directorio padre |
| Space | bajar una línea |
| Shift+letra, 0–9 | siguiente directorio que empieza por ese carácter |
| `\` o `/` | [Treespec](treespec.md): ir a un directorio escribiendo su ruta |
| Enter o Tab | ir a la ventana de archivos (Tab cambia de ventana en [pantalla dividida](split.md)) |

En el árbol, un `+` en el conector (`├─+`) indica un directorio contraído que
tiene subdirectorios.

## Registro de directorios

Los directorios se leen a medida que los abres. `*` registra y expande toda la
rama bajo el cursor; durante una lectura larga, el progreso se muestra en el
pie de página y `Esc` cancela. **F3** (o F2) vuelve a leer el directorio del
disco. Xylem también vigila el directorio actual de cada ventana y lo vuelve a
leer por su cuenta cuando otro programa lo modifica (como máximo una vez cada
medio segundo, tras cualquier operación en curso); los demás directorios siguen
necesitando F3.

`L` ([Log](treespec.md)) abre otro directorio en la ventana.

## Listas de archivos

- `B` (Branch): los archivos de toda la rama, registrándola primero.
- `S` (Showall): los archivos de todo lo que se ha registrado.
- `F` ([Filespec](filespec.md)): qué archivos se muestran, por nombre, fecha y tamaño.

En Branch y Showall, `\` va al directorio del archivo seleccionado.

## Comandos en el árbol

| Tecla | Qué hace |
|---|---|
| `T` / `U` / `I` | marcar / desmarcar / invertir los archivos del directorio ([marcas](tagging.md)) |
| `M` | crear un subdirectorio |
| `R` | renombrar el directorio |
| `D` o Del | eliminar el directorio, si está vacío ([eliminar](delete.md)) |
| Alt+P | Prune: eliminar el directorio con todo su contenido |
| `O` o Ctrl+Z | Oops: restaurar lo enviado a la papelera |
| `N` | cambiar la fecha del directorio ([fechas](attributes.md)) |
| `C` / Alt+C | [Comparar](compare.md) con la otra ventana / toda la rama |
| `A` | Avail: sistemas de archivos montados ([información](info.md)) |
| Alt+I | Info: detalles del directorio |
| Alt+K | tamaños, recuento de archivos o porcentajes junto a cada directorio |
| `Y` | copiar la ruta del directorio al portapapeles |
| `X` | [eJecutar](execute.md) un comando de shell |
| Alt+X | la [terminal integrada](terminal.md) |
| F8 | [pantalla dividida](split.md) |
| F11 / Alt+B | [marcadores](bookmarks.md); Ctrl+B guarda uno |

Véase también: [la ventana de archivos](files.md), [las teclas activas](keys.md).
