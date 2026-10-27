# Filespec: el filtro de archivos

`F` define qué archivos se muestran en la ventana de archivos (cada ventana
tiene el suyo). El filtro actual aparece en el título de la ventana. Las
especificaciones se separan por espacios, `;` o `,`, y son de tres tipos, como
en ZTreeWin:

- **nombre:** un patrón glob (`*.rs`, `a?c*`, `[abc]*`), sin distinguir
  mayúsculas de minúsculas; entre comillas, un nombre puede contener espacios
  (`"mi archivo*"`);
- **fecha:** `<`, `=`, `>`, `<=`, `>=` o `<>` y una fecha en el orden del
  formato configurado, con `/`, `.` o `-` y un año de 2 o 4 dígitos
  (`>01/05/2026`; `2026-05-01` funciona siempre), o `TODAY` y `TODAY-n`
  (`>TODAY-7`: los últimos 7 días);
- **tamaño:** igual, con una `s` después del operador (`>=s200000`); `K`, `M` y
  `G` también funcionan (`>s100M`).

Un `-` al principio niega (`-*.bak`; `-<s1K` es lo mismo que `>=s1K`). Dentro de
cada tipo, las especificaciones positivas se combinan con OR, salvo `>a` seguida
inmediatamente de `<b`, que es el intervalo entre ambas
(`>=01/05/2026,<=31/05/2026`: mayo); `<>` excluye. Entre tipos distintos se
aplica AND: `*.txt,=TODAY,<s1000` son los archivos `.txt` de hoy de menos de
1000 bytes.

`*` (o una entrada vacía) muestra todo. **↑** recupera filtros anteriores.

Véase también: [la ventana de archivos](files.md), [marcas](tagging.md) (Ctrl+T
en el árbol marca lo que coincide con el filtro).
