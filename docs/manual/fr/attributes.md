# Permissions et dates

## Permissions (A)

`A` modifie les permissions du fichier sous le curseur (Ctrl+A : des fichiers
marqués), en octal : `644`, `755`. Alt+A fonctionne comme Ctrl+A.

Sous Windows, `A` modifie les attributs R, H, S et A, comme dans ZTreeWin : `RA`
définit exactement ceux-ci, `+H -A` en active ou en désactive certains. La liste
des fichiers les affiche sous la forme `r-sa`. Voir [Windows](windows.md).

## Nouvelle date (N)

`N` modifie la date de modification du fichier ou du répertoire (Ctrl+N : des
fichiers marqués). L'invite est préremplie avec la date actuelle dans votre
format ; elle accepte aussi `now` et `today`. Avec seulement la date (sans
`HH:MM`), chaque fichier conserve son heure. Les liens symboliques sont ignorés
(la date modifierait celle du fichier vers lequel ils pointent).

Le format de date se règle à la page 1 de la [configuration](configuration.md).
