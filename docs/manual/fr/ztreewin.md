# Comparaison avec ZTreeWin

Relevé commande par commande fondé sur le manuel officiel de ZTreeWin v2.2
([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf), de 2011, en
particulier la « Quick Command Table » et la section 3.7 sur la configuration).
Les versions plus récentes de ZTreeWin peuvent avoir modifié certains détails.

## Implémenté

Arborescence avec lecture à la demande (`+`, `*`), fenêtre de fichiers,
Filespec, Branch, Showall, marquage (T, U, I, Ctrl+T/U),
Copier/Déplacer/Supprimer/Renommer avec les variantes Ctrl (fichiers marqués) et
Alt (fichiers marqués avec chemins), Créer un répertoire, Supprimer un
répertoire, Prune, Attributs, Visionneuse, Éditer, eXécuter, Comparer
(répertoire et branche), Trier, Log, écran partagé (F8 et Tab), relecture (F3),
Ouvrir (`O`), rafraîchissement automatique, navigation et extraction d'archives,
historique dans les invites, aide (F1) et écran de configuration (Alt+F10).

## Simplifié

| Fonction | Dans ZTreeWin | Dans Xylem |
|---|---|---|
| Filespec | nom avec règles propres au nom et à l'extension, date, taille, attribut, filespec inversé (Ctrl+I), étiquettes | glob pour les noms, date et taille comme dans ZTreeWin (avec K/M/G) ; ni attributs, ni inversion, ni étiquettes |
| Visionneuse | modes Alpha, Dump, Hex, Junk et Wrap ; jeux de caractères ; édition hexadécimale ; signets ; défilement automatique | texte (UTF-8, UTF-16, CP850, Windows-1252 et CP437, détectés seuls, O change) ou hexadécimal, avec la recherche de ZTreeWin (F, B, /, \\, Space, + et -), coloration syntaxique (absente de ZTreeWin), retour à la ligne (W), numéros de ligne (#), aller à la ligne (g / :) ; jusqu'à 16 Mio ; pas d'édition hexadécimale, par choix (éditeur externe pour fichiers binaires dans [`[editors]`](execute.md)) |
| Ctrl+S | texte, hexadécimal ou Unicode ; ne cherche que dans l'index des ZIP | texte ou hexadécimal |
| Attributs | attributs Windows (R, A, S, H) | permissions Unix en octal (`644`) ; pas de propriétaire/groupe. Sous Windows, R, H, S et A, comme dans ZTreeWin |
| Tri | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted ; Charset ; same/diff en écran partagé | Name, Ext, Date, Size, croissant/décroissant, Path en premier ; chaque fenêtre a son propre ordre |
| Comparer | options Identical, Unique, Newer, Older, Size, Binary, Case ; filtre d'affichage | marque unique, plus récent, plus ancien, identique, différent ou unique+plus récent ; pas de comparaison binaire |
| Prune | supprime seulement les sous-répertoires ou aussi le répertoire courant ; répertoires vides uniquement ; corbeille | supprime toujours tout le répertoire, définitivement ; confirmation configurable |
| Supprimer | corbeille optionnelle | corbeille système optionnelle (delete_to_trash, choix T/P à l'invite, Shift+Del définitif) ; Oops (O/Ctrl+Z) pour restaurer |
| Log | plusieurs lecteurs lus, Global, changement de lecteur | une racine par fenêtre ; `L` ouvre un autre répertoire |
| eXécuter | paramètres de batch `%1`–`%8`, nouvelle fenêtre (NTW) | `sh` avec `%f`, `%t` et `%d` ; toujours dans le terminal de Xylem ; dispose aussi d'un terminal intégré (Alt+X) |
| Historique | fenêtre d'historique avec signets | ↑/↓ à l'invite, un historique par type, conservé entre les sessions |
| Configuration | 7 pages, éditeur de couleurs par élément (F4), Alt-Export | 4 pages avec les options pertinentes sous Linux ; F4 choisit parmi 12 thèmes prêts à l'emploi ; couleurs individuelles dans `config.toml` |
| Archives | tout format via un archiveur externe, créer (F5), extraire (Alt-F5) | ZIP/JAR/WAR/APK, 7z, TAR (brut, gzip, xz, bzip2, zstd) : ouvrir (Alt+F5), extraire avec Copier, créer avec F5 (ZIP, TAR et TAR.GZ) |
| Écran partagé | panneaux de statistiques optionnels de chaque côté | pas de panneau de statistiques |
| Menus | menu Ctrl/Alt en maintenant la touche | idem dans les terminaux avec le protocole kitty ; F9 dans tous les terminaux |

Options de configuration de ZTreeWin qui n'ont pas été reprises parce qu'elles
sont propres à Windows : attributs RSH, jonctions, OEM/ANSI, EBCDIC, taille de
fenêtre, raccourcis (.lnk). Les autres dépendent de fonctionnalités non
implémentées (ci-dessous).

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

## Non implémenté

- **Arborescence :** Global (G), Graft (Alt+G), Mirror (Alt+M), Release/décharger
  de la mémoire (Alt+R, `-`, Ctrl+Backspace), Hide (Alt+H), Wash (Alt+W), Volume
  (Ctrl+V), F5/F6 (masquer des niveaux de sous-répertoires), marquer par
  attribut (Alt+T/U), Treespec en mode édition (F5, jokers) et dossiers spéciaux
  (F6), recherche directe avec invite (`|`) et mode actif, changement de lecteur
  (`<` et `>`).
- **Fichiers :** Ouvrir dans une nouvelle fenêtre de terminal (Alt+O), Batch du
  fichier courant (Alt+B), JFC (J, comparer le contenu de deux fichiers), ZAAP
  (archiveur externe), sHortcut (H), Print (P), propriétaire/groupe (chown),
  Megaview (Ctrl+F7 dans la fenêtre de fichiers), fusionner les fichiers marqués
  entre fenêtres (Ctrl+F6), Partial Untag (Ctrl+F8).
- **Général :** macros (F12), menu d'applications (F9), Quitter vers un shell
  (Ctrl+Q), enregistrement et chargement du log (Zlist, Ctrl/Alt+Z), couleurs
  par nom et attribut (ZCOLORS.INI), souris, options de ligne de commande de
  ZTreeWin, commandes vidéo (Alt+F6–F9).
