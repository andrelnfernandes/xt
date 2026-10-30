# Bearbeiten, Ausführen und Batch

## Bearbeiten (E)

Öffnet die Datei im Editor: dem auf Seite 1 der
[Konfiguration](configuration.md) eingestellten, oder andernfalls `$VISUAL`,
`$EDITOR`, `editor` unter Debian und Ubuntu, oder `vi`.

Standardmäßig öffnet sich der Editor **innerhalb des Xylem-Rahmens**
(`editor_in_frame = true` auf Seite 1 von **F12**) und verwendet **das
Farbthema von Xylem** (`editor_theme = true`), einschließlich passender Syntax-
und Oberflächenfarben zum aktiven Thema, wenn der Editor `vi`, `vim` oder
`nvim` ist. Setzen Sie `editor_in_frame = false`, um dem Editor stattdessen das
gesamte Terminal zu überlassen, oder `editor_theme = false`, um das eigene
Farbschema des Editors beizubehalten.

### Binärdateien und Editoren nach Dateityp

Ein Texteditor beschädigt ein Bild oder ein Programm nur, daher öffnet `E`
**Binärdateien** (solche mit einem Null-Byte nahe am Anfang, die der Betrachter
im Hex-Modus anzeigt) niemals im Texteditor.

Stattdessen öffnet `E`, wenn **Edit binary files with E** aktiviert ist
(`edit_binary = true`, Taste **I** auf Seite 1 von **F12**), Binärdateien in
einem Hex-/Binäreditor:

1. Ein passendes Muster in `[editors]` (falls vorhanden), oder
2. Den auf Seite 1 von **F12** eingestellten **Binary editor** (`binary_editor`,
   Taste **J**) bzw. `binary` in `[editors]`, oder
3. Wenn leer gelassen, den ersten im `PATH` gefundenen Terminal-Hex-Editor:
   - **Linux**: `hexedit`, `heh`, `bvi`, `dhex`
   - **Windows**: `heh` (`winget install ndd7xv.heh`), `hexedit`, `bvi`

Terminal-Hex-Editoren laufen genauso wie der Texteditor: standardmäßig
**innerhalb des Xylem-Rahmens** (`editor_in_frame = true`) und mit dem
**Farbthema von Xylem** (`editor_theme = true`). Für einen grafischen
Hex-Editor (`ImHex`, `GHex`, `Okteta` unter Linux; `ImHex`, `HxD` unter
Windows), der im eigenen Fenster weiterlaufen soll, hängen Sie `&` ans Ende des
Befehls an (zum Beispiel `imhex %f &` oder `ghex %f &`): Xylem startet ihn dann
direkt, ohne den Terminal-Rahmen zu öffnen.

Wenn Sie möchten, dass `E` überhaupt keine Binärdateien öffnet (und in der
Fußzeile lediglich `O` zum Öffnen im Systemprogramm vorschlägt), stellen Sie
**Edit binary files with E** auf `no` (`edit_binary = false` auf Seite 1 von
**F12**).

Um bestimmten Dateitypen einen eigenen Editor zuzuweisen, tragen Sie diese in
der Tabelle `[editors]` der `config.toml` ein:

```toml
[editors]
"*.png *.jpg *.jpeg *.gif" = "gimp %f &"
"*.md" = "typora %f &"
"*.svg" = ""            # leer: der gewohnte Editor, obwohl das Muster passt
binary = "hexedit"      # Rückfalloption für jede andere Binärdatei
```

Jeder Schlüssel ist eine Liste von Namensmustern wie in [Filespec](filespec.md)
(Groß-/Kleinschreibung spielt keine Rolle); jeder Wert ist eine Befehlszeile
mit `%f` für die Datei (andernfalls wird die Datei ans Ende gehängt). Der erste
passende Schlüssel in der Reihenfolge der Datei gewinnt. `binary` gilt für
Binärdateien, auf die kein Muster zutraf, wenn `binary_editor` leer ist; alle
übrigen Dateien werden im gewohnten Texteditor geöffnet.

Xylem bringt keinen eigenen Hex-Editor mit, und das ist eine bewusste
Entscheidung: Einen eigenen zu entwickeln und zu pflegen wäre viel Aufwand für
geringen Nutzen, und das manuelle Bearbeiten von Bytes führt leicht zu
beschädigten Dateien. Stattdessen integriert Xylem spezialisierte Hex-Editoren
direkt in seinen Rahmen und sein Farbthema — und lässt Sie das Bearbeiten von
Binärdateien mit einer einzigen Option unter **F12** abschalten, wenn Sie es
nicht wünschen.

## Ausführen – eXecute (X)

Fragt nach einer Befehlszeile und führt sie mit `sh` im aktuellen Verzeichnis
aus. Darin stehen:

| | |
|---|---|
| `%f` | die Datei unter dem Cursor |
| `%t` | die markierten Dateien |
| `%d` | das aktuelle Verzeichnis |
| `%%` | ein wörtliches `%` |

Alle Platzhalter werden bereits in Anführungszeichen eingesetzt:
`tar czf backup.tgz %t`. Am Ende wartet Xylem auf Enter, damit Sie die Ausgabe
lesen können, und liest die geöffneten Verzeichnisse neu ein. Ein mit `@`
beginnender Befehl wird nicht im Verlauf gespeichert (bei aktiviertem
`history_skip_at`).

Für längere Sitzungen ist das [integrierte Terminal](terminal.md) (Alt+X) oft
praktischer.

## Batch (Ctrl+B)

Fragt nach dem Namen der zu erstellenden Datei (F2 [wählt den Ordner
aus](picker.md)) und anschließend nach einer Zeilenmaske, und schreibt eine
Zeile pro markierter Datei. Parameter:

| | |
|---|---|
| `%f` | Pfad |
| `%d` | Verzeichnis |
| `%n` | Name |
| `%b` | Name ohne Erweiterung |
| `%e` | Erweiterung |
| `%s` | Größe |

In Kleinbuchstaben werden sie in Anführungszeichen ausgegeben, sicher für die
Shell (`convert %f %b.png`); in Großbuchstaben als reiner Text für Listen
(`%N;%S`). Eine `.sh`-Datei erhält `#!/bin/sh` und Ausführungsrechte (starten
Sie sie mit `X`). Eine bereits vorhandene Datei wird nicht überschrieben.

## Batch im Betrachter

Bei einer im [Betrachter](viewer.md) geöffneten Tabelle oder einem Text schreibt **Ctrl+B** eine Zeile pro Datensatz aus der ganzen Datei: über die 50.000 Zeilen des Bildschirms hinaus, mit vollständigen Werten, genau wie in der Datei. Der aktive Filter gilt (der SQL-Filter bei Tabellen, der [Zeilenfilter](filter.md) bei Text). Eine Tabelle wird dafür neu gelesen; Text kommt so, wie der Betrachter ihn gelesen hat, bis zum Größenlimit, und die Vorschau warnt, wenn das nicht die ganze Datei ist. Es fragt nach der Datei, dann nach der Maske, und zeigt vor dem Schreiben die ersten Zeilen: **Enter** schreibt, **Esc** gibt auf. Während des Schreibens bricht Esc ab und entfernt die unvollständige Datei. Eine vorhandene Datei wird nicht überschrieben.

| Feld | Wert |
|---|---|
| `%{name}` `%{A}` `%{2}` | eine Tabellenspalte, nach Name, Buchstabe oder Position |
| `%{line}` | die ganze Textzeile, mit ihren Tabulatoren |
| `%{ip}` `%{1}` `%{0}` | eine Gruppe des regulären Ausdrucks des Zeilenfilters, nach Name oder Nummer; 0 ist alles, was er gefunden hat |
| `%{#}` | die Nummer des Datensatzes unter den geschriebenen: 1, 2, 3... |
| `%{#line}` | die Nummer der Zeile in der Datei (nur Text) |
| `%{#path}` `%{#dir}` `%{#name}` `%{#base}` `%{#ext}` `%{#size}` | die geöffnete Datei, mit Escape wie jedes Feld; `%N` und die anderen oben haben keins, nehmen Sie in einer Seite oder einem Skript also besser diese |
| `%f %d %n %b %e %s` | die geöffnete Datei, wie beim Batch oben |
| `%%` | ein `%` |

Ein Feld, das es nicht gibt, ist ein Fehler in der Vorschau, nie ein leerer Wert. Da die Vorschau höchstens 1.000 Datensätze ansieht, gilt ein JSON-Lines-Schlüssel, der erst danach auftaucht, als nicht vorhanden.

Die Werte stammen aus einer möglicherweise feindlichen Datei und können in etwas gelangen, das Sie ausführen. Deshalb hat jedes Feld eine Maskierung. Schreiben Sie sie hinter das Feld: `curl "https://api/x?id=%{id:url}"`.

| Maskierung | Für | Standard bei |
|---|---|---|
| `:sh` | ein Argument in Anführungszeichen für einen Shell-Befehl | `.sh` `.bash` `.zsh` |
| `:url` | einen Teil einer URL, prozentkodiert | |
| `:json` | einen JSON-Wert: Zahlen ohne, Text mit Anführungszeichen | `.json` `.jsonl` `.ndjson` |
| `:xml` oder `:html` | Text oder Attribut | `.xml` `.html` `.htm` `.xhtml` `.svg` |
| `:sql` | ein Literal mit Anführungszeichen, `NULL`, `X'00ff'` | `.sql` |
| `:mysql` | dasselbe für MySQL und MariaDB, wo auch der Backslash maskiert | |
| `:csv` | ein Feld, bei Bedarf in Anführungszeichen | `.csv` `.tsv` |
| `:csvsafe` | dasselbe, mit einem `'` vor dem, was eine Tabellenkalkulation für eine Formel hielte | |
| `:line` | der Wert, Steuerzeichen als Leerzeichen, damit er nie eine neue Zeile beginnt | alle anderen |
| `:raw` | der Wert, wie er ist: **nicht in einem Skript verwenden** | |

- Ein Blob kommt hexadezimal, außer bei `:raw` und bei `:sql`, das eine eigene Form hat.
- Für MySQL und MariaDB nehmen Sie `:mysql`, in einer als `utf8mb4` gelesenen Datei: Dort kann ein Backslash ein `:sql`-Literal schließen, und die Vorschau warnt vor solchen Werten. Mit `NO_BACKSLASH_ESCAPES` auf dem Server ist `:sql` richtig.
- Der Filter einer Tabelle gilt für die ganze Datei, nach den Regeln des Bildschirms: `code = 7` findet auch eine Zelle mit `007`, die als `007` herauskommt. Die Datei kann also Zeilen enthalten, die der auf 50.000 begrenzte Bildschirm nicht zeigt.
- Eine `.sh` beginnt mit `#!/bin/sh`, wird aber **nicht ausführbar gemacht**: Lesen Sie sie vor dem Ausführen. `.bat` und `.cmd` werden abgelehnt, weil `cmd` `%VAR%` auch in Anführungszeichen auflöst.
- Vorschau und Abschlussmeldung warnen vor Werten mit Steuer- oder Textrichtungszeichen und in einer CSV vor Werten, die wie eine Formel beginnen. Die Vorschau zählt nur, was sie angesehen hat.

### Ganze Datensätze

Lassen Sie bei einer Datei `.json`, `.jsonl`, `.ndjson`, `.csv`, `.tsv`, `.xml`, `.html` oder `.htm` die Maske **leer**, um ganze Datensätze nach den Regeln des Formats zu schreiben: ein JSON-Array von Objekten, ein Objekt pro Zeile, eine Tabelle mit Kopfzeile, ein `<record>` pro Datensatz oder eine Seite mit einer Tabelle. Um nur einige Felder zu schreiben, beginnen Sie die Maske mit `@`: `@id, name, #`. Text liefert `line_number`, `line` und die Gruppen des Filters. Die Felder werden in den ersten 1.000 Datensätzen ermittelt: Ein JSON-Lines-Schlüssel, der erst später auftaucht, bleibt außen vor, und die Abschlussmeldung weist darauf hin.

### Vorlagen

Für Text um die Datensätze herum speichern Sie eine Vorlage als `~/.config/xylem/batch/name.tpl` und geben `:name` als Maske ein; die Eingabezeile listet die gefundenen auf (die ersten vier).

```
%%# ein Kommentar
%%header
<h1>%{#name}</h1>
<ul>
%%record
<li><a href="https://x/?id=%{id:url}">%{name}</a></li>
%%footer
</ul>
```

`%%header` und `%%footer` sind optional und erlauben nur den Text der Vorlage und die Felder der geöffneten Datei. `%%record` wird einmal pro Datensatz geschrieben und darf mehrere Zeilen haben; ohne Abschnitte ist die ganze Datei der Datensatz. Die Escapes richten sich wie bei einer Maske nach der Ausgabedatei, eine Vorlage für eine Seite sollte also in eine `.html` geschrieben werden. Ein unbekannter oder doppelter Abschnitt oder ein Datensatzfeld in Kopf oder Fuß ist ein Fehler in der Vorschau. Die Vorschau zeigt die letzten 3 Zeilen des Kopfes. Ohne Datensätze wird nichts geschrieben.

## Verlauf

In jeder Eingabezeile rufen ↑ und ↓ frühere Eingaben auf. Jede Art von
Eingabezeile (Filespec, Ziel, Log, eXecute...) hat ihren eigenen Verlauf,
gespeichert in `~/.local/state/xylem/history.toml`, nur für Sie lesbar, da er
eingegebene Befehle enthalten kann.
