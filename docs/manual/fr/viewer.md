# La visionneuse

`V` ouvre le fichier dans la visionneuse (Autoview, F7, l'affiche à côté de la
liste). Les fichiers binaires s'ouvrent en hexadécimal. Pour les fichiers de
plus de 16 Mio, seul le début est affiché.

| Touche | Action |
|---|---|
| ↑ ↓ PgUp PgDn Home End | faire défiler |
| ← → | faire défiler horizontalement |
| `H` | texte ou hexadécimal |
| `C` | activer ou désactiver la coloration syntaxique |
| `W` | activer ou désactiver le retour à la ligne automatique |
| `#` ou `L` | activer ou désactiver les numéros de ligne (en mode texte) |
| `g` ou `:` | aller à une ligne (ou à un décalage d'octet en hexadécimal) |
| `E` | éditer le fichier ; de retour dans Xylem, la visionneuse affiche le nouveau contenu au même endroit (fichiers binaires uniquement avec un éditeur dans [`[editors]`](execute.md)) |
| `D` | avec Git activé : ce qui a changé depuis le dernier commit, et retour ([Git](git.md)) |
| `T` / `U` | marquer / démarquer le fichier |
| `y` ou Ctrl+Insert | copier le chemin du fichier dans le presse-papiers |
| Esc | fermer |

## Rechercher

Comme dans ZTreeWin : **F** cherche depuis le début, **B** à rebours depuis la
fin, **/** (ou `S`) depuis la page courante et **\\** à rebours depuis celle-ci ;
**Space** poursuit dans la même direction, **+** et **-** choisissent la
direction. L'occurrence trouvée est mise en surbrillance. À l'invite de
recherche, **F2** active ou désactive la sensibilité à la casse (en ignorant la
casse, les accents sont aussi pris en compte : `AÇÃO` trouve `ação`) et **F4**
bascule entre texte et hexadécimal (paires d'octets telles que `4d 5a`). Un `*`
au milieu du terme correspond à n'importe quelle suite de caractères sur la même
ligne (`error*disk`).

## Coloration syntaxique

Le code source et les scripts sont colorés dans environ 150 langages (Rust,
Python, shell, C, JavaScript, SQL, YAML, Markdown...), reconnus par leur
extension, leur nom (`Makefile`) ou leur première ligne (`#!/bin/bash`). Le
réglage par défaut est `syntax_highlight` (page 2 de la
[configuration](configuration.md)). Les couleurs restent lisibles sur l'arrière-plan
du thème actif, et seules les 20 000 premières lignes sont colorées, afin que
les très gros fichiers ne ralentissent jamais la visionneuse.

## Caractères de contrôle de direction

Certains caractères invisibles changent le sens d'écriture du texte et peuvent faire paraître un code différent de ce qu'il dit vraiment (Trojan Source, CVE-2021-42574). La visionneuse ne les cache jamais : chacun apparaît sous la forme d'un `·` rouge, et l'en-tête indique combien il y en a.

## Fichiers compressés

Les fichiers qui se terminent par `.gz`, `.xz`, `.zst` ou `.bz2` s'ouvrent décompressés, en texte. Xylem regarde les premiers octets du fichier, pas le nom, et n'ouvre qu'un seul niveau : un `.gz` dans un `.gz` reste compressé. L'en-tête affiche `gzip → N bytes` (le format et la taille après décompression).

La décompression a des limites. La visionneuse lit jusqu'à 16 Mio, prend au plus 3 s et utilise au plus 64 Mio de mémoire pour le dictionnaire xz ou la fenêtre zstd. Dans Autoview, les limites sont 1 Mio, 100 ms et 16 Mio. Quand la limite de sortie ou de temps est atteinte, l'en-tête indique `(showing first N bytes decompressed)`. Une note dit ce qui s'est passé :

- `decompression stopped after 3 s` (100 ms dans Autoview)
- `compressed data truncated or corrupt` (données compressées tronquées ou corrompues)
- `needs more than 64 MiB of memory to decompress` (16 Mio dans Autoview, avec ` (V to view)` quand les 64 Mio de V peuvent suffire)

La note disparaît à la première touche, mais l'en-tête garde le problème : `(truncated or corrupt)`, `(incomplete: …)` quand la limite de mémoire a arrêté la décompression en cours de route, ou `(not decompressed: …)` quand rien n'a pu être décompressé (les octets bruts sont alors affichés en hexadécimal).

Autoview affiche la note dans son titre. En mode hexadécimal (`H`), on voit les octets décompressés. Les couleurs suivent le nom sans l'extension de compression : `app.rs.gz` est coloré comme du Rust.

Pour un `.tar.gz` (ou `.tar.xz`...), la note dit `TAR archive: Alt+F5 opens it as a folder` (Alt+F5 l'ouvre comme un dossier).

Pour voir les octets compressés, désactivez `viewer_decompress` (**J** à la page 2 de la [configuration](configuration.md)).

Voir aussi : [rechercher dans les fichiers marqués](search.md).
