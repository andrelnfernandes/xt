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

### Éditeurs par type de fichier

Un éditeur de texte ne ferait qu'endommager une image ou un programme ; c'est
pourquoi E n'ouvre pas les **fichiers binaires** (ceux contenant un octet nul
près du début, que la visionneuse affiche en hexadécimal) : il l'indique dans le
pied de page et suggère `O`, qui ouvre le fichier avec le programme du système.
Pour les éditer, ou pour attribuer un éditeur particulier à certains fichiers,
listez-les dans la table `[editors]` de `config.toml` :

```toml
[editors]
"*.png *.jpg *.jpeg *.gif" = "gimp %f"
"*.md" = "typora"
"*.svg" = ""            # vide : l'éditeur habituel, même si le motif correspond
binary = "hexedit"      # tout autre fichier binaire
```

Chaque clé est une liste de motifs de noms, comme dans [Filespec](filespec.md)
(la casse n'a pas d'importance) ; chaque valeur est une ligne de commande, avec
`%f` pour le fichier (sinon le fichier est ajouté à la fin). La première clé qui
correspond l'emporte, dans l'ordre du fichier. `binary` s'applique aux fichiers
binaires qu'aucun motif n'a retenus ; les autres fichiers s'ouvrent dans
l'éditeur habituel. Les commandes s'exécutent de la même façon que l'éditeur : à
l'intérieur du cadre de Xylem par défaut, ou en cédant le terminal lorsque
`editor_in_frame = false` (pour un programme graphique que vous souhaitez laisser
tourner, ajoutez `&` et mettez `editor_in_frame = false`, ou utilisez `O`).

Xylem n'a pas d'éditeur hexadécimal intégré, et c'est un choix délibéré, non un
oubli : cela représenterait beaucoup de travail pour peu d'usage, et modifier
des octets à la main est un moyen facile de corrompre un fichier. Ceux qui en
ont besoin peuvent choisir un outil conçu pour cela (`hexedit`, `bvi`, `dhex`,
`ImHex`...) dans `binary`.

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
