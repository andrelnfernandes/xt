# Comparar

`C` en el árbol, con la pantalla [dividida](split.md), compara los archivos del
directorio actual con los del directorio de la otra ventana y marca, según
elijas:

- **U**nique (únicos): no están en el otro lado;
- **N**ewer (más recientes) y **O**lder (más antiguos);
- **I**dentical (idénticos): mismo tamaño y fecha;
- **D**ifferent (diferentes);
- **A**ll new (todos los nuevos): únicos + más recientes, lo que falta para
  poner al día el otro lado.

**B** (o Alt+C en el árbol, como en ZTreeWin) cambia el alcance a toda la rama:
Xylem registra la rama, compara siguiendo las rutas relativas (`src/a.rs` ↔
`<otro>/src/a.rs`) y abre Branch con los archivos marcados.

Las fechas con hasta 2 segundos de diferencia cuentan como iguales (FAT y exFAT
guardan la hora en pasos de 2 segundos); `compare_ignore_1h` también ignora
diferencias de exactamente una hora (horario de verano). Ambos ajustes están en
la página 4 de la [configuración](configuration.md).

Para sincronizar: `C` `B` `A` y luego Alt+C ([copiar con rutas](copy-move.md)),
que ya sugiere la otra ventana como destino.
