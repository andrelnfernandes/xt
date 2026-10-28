# Das Dateifenster

Das Dateifenster listet die Dateien des Verzeichnisses unter dem Baum-Cursor
auf, oder die eines gesamten Zweigs (`B`) bzw. aller eingelesenen
Verzeichnisse (`S`). **Enter** kehrt zum Baum zurück, **Esc** ebenfalls (beim
Verzeichnis der ausgewählten Datei).

## Navigation

| Taste | Funktion |
|---|---|
| ↑ ↓ PgUp PgDn Home End | Cursor bewegen |
| ← → | seitenweise hoch und runter (in den Spaltenmodi: vorherige und nächste Datei) |
| Backspace | Dateien des übergeordneten Verzeichnisses anzeigen |
| Shift+Buchstabe, 0–9 | nächste Datei mit diesem Anfangsbuchstaben |
| Space | eine Zeile nach unten |

## Anzeigemodi (Alt+F)

Alt+F wechselt in der Reihenfolge von ZTreeWin zwischen **Namen in mehreren
Spalten**, **Namen und Größen in Spalten**, **Details** (eine Spalte mit Größe,
Datum und Berechtigungen, Standard) und **langen Namen** (eine Spalte, nur
vollständige Namen).

Vor den langen Namen gibt es zwei Modi, die ZTreeWin nicht hat:

- **owner**: Größe, `Benutzer:Gruppe` und Berechtigungen (wird unter Windows
  oder in Archiven nicht angezeigt);
- **type**: um was für eine Datei es sich handelt, im Sinne des Befehls `file`
  („PNG image, 800 x 600", „ELF 64-bit executable", „python3 script", „Rust,
  UTF-8 text"...), ermittelt aus den ersten Bytes der sichtbaren Dateien.

In den Spaltenmodi verlaufen die Dateien von links nach rechts, ← → bewegen um
eine Datei und ↑ ↓ um eine Zeile; Namen mit mehr als 20 Zeichen werden mit `…`
abgekürzt. Der anfängliche Modus wird über `file_display` in der
[Konfiguration](configuration.md) festgelegt.

## Autoview (F7)

Teilt den Bildschirm zwischen der Dateiliste und dem Inhalt der ausgewählten
Datei, der dem Cursor folgt. Die Tasten des Dateifensters funktionieren
weiterhin (`T` markiert beispielsweise); Shift+↑ ↓ PgUp PgDn Home End scrollen
den Inhalt; `W` schaltet den Zeilenumbruch um, `#` die Zeilennummern; F7, Esc
oder Enter beenden die Ansicht. Angezeigt wird das erste 1 MiB jeder Datei (`V`
zeigt bis zu 16 MiB).

## Dateien öffnen

- `V`: der [Betrachter](viewer.md) (Text oder Hex, mit Suche).
- `E`: der Editor ([Bearbeiten](execute.md)).
- `O`: das Standardprogramm des Systems (`xdg-open`: Bilder im Bildbetrachter,
  PDFs im PDF-Reader...) im Hintergrund. Ctrl+O öffnet alle markierten Dateien.
  Innerhalb eines Archivs wird der Eintrag zuerst in einen temporären Ordner
  entpackt, der beim Beenden von Xylem gelöscht wird. Ein anderes Programm kann
  unter `opener` eingestellt werden.
- Alt+F5: ein [Archiv](archives.md) wie ein Verzeichnis öffnen.

## Versteckte Richtungszeichen

Manche unsichtbaren Zeichen ändern die Schreibrichtung, sodass ein Name anders aussehen kann, als er wirklich lautet (Trojan Source, CVE-2021-42574; zum Beispiel kann `fatura\u{202E}fdp.sh` wie ein harmloses PDF aussehen). Xylem sendet sie nie an das Terminal: Im Namen erscheinen sie rot markiert, die Fußzeile warnt mit "Name has hidden bidirectional characters", wenn der Cursor auf einem solchen Namen steht, und `O` fragt vor dem Öffnen nach und zeigt den Namen mit den als `\u{XXXX}` ausgeschriebenen Zeichen.

## Die Zwischenablage

`Y` oder Ctrl+Insert kopiert den Pfad der Datei (im Baum den des
Verzeichnisses) in die Zwischenablage; Ctrl+Y kopiert die Pfade der markierten
Dateien, einen pro Zeile; `y` im Betrachter kopiert den Pfad der betrachteten
Datei. Dies funktioniert auch über SSH und tmux (OSC 52) und behält den Inhalt
der Zwischenablage nach dem Beenden von Xylem unter X11 und Wayland bei.

## Weitere Befehle

| Taste | Funktion |
|---|---|
| `T` / `U` | Datei markieren / Markierung aufheben und nach unten gehen ([Markierungen](tagging.md)) |
| `C` / `M` | Datei [kopieren / verschieben](copy-move.md) |
| `D` oder Del | Datei [löschen](delete.md) (Shift+Del: endgültig) |
| `R` | Datei [umbenennen](rename.md); Ctrl+R benennt markierte Dateien mit einer Maske um |
| `A` | [Berechtigungen](attributes.md) |
| `N` | [Datum ändern](attributes.md) |
| Alt+S | [Sortieren](sort.md) |
| Ctrl+S / Ctrl+V | [markierte Dateien durchsuchen / betrachten](search.md) |
| Ctrl+B | [Batch](execute.md): ein Skript mit einer Zeile pro markierter Datei |
| F5 / Ctrl+F5 | ein [Archiv](archives.md) mit der Datei / den markierten Dateien erstellen |
| Alt+I | Info: Details zur Datei ([Informationen](info.md)) |

Siehe auch: [das Baumfenster](tree.md), [die aktiven Tasten](keys.md).
