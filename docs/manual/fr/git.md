# Git

Git n'existe ni dans XTree ni dans ZTreeWin ; il est donc désactivé par défaut :
activez-le avec F12 (page 1, « Git commands ») ou avec `git = true` sous
`[options]` dans la [configuration](configuration.md). Le programme `git` doit
être installé.

## Le menu Git (Ctrl+G)

Ctrl+G ouvre le menu Git dans le pied de page ; la lettre suivante choisit :

- **S**tatus, **L**og, **D**iff (par rapport à HEAD : modifications indexées et
  non indexées ensemble, en couleurs) et **B**lame s'ouvrent dans la
  [visionneuse](viewer.md), avec recherche ;
- **A**dd et **U**nstage ;
- **C**ommit ouvre l'éditeur habituel de Git, comme le fait `git commit`.

Les commandes agissent sur les fichiers marqués de la liste, ou à défaut sur le
fichier sous le curseur ; dans l'arborescence, sur le répertoire courant. Status
couvre toujours l'ensemble du dépôt. Pour push, pull et les autres commandes qui
utilisent le réseau, utilisez [eXécuter](execute.md) (`X`), par exemple
`git push`.

## État dans la liste des fichiers

La liste des fichiers affiche l'état de chaque fichier devant son nom, avec les
deux lettres de `git status --short` : `M ` indexé, ` M` modifié mais non
indexé, `A ` ajouté, `??` non suivi, `R ` renommé. Les fichiers inchangés
affichent des espaces. L'état est relu après les opérations de Xylem, après F3,
et toutes les 10 secondes (pour détecter les commits faits dans un autre
terminal).

## Diff dans la visionneuse

Dans la visionneuse, **D** affiche ce qui a changé dans le fichier depuis le
dernier commit (son `git diff HEAD`, en couleurs) et **D** à nouveau revient au
fichier, au même endroit. « Diff » n'apparaît dans le pied de page que pour les
fichiers modifiés ; **E** édite toujours le fichier, et le diff est mis à jour
au retour.
