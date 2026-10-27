# Berechtigungen und Datum

## Berechtigungen (A)

`A` ändert die Berechtigungen der Datei unter dem Cursor (Ctrl+A: der
markierten Dateien) in oktaler Schreibweise: `644`, `755`. Alt+A funktioniert
wie Ctrl+A.

Unter Windows bearbeitet `A` die Attribute R, H, S und A wie in ZTreeWin: `RA`
setzt genau diese, `+H -A` schaltet einzelne ein oder aus. In der Dateiliste
werden sie als `r-sa` angezeigt. Siehe [Windows](windows.md).

## Neues Datum (N)

`N` ändert das Änderungsdatum der Datei oder des Verzeichnisses (Ctrl+N: der
markierten Dateien). Die Eingabezeile ist mit dem aktuellen Datum in Ihrem
Format vorausgefüllt; sie akzeptiert auch `now` und `today`. Wird nur das Datum
angegeben (ohne `HH:MM`), behält jede Datei ihre eigene Uhrzeit bei.
Symbolische Links werden übersprungen (das Datum würde sonst das der Zieldatei
ändern).

Das Datumsformat wird auf Seite 1 der [Konfiguration](configuration.md)
eingestellt.
