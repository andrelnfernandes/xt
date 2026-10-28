# Manuel de Xylem

Xylem est un gestionnaire de fichiers en mode texte inspiré de XTree et
ZTreeWin : une arborescence de répertoires en haut, les fichiers du répertoire
courant en bas et des statistiques à droite. Il lit (charge) les répertoires en
mémoire, ce qui permet de marquer des fichiers dans toute une branche et de les
copier, déplacer, supprimer, comparer ou renommer en une seule fois.

Xylem traite les fichiers non fiables comme hostiles : il ne doit jamais
planter, se figer, exécuter du code, écrire en dehors de la destination ni
épuiser la mémoire en raison du contenu ou du nom d'un fichier. Néanmoins, des
erreurs peuvent survenir. Consultez la déclaration de sécurité dans le README
pour la liste complète des protections et recommandations.

**F1** (ou **Alt+H**, pour les terminaux qui gardent F1 pour eux) ouvre l'aide
sur ce que vous faites : dans une invite, l'aide de cette invite ; dans
l'arborescence ou la fenêtre de fichiers, l'aide de cette fenêtre. Dans l'aide,
**Tab** et **Shift+Tab** choisissent un lien, **Entrée** le suit, **Retour
arrière** revient en arrière, **I** affiche cet index, **K** les touches en
vigueur, **L** change la langue et **Échap** ferme.

## Fenêtres et vues

- [La fenêtre d'arborescence](tree.md) : déplacements, lecture, Branch et Showall
- [La fenêtre de fichiers](files.md) : modes d'affichage, Autoview, ouverture de fichiers
- [Écran partagé](split.md) : deux fenêtres côte à côte
- [La visionneuse](viewer.md) : texte et hexadécimal, recherche, coloration syntaxique
- [Rechercher dans les fichiers marqués](search.md) : Ctrl+S et Ctrl+V
- [Informations](info.md) : Info, Avail, statistiques et tailles des répertoires
- [Utilisation du disque](usage.md) : où va l'espace (Alt+W)

## Travailler avec les fichiers

- [Marquage et opérations groupées](tagging.md)
- [Copier et Déplacer](copy-move.md), [choisir un dossier](picker.md)
- [Supprimer, corbeille et Prune](delete.md)
- [Renommer et renommage par lot](rename.md)
- [Permissions et dates](attributes.md)
- [Filespec : le filtre de fichiers](filespec.md)
- [Treespec et Log](treespec.md)
- [Comparer](compare.md) et [Trier](sort.md)
- [Éditer, eXécuter et Batch](execute.md)
- [Archives](archives.md) : ZIP, 7z et TAR

## Au-delà de XTree

- [Le terminal intégré](terminal.md)
- [Machines distantes (SSH)](remote.md) : parcourir une autre machine et son terminal
- [Stockage cloud (rclone)](cloud.md) : Google Drive, S3 et autres
- [Git](git.md)
- [Signets](bookmarks.md)

## Configuration

- [Configuration](configuration.md) : l'écran F12, thèmes, touches
- [Quitter et changer le répertoire du shell](quit.md)
- [Windows](windows.md)
- [Comparaison avec ZTreeWin](ztreewin.md)
- Les touches en vigueur : appuyez sur **K** dans l'aide.
