# Windows

Xylem también compila para Windows (Windows Terminal o la consola clásica). Allí
pasa las pruebas automatizadas, pero aún no se ha usado en el día a día, así que
cabe esperar detalles por pulir. Qué cambia:

- **Atributos** (`A`) edita R, H, S y A, como en ZTreeWin: `RA` deja exactamente
  esos, `+H -A` activa o desactiva algunos. La lista de archivos los muestra
  como `r-sa`.
- Los archivos y directorios **ocultos** son los que tienen el atributo H (un
  nombre que empieza por un punto es un nombre normal).
- **eJecutar** y **Editar** pasan por `cmd`, con los nombres entre comillas
  dobles. Un `%NOMBRE%` dentro de un nombre de archivo sigue siendo expandido
  por `cmd`.
- El editor predeterminado es `notepad`, `O` usa `start` y Ctrl+B sugiere
  `xy-batch.bat` (empezando por `@echo off`).
- Eliminar un archivo de solo lectura lo elimina, igual que hace `rm` en Linux.
- Copiar enlaces simbólicos requiere el Modo para desarrolladores o permisos de
  administrador.
- El modo de visualización de propietario (Alt+F) muestra `-`.
- Configuración en `%APPDATA%\xylem\config.toml`, historial en
  `%LOCALAPPDATA%\xylem\`.
- La terminal integrada (Alt+X) ejecuta `cmd` (`%COMSPEC%`). Entrar en ella
  sigue llevando la shell al directorio del árbol, pero el árbol no sigue los
  `cd` de la shell y no existe `$XY_TAGGED`.

Para Alt+Q en PowerShell, véase [salir](quit.md).
