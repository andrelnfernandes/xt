# Marquage et opérations groupées

Les fichiers marqués (signalés par `■`) sont ceux sur lesquels agissent les
opérations groupées : les fichiers marqués **de la liste courante** (le
répertoire, ou la vue Branch ou Showall ouverte).

## Marquer

| Touche | Arborescence | Fenêtre de fichiers |
|---|---|---|
| `T` / `U` | marquer / démarquer les fichiers du répertoire | marquer / démarquer le fichier et descendre |
| Ctrl+T / Ctrl+U | marquer / démarquer tout ce qui est lu (et correspond au Filespec) | marquer / démarquer toute la liste |
| `I` | inverser les marques du répertoire | inverser les marques de la liste |
| Ctrl+F7 / F8 / F9 | marquer / démarquer / inverser les fichiers lus de la branche | |

## Opérations sur les fichiers marqués

| Touche | Action |
|---|---|
| Ctrl+C / Ctrl+M | les [copier / déplacer](copy-move.md) dans la destination |
| Alt+C / Alt+M | les copier / déplacer **en recréant leurs chemins** |
| Ctrl+D | les [supprimer](delete.md) |
| Ctrl+A | modifier leurs [permissions](attributes.md) |
| Ctrl+N | modifier leur [date](attributes.md) |
| Ctrl+R | les [renommer](rename.md) avec un masque |
| Ctrl+B | [Batch](execute.md) : un script ou une liste avec une ligne par fichier |
| Ctrl+F5 | les mettre dans une [archive](archives.md) |
| Ctrl+S / Ctrl+V | [y rechercher du texte / les afficher](search.md) |
| Ctrl+O | les ouvrir dans le programme par défaut |
| Ctrl+Y | copier leurs chemins dans le presse-papiers |
| Alt+O | les insérer dans le [terminal intégré](terminal.md) |

Dans eXecute, `%t` représente les fichiers marqués ([eXecute](execute.md)).

## Menus Ctrl et Alt (F9)

Comme dans ZTree, le pied de page n'affiche que les commandes à une seule
touche. **F9** passe au menu Ctrl, puis au menu Alt, et revient au menu normal.
Lorsque l'un de ces menus est ouvert, la lettre suivante compte comme
Ctrl+lettre ou Alt+lettre (`F9` `M` = Ctrl+M) ; `Esc` annule. Dans les
terminaux prenant en charge le protocole clavier kitty, il suffit de **maintenir
Ctrl ou Alt** pour afficher le menu correspondant.

**Ctrl+M :** dans la plupart des terminaux, Ctrl+M arrive comme Enter. Xylem
active le protocole clavier kitty lorsque le terminal le prend en charge (kitty,
WezTerm, foot, Ghostty, Alacritty récent, iTerm2), et Ctrl+M déplace alors les
fichiers marqués comme dans ZTree. Dans les terminaux basés sur VTE (GNOME
Terminal, Ptyxis, Terminator, Tilix), utilisez **F9 M** (ou Alt+M puis F6 à
l'invite).
