# Informations

- **Info (Alt+I) :** type, taille, permissions, propriétaire et groupe, dates
  (modification, accès et changement d'état), inœud, liens et périphérique du
  fichier ou du répertoire ; pour un lien, sa cible ; pour un répertoire, aussi
  ce qui est lu dedans et dans sa branche.
- **Avail (`A` dans l'arborescence) :** les systèmes de fichiers montés avec
  l'espace total, utilisé, libre et le % utilisé, comme `df`.
- **Statistiques (`?`) :** ce que la fenêtre a lu, la liste, les fichiers
  marqués et le disque (point de montage, espace libre et utilisé) ; en dessous,
  la machine (système et noyau, processeur, durée de fonctionnement, charge,
  mémoire et swap, shell, terminal), le réseau (adresses de chaque interface,
  passerelle et DNS) et Xylem lui-même (version, PID, mémoire, exécutable et
  fichier de configuration).

## Tailles des répertoires (Alt+K)

Affiche à droite de chaque répertoire de l'arborescence la **T**aille (**S**ize,
total en K), le **N**ombre (**C**ount) de fichiers ou le **P**ourcentage
(**P**ercent) ; **A**ucun (**N**one) désactive l'affichage. Avant de choisir,
**A**ll ou **T**agged compte tous les fichiers ou seulement les fichiers
marqués, et **B**ranch ou **D**irectory additionne toute la branche ou seulement
le répertoire. Les répertoires non encore lus affichent `?`. Chaque fenêtre a
son propre réglage.

**Percent** indique quelle part du répertoire parent occupe chaque branche, avec
une barre : le plus grand répertoire parmi ses frères remplit la barre, et les
autres sont tracés en proportion, de sorte que celui qui prend de la place
ressorte. Sa couleur indique sa part du répertoire parent : jusqu'à 10 % (vert,
dans la plupart des thèmes), jusqu'à 40 % (ambre) ou plus (rouge). Il additionne
toujours des branches entières. Les fichiers du répertoire parent comptent dans
son total ; la somme des parts de ses sous-répertoires peut donc être inférieure
à 100 %. Pour trouver ce qui remplit un disque, lisez d'abord la branche avec
`*` : un répertoire non encore lu compte comme vide. Avec Google Drive ou S3
monté via rclone, c'est ainsi que l'on trouve ce qui les remplit : voir
[Stockage cloud](cloud.md). Pour un disque entier, la [vue d'utilisation du
disque](usage.md) (Alt+W) va plus loin : fichiers et dossiers ensemble, espace
sur le disque, liens matériels comptés une seule fois.
