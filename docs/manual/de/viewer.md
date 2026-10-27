# Der Betrachter

`V` öffnet die Datei im Betrachter (Autoview, F7, zeigt sie neben der Liste
an). Binärdateien werden im Hex-Modus geöffnet. Bei Dateien über 16 MiB wird
nur der Anfang angezeigt.

| Taste | Funktion |
|---|---|
| ↑ ↓ PgUp PgDn Home End | scrollen |
| ← → | seitlich scrollen |
| `H` | Text oder Hex |
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
eingefärbt, sodass auch riesige Dateien den Betrachter niemals ausbremsen.

## Steuerzeichen für die Schreibrichtung

Manche unsichtbaren Zeichen ändern die Schreibrichtung des Textes und können Code anders aussehen lassen, als er wirklich lautet (Trojan Source, CVE-2021-42574). Der Viewer versteckt sie nie: Jedes erscheint als rotes `·`, und die Kopfzeile nennt ihre Anzahl.

## Komprimierte Dateien

Dateien mit der Endung `.gz`, `.xz`, `.zst` oder `.bz2` werden entpackt als Text geöffnet. Xylem prüft die ersten Bytes der Datei, nicht den Namen, und öffnet nur eine Ebene: Eine `.gz`-Datei in einer `.gz`-Datei bleibt komprimiert. Die Kopfzeile zeigt `gzip → N bytes` (das Format und die Größe nach dem Entpacken).

Das Entpacken hat Grenzen. Der Betrachter liest bis zu 16 MiB, braucht höchstens 3 s und nutzt höchstens 64 MiB Speicher für das xz-Wörterbuch oder das zstd-Fenster. In Autoview gelten 1 MiB, 100 ms und 16 MiB. Wird eine Grenze erreicht, steht in der Kopfzeile `(showing first N bytes decompressed)`, und ein Hinweis nennt den Grund:

- `compressed data truncated or corrupt` (komprimierte Daten abgeschnitten oder beschädigt)
- `decompression stopped after 3 s` (100 ms in Autoview)
- `needs more than 64 MiB of memory to decompress` (16 MiB in Autoview, mit ` (V to view)`)

Autoview zeigt den Hinweis in seinem Titel. Im Hex-Modus (`H`) sehen Sie die entpackten Bytes. Die Farben richten sich nach dem Namen ohne die Kompressionsendung: `app.rs.gz` wird wie Rust gefärbt.

Bei einer `.tar.gz` (oder `.tar.xz` ...) lautet der Hinweis `TAR archive: Alt+F5 opens it as a folder` (Alt+F5 öffnet sie als Ordner).

Um die rohen komprimierten Bytes zu sehen, schalten Sie `viewer_decompress` aus (**J** auf Seite 2 der [Konfiguration](configuration.md)).

Siehe auch: [markierte Dateien durchsuchen](search.md).
