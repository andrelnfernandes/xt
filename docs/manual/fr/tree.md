# La fenêtre d'arborescence

L'arborescence affiche les répertoires qui ont été lus (chargés en mémoire). La
fenêtre de fichiers située en dessous liste les fichiers du répertoire sous le
curseur.

## Se déplacer

| Touche | Action |
|---|---|
| ↑ ↓ PgUp PgDn Home End | déplacer le curseur |
| → ou `+` | déplier le répertoire |
| ← | le replier (ou aller au parent) |
| `-` | libérer la branche de la mémoire : ce qui a été lu en dessous est oublié, marques comprises, et le répertoire est relu |
| Backspace | aller au répertoire parent |
| Space | descendre d'une ligne |
| Shift+lettre, 0–9 | répertoire suivant commençant par ce caractère |
| `\` ou `/` | [Treespec](treespec.md) : aller à un répertoire en tapant son chemin |
| Enter ou Tab | aller à la fenêtre de fichiers (Tab change de fenêtre en [écran partagé](split.md)) |

Dans l'arborescence, un `+` sur le connecteur (`├─+`) signale un répertoire
replié contenant des sous-répertoires.

## Lecture (Logging)

Les répertoires sont lus à mesure que vous les ouvrez. `*` lit et déplie toute
la branche sous le curseur ; pendant une lecture longue, la progression
s'affiche dans le pied de page et `Esc` annule. **F3** (ou F2) relit le
répertoire depuis le disque. Xylem surveille aussi le répertoire courant de
chaque fenêtre et le relit automatiquement lorsqu'un autre programme le modifie
(au plus une fois par demi-seconde, après toute opération en cours) ; les
autres répertoires nécessitent toujours F3.

`L` ([Log](treespec.md)) ouvre un autre répertoire dans la fenêtre.

## Listes de fichiers

- `B` (Branch) : les fichiers de toute la branche, en la lisant d'abord.
- `S` (Showall) : les fichiers de tout ce qui a été lu.
- `F` ([Filespec](filespec.md)) : quels fichiers afficher, par nom, date et taille.

Dans Branch et Showall, `\` va au répertoire du fichier sélectionné.

## Commandes dans l'arborescence

| Touche | Action |
|---|---|
| `T` / `U` / `I` | marquer / démarquer / inverser les fichiers du répertoire ([marquage](tagging.md)) |
| `M` | créer un sous-répertoire |
| `R` | renommer le répertoire |
| `D` ou Del | supprimer le répertoire, s'il est vide ([supprimer](delete.md)) |
| Alt+P | Prune : supprimer le répertoire avec tout son contenu |
| `O` ou Ctrl+Z | Oops : restaurer ce qui a été envoyé à la corbeille |
| `N` | modifier la date du répertoire ([dates](attributes.md)) |
| `C` / Alt+C | [Comparer](compare.md) avec l'autre fenêtre / toute la branche |
| `A` | Avail : systèmes de fichiers montés ([informations](info.md)) |
| Alt+I | Info : détails du répertoire |
| Alt+K | tailles, nombres de fichiers ou pourcentages à côté de chaque répertoire |
| `Y` | copier le chemin du répertoire dans le presse-papiers |
| `X` | [eXécuter](execute.md) une commande shell |
| Alt+X | le [terminal intégré](terminal.md) |
| F8 | [écran partagé](split.md) |
| F11 / Alt+B | [signets](bookmarks.md) ; Ctrl+B en enregistre un |

Voir aussi : [la fenêtre de fichiers](files.md), [les touches en vigueur](keys.md).
