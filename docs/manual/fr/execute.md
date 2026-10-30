# Éditer, eXécuter et Batch

## Éditer (E)

Ouvre le fichier dans l'éditeur : celui défini à la page 1 de la
[configuration](configuration.md), ou à défaut `$VISUAL`, `$EDITOR`, `editor`
sous Debian et Ubuntu, ou `vi`.

Par défaut, l'éditeur s'ouvre **à l'intérieur du cadre de Xylem**
(`editor_in_frame = true` à la page 1 de **F12**) et utilise **le thème de
couleurs de Xylem** (`editor_theme = true`), y compris les couleurs de syntaxe
et d'interface assorties au thème actif lorsque l'éditeur est `vi`, `vim` ou
`nvim`. Mettez `editor_in_frame = false` pour confier tout le terminal à
l'éditeur, ou `editor_theme = false` pour conserver le jeu de couleurs propre à
l'éditeur.

### Fichiers binaires et éditeurs par type de fichier

Un éditeur de texte ne ferait qu'endommager une image ou un programme ; c'est
pourquoi `E` n'ouvre jamais les **fichiers binaires** (ceux contenant un octet
nul près du début, que la visionneuse affiche en hexadécimal) dans l'éditeur de
texte.

À la place, lorsque **Edit binary files with E** est activé
(`edit_binary = true`, touche **I** à la page 1 de **F12**), `E` ouvre les
fichiers binaires dans un éditeur hexadécimal/binaire :

1. Un motif correspondant dans `[editors]` (s'il y en a un), ou
2. Le **Binary editor** défini à la page 1 de **F12** (`binary_editor`, touche
   **J**) ou `binary` dans `[editors]`, ou
3. S'il est laissé vide, le premier éditeur hexadécimal de terminal trouvé dans
   le `PATH` :
   - **Linux** : `hexedit`, `heh`, `bvi`, `dhex`
   - **Windows** : `heh` (`winget install ndd7xv.heh`), `hexedit`, `bvi`

Les éditeurs hexadécimaux de terminal s'exécutent de la même façon que l'éditeur
de texte : **à l'intérieur du cadre de Xylem** par défaut
(`editor_in_frame = true`) et avec **le thème de couleurs de Xylem**
(`editor_theme = true`). Pour un éditeur hexadécimal graphique (`ImHex`, `GHex`,
`Okteta` sous Linux ; `ImHex`, `HxD` sous Windows) que vous souhaitez laisser
tourner dans sa propre fenêtre, ajoutez `&` à la fin de la commande (par
exemple `imhex %f &` ou `ghex %f &`) : Xylem le lance directement sans ouvrir
le cadre de terminal.

Si vous préférez que `E` n'ouvre pas du tout les fichiers binaires (et se
contente de suggérer `O` dans le pied de page pour les ouvrir avec le programme
du système), réglez **Edit binary files with E** sur `no`
(`edit_binary = false` à la page 1 de **F12**).

Pour attribuer un éditeur particulier à certains types de fichiers, listez-les
dans la table `[editors]` de `config.toml` :

```toml
[editors]
"*.png *.jpg *.jpeg *.gif" = "gimp %f &"
"*.md" = "typora %f &"
"*.svg" = ""            # vide : l'éditeur habituel, même si le motif correspond
binary = "hexedit"      # solution de repli pour tout autre fichier binaire
```

Chaque clé est une liste de motifs de noms, comme dans [Filespec](filespec.md)
(la casse n'a pas d'importance) ; chaque valeur est une ligne de commande, avec
`%f` pour le fichier (sinon le fichier est ajouté à la fin). La première clé qui
correspond l'emporte, dans l'ordre du fichier. `binary` s'applique aux fichiers
binaires qu'aucun motif n'a retenus lorsque `binary_editor` est vide ; les
autres fichiers s'ouvrent dans l'éditeur de texte habituel.

Xylem n'a pas d'éditeur hexadécimal intégré, et c'est un choix délibéré : en
développer et en maintenir un représenterait beaucoup de travail pour peu
d'usage, et modifier des octets à la main est un moyen facile de corrompre un
fichier. À la place, Xylem intègre des éditeurs hexadécimaux dédiés directement
dans son cadre et avec son thème de couleurs — et permet de désactiver l'édition
binaire d'une seule option dans **F12** si vous n'en voulez pas.

## eXécuter (X)

Demande une ligne de commande et l'exécute avec `sh` dans le répertoire courant.
On peut y utiliser :

| | |
|---|---|
| `%f` | le fichier sous le curseur |
| `%t` | les fichiers marqués |
| `%d` | le répertoire courant |
| `%%` | un `%` littéral |

Tous sont automatiquement protégés par des guillemets : `tar czf backup.tgz %t`.
À la fin, Xylem attend que vous appuyiez sur Enter pour vous laisser lire la
sortie, puis relit les répertoires ouverts. Une commande commençant par `@`
n'est pas enregistrée dans l'historique (avec `history_skip_at`).

Pour les sessions longues, le [terminal intégré](terminal.md) (Alt+X) est
souvent plus pratique.

## Batch (Ctrl+B)

Demande le nom du fichier à créer (F2 [choisit le dossier](picker.md)) puis un
masque de ligne, et écrit une ligne par fichier marqué. Paramètres :

| | |
|---|---|
| `%f` | chemin |
| `%d` | répertoire |
| `%n` | nom |
| `%b` | nom sans extension |
| `%e` | extension |
| `%s` | taille |

En minuscules, ils sont protégés par des guillemets, prêts pour le shell
(`convert %f %b.png`) ; en majuscules, en texte brut, pour des listes
(`%N;%S`). Un fichier `.sh` reçoit `#!/bin/sh` et la permission d'exécution
(lancez-le avec `X`). Un fichier existant n'est pas écrasé.

## Batch dans la visionneuse

Avec un tableau ou un texte ouvert dans la [visionneuse](viewer.md), **Ctrl+B** écrit une ligne par enregistrement, à partir du fichier entier : au-delà des 50 000 lignes de l'écran, avec les valeurs entières et telles qu'elles sont dans le fichier. Il respecte le filtre en cours (le filtre SQL sur un tableau, le [filtre de lignes](filter.md) sur le texte). Un tableau est relu à ce moment-là ; le texte sort tel que la visionneuse l'a lu, jusqu'à la limite de taille, et l'aperçu avertit quand ce n'est pas tout le fichier. Il demande le fichier, puis le masque, et affiche les premières lignes avant d'écrire : **Entrée** écrit, **Échap** renonce. Pendant l'écriture, Échap annule et supprime le fichier incomplet. Un fichier existant n'est pas écrasé.

| Champ | Valeur |
|---|---|
| `%{nom}` `%{A}` `%{2}` | une colonne du tableau, par nom, lettre ou position |
| `%{line}` | la ligne entière du texte, avec ses tabulations |
| `%{ip}` `%{1}` `%{0}` | un groupe de l'expression régulière du filtre de lignes, par nom ou numéro ; 0 est tout ce qu'elle a trouvé |
| `%{#}` | le numéro de l'enregistrement parmi ceux écrits : 1, 2, 3... |
| `%{#line}` | le numéro de la ligne dans le fichier (texte seulement) |
| `%f %d %n %b %e %s` | le fichier ouvert, comme dans le Batch ci-dessus |
| `%%` | un `%` |

Un champ qui n'existe pas est une erreur dans l'aperçu, jamais une valeur vide. Comme l'aperçu examine 1 000 enregistrements au plus, une clé JSON Lines qui n'apparaît qu'après compte comme inexistante.

Les valeurs viennent d'un fichier qui peut être hostile et peuvent aller dans ce que vous exécuterez : chaque champ a donc un échappement. Écrivez-le après le champ : `curl "https://api/x?id=%{id:url}"`.

| Échappement | Pour | Par défaut pour |
|---|---|---|
| `:sh` | un argument entre guillemets d'une commande shell | `.sh` `.bash` `.zsh` |
| `:url` | une partie d'URL, encodée en pourcentage | |
| `:json` | une valeur JSON : nombres nus, texte entre guillemets | `.json` `.jsonl` `.ndjson` |
| `:xml` ou `:html` | texte ou attribut | `.xml` `.html` `.htm` `.xhtml` `.svg` |
| `:sql` | un littéral entre guillemets, `NULL`, `X'00ff'` | `.sql` |
| `:csv` | un champ, entre guillemets si nécessaire | `.csv` `.tsv` |
| `:csvsafe` | la même chose, avec un `'` devant ce qu'un tableur prendrait pour une formule | |
| `:line` | la valeur, avec les caractères de contrôle en espaces, pour qu'elle n'ouvre jamais une nouvelle ligne | les autres |
| `:raw` | la valeur telle quelle : **à ne pas utiliser dans un script** | |

- Un blob sort en hexadécimal, sauf avec `:raw` et avec `:sql`, qui a sa propre forme.
- Le filtre d'un tableau s'applique à tout le fichier, avec les règles de l'écran : `code = 7` trouve aussi une cellule avec `007`, qui sort comme `007`. Le fichier peut donc avoir des lignes que l'écran, limité à 50 000, ne montre pas.
- Un `.sh` commence par `#!/bin/sh` mais **ne reçoit pas le droit d'exécution** : lisez-le avant de le lancer. `.bat` et `.cmd` sont refusés, car `cmd` développe `%VAR%` même entre guillemets.
- L'aperçu et le message final signalent les valeurs avec des caractères de contrôle ou de direction du texte et, dans un CSV, celles qui commencent comme une formule. L'aperçu ne compte que ce qu'il a examiné.

### Enregistrements entiers

Dans un fichier `.json`, `.jsonl`, `.ndjson`, `.csv`, `.tsv`, `.xml`, `.html` ou `.htm`, laissez le masque **vide** pour écrire des enregistrements entiers, selon les règles du format : un tableau JSON d'objets, un objet par ligne, un tableau avec une ligne d'en-tête, un `<record>` par enregistrement ou une page avec un tableau. Pour n'écrire que certains champs, commencez le masque par `@` : `@id, nom, #`. Le texte donne `line_number`, `line` et les groupes du filtre. Les champs sont découverts dans les 1 000 premiers enregistrements : une clé JSON Lines qui n'apparaît qu'ensuite est laissée de côté, et le message final le signale.

## Historique

Dans chaque invite, ↑ et ↓ rappellent les saisies précédentes. Chaque type
d'invite (Filespec, destination, Log, eXecute...) possède son propre historique,
enregistré dans `~/.local/state/xylem/history.toml`, lisible uniquement par
vous, puisqu'il peut contenir des commandes tapées.
