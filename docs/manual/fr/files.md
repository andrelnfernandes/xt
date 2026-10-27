# La fenêtre de fichiers

La fenêtre de fichiers liste les fichiers du répertoire sous le curseur de
l'arborescence, ou de toute une branche (`B`), ou de tout ce qui est lu (`S`).
**Enter** revient à l'arborescence, **Esc** aussi (sur le répertoire du fichier
sélectionné).

## Se déplacer

| Touche | Action |
|---|---|
| ↑ ↓ PgUp PgDn Home End | déplacer le curseur |
| ← → | page précédente et suivante (dans les modes en colonnes : fichier précédent et suivant) |
| Backspace | afficher les fichiers du répertoire parent |
| Shift+lettre, 0–9 | fichier suivant commençant par ce caractère |
| Space | descendre d'une ligne |

## Modes d'affichage (Alt+F)

Alt+F fait défiler, dans l'ordre de ZTreeWin, **noms sur plusieurs colonnes**,
**noms et tailles en colonnes**, **détails** (une colonne avec taille, date et
permissions, le mode par défaut) et **noms longs** (une colonne, noms complets
uniquement).

Avant les noms longs viennent deux modes que ZTreeWin n'a pas :

- **propriétaire** : taille, `utilisateur:groupe` et permissions (non affiché
  sous Windows ou dans les archives) ;
- **type** : la nature de chaque fichier, dans l'esprit de la commande `file`
  (« PNG image, 800 x 600 », « ELF 64-bit executable », « python3 script »,
  « Rust, UTF-8 text »...), d'après les premiers octets des fichiers visibles.

Dans les modes en colonnes, les fichiers se suivent de gauche à droite, ← →
avancent d'un fichier et ↑ ↓ d'une ligne ; les noms de plus de 20 caractères
sont tronqués avec `…`. Le mode initial est `file_display` dans la
[configuration](configuration.md).

## Autoview (F7)

Partage l'écran entre la liste et le contenu du fichier sélectionné, qui suit
le curseur. Les touches de la fenêtre de fichiers continuent de fonctionner (`T`
marque, par exemple) ; Shift+↑ ↓ PgUp PgDn Home End font défiler le contenu ;
`W` active ou désactive le retour à la ligne automatique, `#` les numéros de
ligne ; F7, Esc ou Enter quittent. Il affiche le premier 1 Mio de chaque fichier
(`V` affiche jusqu'à 16 Mio).

## Ouvrir des fichiers

- `V` : la [visionneuse](viewer.md) (texte ou hexadécimal, avec recherche).
- `E` : l'éditeur ([Éditer](execute.md)).
- `O` : le programme par défaut du système (`xdg-open` : les images dans la
  visionneuse d'images, les PDF dans le lecteur PDF...), en arrière-plan.
  Ctrl+O ouvre tous les fichiers marqués. Dans une archive, l'entrée est
  d'abord extraite vers un dossier temporaire, supprimé à la fermeture de
  Xylem. Un autre programme peut être défini dans `opener`.
- Alt+F5 : ouvrir une [archive](archives.md) comme un répertoire.

## Caractères de direction cachés

Certains caractères invisibles changent le sens d'écriture du texte, si bien qu'un nom peut paraître différent de ce qu'il est vraiment (Trojan Source, CVE-2021-42574 ; par exemple, `fatura\u{202E}fdp.sh` peut ressembler à un PDF inoffensif). Xylem ne les envoie jamais au terminal : dans le nom, ils apparaissent marqués en rouge, le pied de page avertit « Name has hidden bidirectional characters » quand le curseur est sur un tel nom, et `O` demande confirmation avant d'ouvrir, en montrant le nom avec les caractères écrits sous la forme `\u{XXXX}`.

## Le presse-papiers

`Y` ou Ctrl+Insert copie le chemin du fichier (dans l'arborescence, celui du
répertoire) dans le presse-papiers ; Ctrl+Y copie les chemins des fichiers
marqués, un par ligne ; `y` dans la visionneuse copie le chemin du fichier
consulté. Cela fonctionne via SSH et tmux (OSC 52) et conserve le presse-papiers
après la fermeture de Xylem, sous X11 et Wayland.

## Autres commandes

| Touche | Action |
|---|---|
| `T` / `U` | marquer / démarquer le fichier et descendre ([marquage](tagging.md)) |
| `C` / `M` | [copier / déplacer](copy-move.md) le fichier |
| `D` ou Del | le [supprimer](delete.md) (Shift+Del : définitivement) |
| `R` | le [renommer](rename.md) ; Ctrl+R renomme les fichiers marqués avec un masque |
| `A` | [permissions](attributes.md) |
| `N` | [modifier la date](attributes.md) |
| Alt+S | [trier](sort.md) |
| Ctrl+S / Ctrl+V | [rechercher dans les fichiers marqués / les voir](search.md) |
| Ctrl+B | [Batch](execute.md) : un script avec une ligne par fichier marqué |
| F5 / Ctrl+F5 | créer une [archive](archives.md) avec le fichier / les fichiers marqués |
| Alt+I | Info : détails du fichier ([informations](info.md)) |

Voir aussi : [la fenêtre d'arborescence](tree.md), [les touches en vigueur](keys.md).
