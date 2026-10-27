# Geteilter Bildschirm

**F8** öffnet ein zweites Fenster, beginnend im selben Verzeichnis, und ein
erneutes Drücken von F8 kehrt zu einem einzelnen Fenster zurück. **Tab**
wechselt zwischen beiden (ohne Teilung bewirkt Tab dasselbe wie Enter). Jedes
Fenster hat seinen eigenen Baum, Cursor, [Filespec](filespec.md) und seine
eigene [Sortierreihenfolge](sort.md); mit `L` ([Log](treespec.md)) öffnen Sie
darin ein anderes Verzeichnis.

Bei geteiltem Bildschirm:

- schlagen [Kopieren und Verschieben](copy-move.md) das Verzeichnis des anderen
  Fensters als Ziel vor;
- vergleicht [Vergleichen](compare.md) mit dem anderen Fenster;
- wechselt Tab bei der [Ordnerauswahl](picker.md) (F2) zum Baum des anderen
  Fensters;
- werden beide Fenster nach jeder Operation aktualisiert.
