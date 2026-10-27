# Filespec : le filtre de fichiers

`F` définit quels fichiers s'affichent dans la fenêtre de fichiers (chaque
fenêtre a son propre filtre). Le filespec actif apparaît dans le titre de la
fenêtre. Les spécifications sont séparées par des espaces, `;` ou `,`, et sont
de trois types, comme dans ZTreeWin :

- **nom :** un motif glob (`*.rs`, `a?c*`, `[abc]*`), insensible à la casse ;
  entre guillemets, un nom peut contenir des espaces (`"mon fichier*"`) ;
- **date :** `<`, `=`, `>`, `<=`, `>=` ou `<>` suivi d'une date dans l'ordre du
  format configuré, avec `/`, `.` ou `-` et une année à 2 ou 4 chiffres
  (`>01/05/2026` ; `2026-05-01` fonctionne toujours), ou `TODAY` et `TODAY-n`
  (`>TODAY-7` : les 7 derniers jours) ;
- **taille :** de même avec un `s` après l'opérateur (`>=s200000`) ; `K`, `M` et
  `G` fonctionnent aussi (`>s100M`).

Un `-` au début inverse la condition (`-*.bak` ; `-<s1K` équivaut à `>=s1K`). Au
sein de chaque type, les spécifications positives sont combinées par OU (OR),
sauf `>a` immédiatement suivi de `<b`, qui désigne l'intervalle entre les deux
(`>=01/05/2026,<=31/05/2026` : le mois de mai) ; `<>` exclut. Entre différents
types, ET (AND) s'applique : `*.txt,=TODAY,<s1000` désigne les fichiers `.txt`
d'aujourd'hui de moins de 1000 octets.

`*` (ou une invite vide) affiche tout. **↑** rappelle les filespecs précédents.

Voir aussi : [la fenêtre de fichiers](files.md), [marquage](tagging.md) (Ctrl+T
dans l'arborescence marque ce qui correspond au filespec).
