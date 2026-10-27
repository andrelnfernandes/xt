# Rechercher dans les fichiers marqués

## Ctrl+S

Recherche du texte dans tous les fichiers marqués de la liste et **démarque ceux
qui ne le contiennent pas**, avec progression et `Esc` pour arrêter. À l'invite,
**F2** active ou désactive la sensibilité à la casse (en ignorant la casse, les
accents sont aussi pris en compte : `AÇÃO` trouve `ação`) et **F4** bascule
entre texte et hexadécimal (paires d'octets telles que `4d 5a`). Un `*` au
milieu du terme correspond à n'importe quoi sur la même ligne (`error*disk`) ;
un terme ne s'étend jamais sur plusieurs lignes. Cela fonctionne aussi à
l'intérieur des [archives](archives.md).

## Ctrl+V

Ouvre les fichiers marqués dans la [visionneuse](viewer.md), l'un après
l'autre : **N**ext et **P**rev changent de fichier (Alt+Home et Alt+End :
premier et dernier) et **U** démarque le fichier courant. Le terme recherché est
partagé avec Ctrl+S : après un Ctrl+S, **Space** dans Ctrl+V va directement à
l'occurrence suivante, en passant au fichier marqué suivant lorsque la fin du
fichier est atteinte.

Enchaînement courant : marquer des fichiers (`T`, ou Ctrl+T dans une vue
Branch), chercher un terme avec Ctrl+S, ouvrir avec Ctrl+V, puis appuyer sur
Space pour parcourir les occurrences d'un fichier à l'autre.
