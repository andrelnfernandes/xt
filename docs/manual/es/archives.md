# Archivos comprimidos

## Abrir un archivo comprimido (Alt+F5)

Alt+F5 en la ventana de archivos abre, como un árbol y en modo de solo lectura:

- `.zip`, además de `.jar`, `.war` y `.apk`;
- `.7z` (no se admiten archivos 7z cifrados);
- `.tar`, sin comprimir o comprimido con gzip (`.tar.gz`, `.tgz`), xz
  (`.tar.xz`, `.txz`), bzip2 (`.tar.bz2`, `.tbz2`) o zstd (`.tar.zst`).

La navegación, [Filespec](filespec.md), Branch, Showall, las marcas,
[el visor](viewer.md), [Ctrl+S](search.md) y [Comparar](compare.md) funcionan
dentro de él, y [Copiar](copy-move.md) (también Ctrl+C y Alt+C con rutas)
extrae los archivos. `Esc` en el árbol cierra el archivo comprimido y vuelve
adonde estabas.

Por seguridad, se ignoran las entradas con `..` o con ruta absoluta, y los
enlaces simbólicos dentro de archivos comprimidos nunca se extraen.

## Crear un archivo comprimido (F5 / Ctrl+F5)

Crea un `.zip`, `.tar`, `.tar.gz` o `.tgz` (según el nombre que escribas) con el
archivo actual o los archivos marcados; F2 [elige la carpeta](picker.md). Las
rutas internas son relativas a la raíz de la lista, igual que con Alt+C: en un
Branch se conservan las subcarpetas. Muestra el progreso y `Esc` cancela; el
archivo se escribe en un archivo temporal y solo recibe su nombre al final. Los
enlaces simbólicos se guardan como enlaces. Un archivo existente no se
sobrescribe.
