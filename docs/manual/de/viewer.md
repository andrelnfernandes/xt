# Der Betrachter

`V` öffnet die Datei im Betrachter (Autoview, F7, zeigt sie neben der Liste
an). Binärdateien werden im Hex-Modus geöffnet. Bei Dateien über dem Größenlimit wird
nur der Anfang angezeigt.
Tabellenkalkulationen, CSV, DBF, SQLite und Parquet öffnen sich als Tabelle (siehe Tabellenmodus).

| Taste | Funktion |
|---|---|
| ↑ ↓ PgUp PgDn Home End | scrollen |
| ← → | seitlich scrollen |
| `H` | Text oder Hex |
| `O` | nächste Kodierung (UTF-8, CP850, 1252, CP437, UTF-16) |
| `C` | Syntaxfarben ein oder aus |
| `W` | Zeilenumbruch ein oder aus |
| `#` oder `L` | Zeilennummern ein oder aus (im Textmodus) |
| `g` oder `:` | zu einer Zeile springen (Byte-Offset im Hex-Modus, Tabellenzeile im Tabellenmodus) |
| `&` | zeigt nur passende Zeilen, einfacher Text oder Regex ([Zeilen filtern](filter.md)); im Tabellenmodus der SQL-Filter |
| `E` | Datei bearbeiten; zurück in Xylem zeigt der Betrachter den neuen Inhalt an derselben Stelle (Binärdateien nur mit einem Editor unter [`[editors]`](execute.md)) |
| `D` | bei aktiviertem Git: Änderungen seit dem letzten Commit und zurück ([Git](git.md)) |
| `T` | Tabelle oder Text, bei einer Datei, die sich als Tabelle geöffnet hat |
| `U` | in der Ansicht der markierten Dateien (Ctrl+V): Markierung der Datei aufheben |
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

## Tabellenmodus

Manche Dateien öffnen sich als Raster statt als Text. Xylem richtet sich nach der Endung, und nur bei unkomprimierten Dateien auf dieser Festplatte: Eine Datei auf einem entfernten Rechner oder in einem Archiv erscheint als gewöhnlicher Inhalt, mit dem Hinweis `Tables are shown only for files on this disk`.

| Format | Endungen | Spaltennamen |
|---|---|---|
| CSV, TSV | `.csv` `.tsv` `.tab` | die erste Zeile, wenn mindestens die Hälfte davon keine Zahlen sind; sonst A, B, C... |
| JSON Lines | `.jsonl` `.ndjson` | die Schlüssel der Objekte |
| Tabellenkalkulationen | `.xlsx` `.xls` `.xlsb` `.ods` | A, B, C... (die erste Zeile ist eine Zeile wie jede andere) |
| dBase, Clipper, FoxPro | `.dbf`, mit der Memodatei `.dbt` | die Feldnamen |
| SQLite | `.db` `.sqlite` `.sqlite3` | die Spalten der Tabelle |
| Parquet | `.parquet` | die Spalten der Datei |

Das CSV-Trennzeichen (Komma, Semikolon, Tabulator oder senkrechter Strich) wird am Anfang der Datei erkannt.

| Taste | Funktion |
|---|---|
| ↑ ↓ | vorherige oder nächste Zeile |
| ← → | vorherige oder nächste Spalte |
| PgUp PgDn | eine Seite Zeilen nach oben oder unten |
| Home End | erste oder letzte Spalte |
| `[` `]` | vorheriges oder nächstes Blatt einer Tabellenkalkulation oder vorherige oder nächste Tabelle einer SQLite-Datenbank, im Kreis; das löscht den Filter |
| `g` oder `:` | zu einer Zeile springen, gezählt werden nur die vom Filter behaltenen |
| `\` oder `&` | der SQL-Filter |
| `T` | zum Text der Datei und zurück |
| `H` | von der Tabelle zum Text, dann Hex, dann zurück zur Tabelle |

Der Rahmen zeigt `TABLE [2/5]`, wenn es mehrere Blätter oder Tabellen gibt, und die untere Zeile die Position des Cursors: `Cell B7 (Row 7 of 120)`. Die letzte Zeile des Rasters enthält den ganzen Zellinhalt und den aktiven Filter. Die Suche (`F`, `B`, `/`) schaut nicht in das Raster und findet dort nichts; sie funktioniert im Textmodus.

### Der SQL-Filter

`\` oder `&` fragt nach einer Bedingung, wie beim `WHERE` von SQL (das Wort `WHERE` ist optional), und zeigt nur die Zeilen, die sie erfüllen. Die Eingabe beginnt mit dem aktuellen Filter; lassen Sie sie leer, um wieder alle Zeilen zu sehen. Ein Hinweis meldet `Filter matched N rows` oder `SQL error: ...`; nach einem Fehler bleibt der vorherige Filter aktiv.

```
age >= 30 AND name LIKE 'Ali%'
WHERE status IN ('open', 'late') OR total > 1000.50
NOT (city = 'Rio' OR city IS NULL)
B ILIKE '%silva%'
```

| Operator | Funktion |
|---|---|
| `=` `!=` `<>` `<` `<=` `>` `>=` | vergleichen |
| `AND` `OR` `NOT` und Klammern | verknüpfen Bedingungen (`AND` vor `OR`) |
| `LIKE` `ILIKE` | passen auf ein Muster, `%` für beliebigen Text und `_` für ein Zeichen; `ILIKE` ignoriert die Groß- und Kleinschreibung |
| `IN (...)` `NOT IN (...)` | steht, oder steht nicht, in der Liste |
| `IS NULL` `IS NOT NULL` | die Zelle ist, oder ist nicht, leer |

Schlüsselwörter können groß oder klein geschrieben werden.

- **Spalten:** der Name der Spalte, ohne Anführungszeichen und in beliebiger Schreibung, oder ihr Buchstabe von `A` bis `Z`. Mit dem Buchstaben erreichen Sie einen Namen mit Leerzeichen oder Symbolen. Eine Spalte, die es nicht gibt, gilt ohne Fehler als leer.
- **Werte:** Text in einfachen oder doppelten Anführungszeichen (`\'` für ein Anführungszeichen darin), Zahlen, `TRUE`, `FALSE` und `NULL`. Was in Anführungszeichen steht, ist immer Text, nie eine Spalte.
- **Zellen:** Der Typ ergibt sich aus dem Aussehen der Zelle. Leer ist `NULL`, `true` und `false` sind Wahrheitswerte, Zahlen sind Zahlen und der Rest ist Text. Schreiben Sie also `code = 7`, nicht `code = '7'`: Text und Zahl sind nie gleich, und eine Zelle mit `007` ist die Zahl 7. Textvergleiche unterscheiden Groß- und Kleinschreibung.
- **Größe:** Ein Filter hat höchstens 1.024 Byte und 128 Terme.

### Grenzen

Eine Tabelle fasst höchstens 50.000 Zeilen und 256 Spalten. Was darüber hinausgeht, wird nicht gelesen: Es erscheint nicht, der Filter sieht es nicht, und nichts auf dem Bildschirm weist darauf hin. Eine Zelle zeigt höchstens 2.048 Zeichen und dann `…`. Steuerzeichen und Zeichen für die Schreibrichtung erscheinen als `·`.

## Tabellen in C-Bibliotheken

SQLite-Datenbanken (`.db`, `.sqlite`, `.sqlite3`) und Parquet-Dateien werden von in C geschriebenen Bibliotheken gelesen, die nicht die Speichersicherheit von Rust bieten. Deshalb öffnen sie sich nur mit `V` als Tabelle. Autoview liest alles, worüber der Cursor läuft, und zeigt diese Dateien als gewöhnlichen Inhalt mit dem Hinweis `Parquet or SQLite: V opens the table`. Öffnen Sie eine Datenbank aus nicht vertrauenswürdiger Quelle nur, wenn Sie dieses Risiko akzeptieren.

Siehe auch: [markierte Dateien durchsuchen](search.md).
