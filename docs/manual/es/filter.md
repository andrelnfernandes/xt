# Filtrar líneas

En el modo texto del visor, `&` muestra solo las líneas que coinciden. Las
líneas quedan ocultas, no se modifican: con los números de línea activados (`#`),
cada línea conserva su número en el archivo, y los saltos muestran el filtro en
acción. La cabecera indica el resultado: `[filter error: 24 of 850 lines]`.

| Tecla en el prompt | Qué hace |
|---|---|
| **F1** | esta ayuda |
| **F2** | distinguir mayúsculas o ignorarlas |
| **F3** | invertir: conservar las líneas que no coinciden |
| **F4** | texto simple o expresión regular |
| ↑ ↓ | filtros anteriores |
| Enter | aplicar |
| Esc | cancelar |

Pulsa `&` otra vez para cambiar el filtro. **Enter con el prompt vacío lo
desactiva**, y la línea en lo alto de la pantalla sigue allí. Un filtro que no
encuentra nada o no es válido no cambia nada, y el pie indica el motivo.

Con un filtro activo, el desplazamiento y la [búsqueda](viewer.md) recorren solo
las líneas conservadas. `g` recibe un número de línea del archivo; si esa línea
está oculta, va a la siguiente conservada. El filtro sigue tras editar el archivo
(`E`) o cambiar la codificación (`O`), y termina al cerrar el visor.

Solo se filtra lo que el visor leyó: si la cabecera dice `(showing first N MiB)`,
el resto del archivo no se examina. El filtro ve las líneas tal como aparecen, así
que una tabulación son espacios (usa `\s`, no `\t`) y un carácter de control es
`·`. En el modo tabla, `&` abre el filtro SQL.

## Texto simple

Las mismas reglas que la búsqueda: al ignorar mayúsculas, los acentos tampoco
cuentan (`AÇÃO` encuentra `ação`), y un `*` en medio coincide con cualquier cosa
en la misma línea (`error*disco`).

## Expresiones regulares

El término puede coincidir en cualquier parte de la línea, salvo que lo ancles.
Las mayúsculas se ignoran salvo que pulses **F2**; aquí los acentos cuentan.

| Escribe | Para que coincida con |
|---|---|
| `.` | cualquier carácter |
| `\d` `\D` | un dígito, cualquier cosa menos un dígito |
| `\w` `\W` | letra, dígito o `_`, cualquier otra cosa |
| `\s` `\S` | un espacio, cualquier otra cosa |
| `[abc]` `[^abc]` | uno de estos, cualquier cosa menos estos |
| `[a-z0-9]` | uno en estos rangos |
| `^` `$` | el inicio, el final de la línea |
| `\b` | el borde de una palabra |
| `a*` `a+` `a?` | cero o más, uno o más, cero o uno |
| `a{3}` `a{2,5}` | exactamente 3, de 2 a 5 |
| `a+?` | uno o más, los menos posibles |
| `(abc)` | un grupo, para repetir o dar alternativas |
| `(?P<nombre>abc)` | un grupo con nombre |
| `(?i)` `(?-i)` | ignorar mayúsculas o distinguirlas desde aquí |

## Expresiones regulares

El término puede coincidir en cualquier parte de la línea, salvo que lo ancles.
Las mayúsculas se ignoran salvo que pulses **F2**; aquí los acentos cuentan.

| Escribe | Para que coincida con |
|---|---|
| `.` | cualquier carácter |
| `\d` `\D` | un dígito, cualquier cosa menos un dígito |
| `\w` `\W` | letra, dígito o `_`, cualquier otra cosa |
| `\s` `\S` | un espacio, cualquier otra cosa |
| `[abc]` `[^abc]` | uno de estos, cualquier cosa menos estos |
| `[a-z0-9]` | uno en estos rangos |
| `^` `$` | el inicio, el final de la línea |
| `\b` | el borde de una palabra |
| `a*` `a+` `a?` | cero o más, uno o más, cero o uno |
| `a{3}` `a{2,5}` | exactamente 3, de 2 a 5 |
| `a+?` | uno o más, los menos posibles |
| `(abc)` | un grupo, para repetir o dar alternativas |
| `(?P<nombre>abc)` | un grupo con nombre |
| `(?i)` `(?-i)` | ignorar mayúsculas o distinguirlas desde aquí |

La barra vertical separa alternativas: `error|fallo|fatal`, o
`disco (lleno|error)` dentro de un grupo.

Para coincidir con uno de los caracteres especiales `. * + ? ( ) [ ] { } ^ $ \` o
con la propia barra, pon una `\` delante: `\.log$`, `\(1\)`.

## Ejemplos

- `^\d{4}-\d{2}-\d{2}`: líneas que empiezan con una fecha
- `\b(error|fatal)\b`: estas palabras enteras, en mayúsculas o minúsculas
- `^\s*$` con **F3**: oculta las líneas en blanco
- `^\s*#` con **F3**: oculta los comentarios de un script de shell
- ` 50\d `: errores HTTP de 500 a 509 en un registro de acceso
- `\b\d{1,3}(\.\d{1,3}){3}\b`: líneas con una dirección IPv4
- `(?-i)TODO`: solo en mayúsculas, aunque se ignoren

Las referencias a grupos (`\1`) y el lookaround (`(?=...)`) no existen aquí, a
propósito: sin ellos, el tiempo crece solo con el tamaño del texto, y ningún
archivo ni expresión puede colgar el visor con un patrón como `(a+)+$`.

## Texto simple

Las mismas reglas que la búsqueda: al ignorar mayúsculas, los acentos tampoco
cuentan (`AÇÃO` encuentra `ação`), y un `*` en medio coincide con cualquier cosa
en la misma línea (`error*disco`).

## Con un filtro activo

Véase también: [el visor](viewer.md).
