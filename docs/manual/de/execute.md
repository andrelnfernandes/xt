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

### Editoren nach Dateityp

Ein Texteditor beschädigt ein Bild oder ein Programm nur, daher öffnet E keine
**Binärdateien** (solche mit einem Null-Byte nahe am Anfang, die der Betrachter
im Hex-Modus anzeigt): Er weist in der Fußzeile darauf hin und schlägt `O` vor,
womit die Datei im zugehörigen Systemprogramm geöffnet wird. Um solche Dateien
zu bearbeiten oder bestimmten Dateien einen eigenen Editor zuzuweisen, tragen
Sie diese in der Tabelle `[editors]` der `config.toml` ein:

```toml
[editors]
"*.png *.jpg *.jpeg *.gif" = "gimp %f"
"*.md" = "typora"
"*.svg" = ""            # leer: der gewohnte Editor, obwohl das Muster passt
binary = "hexedit"      # jede andere Binärdatei
```

Jeder Schlüssel ist eine Liste von Namensmustern wie in [Filespec](filespec.md)
(Groß-/Kleinschreibung spielt keine Rolle); jeder Wert ist eine Befehlszeile
mit `%f` für die Datei (andernfalls wird die Datei ans Ende gehängt). Der erste
passende Schlüssel in der Reihenfolge der Datei gewinnt. `binary` gilt für
Binärdateien, auf die kein Muster zutraf; alle übrigen Dateien werden im
gewohnten Editor geöffnet. Befehle laufen genauso wie der Editor: standardmäßig
innerhalb des Xylem-Rahmens oder mit übergebenem Terminal, wenn
`editor_in_frame = false` gesetzt ist (für ein grafisches Programm, das
weiterlaufen soll, hängen Sie `&` an und setzen `editor_in_frame = false`, oder
verwenden Sie `O`).

Xylem bringt keinen eigenen Hex-Editor mit, und das ist eine bewusste
Entscheidung, kein Versäumnis: Es wäre viel Aufwand für geringen Nutzen, und
das manuelle Bearbeiten von Bytes führt leicht zu beschädigten Dateien. Wer
einen Hex-Editor benötigt, kann unter `binary` ein dafür geschaffenes Werkzeug
wählen (`hexedit`, `bvi`, `dhex`, `ImHex`...).

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

## Verlauf

In jeder Eingabezeile rufen ↑ und ↓ frühere Eingaben auf. Jede Art von
Eingabezeile (Filespec, Ziel, Log, eXecute...) hat ihren eigenen Verlauf,
gespeichert in `~/.local/state/xylem/history.toml`, nur für Sie lesbar, da er
eingegebene Befehle enthalten kann.
