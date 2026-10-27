# Vergleichen

`C` im Baum vergleicht bei [geteiltem Bildschirm](split.md) die Dateien des
aktuellen Verzeichnisses mit denen im Verzeichnis des anderen Fensters und
markiert nach Ihrer Wahl:

- **U**nique: auf der anderen Seite nicht vorhanden;
- **N**ewer (neuer) und **O**lder (älter);
- **I**dentical: gleiche Größe und gleiches Datum;
- **D**ifferent (unterschiedlich);
- **A**ll new: nur hier vorhanden + neuer – alles, was fehlt, um die andere
  Seite auf den neuesten Stand zu bringen.

**B** (oder Alt+C im Baum, wie in ZTreeWin) erweitert den Bereich auf den
gesamten Zweig: Xylem liest den Zweig ein, vergleicht anhand der relativen
Pfade (`src/a.rs` ↔ `<andere>/src/a.rs`) und öffnet die Branch-Ansicht mit den
markierten Dateien.

Zeitstempel mit bis zu 2 Sekunden Unterschied gelten als gleich (FAT und exFAT
speichern Zeiten in 2-Sekunden-Schritten); `compare_ignore_1h` ignoriert
außerdem Unterschiede von genau einer Stunde (Sommerzeit). Beide Optionen
befinden sich auf Seite 4 der [Konfiguration](configuration.md).

Zum Synchronisieren: `C` `B` `A` und anschließend Alt+C ([Kopieren mit
Pfaden](copy-move.md)), wobei das andere Fenster bereits als Ziel
vorgeschlagen wird.
