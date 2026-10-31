# Xylem-Handbuch

Xylem ist ein textbasierter Dateimanager, inspiriert von XTree und ZTreeWin:
oben ein Verzeichnisbaum, darunter die Dateien des aktuellen Verzeichnisses
und rechts die Statistiken. Er liest (loggt) Verzeichnisse in den Speicher ein,
sodass Sie Dateien über einen gesamten Zweig hinweg markieren und in einem
Schritt kopieren, verschieben, löschen, vergleichen oder umbenennen können.

Xylem behandelt nicht vertrauenswürdige Dateien als potenziell feindlich: Es
sollte niemals abstürzen, einfrieren, Code ausführen, außerhalb des Zielorts
schreiben oder den Speicher erschöpfen, ganz gleich, was eine Datei enthält oder
wie sie benannt ist. Dennoch können Fehler vorkommen. Eine vollständige Liste
der Schutzmaßnahmen und Empfehlungen finden Sie in der Sicherheitserklärung
im README.

**F1** (oder **Alt+H** in Terminals, die F1 selbst belegen) öffnet die Hilfe zu
dem, was Sie gerade tun: in einer Eingabezeile die Hilfe zu dieser Eingabe; im
Baum- oder Dateifenster die Hilfe zu diesem Fenster. In der Hilfe wählen
**Tab** und **Shift+Tab** einen Link aus, **Enter** folgt ihm, **Backspace**
geht zurück, **I** zeigt diesen Index, **K** die aktiven Tasten, **L** wechselt
die Sprache und **Esc** schließt die Hilfe.

**Hilfe bei der Übersetzung:** Diese Fassung des Handbuchs wurde nicht von
Muttersprachlern geprüft. Wenn Sie einen Fehler oder eine seltsam klingende
Stelle finden, schlagen Sie bitte eine Verbesserung vor: `https://github.com/andrelnfernandes/xylem/issues`.

## Fenster und Ansichten

- [Das Baumfenster](tree.md): Navigation, Einlesen, Branch und Showall
- [Das Dateifenster](files.md): Anzeigemodi, Autoview, Dateien öffnen
- [Geteilter Bildschirm](split.md): zwei Fenster nebeneinander
- [Der Betrachter](viewer.md): Text und Hex, Suche, Syntaxfarben
- [Zeilen filtern](filter.md): `&` im Betrachter, mit einer Anleitung zu regulären Ausdrücken
- [Markierte Dateien durchsuchen](search.md): Ctrl+S und Ctrl+V
- [Informationen](info.md): Info, Avail, Statistiken und Verzeichnisgrößen
- [Speicherplatzbelegung](usage.md): wohin der Platz geht (Alt+W)
- [Duplikate](duplicates.md): Dateien und Ordner mit gleichem Inhalt (Alt+Y)

## Arbeiten mit Dateien

- [Markierungen und Gruppenoperationen](tagging.md)
- [Kopieren und Verschieben](copy-move.md), [Ordner auswählen](picker.md)
- [Löschen, Papierkorb und Prune](delete.md)
- [Umbenennen und Stapelumbenennung](rename.md)
- [Berechtigungen und Datum](attributes.md)
- [Filespec: der Dateifilter](filespec.md)
- [Treespec und Log](treespec.md)
- [Vergleichen](compare.md) und [Sortieren](sort.md)
- [Bearbeiten, Ausführen und Batch](execute.md)
- [Archive](archives.md): ZIP, 7z und TAR

## Über XTree hinaus

- [Das integrierte Terminal](terminal.md)
- [Entfernte Rechner (SSH)](remote.md): anderen Rechner durchsuchen, sein Terminal
- [Cloud-Speicher (rclone)](cloud.md): Google Drive, S3 und andere
- [Git](git.md)
- [Lesezeichen](bookmarks.md)

## Einrichtung

- [Konfiguration](configuration.md): der F12-Bildschirm, Themen, Tasten
- [Beenden und Verzeichnis der Shell wechseln](quit.md)
- [Windows](windows.md)
- [Vergleich mit ZTreeWin](ztreewin.md)
- Aktive Tastenbelegung: drücken Sie **K** in der Hilfe.
