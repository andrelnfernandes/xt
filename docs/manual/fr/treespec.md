# Treespec et Log

## Treespec (`\` ou `/`)

Ouvre une invite contenant le chemin courant. À mesure que vous tapez, le
curseur de l'arborescence se déplace vers le premier répertoire commençant par
ce que vous avez saisi (insensible à la casse) :

- `/` ou `\` complète le nom et descend d'un niveau ;
- Tab et Shift+Tab complètent et passent au répertoire suivant ou précédent au
  même niveau ;
- Backspace remonte d'un niveau ;
- Enter valide et Esc revient là où vous étiez ;
- ↑ rappelle les chemins précédents (conservés entre les sessions).

Un chemin extérieur à l'arborescence devient un Log une fois validé. Dans Branch
et Showall, `\` va directement au répertoire du fichier sélectionné, en le
mettant en surbrillance.

## Log (L)

Ouvre un autre répertoire dans la fenêtre, en remplaçant l'arborescence (un
répertoire déjà présent dans l'arborescence courante y déplace simplement le
curseur). Une adresse de la forme `user@host:` ouvre une [machine
distante](remote.md). Le chemin peut être absolu, relatif au répertoire courant
ou commencer par `~` ; **F2** [le choisit dans une arborescence](picker.md) et ↑
rappelle les précédents. Les nouvelles fenêtres conservent l'[ordre de
tri](sort.md) et le mode d'affichage courants.

Voir aussi : [la fenêtre d'arborescence](tree.md), [signets](bookmarks.md).
