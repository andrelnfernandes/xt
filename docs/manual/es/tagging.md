# Marcas y operaciones en grupo

Los archivos marcados (señalados con `■`) son aquellos sobre los que actúan las
operaciones en grupo: los archivos marcados **de la lista actual** (el
directorio, o la vista Branch o Showall abierta).

## Marcar

| Tecla | Árbol | Ventana de archivos |
|---|---|---|
| `T` / `U` | marcar / desmarcar los archivos del directorio | marcar / desmarcar el archivo y bajar |
| Ctrl+T / Ctrl+U | marcar / desmarcar todo lo registrado (que coincida con el Filespec) | marcar / desmarcar toda la lista |
| Ctrl+S | Showall solo de los archivos marcados | |
| `I` | invertir las marcas del directorio | invertir las marcas de la lista |
| Ctrl+F7 / F8 / F9 | marcar / desmarcar / invertir los archivos registrados de la rama | |

## Operaciones sobre los archivos marcados

| Tecla | Qué hace |
|---|---|
| Ctrl+C / Ctrl+M | [copiarlos / moverlos](copy-move.md) al destino |
| Alt+C / Alt+M | copiarlos / moverlos, **recreando sus rutas** |
| Ctrl+D | [eliminarlos](delete.md) |
| Ctrl+A | cambiar sus [permisos](attributes.md) |
| Ctrl+N | cambiar su [fecha](attributes.md) |
| Ctrl+R | [renombrarlos](rename.md) con una máscara |
| Ctrl+B | [Batch](execute.md): un script o lista con una línea por archivo |
| Ctrl+F5 | meterlos en un [archivo comprimido](archives.md) |
| Ctrl+S / Ctrl+V | [buscar texto en ellos / verlos](search.md) |
| Ctrl+O | abrirlos en el programa predeterminado |
| Ctrl+Y | copiar sus rutas al portapapeles |
| Alt+O | escribirlos en la [terminal integrada](terminal.md) |

En eJecutar, `%t` representa los archivos marcados ([eJecutar](execute.md)).

## Menús Ctrl y Alt (F9)

Como en ZTree, el pie de página muestra solo los comandos de una tecla. **F9**
cambia al menú Ctrl, luego al menú Alt y después vuelve al normal. Con uno de
estos menús abierto, la siguiente letra cuenta como Ctrl+letra o Alt+letra
(`F9` `M` = Ctrl+M); `Esc` cancela. En terminales con el protocolo de teclado de
kitty, basta con **mantener pulsado Ctrl o Alt** para mostrar el menú
correspondiente.

**Ctrl+M:** en la mayoría de las terminales, Ctrl+M llega igual que Enter. Xylem
activa el protocolo de teclado de kitty cuando la terminal lo admite (kitty,
WezTerm, foot, Ghostty, Alacritty reciente, iTerm2), y entonces Ctrl+M mueve los
archivos marcados como en ZTree. En terminales basadas en VTE (GNOME Terminal,
Ptyxis, Terminator, Tilix) usa **F9 M** (o Alt+M y F6 en la entrada).
