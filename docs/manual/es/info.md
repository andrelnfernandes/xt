# Información

- **Info (Alt+I):** tipo, tamaño, permisos, propietario y grupo, fechas
  (modificación, acceso y cambio), inodo, enlaces y dispositivo del archivo o
  directorio; para un enlace, adónde apunta; para un directorio, también lo que
  hay registrado en él y en su rama.
- **Avail (`A` en el árbol):** los sistemas de archivos montados con total,
  usado, libre y % usado, como `df`.
- **Estadísticas (`?`):** lo que la ventana ha registrado, la lista, los
  archivos marcados y el disco (punto de montaje, libre y usado); debajo, la
  máquina (sistema y kernel, CPU, tiempo activo, carga, memoria y swap, shell,
  terminal), la red (direcciones de cada interfaz, puerta de enlace y DNS) y el
  propio Xylem (versión, PID, memoria, ejecutable y archivo de configuración).

## Tamaños de directorios (Alt+K)

Muestra a la derecha de cada directorio en el árbol el tamaño (**S**ize, total
en K), el recuento (**C**ount) de archivos o el porcentaje (**P**ercent);
**N**one lo desactiva. Antes de elegir, **A**ll o **T**agged cuentan todos los
archivos o solo los marcados, y **B**ranch o **D**irectory suman toda la rama o
solo el directorio. Los directorios aún no registrados muestran `?`. Cada
ventana tiene su propio ajuste.

**Percent** muestra qué parte del directorio superior ocupa cada rama, junto con
una barra: el directorio más grande entre sus hermanos llena la barra y los
demás se dibujan en proporción, de modo que el que ocupa el espacio destaca. Su
color indica qué proporción del directorio superior representa: hasta el 10%
(verde, en la mayoría de los temas), hasta el 40% (ámbar) o más (rojo). Siempre
suma ramas completas. Los archivos del directorio superior cuentan en su total,
por lo que las partes de sus subdirectorios pueden sumar menos del 100%. Para
encontrar qué llena un disco, registra primero la rama con `*`: un directorio
aún no registrado cuenta como vacío. Con Google Drive o S3 montados mediante
rclone, así es como se encuentra qué los llena: véase
[Almacenamiento en la nube](cloud.md). Para un disco entero, la
[vista de uso](usage.md) (Alt+W) hace más: archivos y carpetas juntos, el
espacio en el disco y los enlaces duros contados una sola vez.
