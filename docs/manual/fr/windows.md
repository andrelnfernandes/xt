# Windows

Xylem se compile aussi pour Windows (Windows Terminal ou la console classique).
Il y passe les tests automatisés, mais n'a pas encore été utilisé au quotidien ;
attendez-vous donc à quelques aspérités. Ce qui change :

- **Attributs** (`A`) modifie R, H, S et A, comme dans ZTreeWin : `RA` définit
  exactement ceux-ci, `+H -A` en active ou en désactive certains. La liste des
  fichiers les affiche sous la forme `r-sa`.
- Les fichiers et répertoires **cachés** sont ceux qui portent l'attribut H (un
  nom commençant par un point est un nom ordinaire).
- **eXécuter** et **Éditer** passent par `cmd`, avec les noms entre guillemets
  doubles. Un `%NOM%` à l'intérieur d'un nom de fichier reste interprété par
  `cmd`.
- L'éditeur par défaut est `notepad`, `O` utilise `start` et Ctrl+B propose
  `xy-batch.bat` (commençant par `@echo off`).
- Supprimer un fichier en lecture seule le supprime, comme le fait `rm` sous
  Linux.
- La copie de liens symboliques nécessite le mode développeur ou les droits
  d'administrateur.
- Le mode d'affichage propriétaire (Alt+F) affiche `-`.
- Configuration dans `%APPDATA%\xylem\config.toml`, historique dans
  `%LOCALAPPDATA%\xylem\`.
- Le terminal intégré (Alt+X) exécute `cmd` (`%COMSPEC%`). Y entrer place
  toujours le shell dans le répertoire de l'arborescence, mais l'arborescence ne
  suit pas les `cd` du shell et `$XY_TAGGED` n'est pas disponible.

Pour Alt+Q dans PowerShell, voir [quitter](quit.md).
