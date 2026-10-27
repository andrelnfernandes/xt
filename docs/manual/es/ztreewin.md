# Comparación con ZTreeWin

Un repaso comando por comando basado en el manual oficial de ZTreeWin v2.2
([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf), de 2011, en especial
la "Quick Command Table" y la sección 3.7 sobre configuración). Las versiones
más recientes de ZTreeWin pueden haber cambiado algunos detalles.

## Implementado

Árbol con lectura bajo demanda (`+`, `*`), ventana de archivos, Filespec,
Branch, Showall, marcas (T, U, I, Ctrl+T/U), Copiar/Mover/Eliminar/Renombrar con
las versiones Ctrl (marcados) y Alt (marcados con rutas), crear directorio,
eliminar directorio, Prune, Atributos, Visor, Editar, eJecutar, Comparar
(directorio y rama), Ordenar, Log, pantalla dividida (F8 y Tab), releer (F3),
Abrir (`O`), actualización automática, exploración y extracción de archivos
comprimidos, historial en las entradas, ayuda (F1) y pantalla de configuración
(Alt+F10).

## Simplificado

| Función | En ZTreeWin | En Xylem |
|---|---|---|
| Filespec | nombre con reglas propias de nombre y extensión, fecha, tamaño, atributo, filtro invertido (Ctrl+I), etiquetas | glob para nombres, fecha y tamaño como en ZTreeWin (con K/M/G); sin atributos, inversión ni etiquetas |
| Visor | modos Alpha, Dump, Hex, Junk y Wrap; juegos de caracteres; edición hexadecimal; marcadores; desplazamiento automático | texto (UTF-8, UTF-16, CP850, Windows-1252 y CP437, detectados solos, O cambia) o hexadecimal, con la búsqueda de ZTreeWin (F, B, /, \\, Space, + y -), colores de sintaxis (que ZTreeWin no tiene), ajuste de línea (W), números de línea (#), ir a línea (g / :); hasta el límite de tamaño (`viewer_max_size`, de 16 MiB a 1 GiB); sin edición hexadecimal, por decisión propia (puedes configurar tu propio editor para archivos binarios en [`[editors]`](execute.md)) |
| Ctrl+S | texto, hexadecimal o Unicode; en archivos ZIP busca solo en el índice | texto o hexadecimal |
| Atributos | atributos de Windows (R, A, S, H) | permisos Unix en octal (`644`); sin propietario/grupo. En Windows, R, H, S y A, como en ZTreeWin |
| Ordenar | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; igual/diferente en pantalla dividida | Nombre, Extensión, Fecha, Tamaño, ascendente/descendente, Ruta primero; cada ventana tiene su propio orden |
| Comparar | opciones Identical, Unique, Newer, Older, Size, Binary, Case; filtro de visualización | marca únicos, más recientes, más antiguos, idénticos, diferentes o únicos+más recientes; sin comparación binaria |
| Prune | elimina solo los subdirectorios o también el actual; solo directorios vacíos; papelera de reciclaje | elimina siempre todo el directorio, permanentemente; confirmación configurable |
| Eliminar | papelera de reciclaje opcional | papelera del sistema opcional (delete_to_trash, alternancia T/P en la pregunta, Shift+Del permanente); Oops (O/Ctrl+Z) para restaurar |
| Log | varias unidades registradas, Global, cambio de unidad | una raíz por ventana; `L` abre otro directorio |
| eJecutar | parámetros de lote `%1`–`%8`, ventana nueva (NTW) | `sh` con `%f`, `%t` y `%d`; siempre en la terminal de Xylem; además, una terminal integrada (Alt+X) |
| Historial | ventana de historial con marcadores | ↑/↓ en la entrada, un historial por tipo, guardado entre sesiones |
| Configuración | 7 páginas, editor de color por elemento (F4), Alt-Export | 4 páginas con las opciones que tienen sentido en Linux; F4 elige entre 12 temas listos para usar; colores individuales en `config.toml` |
| Archivos comprimidos | cualquier formato con un compresor externo, crear (F5), extraer (Alt-F5) | ZIP/JAR/WAR/APK, 7z, TAR (sin comprimir, gzip, xz, bzip2, zstd): abrir (Alt+F5), extraer con Copiar, crear con F5 (ZIP, TAR y TAR.GZ) |
| Pantalla dividida | paneles de estadísticas opcionales por lado | sin panel de estadísticas |
| Menús | menú Ctrl/Alt al mantener pulsada la tecla | igual en terminales con el protocolo de kitty; F9 en cualquier terminal |

Opciones de configuración de ZTreeWin que no se trajeron por ser específicas de
Windows: atributos RSH, uniones (junctions), OEM/ANSI, EBCDIC, tamaño de
ventana, accesos directos (.lnk). El resto depende de funciones aún no
implementadas (más abajo).

## Teclas diferentes

Las teclas siguen las de ZTreeWin. Las excepciones:

| Función | ZTreeWin | Xylem |
|---|---|---|
| Ayuda | F1 | F1 o Alt+H (en GNOME Terminal, F1 abre la ayuda de la terminal) |
| Configuración | Alt+F10 | F12 o Alt+F10 (en GNOME, Alt+F10 maximiza la ventana y nunca llega a Xylem) |
| Marcadores | F11 | F11, Alt+B o comilla simple `'` (en GNOME Terminal, F11 alterna la pantalla completa) |
| Mover marcados | Ctrl+M | Ctrl+M en terminales con el protocolo de kitty; en otras, `F9` `M` (o Alt+M y F6) |
| `-` en el árbol | descarga la rama de la memoria | solo contrae (sigue registrada) |
| F2 | (nada) | vuelve a leer el directorio, igual que F3 |
| F9 | menú de aplicaciones (no implementado) | menús Ctrl/Alt |
| F12 | macros (no implementadas) | configuración |
| `/` | igual que `?` (estadísticas) | Treespec, como `\` (`?` sigue mostrando las estadísticas) |
| Ctrl+C, Ctrl+M en el árbol | (nada) | Copiar/Mover los archivos marcados del directorio actual |
| Copiar ruta al portapapeles | Ctrl+Insert | Y o Ctrl+Insert (Ctrl+Y para los marcados; funciona mediante OSC 52 y el portapapeles del sistema) |

Todas las teclas se pueden reasignar bajo `[keys]` en `config.toml`.

## No implementado

- **Árbol:** Global (G), Graft (Alt+G), Mirror (Alt+M), liberar/descargar de la
  memoria (Alt+R, `-`, Ctrl+Backspace), Hide (Alt+H), Wash (Alt+W), Volume
  (Ctrl+V), F5/F6 (ocultar niveles de subdirectorios), marcar por atributo
  (Alt+T/U), Treespec en modo de edición (F5, comodines) y carpetas especiales
  (F6), búsqueda directa con entrada (`|`) y modo activo, cambio de unidad (`<`
  y `>`).
- **Archivos:** Abrir en una nueva ventana de terminal (Alt+O), Batch del
  archivo actual (Alt+B), JFC (J, comparar el contenido de dos archivos), ZAAP
  (compresor externo), sHortcut (H), Imprimir (P), propietario/grupo (chown),
  Megaview (Ctrl+F7 en la ventana de archivos), combinar archivos marcados entre
  ventanas (Ctrl+F6), desmarcado parcial (Ctrl+F8).
- **General:** macros (F12), menú de aplicaciones (F9), salir a la shell
  (Ctrl+Q), guardar y cargar el registro (Zlist, Ctrl/Alt+Z), colores por nombre
  y atributo (ZCOLORS.INI), ratón, opciones de línea de comandos de ZTreeWin,
  comandos de vídeo (Alt+F6–F9).
