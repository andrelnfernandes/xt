# Comparación con ZTreeWin

## Antes que nada, un agradecimiento

Uso alguna variante de XTree desde los años noventa y tengo una licencia
pagada de ZTreeWin desde 2013. Para mí, ZTreeWin sigue siendo el mejor gestor
de archivos en modo texto que existe: rápido, predecible, manejado por
completo con el teclado y construido en torno a una forma de trabajar (leer el
árbol, marcar archivos en ramas enteras, actuar sobre las marcas) que ningún
otro gestor de archivos ha igualado. Kim Henkel mantiene viva la forma de
trabajar de XTree en Windows desde hace décadas, y Xylem no existiría sin ese
trabajo.

Xylem no pretende reemplazar a ZTreeWin. Existe porque yo quería la misma
forma de trabajar en Linux, donde ZTreeWin no funciona de forma nativa. Xylem
también compila para Windows, porque Rust lo hace fácil, pero esa no es una
meta mía: en Windows, ZTreeWin vale con creces su licencia:
[ztree.com](https://www.ztree.com/), y el [foro](https://groups.io/g/ZTreeWin)
de sus usuarios.

Esta página compara los dos comando por comando, con base en el manual oficial
de ZTreeWin v2.2 ([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf), de
2011, en especial la "Quick Command Table" y la sección 3.7 sobre
configuración). Las versiones más recientes de ZTreeWin pueden haber cambiado
algunos detalles; si algo aquí está desactualizado, abre una issue.

## La misma forma de trabajar

Árbol con lectura bajo demanda (`+`, `*`), ventana de archivos, Filespec,
Branch, Showall (también solo de los marcados, Ctrl+S), marcas (T, U, I,
Ctrl+T/U), Copiar/Mover/Eliminar/Renombrar con las versiones Ctrl (marcados) y
Alt (marcados con rutas), crear directorio, eliminar directorio, Prune,
Atributos, Visor, Editar, eJecutar, Comparar (directorio y rama), Ordenar, Log,
pantalla dividida (F8 y Tab), releer (F3), Abrir (`O`), actualización
automática, exploración y extracción de archivos comprimidos, historial en las
entradas, marcadores, ayuda (F1) y pantalla de configuración. Las teclas son las
de ZTreeWin siempre que es posible; las excepciones están en "Teclas
diferentes", al final.

## Lo que Xylem todavía no tiene

- **Árbol:** Global (G), Graft (Alt+G), Mirror (Alt+M), liberar/descargar de la
  memoria (Alt+R, Ctrl+Backspace), Hide (Alt+H), Wash (Alt+W), Volume
  (Ctrl+V), F5/F6 (ocultar niveles de subdirectorios), marcar por atributo
  (Alt+T/U), Treespec en modo de edición (F5, comodines) y carpetas especiales
  (F6), búsqueda directa con entrada (`|`) y modo activo, cambio de unidad (`<`
  y `>`).
- **Archivos:** Abrir en una nueva ventana de terminal (Alt+O), Batch del
  archivo actual (Alt+B), JFC (J, comparar el contenido de dos archivos),
  sHortcut (H), Imprimir (P), propietario/grupo (chown), Megaview (Ctrl+F7 en la
  ventana de archivos), combinar archivos marcados entre ventanas (Ctrl+F6),
  desmarcado parcial (Ctrl+F8), Tags-Only fuera de Showall (Ctrl+Enter y
  Ctrl+F4, Ctrl+B en el árbol).
- **General:** macros (F12), menú de aplicaciones (F9), salir a la shell
  (Ctrl+Q), guardar y cargar el registro (Zlist, Ctrl/Alt+Z), colores por nombre
  y atributo (ZCOLORS.INI), ratón, opciones de línea de comandos de ZTreeWin,
  comandos de vídeo (Alt+F6–F9).

Para algunas de ellas hay un sustituto en otro programa; consulta "Lo que Xylem
deja a otros programas", más abajo.

## Donde Xylem es más sencillo

| Función | En ZTreeWin | En Xylem |
|---|---|---|
| Filespec | nombre con reglas propias de nombre y extensión, fecha, tamaño, atributo, filtro invertido (Ctrl+I), etiquetas | glob para nombres, fecha y tamaño como en ZTreeWin (con K/M/G); sin atributos, inversión ni etiquetas |
| Visor | modos Alpha, Dump, Hex, Junk y Wrap; juegos de caracteres; edición hexadecimal; marcadores; desplazamiento automático | texto o hexadecimal, con la búsqueda de ZTreeWin (F, B, /, \\, Space, + y -), ajuste de línea (W), números de línea (#), ir a línea (g / :); hasta el límite de tamaño (`viewer_max_size`, de 16 MiB a 1 GiB); sin edición hexadecimal, por decisión propia (consulta "Lo que Xylem deja a otros programas") |
| Ctrl+S | texto, hexadecimal o Unicode; en archivos ZIP busca solo en el índice | texto o hexadecimal |
| Atributos | atributos de Windows (R, A, S, H) | permisos Unix en octal (`644`); sin propietario/grupo. En Windows, R, H, S y A, como en ZTreeWin |
| Ordenar | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; igual/diferente en pantalla dividida | Nombre, Extensión, Fecha, Tamaño, ascendente/descendente, Ruta primero; cada ventana tiene su propio orden |
| Comparar | opciones Identical, Unique, Newer, Older, Size, Binary, Case; filtro de visualización | marca únicos, más recientes, más antiguos, idénticos, diferentes o únicos+más recientes; sin comparación binaria |
| Prune | elimina solo los subdirectorios o también el actual; solo directorios vacíos; papelera de reciclaje | elimina siempre todo el directorio, permanentemente; confirmación configurable |
| Log | varias unidades registradas, Global, cambio de unidad | una raíz por ventana; `L` abre otro directorio |
| eJecutar | parámetros de lote `%1`–`%8`, ventana nueva (NTW) | `sh` con `%f`, `%t` y `%d`; siempre en la terminal de Xylem |
| Historial | ventana de historial con marcadores | ↑/↓ en la entrada, un historial por tipo, guardado entre sesiones |
| Configuración | 7 páginas, editor de color por elemento (F4), Alt-Export | 4 páginas con las opciones que tienen sentido en Linux; F4 elige entre 12 temas listos para usar; colores individuales en `config.toml` |
| Pantalla dividida | paneles de estadísticas opcionales por lado | sin panel de estadísticas |
| Menús | menú Ctrl/Alt al mantener pulsada la tecla | igual en terminales con el protocolo de kitty; F9 en cualquier terminal |

Opciones de configuración de ZTreeWin que no se trajeron por ser específicas de
Windows: atributos RSH, uniones (junctions), OEM/ANSI, EBCDIC, tamaño de
ventana, accesos directos (.lnk).

## Donde Xylem va más allá

Cosas para las que el manual de ZTreeWin v2.2 no tiene equivalente:

- **Una terminal integrada** (Alt+X): la shell sigue en marcha mientras usas
  el árbol; un `cd` en la shell lleva el árbol consigo, y la shell sigue al
  árbol cuando vuelves a ella. Alt+O escribe los archivos marcados en la línea
  de comandos. Consulta [la terminal integrada](terminal.md).
- **Máquinas remotas por SSH** (`xy usuario@host:`): el árbol, Branch, el visor
  y Copiar funcionan en otra máquina, que también tiene su propia terminal.
  Consulta [máquinas remotas](remote.md).
- **Git** (opcional): letras de estado en la lista de archivos, un menú de Git
  (Ctrl+G) y diffs en el visor. Consulta [Git](git.md).
- **Uso del disco** (Alt+W): las carpetas y archivos de una rama juntos, los
  más grandes primero, hasta llegar a lo que llena el disco; Alt+K muestra en
  el árbol la parte de cada carpeta. Consulta [uso del disco](usage.md).
- **Un visor que lee más:** colores de sintaxis, la codificación del texto
  detectada sola (UTF-8, UTF-16, CP850, Windows-1252, CP437), archivos `.gz`,
  `.xz`, `.zst` y `.bz2` mostrados descomprimidos, y los caracteres ocultos de
  dirección del texto (Trojan Source) mostrados en vez de obedecidos. Consulta
  [el visor](viewer.md).
- **Archivos comprimidos sin compresor externo:** ZIP, 7z y TAR (sin
  comprimir, gzip, xz, bzip2, zstd) se abren como carpetas, y ZIP y TAR se
  crean, todo por el propio Xylem. Consulta [archivos comprimidos](archives.md).
- **Renombrado por lotes** con buscar y reemplazar (`s/foo/bar/`), contadores
  y cambios de mayúsculas/minúsculas. Consulta [renombrar](rename.md).
- **Oops:** O (o Ctrl+Z) devuelve a su lugar lo último que se envió a la
  papelera del sistema. Consulta [eliminar](delete.md).
- **El portapapeles en todas partes:** Y copia la ruta incluso por SSH o
  dentro de tmux (OSC 52).
- **Qué es cada archivo:** un modo de visualización (Alt+F) que dice, en el
  espíritu del comando `file`, qué contiene cada archivo, y otro con
  propietarios y permisos.
- **Aspecto:** 12 temas listos para usar e iconos Nerd Fonts opcionales. Toda
  tecla se puede reasignar en `[keys]` de `config.toml`. Consulta
  [configuración](configuration.md).
- **Archivos hostiles:** los archivos comprimidos, los nombres y los contenidos
  se tratan como potencialmente hostiles, con límites en todo lo que se lee y
  se descomprime (consulta la declaración de seguridad en el README).
- **Ayuda en seis idiomas** (F1 y luego L), el mismo texto de este manual y de
  la página man.

## Lo que Xylem deja a otros programas

Algunas cosas Xylem no las hace por sí mismo, a propósito: un programa
dedicado las hace mejor, y Xylem lo integra.

- **Edición hexadecimal:** ZTreeWin edita bytes en su propio visor. Xylem no
  tiene un editor hexadecimal propio: escribir y mantener uno sería mucho
  trabajo para poco uso, y editar bytes a mano es una forma fácil de corromper
  un archivo. En su lugar, E sobre un archivo binario abre `hexedit`, `heh`,
  `bvi` o `dhex` (el que esté instalado, o el que elijas) dentro del marco y
  los colores de Xylem, o uno gráfico como ImHex o GHex. Consulta
  [Editar](execute.md).
- **Editar texto:** tu editor (`$VISUAL`, `$EDITOR` o el definido en F12), y un
  editor distinto por tipo de archivo si quieres (`[editors]`).
- **SSH:** el `ssh` del sistema, así que tus claves, el agente,
  `~/.ssh/config` y los *jump hosts* funcionan como en cualquier otro sitio.
- **Almacenamiento en la nube:** Google Drive, S3 y otros mediante
  `rclone mount`; Xylem reconoce el montaje y se adapta. Consulta
  [almacenamiento en la nube](cloud.md).
- **Git:** el propio programa `git`.
- **Abrir archivos** (O): el programa del escritorio para cada tipo
  (`xdg-open`).
- **Comparar el contenido de dos archivos** (JFC en ZTreeWin): mientras no esté
  integrado, marca los dos archivos y ejecuta `diff %t` con eJecutar (X), o usa
  la terminal.
- **Imprimir** (P en ZTreeWin): `lp %t` con eJecutar.
- **Otros formatos de archivo comprimido** (el ZAAP de ZTreeWin): tu compresor,
  con eJecutar o en la terminal.

## Teclas diferentes

Las teclas siguen las de ZTreeWin. Las excepciones:

| Función | ZTreeWin | Xylem |
|---|---|---|
| Ayuda | F1 | F1 o Alt+H (en GNOME Terminal, F1 abre la ayuda de la terminal) |
| Configuración | Alt+F10 | F12 o Alt+F10 (en GNOME, Alt+F10 maximiza la ventana y nunca llega a Xylem) |
| Marcadores | F11 | F11, Alt+B o comilla simple `'` (en GNOME Terminal, F11 alterna la pantalla completa) |
| Mover marcados | Ctrl+M | Ctrl+M en terminales con el protocolo de kitty; en otras, `F9` `M` (o Alt+M y F6) |
| F2 | (nada) | vuelve a leer el directorio, igual que F3 |
| F9 | menú de aplicaciones (no implementado) | menús Ctrl/Alt |
| F12 | macros (no implementadas) | configuración |
| `/` | igual que `?` (estadísticas) | Treespec, como `\` (`?` sigue mostrando las estadísticas) |
| Ctrl+C, Ctrl+M en el árbol | (nada) | Copiar/Mover los archivos marcados del directorio actual |
| Copiar ruta al portapapeles | Ctrl+Insert | Y o Ctrl+Insert (Ctrl+Y para los marcados; funciona mediante OSC 52 y el portapapeles del sistema) |

Todas las teclas se pueden reasignar bajo `[keys]` en `config.toml`.
