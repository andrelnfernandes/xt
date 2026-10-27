# Kopieren und Verschieben

`C` kopiert und `M` verschiebt die Datei unter dem Cursor; Ctrl+C und Ctrl+M
die markierten Dateien; Alt+C und Alt+M die markierten Dateien unter
Wiederherstellung ihrer Pfade. Die Eingabezeile fragt nach dem Ziel.

## Das Ziel

Ein Verzeichnis (relativ zum Verzeichnis unter dem Baum-Cursor, absolut oder
mit `~`) oder, bei einer einzelnen Datei, ein neuer Name. Bei
[geteiltem Bildschirm](split.md) wird das Verzeichnis des anderen Fensters
vorgeschlagen. **F2** öffnet einen Baum zur [Ordnerauswahl](picker.md); **↑**
ruft frühere Ziele auf.

Ein Zielordner, der noch nicht existiert, wird nach Ihrer Bestätigung erstellt.

## Mit oder ohne Pfade

Ctrl+C / Ctrl+M legen alle markierten Dateien direkt im Zielverzeichnis ab.
Alt+C / Alt+M stellen dort den Pfad jeder Datei wieder her; wo dieser Pfad
beginnt, hängt vom Modus ab. Für `/home/me/proj/src/ops/mod.rs`, aufgelistet ab
`src/ops` (dem Wurzelverzeichnis der Liste: dem aktuellen Verzeichnis, der
Branch-Wurzel oder der Baumwurzel in Showall), kopiert nach `dest`:

| Modus | Ergebnis | |
|---|---|---|
| Full | `dest/home/me/proj/src/ops/mod.rs` | ab der Wurzel des Laufwerks, wie in ZTreeWin (Standard) |
| Tree | `dest/src/ops/mod.rs` | ab der Wurzel des Xylem-Baums (hier `proj`) |
| Current | `dest/ops/mod.rs` | ab der Wurzel der Liste, einschließlich ihres Namens |
| Relative | `dest/mod.rs` | nur das, was unterhalb der Wurzel der Liste liegt |

In der Eingabezeile wechselt **F6** zwischen mit und ohne Pfade und **F7**
schaltet die Modi durch; beides wird direkt in der Eingabezeile angezeigt. Der
Startmodus ist `copy_paths` (Seite 4 der [Konfiguration](configuration.md)).
Innerhalb eines Archivs beginnen Pfade höchstens an der Wurzel des Archivs.

## Wenn die Datei bereits existiert

Xylem fragt **Yes** / **No** / **All** / **Skip all** und zeigt Größe und Datum
der neuen sowie der vorhandenen Datei an – und ob die vorhandene Datei neuer
oder älter, größer oder kleiner ist. `Esc` bricht den gesamten Vorgang ab.

## Sicherheit

Die Kopie wird zunächst in eine temporäre Datei geschrieben, die erst am Ende
das Ziel ersetzt: Ein Abbruch oder ein Fehler hinterlässt niemals eine halb
geschriebene Datei. Berechtigungen und Änderungszeitpunkt bleiben erhalten, und
symbolische Links werden als Links kopiert. Beim Verschieben über
Laufwerksgrenzen hinweg wird zuerst kopiert und anschließend gelöscht.

Siehe auch: [Markierungen](tagging.md), [Archive](archives.md) (Kopieren
entpackt aus ihnen).
