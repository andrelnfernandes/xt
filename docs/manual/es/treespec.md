# Treespec y Log

## Treespec (`\` o `/`)

Abre una entrada con la ruta actual. A medida que escribes, el cursor del árbol
va al primer directorio que empieza por lo que has escrito (sin distinguir
mayúsculas de minúsculas):

- `/` o `\` completan el nombre y bajan un nivel;
- Tab y Shift+Tab completan y pasan al directorio siguiente o anterior del mismo
  nivel;
- Backspace sube niveles;
- Enter confirma y Esc vuelve adonde estabas;
- ↑ recupera rutas anteriores (guardadas entre sesiones).

Una ruta fuera del árbol se convierte en un Log al confirmarla. En Branch y
Showall, `\` va directamente al directorio del archivo seleccionado, dejándolo
resaltado.

## Log (L)

Abre otro directorio en la ventana, reemplazando el árbol (un directorio dentro
del árbol actual simplemente mueve el cursor allí). Una dirección como
`user@host:` abre una [máquina remota](remote.md). La ruta puede ser absoluta,
relativa al directorio actual o empezar por `~`; **F2**
[la elige desde un árbol](picker.md) y ↑ recupera las anteriores. Las ventanas
nuevas conservan el [orden](sort.md) y el modo de visualización actuales.

Véase también: [la ventana del árbol](tree.md), [marcadores](bookmarks.md).
