# Comparaison avec ZTreeWin

## Avant tout, un remerciement

J'utilise une variante de XTree depuis les années 1990, et j'ai une licence
payée de ZTreeWin depuis 2013. Pour moi, ZTreeWin reste le meilleur
gestionnaire de fichiers en mode texte qui existe : rapide, prévisible,
entièrement piloté au clavier, et construit autour d'une manière de travailler
(lire l'arborescence, marquer des fichiers dans des branches entières, agir sur
les fichiers marqués) qu'aucun autre gestionnaire de fichiers n'a égalée. Kim
Henkel fait vivre la manière XTree sous Windows depuis des décennies, et Xylem
n'existerait pas sans ce travail.

Xylem n'a pas vocation à remplacer ZTreeWin. Il existe parce que je voulais la
même manière de travailler sous Linux, où ZTreeWin ne tourne pas nativement.
Xylem se compile aussi pour Windows, parce que Rust le rend facile, mais ce
n'est pas un objectif pour moi : sous Windows, ZTreeWin vaut largement sa
licence : [ztree.com](https://www.ztree.com/), et le
[forum](https://groups.io/g/ZTreeWin) de ses utilisateurs.

Cette page compare les deux commande par commande, à partir du manuel officiel
de ZTreeWin v2.2 ([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf), de
2011, en particulier la « Quick Command Table » et la section 3.7 sur la
configuration). Les versions plus récentes de ZTreeWin peuvent avoir modifié
certains détails ; si quelque chose ici n'est plus à jour, ouvrez une issue.

## La même manière de travailler

Arborescence avec lecture à la demande (`+`, `*`), fenêtre de fichiers,
Filespec, Branch, Showall, marquage (T, U, I, Ctrl+T/U),
Copier/Déplacer/Supprimer/Renommer avec les variantes Ctrl (fichiers marqués) et
Alt (fichiers marqués avec chemins), Créer un répertoire, Supprimer un
répertoire, Prune, Attributs, Visionneuse, Éditer, eXécuter, Comparer
(répertoire et branche), Trier, Log, écran partagé (F8 et Tab), relecture (F3),
Ouvrir (`O`), rafraîchissement automatique, navigation et extraction d'archives,
historique dans les invites, signets, aide (F1) et écran de configuration. Les
touches sont celles de ZTreeWin chaque fois que possible ; les exceptions sont
dans « Touches différentes », à la fin.

## Ce que Xylem n'a pas encore

- **Arborescence :** Global (G), Graft (Alt+G), Mirror (Alt+M), Release/décharger
  de la mémoire (Alt+R, `-`, Ctrl+Backspace), Hide (Alt+H), Wash (Alt+W), Volume
  (Ctrl+V), F5/F6 (masquer des niveaux de sous-répertoires), marquer par
  attribut (Alt+T/U), Treespec en mode édition (F5, jokers) et dossiers spéciaux
  (F6), recherche directe avec invite (`|`) et mode actif, changement de lecteur
  (`<` et `>`).
- **Fichiers :** Ouvrir dans une nouvelle fenêtre de terminal (Alt+O), Batch du
  fichier courant (Alt+B), JFC (J, comparer le contenu de deux fichiers),
  sHortcut (H), Print (P), propriétaire/groupe (chown), Megaview (Ctrl+F7 dans
  la fenêtre de fichiers), fusionner les fichiers marqués entre fenêtres
  (Ctrl+F6), Partial Untag (Ctrl+F8).
- **Général :** macros (F12), menu d'applications (F9), Quitter vers un shell
  (Ctrl+Q), enregistrement et chargement du log (Zlist, Ctrl/Alt+Z), couleurs
  par nom et attribut (ZCOLORS.INI), souris, options de ligne de commande de
  ZTreeWin, commandes vidéo (Alt+F6–F9).

Pour certaines, un autre programme fait l'affaire en attendant ; voir « Ce que
Xylem laisse à d'autres programmes », plus bas.

## Là où Xylem est plus simple

| Fonction | Dans ZTreeWin | Dans Xylem |
|---|---|---|
| Filespec | nom avec règles propres au nom et à l'extension, date, taille, attribut, filespec inversé (Ctrl+I), étiquettes | glob pour les noms, date et taille comme dans ZTreeWin (avec K/M/G) ; ni attributs, ni inversion, ni étiquettes |
| Visionneuse | modes Alpha, Dump, Hex, Junk et Wrap ; jeux de caractères ; édition hexadécimale ; signets ; défilement automatique | texte ou hexadécimal, avec la recherche de ZTreeWin (F, B, /, \\, Space, + et -), retour à la ligne (W), numéros de ligne (#), aller à la ligne (g / :) ; jusqu'à la limite de taille (`viewer_max_size`, de 16 Mio à 1 Gio) ; pas d'édition hexadécimale, par choix (voir « Ce que Xylem laisse à d'autres programmes ») |
| Ctrl+S | texte, hexadécimal ou Unicode ; ne cherche que dans l'index des ZIP | texte ou hexadécimal |
| Attributs | attributs Windows (R, A, S, H) | permissions Unix en octal (`644`) ; pas de propriétaire/groupe. Sous Windows, R, H, S et A, comme dans ZTreeWin |
| Tri | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted ; Charset ; same/diff en écran partagé | Name, Ext, Date, Size, croissant/décroissant, Path en premier ; chaque fenêtre a son propre ordre |
| Comparer | options Identical, Unique, Newer, Older, Size, Binary, Case ; filtre d'affichage | marque unique, plus récent, plus ancien, identique, différent ou unique+plus récent ; pas de comparaison binaire |
| Prune | supprime seulement les sous-répertoires ou aussi le répertoire courant ; répertoires vides uniquement ; corbeille | supprime toujours tout le répertoire, définitivement ; confirmation configurable |
| Log | plusieurs lecteurs lus, Global, changement de lecteur | une racine par fenêtre ; `L` ouvre un autre répertoire |
| eXécuter | paramètres de batch `%1`–`%8`, nouvelle fenêtre (NTW) | `sh` avec `%f`, `%t` et `%d` ; toujours dans le terminal de Xylem |
| Historique | fenêtre d'historique avec signets | ↑/↓ à l'invite, un historique par type, conservé entre les sessions |
| Configuration | 7 pages, éditeur de couleurs par élément (F4), Alt-Export | 4 pages avec les options pertinentes sous Linux ; F4 choisit parmi 12 thèmes prêts à l'emploi ; couleurs individuelles dans `config.toml` |
| Écran partagé | panneaux de statistiques optionnels de chaque côté | pas de panneau de statistiques |
| Menus | menu Ctrl/Alt en maintenant la touche | idem dans les terminaux avec le protocole kitty ; F9 dans tous les terminaux |

Options de configuration de ZTreeWin qui n'ont pas été reprises parce qu'elles
sont propres à Windows : attributs RSH, jonctions, OEM/ANSI, EBCDIC, taille de
fenêtre, raccourcis (.lnk).

## Là où Xylem va plus loin

Ce qui n'a pas d'équivalent dans le manuel de ZTreeWin v2.2 :

- **Un terminal intégré** (Alt+X) : le shell continue de tourner pendant que
  vous utilisez l'arborescence ; un `cd` dans le shell emmène l'arborescence,
  et le shell suit l'arborescence quand vous y revenez. Alt+O tape les fichiers
  marqués à l'invite. Voir [le terminal intégré](terminal.md).
- **Machines distantes via SSH** (`xy utilisateur@hote:`) : l'arborescence,
  Branch, la visionneuse et Copier fonctionnent sur une autre machine, qui a
  aussi son propre terminal. Voir [machines distantes](remote.md).
- **Git** (optionnel) : lettres d'état dans la liste des fichiers, un menu Git
  (Ctrl+G) et des diffs dans la visionneuse. Voir [Git](git.md).
- **Occupation du disque** (Alt+W) : les dossiers et fichiers d'une branche
  ensemble, les plus gros d'abord, jusqu'à ce qui remplit le disque ; Alt+K
  montre dans l'arborescence la part de chaque dossier. Voir
  [occupation du disque](usage.md).
- **Une visionneuse qui lit davantage :** coloration syntaxique, encodage du
  texte détecté seul (UTF-8, UTF-16, CP850, Windows-1252, CP437), fichiers
  `.gz`, `.xz`, `.zst` et `.bz2` affichés décompressés, et caractères cachés de
  direction du texte (Trojan Source) affichés au lieu d'être obéis. Voir
  [la visionneuse](viewer.md).
- **Des archives sans archiveur externe :** ZIP, 7z et TAR (brut, gzip, xz,
  bzip2, zstd) s'ouvrent comme des dossiers, et ZIP et TAR se créent, le tout
  par Xylem lui-même. Voir [archives](archives.md).
- **Renommage par lot** avec rechercher-remplacer (`s/foo/bar/`), compteurs et
  changements de casse. Voir [renommer](rename.md).
- **Oops :** O (ou Ctrl+Z) remet à sa place ce qui a été envoyé en dernier
  dans la corbeille du système. Voir [supprimer](delete.md).
- **Le presse-papiers partout :** Y copie le chemin même via SSH ou dans tmux
  (OSC 52).
- **Ce qu'est chaque fichier :** un mode d'affichage (Alt+F) qui dit, dans
  l'esprit de la commande `file`, ce que contient chaque fichier, et un autre
  avec propriétaires et permissions.
- **Apparence :** 12 thèmes prêts à l'emploi et icônes Nerd Fonts en option.
  Toute touche peut être redéfinie sous `[keys]` dans `config.toml`. Voir
  [configuration](configuration.md).
- **Fichiers hostiles :** archives, noms et contenus sont traités comme
  potentiellement hostiles, avec des limites sur tout ce qui est lu et
  décompressé (voir la déclaration de sécurité dans le README).
- **Une aide en six langues** (F1, puis L), le même texte que ce manuel et la
  page de manuel.

## Ce que Xylem laisse à d'autres programmes

Certaines choses, Xylem ne les fait pas lui-même, exprès : un programme dédié
les fait mieux, et Xylem l'intègre.

- **Édition hexadécimale :** ZTreeWin édite les octets dans sa visionneuse.
  Xylem n'a pas d'éditeur hexadécimal à lui : en écrire et en maintenir un
  demanderait beaucoup de travail pour peu d'usage, et modifier des octets à la
  main est un moyen facile de corrompre un fichier. À la place, E sur un
  fichier binaire ouvre `hexedit`, `heh`, `bvi` ou `dhex` (celui qui est
  installé, ou celui que vous choisissez) dans le cadre et les couleurs de
  Xylem, ou un éditeur graphique comme ImHex ou GHex. Voir [Éditer](execute.md).
- **Éditer du texte :** votre éditeur (`$VISUAL`, `$EDITOR` ou celui défini
  dans F12), et un éditeur différent par type de fichier si vous le souhaitez
  (`[editors]`).
- **SSH :** le `ssh` du système, donc vos clés, l'agent, `~/.ssh/config` et les
  *jump hosts* fonctionnent comme partout ailleurs.
- **Stockage en ligne :** Google Drive, S3 et d'autres via `rclone mount` ;
  Xylem reconnaît le montage et s'adapte. Voir [stockage en ligne](cloud.md).
- **Git :** le programme `git` lui-même.
- **Ouvrir des fichiers** (O) : le programme du bureau pour chaque type
  (`xdg-open`).
- **Comparer le contenu de deux fichiers** (JFC dans ZTreeWin) : tant que ce
  n'est pas intégré, marquez les deux fichiers et lancez `diff %t` avec
  eXécuter (X), ou utilisez le terminal.
- **Imprimer** (P dans ZTreeWin) : `lp %t` avec eXécuter.
- **Autres formats d'archive** (le ZAAP de ZTreeWin) : votre archiveur, avec
  eXécuter ou dans le terminal.

## Touches différentes

Les touches suivent celles de ZTreeWin. Les exceptions :

| Fonction | ZTreeWin | Xylem |
|---|---|---|
| Aide | F1 | F1 ou Alt+H (dans GNOME Terminal, F1 ouvre l'aide du terminal) |
| Configuration | Alt+F10 | F12 ou Alt+F10 (sous GNOME, Alt+F10 maximise la fenêtre et n'atteint jamais Xylem) |
| Signets | F11 | F11, Alt+B ou apostrophe `'` (dans GNOME Terminal, F11 bascule le plein écran) |
| Déplacer les fichiers marqués | Ctrl+M | Ctrl+M dans les terminaux avec le protocole kitty ; dans les autres, `F9` `M` (ou Alt+M puis F6) |
| `-` dans l'arborescence | décharge la branche de la mémoire | replie seulement (reste lue en mémoire) |
| F2 | (rien) | relit le répertoire, comme F3 |
| F9 | menu d'applications (non implémenté) | menus Ctrl/Alt |
| F12 | macros (non implémenté) | configuration |
| `/` | identique à `?` (statistiques) | Treespec, comme `\` (`?` affiche toujours les statistiques) |
| Ctrl+C, Ctrl+M dans l'arborescence | (rien) | Copier/Déplacer les fichiers marqués du répertoire courant |
| Copier le chemin dans le presse-papiers | Ctrl+Insert | Y ou Ctrl+Insert (Ctrl+Y pour les fichiers marqués ; fonctionne via OSC 52 et le presse-papiers système) |

Toutes les touches peuvent être redéfinies sous `[keys]` dans `config.toml`.
