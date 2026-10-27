# Comparer

`C` dans l'arborescence, avec l'écran [partagé](split.md), compare les fichiers
du répertoire courant avec ceux du répertoire de l'autre fenêtre et marque, au
choix :

- **U**nique : absent de l'autre côté ;
- **N**ewer (plus récent) et **O**lder (plus ancien) ;
- **I**dentical : même taille et même date ;
- **D**ifferent (différent) ;
- **A**ll new : unique + plus récent, ce qui manque pour mettre l'autre côté à
  jour.

**B** (ou Alt+C dans l'arborescence, comme dans ZTreeWin) étend la portée à
toute la branche : Xylem lit la branche, compare en suivant les chemins relatifs
(`src/a.rs` ↔ `<autre>/src/a.rs`) et ouvre la vue Branch avec les fichiers
marqués.

Des dates séparées d'au plus 2 secondes sont considérées comme égales (FAT et
exFAT enregistrent l'heure par pas de 2 secondes) ; `compare_ignore_1h` ignore
aussi les écarts d'exactement une heure (passage à l'heure d'été/hiver). Ces
deux options se trouvent à la page 4 de la [configuration](configuration.md).

Pour synchroniser : `C` `B` `A` puis Alt+C ([copier avec les
chemins](copy-move.md)), qui propose déjà l'autre fenêtre comme destination.
