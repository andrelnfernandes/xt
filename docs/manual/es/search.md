# Buscar en los archivos marcados

## Ctrl+S

Busca texto en todos los archivos marcados de la lista y **desmarca los que no
lo contienen**, con barra de progreso y `Esc` para detener. En la entrada,
**F2** alterna la distinción de mayúsculas y minúsculas (al ignorarlas, los
acentos también se equiparan: `AÇÃO` encuentra `ação`) y **F4** alterna entre
texto y hexadecimal (pares de bytes como `4d 5a`). Un `*` en mitad del término
coincide con cualquier cosa en la misma línea (`error*disk`); un término nunca
cruza un salto de línea. También funciona dentro de
[archivos comprimidos](archives.md).

## Ctrl+V

Abre los archivos marcados en el [visor](viewer.md), uno tras otro: **N**ext y
**P**rev cambian de archivo (Alt+Home y Alt+End: primero y último) y **U**
desmarca el archivo actual. El término se comparte con Ctrl+S: después de un
Ctrl+S, **Space** en Ctrl+V va directamente a la siguiente coincidencia, pasando
al siguiente archivo marcado cuando se acaba el archivo actual.

Un recorrido habitual: marcar archivos (`T`, o Ctrl+T en un Branch), buscar un
término con Ctrl+S, abrir Ctrl+V y pulsar Space para recorrer las coincidencias
entre los archivos.
