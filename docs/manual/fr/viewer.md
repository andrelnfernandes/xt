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

Voir aussi : [rechercher dans les fichiers marqués](search.md).
