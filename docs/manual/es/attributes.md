# Permisos y fechas

## Permisos (A)

`A` cambia los permisos del archivo bajo el cursor (Ctrl+A: de los archivos
marcados), en octal: `644`, `755`. Alt+A funciona como Ctrl+A.

En Windows, `A` edita los atributos R, H, S y A, como en ZTreeWin: `RA` deja
exactamente esos, `+H -A` activa o desactiva algunos. La lista de archivos los
muestra como `r-sa`. Véase [Windows](windows.md).

## Nueva fecha (N)

`N` cambia la fecha de modificación del archivo o directorio (Ctrl+N: de los
archivos marcados). La entrada aparece rellena con la fecha actual en tu
formato; también acepta `now` y `today`. Si solo se indica la fecha (sin
`HH:MM`), cada archivo conserva su propia hora del día. Los enlaces simbólicos
se omiten (cambiar la fecha cambiaría la del archivo al que apuntan).

El formato de fecha se configura en la página 1 de la
[configuración](configuration.md).
