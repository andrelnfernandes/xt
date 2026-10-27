# Git

Git no está en XTree ni en ZTreeWin, así que viene desactivado: actívalo con F12
(página 1, "Comandos de Git") o con `git = true` bajo `[options]` en la
[configuración](configuration.md). Necesita el programa `git`.

## El menú de Git (Ctrl+G)

Ctrl+G abre el menú de Git en el pie de página; la siguiente letra elige:

- **S**tatus, **L**og, **D**iff (contra HEAD: preparado y no preparado juntos,
  con colores) y **B**lame se abren en el [visor](viewer.md), con búsqueda;
- **A**dd y **U**nstage;
- **C**ommit abre el propio editor de Git, igual que `git commit`.

Los comandos actúan sobre los archivos marcados de la lista o, si no hay
ninguno, sobre el archivo actual; en el árbol, sobre el directorio actual.
Status abarca siempre todo el repositorio. Para push, pull y otros comandos que
usan la red, usa [eJecutar](execute.md) (`X`), por ejemplo `git push`.

## Estado en la lista de archivos

La lista de archivos muestra el estado de cada archivo delante de su nombre, con
las dos letras de `git status --short`: `M ` preparado (staged), ` M` modificado
pero no preparado, `A ` añadido, `??` sin seguimiento, `R ` renombrado. Los
archivos sin cambios muestran espacios en blanco. El estado se vuelve a leer
tras las operaciones de Xylem, tras pulsar F3 y cada 10 segundos (para detectar
commits hechos en otra terminal).

## Diff en el visor

En el visor, **D** muestra qué ha cambiado en el archivo desde el último commit
(su `git diff HEAD`, con colores) y **D** de nuevo vuelve al archivo, en el
mismo punto. "Diff" aparece en el pie de página solo en los archivos con
cambios; **E** sigue editando el archivo y el diff se actualiza al volver.
