# Archives

## Ouvrir une archive (Alt+F5)

Alt+F5 dans la fenêtre de fichiers ouvre, sous forme d'arborescence et en
lecture seule :

- `.zip`, ainsi que `.jar`, `.war` et `.apk` ;
- `.7z` (les archives 7z chiffrées ne sont pas prises en charge) ;
- `.tar`, non compressé ou compressé avec gzip (`.tar.gz`, `.tgz`), xz
  (`.tar.xz`, `.txz`), bzip2 (`.tar.bz2`, `.tbz2`) ou zstd (`.tar.zst`).

La navigation, [Filespec](filespec.md), Branch, Showall, le marquage, la
[visionneuse](viewer.md), [Ctrl+S](search.md) et [Comparer](compare.md) y
fonctionnent, et [Copier](copy-move.md) (Ctrl+C et Alt+C avec chemins également)
en extrait les fichiers. `Esc` dans l'arborescence ferme l'archive et revient là
où vous étiez.

Par sécurité, les entrées contenant `..` ou un chemin absolu sont ignorées, et
les liens symboliques présents dans les archives ne sont jamais extraits.

## Créer une archive (F5 / Ctrl+F5)

Crée une archive `.zip`, `.tar`, `.tar.gz` ou `.tgz` (selon le nom que vous
tapez) avec le fichier courant ou les fichiers marqués ; F2 [choisit le
dossier](picker.md). Les chemins à l'intérieur sont relatifs à la racine de la
liste, comme avec Alt+C : dans une vue Branch, les sous-dossiers sont conservés.
Une barre de progression s'affiche et `Esc` annule ; l'archive est écrite dans
un fichier temporaire et ne reçoit son nom définitif qu'à la fin. Les liens
symboliques sont stockés en tant que liens. Un fichier existant n'est pas
écrasé.
