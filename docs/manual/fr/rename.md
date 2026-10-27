# Renommer et renommage par lot

`R` renomme le fichier (dans l'arborescence, le répertoire) sous le curseur :
tapez le nouveau nom. Un nom qui existe déjà est refusé.

## Renommage par lot (Ctrl+R ou Alt+R)

Renomme tous les fichiers marqués à l'aide d'un masque, numérotés dans l'ordre
de la liste :

- **Jokers :** `*.jpg` (change l'extension), `new_*.*`, `*_old.*`, `???.*`
  (garde ce nombre de caractères). `*.*` conserve les noms tels quels ; `*.`
  supprime l'extension.
- **Rechercher et remplacer :** `s/foo/bar/` remplace chaque `foo` par `bar` ;
  `s/foo/bar/i` ignore la casse.
- **Compteurs :** `%d`, `%02d`, `%03d`, `%04d`, `<#>`, `<01>`, `<001>`, `<0001>`
  numérotent les fichiers (1, 01, 001...) : `photo_<001>.*` donne
  `photo_001.jpg`, `photo_002.jpg`...
- **Casse :** `<lower>`, `<upper>`, `<title>` (aussi comme partie nom ou
  extension : `*.<lower>`), ou **F2** à l'invite pour faire défiler minuscules,
  MAJUSCULES et Casse De Titre sur le résultat.

Si le masque donnait le même nom à deux fichiers, rien n'est renommé. Lorsqu'un
nouveau nom appartient à un fichier qui doit encore être renommé, Xylem demande
confirmation avant d'écraser un fichier existant, comme dans [Copier et
Déplacer](copy-move.md).

Voir aussi : [marquage](tagging.md).
