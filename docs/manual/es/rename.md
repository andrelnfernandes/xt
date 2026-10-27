# Renombrar y renombrado por lotes

`R` renombra el archivo (en el árbol, el directorio) bajo el cursor: escribe el
nombre nuevo. Un nombre que ya exista se rechaza.

## Renombrado por lotes (Ctrl+R o Alt+R)

Renombra todos los archivos marcados con una máscara, numerados en el orden de
la lista:

- **Comodines:** `*.jpg` (cambia la extensión), `new_*.*`, `*_old.*`, `???.*`
  (conserva ese número de caracteres). `*.*` deja los nombres como están; `*.`
  quita la extensión.
- **Buscar y reemplazar:** `s/foo/bar/` reemplaza cada `foo` por `bar`;
  `s/foo/bar/i` ignora mayúsculas y minúsculas.
- **Contadores:** `%d`, `%02d`, `%03d`, `%04d`, `<#>`, `<01>`, `<001>`, `<0001>`
  numeran los archivos (1, 01, 001...): `photo_<001>.*` produce
  `photo_001.jpg`, `photo_002.jpg`...
- **Mayúsculas y minúsculas:** `<lower>`, `<upper>`, `<title>` (también en la
  parte del nombre o de la extensión: `*.<lower>`), o **F2** en la entrada para
  alternar minúsculas, MAYÚSCULAS y Tipo Título sobre el resultado.

Si la máscara diera el mismo nombre a dos archivos, no se renombra nada. Cuando
un nombre nuevo pertenece a un archivo que aún está pendiente de renombrarse,
Xylem pregunta antes de sobrescribirlo, como en
[Copiar y mover](copy-move.md).

Véase también: [marcas](tagging.md).
