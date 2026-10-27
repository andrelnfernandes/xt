# Eliminar, papelera y Prune

## Eliminar

`D` o Del elimina el archivo bajo el cursor (en el árbol, el directorio, solo si
está vacío); Ctrl+D elimina los archivos marcados. Xylem siempre pregunta antes.

Con `delete_to_trash = true` en la [configuración](configuration.md), los
archivos y directorios vacíos van a la papelera del sistema (FreeDesktop Trash
en Linux, la Papelera de reciclaje en Windows) en lugar de borrarse. En la
pregunta, **T** los envía a la papelera y **P** los elimina permanentemente, sea
cual sea el ajuste. **Shift+Del** siempre elimina permanentemente.

## Oops

**Oops** (`O` en el árbol, o Ctrl+Z en ambas ventanas) restaura a su lugar lo
último enviado a la papelera. Si no se ha enviado nada a la papelera en esta
sesión, ofrece el elemento más reciente de la papelera del sistema.

## Prune (Alt+P en el árbol)

Elimina el directorio con todo lo que hay dentro. Xylem registra toda la rama,
muestra cuántos archivos, directorios y bytes se borrarán, y te pide **escribir
PRUNE** (o el nombre del directorio, o Y, según `prune_confirmation`). Elimina
primero los archivos y luego los directorios, desde el más profundo hacia
arriba, con barra de progreso y `Esc` para cancelar. Los enlaces simbólicos se
eliminan sin seguirlos, y **otros puntos de montaje dentro de la rama se
omiten** (una unidad USB montada allí dentro no se borra). La raíz del árbol no
se puede eliminar con Prune. Prune siempre elimina permanentemente.
