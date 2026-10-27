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

## Historique

Dans chaque invite, ↑ et ↓ rappellent les saisies précédentes. Chaque type
d'invite (Filespec, destination, Log, eXecute...) possède son propre historique,
enregistré dans `~/.local/state/xylem/history.toml`, lisible uniquement par
vous, puisqu'il peut contenir des commandes tapées.
