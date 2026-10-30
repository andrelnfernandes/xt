# Git

Git no está en XTree ni en ZTreeWin, así que viene desactivado: actívalo con F12
(página 1, "Comandos de Git") o con `git = true` bajo `[options]` en la
[configuración](configuration.md). Necesita el programa `git`.

## El menú de Git (Ctrl+G)

Ctrl+G abre el menú de Git en el pie de página; la siguiente letra elige:

- **S**tatus, **L**og, **D**iff (contra HEAD: preparado y no preparado juntos,
  con colores) y **B**lame se abren en el [visor](viewer.md), con búsqueda;
- **G**raph: `git log --graph --decorate --oneline --all`, los
  últimos 1000 commits de todas las ramas, con los colores del propio Git (uno
  por línea de rama), también en el visor;
- **A**dd y **U**nstage;
- **C**ommit abre el propio editor de Git, igual que `git commit`.

Los comandos actúan sobre los archivos marcados de la lista o, si no hay
ninguno, sobre el archivo actual; en el árbol, sobre el directorio actual.
Status y Graph abarcan siempre todo el repositorio. Para push, pull y otros comandos que
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

## Repositorios que no creaste

El `.git` de un repositorio puede nombrar comandos para que Git los ejecute, y una carpeta descargada o extraída viene con su `.git` (un `git clone` no lo trae). Por eso, los marcadores de archivo y Status, Log, Graph, Diff y Blame no ejecutan ninguno: ni ganchos, `core.fsmonitor`, diffs externos, conversiones de texto o verificación de firmas. El coste:

- Un repositorio que define filtros propios, como hace `git-crypt`, se queda sin marcadores, Status, Diff y Blame aquí, con un mensaje que explica el motivo. Log y Graph funcionan. Los filtros de tu configuración global, como Git LFS, son de confianza.
- Un submódulo con archivos modificados, pero sin commit nuevo, no aparece como modificado.

Add, Unstage y Commit son peticiones tuyas y siguen la configuración del repositorio: pueden ejecutar lo que ella nombra.

### Carpetas de confianza

Por defecto, nada es de confianza: todo repositorio se trata como hostil. Para que los repositorios de una carpeta funcionen con toda su configuración, como `git-crypt` o una herramienta de diff tuya, añade la carpeta en **Trusted Git folders** (**K** en la página 1 de **F12**), con `;` entre carpetas:

```
~/proyectos; /trabajo/codigo
```

En el archivo, es una lista: `git_trusted = ["~/proyectos", "/trabajo/codigo"]`. Una carpeta es una ruta completa o bajo `~`, y vale para todo lo que contiene. Un enlace que apunta fuera no lleva la confianza consigo.

**Peligro:** dentro de una carpeta de confianza, basta con entrar en un repositorio para que Git ejecute lo que este nombra, con tus permisos. Nunca extraigas ni descargues ahí nada de otras personas, y no confíes en `~` ni en `/`, que contienen lo que descargas.
