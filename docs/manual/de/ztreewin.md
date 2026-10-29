# Vergleich mit ZTreeWin

## Zuerst ein Dankeschön

Ich benutze seit den 1990er-Jahren die eine oder andere Variante von XTree
und habe seit 2013 eine bezahlte ZTreeWin-Lizenz. Für mich ist ZTreeWin nach
wie vor der beste Dateimanager im Textmodus, den es gibt: schnell,
berechenbar, vollständig über die Tastatur bedienbar und aufgebaut um eine
Arbeitsweise (den Baum einlesen, Dateien in ganzen Zweigen markieren, mit den
Markierungen arbeiten), die kein anderer Dateimanager erreicht hat. Kim Henkel
hält die XTree-Arbeitsweise unter Windows seit Jahrzehnten lebendig, und ohne
diese Arbeit gäbe es Xylem nicht.

Xylem soll ZTreeWin nicht ersetzen. Es gibt Xylem, weil ich dieselbe
Arbeitsweise unter Linux haben wollte, wo ZTreeWin nicht nativ läuft. Xylem
lässt sich auch für Windows übersetzen, weil Rust das leicht macht, aber das
ist kein Ziel von mir: Unter Windows ist ZTreeWin seine Lizenz mehr als wert:
[ztree.com](https://www.ztree.com/), und das
[Forum](https://groups.io/g/ZTreeWin) seiner Benutzer.

Diese Seite vergleicht beide Befehl für Befehl, auf Grundlage des offiziellen
ZTreeWin-v2.2-Handbuchs ([ztwhlp.pdf](https://www.zedtek.com/download/ztwhlp.pdf)
aus dem Jahr 2011, insbesondere der „Quick Command Table" und Abschnitt 3.7 zur
Konfiguration). In neueren Versionen von ZTreeWin können sich Einzelheiten
geändert haben; wenn hier etwas veraltet ist, eröffnen Sie bitte ein Issue.

## Dieselbe Arbeitsweise

Baum mit bedarfsweisem Einlesen (`+`, `*`), Dateifenster, Filespec, Branch,
Showall (auch nur der markierten Dateien, Ctrl+S), Markierungen (T, U, I,
Ctrl+T/U), Kopieren/Verschieben/Löschen/Umbenennen samt den Varianten mit Ctrl
(markierte Dateien) und Alt (markierte Dateien mit Pfaden), Verzeichnis
erstellen, Verzeichnis löschen, Prune, Attribute, Betrachter, Bearbeiten,
Ausführen (eXecute), Vergleichen (Verzeichnis und Zweig), Sortieren, Log,
geteiltes Fenster (F8 und Tab), neu einlesen (F3), Öffnen (`O`), automatische
Aktualisierung, Durchsuchen und Entpacken von Archiven, Verlauf in
Eingabezeilen, Lesezeichen, Hilfe (F1) und ein Konfigurationsbildschirm. Die
Tasten sind, wo immer möglich, die von ZTreeWin; die Ausnahmen stehen unter
„Abweichende Tasten" am Ende.

## Was Xylem noch fehlt

- **Baum:** Global (G), Graft (Alt+G), Mirror (Alt+M), Freigeben/Aus dem
  Speicher entladen (Alt+R, Ctrl+Backspace), Ausblenden (Alt+H), Wash
  (Alt+W), Volume (Ctrl+V), F5/F6 (Unterverzeichnisebenen ausblenden), nach
  Attribut markieren (Alt+T/U), Treespec im Bearbeitungsmodus (F5,
  Platzhalter) und spezielle Ordner (F6), direkte Suche mit Eingabezeile (`|`)
  und aktivem Modus, Laufwerkswechsel (`<` und `>`).
- **Dateien:** In neuem Terminalfenster öffnen (Alt+O), Batch der aktuellen
  Datei (Alt+B), JFC (J, Inhalt zweier Dateien vergleichen), Verknüpfung
  erstellen (H), Drucken (P), Eigentümer/Gruppe (chown), Megaview (Ctrl+F7 im
  Dateifenster), markierte Dateien zwischen Fenstern zusammenführen (Ctrl+F6),
  teilweises Aufheben von Markierungen (Ctrl+F8), Tags-Only außerhalb von
  Showall (Ctrl+Enter und Ctrl+F4, Ctrl+B im Baum).
- **Allgemein:** Makros (F12), Anwendungsmenü (F9), Beenden und zur Shell
  (Ctrl+Q), Speichern und Laden des eingelesenen Zustands (Zlist, Ctrl/Alt+Z),
  Farben nach Name und Attribut (ZCOLORS.INI), Mausbedienung,
  Befehlszeilenoptionen von ZTreeWin, Anzeigegrößenbefehle (Alt+F6–F9).

Für manches davon gibt es einen Ersatz in einem anderen Programm; siehe „Was
Xylem anderen Programmen überlässt" weiter unten.

## Wo Xylem einfacher ist

| Funktion | In ZTreeWin | In Xylem |
|---|---|---|
| Filespec | Name mit eigenen Regeln für Stamm und Erweiterung, Datum, Größe, Attribut, umgekehrter Filespec (Ctrl+I), Labels | Glob-Muster für Namen, Datum und Größe wie in ZTreeWin (mit K/M/G); keine Attribute, Umkehrung oder Labels |
| Betrachter | Modi Alpha, Dump, Hex, Junk und Wrap; Zeichensätze; Hex-Bearbeitung; Lesezeichen; Auto-Scroll | Text oder Hex, mit ZTreeWin-Suche (F, B, /, \\, Space, + und -), Zeilenumbruch (W), Zeilennummern (#), Springen zu Zeile (g / :); bis zum Größenlimit (`viewer_max_size`, 16 MiB bis 1 GiB); bewusst keine Hex-Bearbeitung (siehe „Was Xylem anderen Programmen überlässt") |
| Ctrl+S | Text, Hex oder Unicode; durchsucht nur den ZIP-Index | Text oder Hex |
| Attribute | Windows-Attribute (R, A, S, H) | Unix-Berechtigungen oktal (`644`); kein Eigentümer/Gruppe. Unter Windows R, H, S und A wie in ZTreeWin |
| Sortieren | Name, Ext, Date, Time, Length, Alpha, nuMber, Size, Value, Unsorted; Charset; gleich/unterschiedlich bei geteiltem Bildschirm | Name, Ext, Date, Size, aufsteigend/absteigend, Pfad zuerst; jedes Fenster hat seine eigene Reihenfolge |
| Vergleichen | Optionen Identical, Unique, Newer, Older, Size, Binary, Case; Anzeigefilter | markiert Unique, Newer, Older, Identical, Different oder Unique+Newer; kein Binärvergleich |
| Prune | löscht nur Unterverzeichnisse oder auch das aktuelle; nur leere Verzeichnisse; Papierkorb | löscht immer das gesamte Verzeichnis endgültig; einstellbare Sicherheitsabfrage |
| Log | mehrere eingelesene Laufwerke, Global, Laufwerkswechsel | eine Wurzel pro Fenster; `L` öffnet ein anderes Verzeichnis |
| eXecute | Batch-Parameter `%1`–`%8`, neues Fenster (NTW) | `sh` mit `%f`, `%t` und `%d`; immer im Terminal von Xylem |
| Verlauf | Verlaufsfenster mit Lesezeichen | ↑/↓ in der Eingabezeile, ein Verlauf pro Eingabetyp, bleibt zwischen Sitzungen erhalten |
| Konfiguration | 7 Seiten, Farbeditor für einzelne Elemente (F4), Alt-Export | 4 Seiten mit den unter Linux sinnvollen Optionen; F4 wählt aus 12 fertigen Themen; Einzelfarben in `config.toml` |
| Geteilter Bildschirm | optionale Statistikfelder je Seite | kein Statistikfeld |
| Menüs | Ctrl-/Alt-Menü beim Gedrückthalten der Taste | ebenso in Terminals mit Kitty-Protokoll; F9 in jedem Terminal |

Konfigurationsoptionen aus ZTreeWin, die nicht übernommen wurden, da sie
Windows-spezifisch sind: RSH-Attribute, Junctions, OEM/ANSI, EBCDIC,
Fenstergröße, Verknüpfungen (.lnk).

## Wo Xylem weiter geht

Dinge, für die das ZTreeWin-v2.2-Handbuch keine Entsprechung hat:

- **Ein integriertes Terminal** (Alt+X): Die Shell läuft weiter, während Sie
  den Baum benutzen; ein `cd` in der Shell nimmt den Baum mit, und die Shell
  folgt dem Baum, wenn Sie zu ihr zurückkehren. Alt+O tippt die markierten
  Dateien in die Eingabezeile. Siehe [das integrierte Terminal](terminal.md).
- **Entfernte Rechner über SSH** (`xy benutzer@host:`): Baum, Branch,
  Betrachter und Kopieren funktionieren auf einem anderen Rechner, der auch
  sein eigenes Terminal hat. Siehe [entfernte Rechner](remote.md).
- **Git** (optional): Statusbuchstaben in der Dateiliste, ein Git-Menü
  (Ctrl+G) und Diffs im Betrachter. Siehe [Git](git.md).
- **Speicherbelegung** (Alt+W): die Ordner und Dateien eines Zweigs zusammen,
  die größten zuerst, bis hin zu dem, was die Platte füllt; Alt+K zeigt im
  Baum den Anteil jedes Ordners. Siehe [Speicherbelegung](usage.md).
- **Ein Betrachter, der mehr liest:** Syntaxfarben, die Textkodierung selbst
  erkannt (UTF-8, UTF-16, CP850, Windows-1252, CP437), `.gz`-, `.xz`-, `.zst`-
  und `.bz2`-Dateien entpackt angezeigt, und versteckte
  Textrichtungszeichen (Trojan Source) angezeigt statt befolgt. Siehe
  [den Betrachter](viewer.md).
- **Archive ohne externes Packprogramm:** ZIP, 7z und TAR (unkomprimiert, gzip,
  xz, bzip2, zstd) öffnen sich als Ordner, und ZIP und TAR werden erstellt,
  alles von Xylem selbst. Siehe [Archive](archives.md).
- **Stapelumbenennung** mit Suchen und Ersetzen (`s/foo/bar/`), Zählern und
  Groß-/Kleinschreibung. Siehe [Umbenennen](rename.md).
- **Oops:** O (oder Ctrl+Z) legt das zuletzt in den System-Papierkorb
  Verschobene an seinen Platz zurück. Siehe [Löschen](delete.md).
- **Die Zwischenablage überall:** Y kopiert den Pfad auch über SSH oder in
  tmux (OSC 52).
- **Was eine Datei ist:** ein Anzeigemodus (Alt+F), der im Sinne des Befehls
  `file` sagt, was jede Datei enthält, und ein weiterer mit Eigentümern und
  Berechtigungen.
- **Aussehen:** 12 fertige Themen und optionale Nerd-Fonts-Symbole. Jede Taste
  kann unter `[keys]` in `config.toml` neu belegt werden. Siehe
  [Konfiguration](configuration.md).
- **Feindselige Dateien:** Archive, Namen und Inhalte werden als potenziell
  feindselig behandelt, mit Grenzen für alles, was gelesen und entpackt wird
  (siehe die Sicherheitserklärung im README).
- **Hilfe in sechs Sprachen** (F1, dann L), derselbe Text wie dieses Handbuch
  und die Manpage.

## Was Xylem anderen Programmen überlässt

Manches macht Xylem absichtlich nicht selbst: Ein spezialisiertes Programm
kann es besser, und Xylem bindet es ein.

- **Hex-Bearbeitung:** ZTreeWin bearbeitet Bytes in seinem Betrachter. Xylem
  hat keinen eigenen Hex-Editor: Einen zu schreiben und zu pflegen wäre viel
  Arbeit für wenig Nutzen, und Bytes von Hand zu bearbeiten ist ein leichter
  Weg, eine Datei zu beschädigen. Stattdessen öffnet E auf einer Binärdatei
  `hexedit`, `heh`, `bvi` oder `dhex` (den installierten oder den von Ihnen
  gewählten) im Rahmen und in den Farben von Xylem, oder einen grafischen wie
  ImHex oder GHex. Siehe [Bearbeiten](execute.md).
- **Text bearbeiten:** Ihr Editor (`$VISUAL`, `$EDITOR` oder der in F12
  eingestellte), und auf Wunsch ein eigener Editor je Dateityp (`[editors]`).
- **SSH:** das `ssh` des Systems, sodass Ihre Schlüssel, der Agent,
  `~/.ssh/config` und Jump-Hosts wie überall sonst funktionieren.
- **Cloud-Speicher:** Google Drive, S3 und andere über `rclone mount`; Xylem
  erkennt die Einbindung und passt sich an. Siehe [Cloud-Speicher](cloud.md).
- **Git:** das Programm `git` selbst.
- **Dateien öffnen** (O): das Programm der Arbeitsumgebung für jeden Typ
  (`xdg-open`).
- **Den Inhalt zweier Dateien vergleichen** (JFC in ZTreeWin): Bis das
  eingebaut ist, markieren Sie die beiden Dateien und führen `diff %t` mit
  eXecute (X) aus, oder Sie nutzen das Terminal.
- **Drucken** (P in ZTreeWin): `lp %t` mit eXecute.
- **Andere Archivformate** (ZAAP in ZTreeWin): Ihr Packprogramm, mit eXecute
  oder im Terminal.

## Abweichende Tasten

Die Tastenbelegung folgt ZTreeWin, mit diesen Ausnahmen:

| Funktion | ZTreeWin | Xylem |
|---|---|---|
| Hilfe | F1 | F1 oder Alt+H (in GNOME Terminal öffnet F1 die Hilfe des Terminals) |
| Konfiguration | Alt+F10 | F12 oder Alt+F10 (unter GNOME maximiert Alt+F10 das Fenster und erreicht Xylem nicht) |
| Lesezeichen | F11 | F11, Alt+B oder einfaches Anführungszeichen `'` (in GNOME Terminal schaltet F11 das Vollbild um) |
| Markierte verschieben | Ctrl+M | Ctrl+M in Terminals mit Kitty-Protokoll; in anderen `F9` `M` (oder Alt+M und F6) |
| F2 | (nichts) | liest das Verzeichnis neu ein, genau wie F3 |
| F9 | Anwendungsmenü (nicht implementiert) | Ctrl-/Alt-Menüs |
| F12 | Makros (nicht implementiert) | Konfiguration |
| `/` | wie `?` (Statistiken) | Treespec, genau wie `\` (`?` zeigt weiterhin die Statistiken) |
| Ctrl+C, Ctrl+M im Baum | (nichts) | Kopieren/Verschieben der markierten Dateien des aktuellen Verzeichnisses |
| Pfad in Zwischenablage kopieren | Ctrl+Insert | Y oder Ctrl+Insert (Ctrl+Y für markierte; funktioniert über OSC 52 und System-Zwischenablage) |

Alle Tasten können unter `[keys]` in `config.toml` neu belegt werden.
