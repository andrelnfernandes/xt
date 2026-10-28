# Vergleich mit ZTreeWin

Eine Befehlsübersicht auf Grundlage des offiziellen ZTreeWin-v2.2-Handbuchs
([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf) aus dem Jahr 2011,
insbesondere der „Quick Command Table" und Abschnitt 3.7 zur Konfiguration). In
neueren Versionen von ZTreeWin können sich Einzelheiten geändert haben.

## Implementiert

Baum mit bedarfsweisem Einlesen (`+`, `*`), Dateifenster, Filespec, Branch,
Showall, Markierungen (T, U, I, Ctrl+T/U), Kopieren/Verschieben/Löschen/Umbenennen
samt den Varianten mit Ctrl (markierte Dateien) und Alt (markierte Dateien mit
Pfaden), Verzeichnis erstellen, Verzeichnis löschen, Prune, Attribute,
Betrachter, Bearbeiten, Ausführen (eXecute), Vergleichen (Verzeichnis und
Zweig), Sortieren, Log, geteiltes Fenster (F8 und Tab), neu einlesen (F3),
Öffnen (`O`), automatische Aktualisierung, Durchsuchen und Entpacken von
Archiven, Verlauf in Eingabezeilen, Hilfe (F1) und Konfigurationsbildschirm
(Alt+F10).

## Vereinfacht

| Funktion | In ZTreeWin | In Xylem |
|---|---|---|
| Filespec | Name mit eigenen Regeln für Stamm und Erweiterung, Datum, Größe, Attribut, umgekehrter Filespec (Ctrl+I), Labels | Glob-Muster für Namen, Datum und Größe wie in ZTreeWin (mit K/M/G); keine Attribute, Umkehrung oder Labels |
| Betrachter | Modi Alpha, Dump, Hex, Junk und Wrap; Zeichensätze; Hex-Bearbeitung; Lesezeichen; Auto-Scroll | Text (UTF-8, UTF-16, CP850, Windows-1252 und CP437, selbst erkannt, O wechselt) oder Hex, mit ZTreeWin-Suche (F, B, /, \\, Space, + und -), Syntaxfarben (hat ZTreeWin nicht), Zeilenumbruch (W), Zeilennummern (#), Springen zu Zeile (g / :); bis 16 MiB; bewusst keine Hex-Bearbeitung (eigener Editor für Binärdateien unter [`[editors]`](execute.md)) |
| Ctrl+S | Text, Hex oder Unicode; durchsucht nur den ZIP-Index | Text oder Hex |
| Attribute | Windows-Attribute (R, A, S, H) | Unix-Berechtigungen oktal (`644`); kein Eigentümer/Gruppe. Unter Windows R, H, S und A wie in ZTreeWin |
| Sortieren | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; gleich/unterschiedlich bei geteiltem Bildschirm | Name, Ext, Date, Size, aufsteigend/absteigend, Pfad zuerst; jedes Fenster hat seine eigene Reihenfolge |
| Vergleichen | Optionen Identical, Unique, Newer, Older, Size, Binary, Case; Anzeigefilter | markiert Unique, Newer, Older, Identical, Different oder Unique+Newer; kein Binärvergleich |
| Prune | löscht nur Unterverzeichnisse oder auch das aktuelle; nur leere Verzeichnisse; Papierkorb | löscht immer das gesamte Verzeichnis endgültig; einstellbare Sicherheitsabfrage |
| Löschen | optionaler Papierkorb | optionaler System-Papierkorb (delete_to_trash, T/P-Umschaltung bei Abfrage, Shift+Del endgültig); Oops (O/Ctrl+Z) zum Wiederherstellen |
| Log | mehrere eingelesene Laufwerke, Global, Laufwerkswechsel | eine Wurzel pro Fenster; `L` öffnet ein anderes Verzeichnis |
| eXecute | Batch-Parameter `%1`–`%8`, neues Fenster (NTW) | `sh` mit `%f`, `%t` und `%d`; immer im Terminal von Xylem; zusätzlich ein integriertes Terminal (Alt+X) |
| Verlauf | Verlaufsfenster mit Lesezeichen | ↑/↓ in der Eingabezeile, ein Verlauf pro Eingabetyp, bleibt zwischen Sitzungen erhalten |
| Konfiguration | 7 Seiten, Farbeditor für einzelne Elemente (F4), Alt-Export | 4 Seiten mit den unter Linux sinnvollen Optionen; F4 wählt aus 12 fertigen Themen; Einzelfarben in `config.toml` |
| Archive | beliebige Formate über externe Packprogramme, Erstellen (F5), Entpacken (Alt-F5) | ZIP/JAR/WAR/APK, 7z, TAR (unkomprimiert, gzip, xz, bzip2, zstd): Öffnen (Alt+F5), Entpacken mit Kopieren, Erstellen mit F5 (ZIP, TAR und TAR.GZ) |
| Geteilter Bildschirm | optionale Statistikfelder je Seite | kein Statistikfeld |
| Menüs | Ctrl-/Alt-Menü beim Gedrückthalten der Taste | ebenso in Terminals mit Kitty-Protokoll; F9 in jedem Terminal |

Konfigurationsoptionen aus ZTreeWin, die nicht übernommen wurden, da sie
Windows-spezifisch sind: RSH-Attribute, Junctions, OEM/ANSI, EBCDIC,
Fenstergröße, Verknüpfungen (.lnk). Die übrigen hängen von Funktionen ab, die
noch nicht implementiert sind (siehe unten).

## Abweichende Tasten

Die Tastenbelegung folgt ZTreeWin, mit diesen Ausnahmen:

| Funktion | ZTreeWin | Xylem |
|---|---|---|
| Hilfe | F1 | F1 oder Alt+H (in GNOME Terminal öffnet F1 die Hilfe des Terminals) |
| Konfiguration | Alt+F10 | F12 oder Alt+F10 (unter GNOME maximiert Alt+F10 das Fenster und erreicht Xylem nicht) |
| Lesezeichen | F11 | F11, Alt+B oder einfaches Anführungszeichen `'` (in GNOME Terminal schaltet F11 das Vollbild um) |
| Markierte verschieben | Ctrl+M | Ctrl+M in Terminals mit Kitty-Protokoll; in anderen `F9` `M` (oder Alt+M und F6) |
| `-` im Baum | entfernt den Zweig aus dem Speicher | klappt nur zu (bleibt eingelesen) |
| F2 | (nichts) | liest das Verzeichnis neu ein, genau wie F3 |
| F9 | Anwendungsmenü (nicht implementiert) | Ctrl-/Alt-Menüs |
| F12 | Makros (nicht implementiert) | Konfiguration |
| `/` | wie `?` (Statistiken) | Treespec, genau wie `\` (`?` zeigt weiterhin die Statistiken) |
| Ctrl+C, Ctrl+M im Baum | (nichts) | Kopieren/Verschieben der markierten Dateien des aktuellen Verzeichnisses |
| Pfad in Zwischenablage kopieren | Ctrl+Insert | Y oder Ctrl+Insert (Ctrl+Y für markierte; funktioniert über OSC 52 und System-Zwischenablage) |

Alle Tasten können unter `[keys]` in `config.toml` neu belegt werden.

## Nicht implementiert

- **Baum:** Global (G), Graft (Alt+G), Mirror (Alt+M), Freigeben/Aus dem
  Speicher entladen (Alt+R, `-`, Ctrl+Backspace), Ausblenden (Alt+H), Wash
  (Alt+W), Volume (Ctrl+V), F5/F6 (Unterverzeichnisebenen ausblenden), nach
  Attribut markieren (Alt+T/U), Treespec im Bearbeitungsmodus (F5,
  Platzhalter) und spezielle Ordner (F6), direkte Suche mit Eingabezeile (`|`)
  und aktivem Modus, Laufwerkswechsel (`<` und `>`).
- **Dateien:** In neuem Terminalfenster öffnen (Alt+O), Batch der aktuellen
  Datei (Alt+B), JFC (J, Inhalt zweier Dateien vergleichen), ZAAP (externes
  Packprogramm), Verknüpfung erstellen (H), Drucken (P), Eigentümer/Gruppe
  (chown), Megaview (Ctrl+F7 im Dateifenster), markierte Dateien zwischen
  Fenstern zusammenführen (Ctrl+F6), teilweises Aufheben von Markierungen
  (Ctrl+F8).
- **Allgemein:** Makros (F12), Anwendungsmenü (F9), Beenden und zur Shell
  (Ctrl+Q), Speichern und Laden des eingelesenen Zustands (Zlist, Ctrl/Alt+Z),
  Farben nach Name und Attribut (ZCOLORS.INI), Mausbedienung,
  Befehlszeilenoptionen von ZTreeWin, Anzeigegrößenbefehle (Alt+F6–F9).
