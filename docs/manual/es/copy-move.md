# Copiar y mover

`C` copia y `M` mueve el archivo bajo el cursor; Ctrl+C y Ctrl+M los archivos
marcados; Alt+C y Alt+M los archivos marcados recreando sus rutas. La entrada
pide el destino.

## El destino

Un directorio (relativo al directorio bajo el cursor del árbol, absoluto o con
`~`) o, para un solo archivo, un nombre nuevo. Con la pantalla
[dividida](split.md), se sugiere el directorio de la otra ventana. **F2** abre
un árbol para [elegir la carpeta](picker.md); **↑** recupera destinos
anteriores.

Una carpeta de destino que aún no exista se crea después de confirmar.

## Con o sin rutas

Ctrl+C / Ctrl+M ponen todos los archivos marcados directamente en el destino.
Alt+C / Alt+M recrean allí la ruta de cada archivo; dónde empieza esa ruta
depende del modo. Para `/home/me/proj/src/ops/mod.rs`, listado desde `src/ops`
(la raíz de la lista: el directorio actual, la raíz de Branch o la raíz del
árbol en Showall), copiado a `dest`:

| Modo | Resultado | |
|---|---|---|
| Full | `dest/home/me/proj/src/ops/mod.rs` | desde la raíz del disco, como en ZTreeWin (predeterminado) |
| Tree | `dest/src/ops/mod.rs` | desde la raíz del árbol de Xylem (aquí `proj`) |
| Current | `dest/ops/mod.rs` | desde la raíz de la lista, incluido su nombre |
| Relative | `dest/mod.rs` | solo lo que está por debajo de la raíz de la lista |

En la entrada, **F6** alterna entre con y sin rutas y **F7** recorre los modos;
ambos se muestran en la propia entrada. El modo inicial es `copy_paths` (página
4 de la [configuración](configuration.md)). Dentro de un archivo comprimido, las
rutas empiezan como máximo en la raíz del archivo comprimido.

## Cuando el archivo ya existe

Xylem pregunta **Sí** / **No** / **Todos** / **Omitir todos**, mostrando el
tamaño y la fecha del archivo nuevo y del existente, y si el existente es más
reciente o más antiguo, más grande o más pequeño. `Esc` cancela toda la
operación.

## Seguridad

La copia se hace en un archivo temporal que solo reemplaza al destino al final:
cancelar o un error nunca deja un archivo a medio escribir. Los permisos y la
fecha de modificación se conservan, y los enlaces simbólicos se copian como
enlaces. Mover entre discos distintos copia y luego elimina.

Véase también: [marcas](tagging.md), [archivos comprimidos](archives.md) (Copiar
extrae de ellos).
