# Doublons (Alt+Y)

Les fichiers au même contenu, quels que soient leurs noms et leurs dossiers, et
l'espace gaspillé par les copies. **Alt+Y** cherche dans la branche du répertoire
courant ; **D** dans la vue Utilisation cherche dans le dossier affiché.

Ce qui libère le plus vient en premier. Rien n'est marqué tant que vous ne le
demandez pas, et une copie de chaque groupe reste toujours : Xylem ne laisse pas
marquer la dernière.

## Choisir ce qui part

| Touche | Action |
|---|---|
| ↑ ↓ PgUp PgDn Home End | se déplacer |
| `T` ou Espace | marque ou démarque la copie |
| `K` | garde cette copie : marque les autres du groupe |
| `A` | dans chaque groupe, marque toutes sauf une ; `A` à nouveau change celle qui reste : la plus ancienne, la plus récente, le chemin le plus court |
| `F` | dans chaque groupe ayant une copie dans le dossier de celle-ci, garde celle-là |
| `U` | démarque tout |
| `D`, Suppr ou Ctrl+D | supprime les copies marquées, comme la commande Delete |
| `M` ou Ctrl+M | déplace les fichiers marqués |
| `G` | fichiers ou dossiers entiers |
| `V` ou Entrée | affiche le fichier |
| `E` | écrit la liste dans un fichier (jamais par-dessus un fichier existant) |
| `\` | ferme la vue, avec l'arbre sur ce dossier |
| F3 | relit les dossiers et compare à nouveau |
| Échap ou Alt+Y | ferme |

Avant de supprimer ou de déplacer, chaque groupe est revérifié : si une copie a
changé depuis l'analyse, ou si celle qui devait rester a disparu, rien de ce
groupe n'est touché.

## Dossiers entiers

**G** montre les dossiers au contenu strictement identique : mêmes noms, même
contenu de chaque fichier et mêmes liens. Un dossier n'est jamais appelé copie si
quelque chose en lui n'a pas été comparé : un dossier illisible, un autre disque
monté dedans, un fichier spécial ou un dossier ignoré.

## Comment les fichiers sont comparés

Seuls les fichiers dont la taille est partagée sont lus : d'abord les deux bouts,
puis le milieu, par parties, avec SHA-256. Cela se fait en arrière-plan et Échap
annule. Restent à l'écart les fichiers plus petits que `duplicates_min_size`
(1 Kio par défaut), les vides, les liens, les autres liens physiques d'un même
fichier, les autres disques et les dossiers de `duplicates_ignore` (`.git` et
`node_modules`).

Sur un dossier réseau, le total à lire est demandé d'abord. Sur une machine
distante ou dans une archive, ce n'est pas encore disponible.

## Dans le cloud (rclone)

Dans un dossier monté par `rclone mount`, Xylem demande à rclone les empreintes
que le service conserve déjà (`rclone lsjson --hash`) et ne télécharge rien.
rclone doit être dans le PATH ; sinon, ou si le distant ne donne pas d'empreintes,
Xylem explique pourquoi et demande avant de lire les fichiers par le montage. Les
dossiers entiers ne sont comparés que sur des disques locaux.

Sur Google Drive : un fichier dans deux dossiers est un seul fichier ; Docs,
Sheets et Slides n'ont pas d'empreinte ; ce que d'autres ont partagé apparaît avec
`s` et n'est pas marqué par « toutes sauf une » ; deux noms identiques dans un
dossier se règlent avec `rclone dedupe` ; et supprimer envoie à la corbeille du
Drive, que Xylem ne vide pas : il montre la commande `rclone cleanup gdrive:`.

L'empreinte du service est acceptée de confiance, et MD5 peut être falsifié exprès.
