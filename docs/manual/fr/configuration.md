# Configuration

## L'écran de configuration (F12)

**F12** (ou Alt+F10, comme dans ZTreeWin ; sous GNOME, Alt+F10 maximise
généralement la fenêtre et n'atteint jamais Xylem) ouvre quatre pages :

| Page | Options |
|---|---|
| 1. Programmes et formats | éditeur, ouvrir l'éditeur dans le cadre de Xylem, thème Xylem dans l'éditeur, langue de l'aide, programme d'ouverture, format de date, format de nombres, commandes Git |
| 2. Affichage et tri | fichiers cachés, ordre et sens de tri initiaux, action d'Enter dans la fenêtre de fichiers, mode d'affichage initial, coloration syntaxique, retour à la ligne, numéros de ligne, fichiers compressés, encodage des vieux textes, limite de taille de ce que lit la visionneuse, icônes et glyphes (Nerd Fonts), couleurs du terminal intégré |
| 3. Confirmations et historique | confirmer avant de quitter, supprimer vers la corbeille, confirmation de Prune, entrées avec `@` non enregistrées, historique propre au terminal intégré |
| 4. Copie, comparaison et rafraîchissement | ignorer 2 s, ignorer 1 h, rafraîchissement automatique, point de départ des chemins Alt+C/Alt+M |

La limite de taille de la visionneuse (**L** à la page 2) coûte de la mémoire et du temps : avec `1024` Mio, ouvrir un fichier texte ordinaire de cette taille occupe environ 2,5 Gio de RAM et prend environ 5 s, et les fichiers aux lignes très courtes coûtent plus par octet (voir [la visionneuse](viewer.md)).

Les icônes et glyphes (**M** à la page 2) nécessitent une police intégrant le patch Nerd Fonts. Aucune police spécifique n'est imposée : n'importe quelle police à chasse fixe convient (comme Fira Code, JetBrains Mono, Hack, Meslo ou Cascadia Code), tant que vous utilisez son édition « Nerd Font ». Nerd Fonts injecte des milliers de glyphes standards dans la zone privée d'Unicode. Une police classique sans ce patch (même la Fira Code originale) ne contient pas ces icônes et affichera des carrés vides (`□`). Pour les utiliser :
- Téléchargez la Nerd Font de votre choix sur nerdfonts.com ou installez un paquet Nerd Fonts via le gestionnaire de paquets de votre distribution.
- Sélectionnez cette police dans les réglages de votre émulateur de terminal.

La lettre de l'option (ou Enter) modifie la valeur, PgUp/PgDn ou 1–4 changent de
page, **F4** choisit le thème dans une liste (avec aperçu immédiat), **F3**
enregistre et applique, **Esc** annule (en demandant confirmation s'il y a des
modifications).

## Le fichier de configuration

Tout est enregistré dans `~/.config/xylem/config.toml` (ou
`$XDG_CONFIG_HOME/xylem/` ; `%APPDATA%\xylem\` sous Windows), qui peut aussi
être édité à la main : l'écran de configuration conserve vos commentaires. Pour
partir d'un fichier commenté contenant toutes les valeurs par défaut :

```bash
mkdir -p ~/.config/xylem && xy --default-config > ~/.config/xylem/config.toml
```

- `theme` : `classic` (bleu XTree), `dark`, `light`, `mono` (les couleurs du
  terminal), `norton`, `solarized`, `gruvbox`, `nord`, `slate` (dans le style de
  Windows Terminal), `graphite` (macOS en mode sombre), `daylight` (macOS en
  mode clair) ou `vivid` (couleurs vives, pour les présentations et captures
  d'écran). F4 les affiche tous avec un aperçu.
- `[options]` : les options de l'écran de configuration.
- `[colors]` : couleurs individuelles surchargeant le thème, sous forme de noms
  (`"yellow"`), de codes `"#rrggbb"` ou d'indices de 0 à 255.
- `[keys.any]`, `[keys.tree]`, `[keys.files]` : redéfinissent les touches dans
  les deux fenêtres ou dans une seule (qui a priorité). Par ex. `"f3" = "view"`,
  `"ctrl+r" = "relog"` ; `"none"` désactive une touche. La liste des touches de
  l'aide (**K**) affiche les touches en vigueur, avec vos modifications.
- `[editors]` : l'éditeur pour chaque type de fichier
  (`"*.png *.jpg" = "gimp %f"`, `binary = "hexedit"`) ; voir
  [Éditer](execute.md).

Une erreur dans le fichier n'empêche pas Xylem de démarrer : il utilise les
valeurs par défaut et affiche l'erreur dans le pied de page, et l'écran de
configuration refusera d'enregistrer tant que le fichier n'aura pas été corrigé,
afin de ne pas écraser ce que vous avez écrit.

Xylem s'appelait xt jusqu'à la version 0.1. Au premier lancement, il déplace la
configuration, l'historique et les signets de xt vers ses propres dossiers.
