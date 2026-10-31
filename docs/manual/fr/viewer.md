# La visionneuse

`V` ouvre le fichier dans la visionneuse (Autoview, F7, l'affiche à côté de la
liste). Les fichiers binaires s'ouvrent en hexadécimal. Pour les fichiers qui dépassent la limite de taille,
seul le début est affiché.
Les feuilles de calcul, CSV, DBF, SQLite et Parquet s'ouvrent en tableau (voir Mode tableau).

| Touche | Action |
|---|---|
| ↑ ↓ PgUp PgDn Home End | faire défiler |
| ← → | faire défiler horizontalement |
| `H` | mode suivant : texte et hexadécimal ; dans un fichier Markdown, rendu, texte et hexadécimal |
| `O` | encodage suivant (UTF-8, CP850, 1252, CP437, UTF-16) |
| `C` | activer ou désactiver la coloration syntaxique |
| `W` | activer ou désactiver le retour à la ligne automatique |
| `#` ou `L` | activer ou désactiver les numéros de ligne (en mode texte) |
| `g` ou `:` | aller à une ligne (un décalage d'octet en hexadécimal, une ligne de la grille en mode tableau) |
| `&` | n'affiche que les lignes qui correspondent, texte simple ou regex ([filtrer les lignes](filter.md)) ; en mode tableau, le filtre SQL |
| Ctrl+B | une ligne de sortie par enregistrement ou ligne affichée ([Batch dans la visionneuse](execute.md)) |
| `E` | éditer le fichier ; de retour dans Xylem, la visionneuse affiche le nouveau contenu au même endroit (fichiers binaires uniquement avec un éditeur dans [`[editors]`](execute.md)) |
| `D` | avec Git activé : ce qui a changé depuis le dernier commit, et retour ([Git](git.md)) |
| `T` | tableau ou texte, dans un fichier ouvert en tableau ; rendu ou source, dans un fichier Markdown |
| `U` | dans la vue des fichiers marqués (Ctrl+V) : démarquer le fichier |
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
du thème actif, et seules les 20 000 premières lignes sont colorées, et aucune
ligne de plus de 10 000 caractères, afin que les très gros fichiers ne
ralentissent jamais la visionneuse.

## Markdown rendu

Les fichiers `.md` et `.markdown` s'ouvrent rendus, aussi dans Autoview, dans les archives et compressés (`x.md.gz`), et le cadre indique `RENDERED`. `T` montre la source au même endroit, et revient ; `H` passe par le rendu, le texte et l'hexadécimal.

Vous voyez les titres, le gras, l'italique, le barré, le code, les blocs de code avec coloration syntaxique (`C` la coupe), les listes imbriquées et numérotées, les listes de tâches (`[x]`), les citations, les lignes horizontales et les tableaux. Rien n'est jamais téléchargé, et aucun lien n'ouvre quoi que ce soit :

- Un lien montre sa véritable adresse à côté du texte, comme dans `sûr <https://example.com>`, pour que le texte ne se fasse pas passer pour une autre adresse.
- Une image est une ligne de texte : `[image] légende <photo.png>`.
- Un diagramme Mermaid est un bloc de code étiqueté `mermaid`.
- Le HTML et les commentaires HTML restent visibles comme texte.
- Le bloc YAML au début d'un fichier est un bloc de code étiqueté `yaml`.

Le texte tient toujours dans la fenêtre, donc `W` ne fait rien ici ; ← → atteignent les colonnes d'un tableau trop large. La recherche regarde le texte à l'écran, adresses comprises, et trouve une phrase qui continue sur la rangée suivante. `g` et `#` comptent les lignes de la source. Ctrl+B agit sur les lignes de la source, toutes. `&` passe en mode texte et y filtre.

Un fichier de plus de 8 Mio s'ouvre en source, avec un avis. Le rendu s'arrête aussi à la limite de temps (3 s, 100 ms dans Autoview) ou à sa limite de mémoire, et l'en-tête indique alors `(rendered only the start)`. Un tableau montre au plus 256 colonnes. Les caractères de contrôle apparaissent en `·`, y compris ceux écrits en entités comme `&#27;`.

Markdown lui-même cache certaines choses : les définitions de liens que rien n'utilise et les cellules au-delà des colonnes de l'en-tête. Pour tout voir, `T` montre la source. L'italique et le barré dépendent du terminal et de sa police, ils ont donc aussi leur propre couleur.

## Caractères de contrôle de direction

Certains caractères invisibles changent le sens d'écriture du texte et peuvent faire paraître un code différent de ce qu'il dit vraiment (Trojan Source, CVE-2021-42574). La visionneuse ne les cache jamais : chacun apparaît sous la forme d'un `·` rouge, et l'en-tête indique combien il y en a.

## Encodage du texte

Xylem trouve l'encodage seul, dans cet ordre : d'abord il décompresse le fichier, puis cherche un BOM (UTF-8, UTF-16 LE ou BE), puis de l'UTF-16 sans BOM, puis de l'UTF-8. Un fichier qui n'est rien de tout cela est un vieux texte : Xylem choisit CP850 (DOS, Europe occidentale) ou Windows-1252 en comptant les lettres accentuées. Les exports `.reg` de Windows sont en UTF-16 et s'ouvrent comme du texte normal.

**O** affiche le fichier dans l'encodage suivant : UTF-8, CP850, Windows-1252, CP437, UTF-16 LE, UTF-16 BE, puis de nouveau UTF-8. Le pied de page dit lequel, et l'en-tête montre son nom quand ce n'est pas de l'UTF-8. Si vous choisissez l'UTF-16 pour un fichier ouvert en hexadécimal, il passe en texte. Votre choix reste après avoir modifié le fichier et être revenu.

Pour toujours utiliser un seul encodage pour les vieux textes, au lieu de la devinette, réglez `legacy_charset` sur `cp850`, `cp1252` ou `cp437` (**K** à la page 2 de la [configuration](configuration.md)). La valeur par défaut est `auto`.

## Limite de taille

La visionneuse ne lit que le début d'un gros fichier, et l'en-tête indique `(showing first N MiB)`. La quantité lue dépend de l'option `viewer_max_size` (**L** à la page 2 de la [configuration](configuration.md)) : `auto`, `16`, `64`, `256` ou `1024` (Mio). Avec `auto`, la valeur par défaut, la limite est 1/32 de la RAM de la machine, jamais moins de 16 Mio ni plus de 256 Mio (16 Mio si la RAM ne peut pas être lue). L'écran de configuration affiche la valeur utilisée, par exemple `auto (256 MiB)`.

Deux cas font exception. Autoview lit toujours 1 Mio. Un fichier sur une machine distante (SFTP) est lu jusqu'à 16 Mio au plus, car Échap ne peut pas annuler cette lecture.

Une limite plus grande coûte de la mémoire et du temps : avec `1024`, un fichier texte ordinaire de cette taille occupe environ 2,5 Gio de RAM et met environ 5 s à s'ouvrir. Les lignes très courtes coûtent plus que leurs octets, car chaque ligne a sa propre gestion, et il en va de même pour les tabulations et les caractères de contrôle de direction cachés : la visionneuse garde des lignes jusqu'à environ deux fois la limite de taille en mémoire et, au-delà, l'en-tête indique `(showing first N lines)`. Un journal de lignes ordinaires (environ 100 octets) n'est jamais coupé pour cette raison.

## Fichiers compressés

Les fichiers qui se terminent par `.gz`, `.xz`, `.zst` ou `.bz2` s'ouvrent décompressés, en texte. Xylem regarde les premiers octets du fichier, pas le nom, et n'ouvre qu'un seul niveau : un `.gz` dans un `.gz` reste compressé. L'en-tête affiche `gzip → N bytes` (le format et la taille après décompression).

La décompression a des limites. La visionneuse lit jusqu'à la limite de taille, prend au plus 3 s et utilise au plus 64 Mio de mémoire pour le dictionnaire xz ou la fenêtre zstd. Dans Autoview, les limites sont 1 Mio, 100 ms et 16 Mio. Quand la limite de sortie ou de temps est atteinte, l'en-tête indique `(showing first N bytes decompressed)`. Une note dit ce qui s'est passé :

- `decompression stopped after 3 s` (100 ms dans Autoview)
- `compressed data truncated or corrupt` (données compressées tronquées ou corrompues)
- `needs more than 64 MiB of memory to decompress` (16 Mio dans Autoview, avec ` (V to view)` quand les 64 Mio de V peuvent suffire)

La note disparaît à la première touche, mais l'en-tête garde le problème : `(truncated or corrupt)`, `(incomplete: …)` quand la limite de mémoire a arrêté la décompression en cours de route, ou `(not decompressed: …)` quand rien n'a pu être décompressé (les octets bruts sont alors affichés en hexadécimal).

Autoview affiche la note dans son titre. En mode hexadécimal (`H`), on voit les octets décompressés. Les couleurs suivent le nom sans l'extension de compression : `app.rs.gz` est coloré comme du Rust.

Pour un `.tar.gz` (ou `.tar.xz`...), la note dit `TAR archive: Alt+F5 opens it as a folder` (Alt+F5 l'ouvre comme un dossier).

Pour voir les octets compressés, désactivez `viewer_decompress` (**J** à la page 2 de la [configuration](configuration.md)).

## Mode tableau

Certains fichiers s'ouvrent sous forme de grille plutôt que de texte. Xylem se fie à l'extension, et seulement pour les fichiers non compressés de ce disque : un fichier d'une machine distante ou d'une archive s'affiche comme un contenu ordinaire, avec la note `Tables are shown only for files on this disk`.

| Format | Extensions | Noms des colonnes |
|---|---|---|
| CSV, TSV | `.csv` `.tsv` `.tab` | la première ligne, si au moins la moitié n'est pas numérique ; sinon A, B, C... |
| JSON Lines | `.jsonl` `.ndjson` | les clés des objets |
| Feuilles de calcul | `.xlsx` `.xls` `.xlsb` `.ods` | A, B, C... (la première ligne est une ligne comme les autres) |
| dBase, Clipper, FoxPro | `.dbf`, avec son fichier mémo `.dbt` | les noms des champs |
| SQLite | `.db` `.sqlite` `.sqlite3` | les colonnes de la table |
| Parquet | `.parquet` | les colonnes du fichier |

Le séparateur du CSV (virgule, point-virgule, tabulation ou barre verticale) est détecté d'après le début du fichier.

| Touche | Action |
|---|---|
| ↑ ↓ | ligne précédente ou suivante |
| ← → | colonne précédente ou suivante |
| PgUp PgDn | une page de lignes vers le haut ou vers le bas |
| Home End | première ou dernière colonne |
| `[` `]` | feuille précédente ou suivante d'un classeur, ou table d'une base SQLite, en boucle ; cela efface le filtre |
| `g` ou `:` | aller à une ligne, en ne comptant que celles gardées par le filtre |
| `\` ou `&` | le filtre SQL |
| `T` | aller au texte du fichier et revenir |
| `H` | du tableau au texte, puis à l'hexadécimal, puis retour au tableau |

Le cadre affiche `TABLE [2/5]` s'il y a plusieurs feuilles ou tables, et la ligne du bas la position du curseur : `Cell B7 (Row 7 of 120)`. La dernière ligne de la grille donne le contenu entier de la cellule et le filtre en cours. La recherche (`F`, `B`, `/`) ne regarde pas dans la grille et n'y trouve rien ; elle fonctionne en mode texte.

### Le filtre SQL

`\` ou `&` demande une condition, comme le `WHERE` de SQL (le mot `WHERE` est facultatif), et n'affiche que les lignes qui la remplissent. L'invite commence avec le filtre en cours ; laissez-la vide pour revoir toutes les lignes. Une note indique `Filter matched N rows` ou `SQL error: ...` ; après une erreur, le filtre précédent reste actif.

```
age >= 30 AND name LIKE 'Ali%'
WHERE status IN ('open', 'late') OR total > 1000.50
NOT (city = 'Rio' OR city IS NULL)
B ILIKE '%silva%'
```

| Opérateur | Action |
|---|---|
| `=` `!=` `<>` `<` `<=` `>` `>=` | comparent |
| `AND` `OR` `NOT` et parenthèses | combinent des conditions (`AND` passe avant `OR`) |
| `LIKE` `ILIKE` | correspondent à un motif, `%` pour n'importe quel texte et `_` pour un caractère ; `ILIKE` ignore la casse |
| `IN (...)` `NOT IN (...)` | est, ou n'est pas, dans la liste |
| `IS NULL` `IS NOT NULL` | la cellule est, ou n'est pas, vide |

Les mots-clés s'écrivent en majuscules ou en minuscules.

- **Colonnes :** le nom de la colonne, sans guillemets et quelle que soit la casse, ou sa lettre, de `A` à `Z`. La lettre permet d'atteindre un nom avec des espaces ou des symboles. Une colonne qui n'existe pas compte comme vide, sans erreur.
- **Valeurs :** du texte entre apostrophes ou guillemets (`\'` pour une apostrophe à l'intérieur), des nombres, `TRUE`, `FALSE` et `NULL`. Ce qui est entre guillemets est toujours du texte, jamais une colonne.
- **Cellules :** le type vient de l'apparence de la cellule. Vide, c'est `NULL`, `true` et `false` sont des booléens, les nombres sont des nombres et le reste est du texte. Écrivez donc `code = 7`, et non `code = '7'` : un texte et un nombre ne sont jamais égaux, et une cellule contenant `007` est le nombre 7. Les comparaisons de texte respectent la casse.
- **Taille :** un filtre compte au plus 1 024 octets et 128 termes.

### Limites

Un tableau contient au plus 50 000 lignes et 256 colonnes, et prend au plus ce que permettent les autres limites de la visionneuse : 3 s (100 ms dans Autoview), et un classeur qui se décompresse au-delà de la limite de taille (1 Mio dans Autoview) ne s'ouvre pas en tableau. Ce qui dépasse n'apparaît pas et le filtre ne le voit pas ; l'en-tête le signale : `(showing first 50,000 rows)` ou `(stopped by the time limit: showing N rows)`. Une cellule affiche au plus 2 048 caractères, puis `…`. Les caractères de contrôle et de direction du texte s'affichent comme `·`.

## Tableaux lus par des bibliothèques C

Les bases SQLite (`.db`, `.sqlite`, `.sqlite3`) et les fichiers Parquet sont lus par des bibliothèques écrites en C, qui n'ont pas la sûreté mémoire de Rust. Ils ne s'ouvrent donc en tableau qu'avec `V`. Autoview lit tout ce que le curseur survole et affiche ces fichiers comme un contenu ordinaire, avec la note `Parquet or SQLite: V opens the table`. N'ouvrez une base de source non fiable que si vous acceptez ce risque.

Voir aussi : [rechercher dans les fichiers marqués](search.md).
