# Configuración

## La pantalla de configuración (F12)

**F12** (o Alt+F10, como en ZTreeWin; en GNOME, Alt+F10 suele maximizar la
ventana y nunca llega a Xylem) abre cuatro páginas:

| Página | Opciones |
|---|---|
| 1. Programas y formatos | editor, abrir el editor dentro del marco de Xylem, tema de Xylem en el editor, editar archivos binarios con E, editor binario, idioma de la ayuda, programa para abrir, formato de fecha, formato numérico, comandos de Git, carpetas Git de confianza |
| 2. Visualización y orden | archivos ocultos, orden y dirección iniciales, qué hace Enter en la ventana de archivos, modo de visualización inicial, colores de sintaxis, ajuste de línea, números de línea, archivos comprimidos, la codificación de textos antiguos, el límite de tamaño de lo que lee el visor, iconos y glifos (Nerd Fonts), colores de la terminal integrada |
| 3. Confirmaciones e historial | confirmar al salir, eliminar a la papelera, confirmación de Prune, no guardar entradas con `@`, historial propio de la terminal integrada |
| 4. Copiar, comparar y actualizar | ignorar 2 s, ignorar 1 h, actualización automática, dónde empiezan las rutas de Alt+C/Alt+M |

El límite de tamaño del visor (**L** en la página 2) cuesta memoria y tiempo: con `1024` MiB, abrir un archivo de texto normal de ese tamaño ocupa unos 2,5 GiB de RAM y tarda unos 5 s, y los archivos de líneas muy cortas cuestan más por byte (consulta [el visor](viewer.md)).

Los iconos y glifos (**M** en la página 2) requieren una fuente con el parche de Nerd Fonts. No necesita una fuente específica: cualquier fuente monoespaciada sirve (como Fira Code, JetBrains Mono, Hack, Meslo o Cascadia Code), siempre que sea su edición "Nerd Font". Nerd Fonts añade miles de glifos e iconos estándar a la zona privada de Unicode. Las fuentes comunes sin el parche (incluso la Fira Code original) no contienen estos dibujos y mostrarán cuadros vacíos (`□`). Para usarlos:
- Descargue cualquier Nerd Font de su preferencia desde nerdfonts.com o instale un paquete de Nerd Fonts con el gestor de paquetes de su distribución.
- Seleccione esa fuente en las preferencias de su emulador de terminal.

Si las columnas se desalinean con los iconos activados, probablemente la terminal dibuja los caracteres de "ancho ambiguo" como anchos (un ajuste habitual con fuentes chinas, japonesas y coreanas), mientras que Xylem cuenta cada icono como una celda. Ponga esa opción en estrecho o desactive los iconos.

La letra de la opción (o Enter) cambia el valor, PgUp/PgDn o 1–4 cambian de
página, **F4** elige el tema de una lista (con vista previa instantánea), **F3**
guarda y aplica, y **Esc** cancela (pregunta si hay cambios).

## El archivo de configuración

Todo se guarda en `~/.config/xylem/config.toml` (o `$XDG_CONFIG_HOME/xylem/`;
`%APPDATA%\xylem\` en Windows), que también se puede editar a mano: la pantalla
conserva tus comentarios. Para empezar a partir de un archivo comentado con
todos los valores predeterminados:

```bash
mkdir -p ~/.config/xylem && xy --default-config > ~/.config/xylem/config.toml
```

- `theme`: `classic` (azul XTree), `dark`, `light`, `mono` (los colores propios
  de la terminal), `norton`, `solarized`, `gruvbox`, `nord`, `slate` (al estilo
  de Windows Terminal), `graphite` (macOS en modo oscuro), `daylight` (macOS en
  modo claro) o `vivid` (colores intensos, para presentaciones y capturas de
  pantalla). F4 los muestra todos con vista previa.
- `[options]`: las opciones de la pantalla de configuración.
- `[colors]`: colores individuales sobre el tema, como nombres (`"yellow"`),
  `"#rrggbb"` o índices de 0 a 255.
- `[keys.any]`, `[keys.tree]`, `[keys.files]`: reasignan teclas en ambas
  ventanas o solo en una (que tiene prioridad). Por ejemplo, `"f3" = "view"`,
  `"ctrl+r" = "relog"`; `"none"` desactiva una tecla. La lista de teclas de la
  ayuda (**K**) muestra las teclas activas con tus cambios.
- `[editors]`: el editor para cada tipo de archivo
  (`"*.png *.jpg" = "gimp %f"`, `binary = "hexedit"`); véase
  [Editar](execute.md).

Un error en el archivo no impide que Xylem arranque: usa los valores
predeterminados, muestra el error en el pie de página y la pantalla de
configuración no guardará hasta que se corrija el archivo, para no borrar lo que
hayas escrito.

Xylem se llamaba xt hasta la versión 0.1. En su primera ejecución mueve la
configuración, el historial y los marcadores de xt a sus propias carpetas.
