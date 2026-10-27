# Almacenamiento en la nube (rclone)

Xylem no tiene un cliente propio para Google Drive ni S3, pero no lo necesita:
[rclone](https://rclone.org) los monta (junto con unos 70 servicios más:
OneDrive, Dropbox, Backblaze B2, Cloudflare R2, MinIO...) como una carpeta, y
Xylem trabaja sobre esa carpeta como sobre cualquier otra. Allí funciona todo:
registro de directorios, Branch, marcas, el visor, Copiar y Mover en ambas
direcciones, Renombrar, crear directorios y Eliminar.

## Por qué rclone

Elegimos trabajar con rclone en lugar de hablar directamente con Google Drive y
Amazon S3. Tener código propio para cada servicio sería un lugar más donde
cometer errores, y Google Drive traería además las políticas de Google: una
aplicación que lee todos los archivos de alguien requiere la verificación de
Google y una auditoría de seguridad anual, o de lo contrario muestra un aviso de
"aplicación no verificada" y queda limitada a unos pocos usuarios. Para S3, la
biblioteca oficial es muy grande y añadiría mucha complejidad, algo que Xylem,
por diseño, evita siempre que puede. rclone ya hace todo esto bien y para muchos
servicios a la vez.

## Configuración inicial (una sola vez)

Instala rclone (`apt install rclone`, `dnf install rclone`, `brew install
rclone`, o desde [rclone.org](https://rclone.org/downloads/)) y crea un remoto
con `rclone config`. Le das un nombre, que es lo que va antes de los dos puntos
más abajo.

- **Google Drive:** tipo de almacenamiento `drive`; las opciones predeterminadas
  valen. rclone abre el navegador para que inicies sesión con Google y des
  permiso. En una máquina sin navegador (por SSH), responde *no* a la pregunta
  del navegador: rclone muestra un comando para ejecutar con `rclone authorize`
  en una máquina que sí tenga navegador.
- **Amazon S3:** tipo de almacenamiento `s3`, proveedor `AWS`, y luego la clave
  de acceso y el secreto, o bien *env_auth* para usar lo que ya usan las
  herramientas de AWS (`~/.aws/credentials`, variables de entorno, el rol de la
  instancia). Otros servicios compatibles con S3 (R2, MinIO, Wasabi...) aparecen
  como otros proveedores en la misma lista.

`rclone lsd gdrive:` (o `rclone lsd s3:`) lista las carpetas superiores (los
buckets, en S3): si funciona, el remoto está listo.

## Montar y abrir

En Linux, con un remoto llamado `gdrive`:

```bash
mkdir -p ~/gdrive
rclone mount gdrive: ~/gdrive --vfs-cache-mode writes --daemon
xy ~/gdrive
```

Para S3, igual con `s3:` (todos los buckets) o `s3:mi-bucket` (uno de ellos).
`fusermount -u ~/gdrive` desmonta la carpeta.

- `--vfs-cache-mode writes` guarda una copia local de los archivos que se están
  escribiendo, algo que necesitan los editores (`E`) y algunos programas.
- `--dir-cache-time 1h` conserva las listas de directorios durante más tiempo
  que los 5 minutos predeterminados: volver a leer (F3) es más rápido, pero los
  cambios hechos desde otro lugar tardan más en aparecer.

En **macOS**, `rclone mount` necesita [macFUSE](https://osxfuse.github.io);
`rclone nfsmount gdrive: ~/gdrive` no necesita nada adicional. En **Windows**,
necesita [WinFsp](https://winfsp.dev) y se monta en una letra de unidad:
`rclone mount gdrive: X:`, y luego `xy X:\`.

Para tener la carpeta a mano, guárdala como [marcador](bookmarks.md) (Alt+B).

En Linux, Xylem detecta las carpetas montadas por rclone: el panel DISK muestra
el servicio (Google Drive, Amazon S3, OneDrive...) y, debajo, el remoto
(`rclone gdrive:`). Info (Alt+I) y Avail (`A`) también los muestran. El servicio
se obtiene del archivo de configuración de rclone
(`~/.config/rclone/rclone.conf` o `$RCLONE_CONFIG`); cuando está cifrado, el
panel indica simplemente rclone.

## Qué cambia allí

- **Todo va por la red.** Cada lectura de directorio es una petición: en un
  Drive o bucket grande, `*` (registrar toda la rama) y Branch (`B`) pueden
  tardar un rato. Abrir solo los directorios que necesitas (→ o `+`) es rápido,
  ya que se leen a medida que los abres.
- **Los cambios hechos en otro lugar** (en el navegador, en el teléfono) solo
  aparecen después de pulsar F3 y una vez que expira la caché de directorios
  (`--dir-cache-time`).
- **Eliminar:** responde `P` (permanentemente) en lugar de `T`: en Linux, la
  papelera de una carpeta montada es una carpeta `.Trash-` (con tu número de
  usuario) en su raíz, de modo que el archivo seguiría en el remoto ocupando
  espacio. Google Drive guarda igualmente en su propia papelera lo que rclone
  elimina; en S3 un archivo eliminado desaparece, a menos que el bucket tenga
  versionado.
- **Los permisos y propietarios** no se conservan: el montaje muestra los mismos
  para todos los archivos y Atributos no puede cambiarlos. Las fechas sí se
  conservan.
- **Google Docs, Sheets y Slides** no son archivos normales: rclone los muestra
  exportados (`.docx`, `.xlsx`, `.pptx`), con tamaño 0. `--drive-skip-gdocs` los
  oculta.
- **S3 no tiene directorios reales**, solo nombres con `/` en ellos: un
  directorio vacío puede desaparecer al desmontar, y renombrar o mover un
  directorio copia y elimina cada archivo que contiene.

## Encontrar qué llena tu Google Drive

Cuando Google Drive insiste en pedirte un plan de almacenamiento mayor, Xylem
puede mostrarte adónde se va el espacio antes de pagar por más. Monta el Drive,
ve a su raíz, regístralo todo con `*` (en un Drive grande lleva un rato) y elige
Alt+K → **Percent** ([Tamaños de directorios](info.md)): cada carpeta muestra
qué porcentaje de la carpeta superior ocupa, con una barra proporcional a la más
grande de sus carpetas hermanas. Sigue las barras más grandes hasta dar con las
copias de seguridad antiguas, los vídeos y los duplicados.

La [vista de uso](usage.md) (Alt+W, en la raíz del Drive) va más allá: muestra
los archivos y carpetas juntos, de mayor a menor, para que un único archivo
enorme destaque igual que una carpeta llena. Y Branch (`B`) lista todos los
archivos de una carpeta completa: [ordenados](sort.md) por tamaño de mayor a
menor (Alt+S, luego `O` para descendente y `S` para tamaño), los más grandes
quedan arriba del todo.

Algunas cosas consumen espacio del plan sin aparecer en el montaje:

- **La papelera del Drive** ocupa espacio hasta que se vacía (o pasan 30 días).
  `rclone about gdrive:` muestra el total, lo usado y lo que hay en la papelera;
  montar con `--drive-trashed-only` muestra únicamente lo que está en la
  papelera.
- **Gmail y Google Fotos** comparten el mismo almacenamiento, y en el montaje
  solo están los archivos de Drive. Si `rclone about` muestra mucho más espacio
  usado del que suma el montaje, es probable que el resto esté ahí.
- **Google Docs, Sheets y Slides** aparecen con tamaño 0, pero los más recientes
  sí cuentan para el almacenamiento.
- Los archivos **compartidos contigo** no cuentan: están en el almacenamiento de
  su propietario.
