# Duplicados (Alt+Y)

Archivos con el mismo contenido, sean cuales sean sus nombres y carpetas, y el
espacio que desperdician las copias. **Alt+Y** busca en la rama del directorio
actual; **D** en la vista de Uso busca en la carpeta listada allí.

Lo que más libera aparece primero. Nada queda marcado hasta que lo pidas, y una
copia de cada grupo siempre queda: Xylem no deja marcar la última.

## Elegir lo que se va

| Tecla | Qué hace |
|---|---|
| ↑ ↓ PgUp PgDn Home End | mover |
| `T` o Espacio | marca o desmarca la copia |
| `K` | conserva esta copia: marca las otras del grupo |
| `A` | en cada grupo, marca todas menos una; `A` otra vez cambia cuál queda: la más antigua, la más nueva, la ruta más corta |
| `F` | en cada grupo con una copia en la carpeta de esta copia, conserva esa |
| `U` | desmarca todo |
| `D`, Supr o Ctrl+D | borra las marcadas, como el comando Delete |
| `M` o Ctrl+M | mueve los archivos marcados |
| `G` | archivos o carpetas enteras |
| `V` o Enter | muestra el archivo |
| `E` | escribe la lista en un archivo (nunca sobre uno que exista) |
| `\` | cierra la vista, con el árbol en esa carpeta |
| F3 | lee las carpetas otra vez y compara otra vez |
| Esc o Alt+Y | cierra |

Antes de borrar o mover, cada grupo se revisa otra vez: si una copia cambió desde
el análisis, o la que iba a quedar desapareció, no se toca nada de ese grupo.

## Carpetas enteras

**G** muestra las carpetas con exactamente el mismo contenido: los mismos nombres,
el mismo contenido de cada archivo y los mismos enlaces. Una carpeta nunca se
llama copia si algo en ella no se comparó: una carpeta ilegible, otro disco
montado dentro, un archivo especial o una carpeta ignorada.

## Cómo se comparan los archivos

Solo se leen los archivos cuyo tamaño coincide con el de otro: primero los dos
extremos, luego el medio, por partes, con SHA-256. Corre en segundo plano y Esc
cancela. Quedan fuera los archivos menores que `duplicates_min_size` (1 KiB por
defecto), los vacíos, los enlaces, otros enlaces duros del mismo archivo, otros
discos y las carpetas de `duplicates_ignore` (`.git` y `node_modules`).

En una carpeta de red se pregunta antes el total a leer. En una máquina remota o
dentro de un archivo comprimido aún no está disponible.

## En la nube (rclone)

En una carpeta montada con `rclone mount`, Xylem pide a rclone los hashes que el
servicio ya guarda (`rclone lsjson --hash`) y no descarga nada. rclone debe estar
en el PATH; si no, o si el remoto no da hashes, Xylem explica el motivo y pregunta
antes de leer los archivos por el montaje. Las carpetas enteras solo se comparan
en discos locales.

En Google Drive: un archivo en dos carpetas es un solo archivo; Docs, Hojas y
Presentaciones no tienen hash; lo que otros compartieron aparece con `s` y marcar
todas menos una no lo toca; dos nombres iguales en una carpeta se resuelven con
`rclone dedupe`; y borrar envía a la papelera del Drive, que Xylem no vacía:
muestra el comando `rclone cleanup gdrive:`.

El hash del servicio se acepta por confianza, y MD5 puede falsificarse a propósito.
