# Umbenennen und Stapelumbenennung

`R` benennt die Datei (im Baum das Verzeichnis) unter dem Cursor um: Geben Sie
den neuen Namen ein. Ein bereits vergebener Name wird abgelehnt.

## Stapelumbenennung (Ctrl+R oder Alt+R)

Benennt alle markierten Dateien anhand einer Maske um, nummeriert in der
Reihenfolge der Liste:

- **Platzhalter:** `*.jpg` (ändert die Erweiterung), `neu_*.*`, `*_alt.*`,
  `???.*` (behält genauso viele Zeichen bei). `*.*` lässt die Namen
  unverändert; `*.` entfernt die Erweiterung.
- **Suchen und Ersetzen:** `s/foo/bar/` ersetzt jedes `foo` durch `bar`;
  `s/foo/bar/i` ignoriert die Groß-/Kleinschreibung.
- **Zähler:** `%d`, `%02d`, `%03d`, `%04d`, `<#>`, `<01>`, `<001>`, `<0001>`
  nummerieren die Dateien (1, 01, 001...): `foto_<001>.*` ergibt
  `foto_001.jpg`, `foto_002.jpg`...
- **Groß-/Kleinschreibung:** `<lower>`, `<upper>`, `<title>` (auch für den
  Namen- oder Erweiterungsteil: `*.<lower>`), oder **F2** in der Eingabezeile,
  um zwischen Kleinbuchstaben, GROSSBUCHSTABEN und Titel-Schreibweise für das
  Ergebnis zu wechseln.

Würde die Maske zwei Dateien denselben Namen geben, wird nichts umbenannt.
Gehört ein neuer Name zu einer Datei, die selbst noch umbenannt werden soll,
fragt Xylem vor dem Überschreiben nach, genau wie bei [Kopieren und
Verschieben](copy-move.md).

Siehe auch: [Markierungen](tagging.md).
