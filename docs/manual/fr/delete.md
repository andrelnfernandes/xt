# Supprimer, corbeille et Prune

## Supprimer

`D` ou Del supprime le fichier sous le curseur (dans l'arborescence, le
répertoire, uniquement s'il est vide) ; Ctrl+D supprime les fichiers marqués.
Xylem demande toujours confirmation.

Avec `delete_to_trash = true` dans la [configuration](configuration.md), les
fichiers et les répertoires vides vont à la corbeille du système (FreeDesktop
Trash sous Linux, la Corbeille sous Windows) au lieu d'être effacés. Lors de la
confirmation, **T** les envoie à la corbeille et **P** les supprime
définitivement, quel que soit le réglage. **Shift+Del** supprime toujours
définitivement.

## Oops

**Oops** (`O` dans l'arborescence, ou Ctrl+Z dans les deux fenêtres) restaure à
son emplacement d'origine le dernier élément envoyé à la corbeille. Si rien n'a
été mis à la corbeille au cours de cette session, il propose l'élément le plus
récent de la corbeille du système.

## Prune (Alt+P dans l'arborescence)

Supprime le répertoire avec tout ce qu'il contient. Xylem lit toute la branche,
indique combien de fichiers, de répertoires et d'octets seront supprimés, et
vous demande de **taper PRUNE** (ou le nom du répertoire, ou Y, selon
`prune_confirmation`). Il supprime les fichiers puis les répertoires, du plus
profond au plus haut, avec une barre de progression et `Esc` pour annuler. Les
liens symboliques sont supprimés sans être suivis, et **les autres points de
montage situés dans la branche sont ignorés** (une clé USB montée dedans n'est
pas effacée). La racine de l'arborescence ne peut pas être élaguée. Prune
supprime toujours définitivement.
