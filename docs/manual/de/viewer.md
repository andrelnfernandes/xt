# Der Betrachter

`V` öffnet die Datei im Betrachter (Autoview, F7, zeigt sie neben der Liste
an). Binärdateien werden im Hex-Modus geöffnet. Bei Dateien über dem Größenlimit wird
nur der Anfang angezeigt.

| Taste | Funktion |
|---|---|
| ↑ ↓ PgUp PgDn Home End | scrollen |
| ← → | seitlich scrollen |
| `H` | Text oder Hex |
| `O` | nächste Kodierung (UTF-8, CP850, 1252, CP437, UTF-16) |
| `C` | Syntaxfarben ein oder aus |
| `W` | Zeilenumbruch ein oder aus |
| `#` oder `L` | Zeilennummern ein oder aus (im Textmodus) |
| `g` oder `:` | zu einer Zeile springen (oder Byte-Offset im Hex-Modus) |
| `E` | Datei bearbeiten; zurück in Xylem zeigt der Betrachter den neuen Inhalt an derselben Stelle (Binärdateien nur mit einem Editor unter [`[editors]`](execute.md)) |
| `D` | bei aktiviertem Git: Änderungen seit dem letzten Commit und zurück ([Git](git.md)) |
| `T` / `U` | Datei markieren / Markierung aufheben |
| `y` oder Ctrl+Insert | Dateipfad in die Zwischenablage kopieren |
| Esc | schließen |

## Suchen

Wie in ZTreeWin: **F** sucht vom Anfang an, **B** rückwärts vom Ende aus, **/**
(oder `S`) ab der aktuellen Seite und **\\** von dort rückwärts; **Space**
sucht in derselben Richtung weiter, **+** und **-** wählen die Richtung. Der
Treffer wird hervorgehoben. In der Sucheingabezeile schaltet **F2** die
Groß-/Kleinschreibung um (wird sie ignoriert, zählen auch Akzente und Umlaute
mit: `AÇÃO` findet `ação`) und **F4** wechselt zwischen Text und Hex
(Byte-Paare wie `4d 5a`). Ein `*` in der Mitte des Suchbegriffs steht für
beliebige Zeichen in derselben Zeile (`error*disk`).

## Syntaxfarben

Quellcode und Skripte werden in rund 150 Sprachen farbig hervorgehoben (Rust,
Python, Shell, C, JavaScript, SQL, YAML, Markdown...), erkannt anhand der
Erweiterung, des Namens (`Makefile`) oder der ersten Zeile (`#!/bin/bash`). Die
Standardeinstellung ist `syntax_highlight` (Seite 2 der
[Konfiguration](configuration.md)). Die Farben bleiben auf dem Hintergrund des
aktuellen Themas gut lesbar, und es werden nur die ersten 20.000 Zeilen
eingefärbt und keine Zeile mit mehr als 10.000 Zeichen, sodass auch riesige
Dateien den Betrachter niemals ausbremsen.

## Steuerzeichen für die Schreibrichtung

Manche unsichtbaren Zeichen ändern die Schreibrichtung des Textes und können Code anders aussehen lassen, als er wirklich lautet (Trojan Source, CVE-2021-42574). Der Viewer versteckt sie nie: Jedes erscheint als rotes `·`, und die Kopfzeile nennt ihre Anzahl.

## Textkodierung

Xylem erkennt die Kodierung selbst, in dieser Reihenfolge: zuerst wird die Datei entpackt, dann wird nach einer BOM gesucht (UTF-8, UTF-16 LE oder BE), dann nach UTF-16 ohne BOM, dann nach UTF-8. Passt nichts davon, ist es alter Text: Xylem rät CP850 (DOS, Westeuropa) oder Windows-1252, indem es die Buchstaben mit Akzent zählt. `.reg`-Dateien von Windows sind UTF-16 und öffnen sich als normaler Text.

**O** zeigt die Datei in der nächsten Kodierung: UTF-8, CP850, Windows-1252, CP437, UTF-16 LE, UTF-16 BE und wieder UTF-8. Die Fußzeile sagt, welche, und die Kopfzeile zeigt den Namen, wenn es nicht UTF-8 ist. Wählen Sie UTF-16 bei einer Datei, die in Hex geöffnet wurde, wechselt sie zu Text. Ihre Wahl bleibt, wenn Sie die Datei bearbeiten und zurückkehren.

Um für alte Textdateien immer eine feste Kodierung statt der Vermutung zu nutzen, setzen Sie `legacy_charset` auf `cp850`, `cp1252` oder `cp437` (**K** auf Seite 2 der [Konfiguration](configuration.md)). Standard ist `auto`.

## Größenlimit

Der Betrachter liest von einer großen Datei nur den Anfang, und die Kopfzeile zeigt `(showing first N MiB)`. Wie viel er liest, bestimmt die Option `viewer_max_size` (**L** auf Seite 2 der [Konfiguration](configuration.md)): `auto`, `16`, `64`, `256` oder `1024` (MiB). Bei `auto`, dem Standard, beträgt das Limit 1/32 des Arbeitsspeichers des Rechners, nie weniger als 16 MiB und nie mehr als 256 MiB (16 MiB, wenn sich der Arbeitsspeicher nicht auslesen lässt). Die Konfigurationsseite zeigt den verwendeten Wert, zum Beispiel `auto (256 MiB)`.

Zwei Fälle folgen dieser Regel nicht. Autoview liest immer 1 MiB. Eine Datei auf einem entfernten Rechner (SFTP) wird höchstens bis 16 MiB gelesen, weil Esc dieses Lesen nicht abbrechen kann.

Ein größeres Limit kostet Speicher und Zeit: Mit `1024` belegt eine gewöhnliche Textdatei dieser Größe etwa 2,5 GiB RAM und braucht rund 5 s zum Öffnen. Sehr kurze Zeilen kosten mehr als ihre Bytes, weil jede Zeile eigene Verwaltungsdaten hat, und dasselbe gilt für Tabulatoren und versteckte Steuerzeichen für die Schreibrichtung: Der Betrachter hält Zeilen bis etwa zum Doppelten des Größenlimits im Speicher, danach steht in der Kopfzeile `(showing first N lines)`. Ein Log mit gewöhnlichen Zeilen (etwa 100 Bytes) wird dadurch nie abgeschnitten.

## Komprimierte Dateien

Dateien mit der Endung `.gz`, `.xz`, `.zst` oder `.bz2` werden entpackt als Text geöffnet. Xylem prüft die ersten Bytes der Datei, nicht den Namen, und öffnet nur eine Ebene: Eine `.gz`-Datei in einer `.gz`-Datei bleibt komprimiert. Die Kopfzeile zeigt `gzip → N bytes` (das Format und die Größe nach dem Entpacken).

Das Entpacken hat Grenzen. Der Betrachter liest bis zum Größenlimit, braucht höchstens 3 s und nutzt höchstens 64 MiB Speicher für das xz-Wörterbuch oder das zstd-Fenster. In Autoview gelten 1 MiB, 100 ms und 16 MiB. Wird die Ausgabe- oder die Zeitgrenze erreicht, steht in der Kopfzeile `(showing first N bytes decompressed)`. Ein Hinweis sagt, was passiert ist:

- `decompression stopped after 3 s` (100 ms in Autoview)
- `compressed data truncated or corrupt` (komprimierte Daten abgeschnitten oder beschädigt)
- `needs more than 64 MiB of memory to decompress` (16 MiB in Autoview, mit ` (V to view)`, wenn die 64 MiB von V reichen können)

Der Hinweis verschwindet beim ersten Tastendruck, die Kopfzeile behält das Problem aber: `(truncated or corrupt)`, `(incomplete: …)`, wenn die Speichergrenze das Entpacken mittendrin gestoppt hat, oder `(not decompressed: …)`, wenn nichts entpackt werden konnte (dann erscheinen die Rohbytes in Hex).

Autoview zeigt den Hinweis in seinem Titel. Im Hex-Modus (`H`) sehen Sie die entpackten Bytes. Die Farben richten sich nach dem Namen ohne die Kompressionsendung: `app.rs.gz` wird wie Rust gefärbt.

Bei einer `.tar.gz` (oder `.tar.xz` ...) lautet der Hinweis `TAR archive: Alt+F5 opens it as a folder` (Alt+F5 öffnet sie als Ordner).

Um die rohen komprimierten Bytes zu sehen, schalten Sie `viewer_decompress` aus (**J** auf Seite 2 der [Konfiguration](configuration.md)).

Siehe auch: [markierte Dateien durchsuchen](search.md).
