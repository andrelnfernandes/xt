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
- **G**raph : `git log --graph --decorate --oneline --all`, les
  1000 derniers commits de toutes les branches, dans les couleurs de Git
  lui-même (une par ligne de branche), aussi dans la visionneuse ;
- **A**dd et **U**nstage ;
- **C**ommit ouvre l'éditeur habituel de Git, comme le fait `git commit`.

Les commandes agissent sur les fichiers marqués de la liste, ou à défaut sur le
fichier sous le curseur ; dans l'arborescence, sur le répertoire courant. Status
et Graph couvrent toujours l'ensemble du dépôt. Pour push, pull et les autres commandes qui
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

## Dépôts que vous n'avez pas créés

Le `.git` d'un dépôt peut nommer des commandes que Git exécutera, et un dossier téléchargé ou extrait arrive avec son `.git` (un `git clone` ne l'apporte pas). C'est pourquoi les marqueurs de fichiers et Status, Log, Graph, Diff et Blame n'en exécutent aucune : ni hooks, ni `core.fsmonitor`, ni diffs externes, ni conversions de texte, ni vérification de signatures. Ce que cela coûte :

- Un dépôt qui définit ses propres filtres, comme le fait `git-crypt`, n'a ici ni marqueurs, ni Status, ni Diff, ni Blame, avec un message qui explique pourquoi. Log et Graph fonctionnent. Les filtres de votre configuration globale, comme Git LFS, sont considérés comme fiables.
- Un sous-module avec des fichiers modifiés, mais sans nouveau commit, n'apparaît pas comme modifié.

Add, Unstage et Commit sont des demandes de votre part et suivent la configuration du dépôt : ils peuvent exécuter ce qu'elle nomme.

### Dossiers de confiance

Par défaut, rien n'est de confiance : tout dépôt est traité comme hostile. Pour que les dépôts d'un dossier fonctionnent avec toute leur configuration, comme `git-crypt` ou votre propre outil de diff, ajoutez le dossier dans **Trusted Git folders** (**K** à la page 1 de **F12**), avec `;` entre les dossiers :

```
~/projets; /travail/code
```

Dans le fichier, c'est une liste : `git_trusted = ["~/projets", "/travail/code"]`. Un dossier est un chemin complet ou sous `~`, et il couvre tout ce qu'il contient. Un lien qui pointe vers l'extérieur n'emporte pas la confiance avec lui.

**Danger :** dans un dossier de confiance, il suffit d'entrer dans un dépôt pour que Git exécute ce qu'il nomme, avec vos permissions. N'y extrayez ni n'y téléchargez jamais rien venant d'autres personnes, et ne faites pas confiance à `~` ni à `/`, qui contiennent ce que vous téléchargez.
