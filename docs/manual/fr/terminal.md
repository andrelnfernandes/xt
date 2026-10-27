# Le terminal intégré (Alt+X)

Absent de XTree et de ZTreeWin : votre shell (`$SHELL`) tourne à l'intérieur de
l'écran de Xylem, dans la partie inférieure, tandis que l'arborescence et les
fichiers restent visibles au-dessus.

- **Alt+X** ouvre le terminal et revient à Xylem. Le shell continue de tourner
  lorsqu'il est masqué, avec son historique et ce qu'il affichait à l'écran ;
  `exit` le ferme.
- **Alt+Z** l'ouvre en plein écran ; à l'intérieur, il bascule entre le plein
  écran et le panneau inférieur. Alt+X le rouvre ensuite tel qu'il était.
- **L'arborescence suit le shell :** un `cd` dans le terminal déplace le curseur
  de l'arborescence vers le même répertoire (Linux).
- **Le shell suit l'arborescence :** lorsque vous entrez dans le terminal depuis
  un autre répertoire, il s'y place. Cela ne se produit que lorsque le shell
  attend à l'invite ; si un programme est en cours d'exécution (vim, less...),
  rien n'est tapé.
- **`$XY_TAGGED`** contient les fichiers marqués, définis lorsque vous entrez
  dans le terminal. C'est un tableau : en bash, `"${XY_TAGGED[@]}"`
  (`cp "${XY_TAGGED[@]}" ~/backup`) ; en zsh et fish, `$XY_TAGGED`. Avec les
  autres shells (sh, dash), seul le `cd` est effectué.
- **Alt+O** insère les fichiers marqués à la position du curseur, entre
  guillemets et relatifs au répertoire du shell (`wc -l 'a.txt' 'b c.txt'`).
  Depuis l'arborescence ou la fenêtre de fichiers, il ouvre le terminal et les
  insère.
- **Shift+PgUp/PgDn** font défiler ce qui est sorti de l'écran.
- En quittant le terminal, les répertoires ouverts sont relus.
- **Quitter Xylem ferme le shell** et tout ce qui y tourne. Si un programme y
  est en cours d'exécution (par exemple `vim`), Q et Alt+Q demandent d'abord
  confirmation, même si `confirm_quit` est désactivé (Linux et macOS).
- Les programmes plein écran (vim, htop, less) fonctionnent à l'intérieur ;
  toutes les touches leur sont transmises, sauf Alt+X, Alt+Z et Alt+O. C'est
  pourquoi F1 et Alt+H n'ouvrent pas cette aide depuis le terminal.

## Son propre historique

Les commandes qui y sont tapées vont dans un historique de shell séparé de celui
de vos autres terminaux : `~/.local/state/xylem/bash_history` (`zsh_history`...),
via `HISTFILE` ; dans fish, la session `xylem`. Si votre fichier rc définit
`HISTFILE` sans condition, c'est lui qui l'emporte. Désactivez cette option avec
F12 (page 3) ou `terminal_history = false`.

## Touches et couleurs

Les touches peuvent être modifiées dans `[keys.any]` (`terminal`,
`terminal_full`, `terminal_tagged`) : Alt+T, par exemple, ouvre le menu
« Terminal » dans certains terminaux. Les couleurs se choisissent avec F12
(page 2) ou `terminal_theme` : `native` (celles du terminal dans lequel tourne
Xylem), `xylem` (l'arrière-plan et le texte du thème de Xylem), `solarized`,
`gruvbox`, `nord`, `dracula` ou `one_dark`.

Pour entrer dans le terminal et changer de répertoire, Xylem écrit un petit
script dans un dossier qui lui est propre (dans `~/.cache/xylem/`, lisible
uniquement par vous) et tape ` . script` à l'invite ; l'espace initial évite que
la ligne soit enregistrée dans l'historique en bash (`ignorespace`) et en zsh
(`HIST_IGNORE_SPACE`). Dans le terminal, `XY_TERMINAL=1` est défini pour les
invites et scripts qui veulent le savoir ; le terminal d'une [machine
distante](remote.md) le définit aussi (utile, par exemple, pour empêcher un
`~/.bashrc` d'y lancer tmux).
