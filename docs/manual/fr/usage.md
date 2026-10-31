# Utilisation du disque (Alt+W)

Où va l'espace. **Alt+W** lit toute la branche du répertoire courant et liste
ses dossiers et fichiers selon l'espace qu'ils occupent, les plus grands en
premier, avec une barre et leur part du répertoire. Enter entre dans un
dossier ; de dossier en dossier, vous descendez jusqu'à ce qui remplit le
disque.

```
       32.9 GiB ████████████████   45%   wsl-ubuntu-backup.tar
       18.9 GiB █████████▏         26%  /projetos
 .      5.5 GiB ██▋               7.6%  /.local
 >    154.6 MiB                      -  /boot
```

- **La barre** est tracée par rapport au plus grand élément du répertoire, afin
  de faire ressortir les différences entre éléments sur une piste aussi large
  que le plus grand ; **le pourcentage** représente la part de l'élément dans le
  répertoire. La couleur indique cette part : jusqu'à 10 % (vert, dans la
  plupart des thèmes), jusqu'à 40 % (ambre), davantage (rouge). Chaque thème a
  ses propres couleurs (`bar_low`, `bar_mid` et `bar_high` dans `[colors]`).
- **Dossiers et fichiers sont mélangés**, par taille : un seul gros fichier (une
  ancienne sauvegarde, une image ISO) apparaît parmi les dossiers. Les dossiers
  commencent par `/`.
- **La ligne du bas** indique les totaux du répertoire : espace sur le disque,
  taille apparente, nombre de fichiers, ainsi que leur part de l'espace utilisé
  du disque et du disque entier. À la racine d'un disque, elle indique quelle
  part de l'espace utilisé a été trouvée : ce qui manque se trouve dans des
  dossiers qui n'ont pas pu être lus, dans les données propres au système de
  fichiers ou dans des fichiers supprimés encore ouverts.
- **Le panneau de droite** détaille l'élément sous le curseur : espace sur le
  disque et taille apparente, nombre de fichiers, date, et sa part de l'espace
  utilisé et du disque.

## Sur le disque ou taille apparente

Par défaut, les tailles correspondent à **l'espace sur le disque**, tel que le
compte `du` : un fichier occupe des blocs entiers (4 Kio, en général), si bien
qu'un petit fichier prend plus que sa taille, et qu'un fichier creux (sparse,
comme le disque d'une machine virtuelle) ou compressé prend moins. **A** bascule
sur la taille **apparente**, c'est-à-dire la taille propre des fichiers. La
ligne du bas affiche toujours les deux.

## Ce que compte l'analyse

- **Un disque à la fois.** Un autre système de fichiers monté à l'intérieur de
  la branche (un autre disque, `/boot`, une clé USB, un dossier monté par
  rclone, `/proc`) n'est ni lu ni compté : il apparaît avec `>` et l'espace que
  ce système de fichiers utilise, et Enter lance son analyse.
- **Les liens matériels ne comptent qu'une fois.** Deux noms désignant le même
  fichier n'occupent son espace qu'une seule fois : le premier rencontré est
  compté, les autres s'affichent avec `H` et sans taille.
- **Les fichiers cachés sont comptés**, même lorsque l'arborescence ne les
  affiche pas, afin que les totaux soient exacts.
- Les liens symboliques (`@`) comptent pour le lien lui-même ; ils ne sont pas
  suivis.

## Marques

| Marque | Signification |
|---|---|
| `>` | un autre disque, non compté |
| `!` | le dossier n'a pas pu être lu (généralement faute de permissions) |
| `.` | un élément à l'intérieur n'a pas pu être lu : le dossier peut occuper davantage |
| `H` | un autre lien matériel d'un fichier déjà compté |
| `@` | un lien symbolique ou un fichier spécial |
| `e` | un dossier vide |

## Touches

| Touche | Action |
|---|---|
| ↑ ↓ PgUp PgDn Home End | se déplacer |
| Enter ou → | entrer dans le dossier (sur `>` : analyser ce disque) |
| Backspace ou ← | remonter au dossier parent |
| `\` | fermer la vue en plaçant l'arborescence sur le dossier affiché |
| `A` | espace sur le disque ou taille apparente |
| `C` | afficher ou masquer le nombre de fichiers |
| `S` | trier par taille, nom, nombre de fichiers ou date |
| `D` | chercher les [doublons](duplicates.md) dans le dossier affiché |
| F3 | relire le dossier affiché |
| Esc ou Alt+W | fermer |

Tab, F8, Alt+X (le terminal) et F12 fonctionnent normalement lorsque la vue est
ouverte.

L'analyse d'un grand disque lit chaque dossier en mémoire : cela prend un
certain temps (le pied de page affiche la progression et Esc annule) et un peu
de mémoire, quelques centaines de mégaoctets pour des millions de fichiers. En
tant qu'utilisateur ordinaire, certains dossiers du système ne peuvent pas être
lus (`!`), si bien que les totaux de `/` sont inférieurs à l'espace utilisé : la
ligne du bas indique combien de dossiers n'ont pas pu être lus.
