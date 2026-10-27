# Copier et Déplacer

`C` copie et `M` déplace le fichier sous le curseur ; Ctrl+C et Ctrl+M les
fichiers marqués ; Alt+C et Alt+M les fichiers marqués en recréant leurs
chemins. L'invite demande la destination.

## La destination

Un répertoire (relatif au répertoire sous le curseur de l'arborescence, absolu,
ou avec `~`) ou, pour un seul fichier, un nouveau nom. Avec l'écran
[partagé](split.md), le répertoire de l'autre fenêtre est proposé. **F2** ouvre
une arborescence pour [choisir le dossier](picker.md) ; **↑** rappelle les
destinations précédentes.

Un dossier de destination qui n'existe pas encore est créé après confirmation.

## Avec ou sans chemins

Ctrl+C / Ctrl+M placent tous les fichiers marqués directement dans la
destination. Alt+C / Alt+M y recréent le chemin de chaque fichier ; le point de
départ de ce chemin dépend du mode. Pour `/home/me/proj/src/ops/mod.rs`, listé
depuis `src/ops` (la racine de la liste : le répertoire courant, la racine de
Branch ou la racine de l'arborescence dans Showall), copié vers `dest` :

| Mode | Résultat | |
|---|---|---|
| Full | `dest/home/me/proj/src/ops/mod.rs` | depuis la racine du disque, comme dans ZTreeWin (par défaut) |
| Tree | `dest/src/ops/mod.rs` | depuis la racine de l'arborescence de Xylem (ici `proj`) |
| Current | `dest/ops/mod.rs` | depuis la racine de la liste, son nom inclus |
| Relative | `dest/mod.rs` | uniquement ce qui se trouve sous la racine de la liste |

À l'invite, **F6** bascule entre avec et sans chemins et **F7** fait défiler les
modes ; tous deux s'affichent dans l'invite elle-même. Le mode initial est
`copy_paths` (page 4 de la [configuration](configuration.md)). Dans une archive,
les chemins commencent au plus haut à la racine de l'archive.

## Lorsque le fichier existe déjà

Xylem demande **Yes** / **No** / **All** / **Skip all**, en indiquant la taille
et la date du nouveau fichier et du fichier existant, ainsi que si le fichier
existant est plus récent ou plus ancien, plus grand ou plus petit. `Esc` annule
toute l'opération.

## Sécurité

La copie est effectuée vers un fichier temporaire qui ne remplace la destination
qu'à la fin : une annulation ou une erreur ne laisse jamais de fichier à moitié
écrit. Les permissions et la date de modification sont conservées, et les liens
symboliques sont copiés en tant que liens. Un déplacement entre disques copie
puis supprime.

Voir aussi : [marquage](tagging.md), [archives](archives.md) (Copier en extrait
les fichiers).
