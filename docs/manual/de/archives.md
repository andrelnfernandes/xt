# Archive

## Ein Archiv öffnen (Alt+F5)

Alt+F5 im Dateifenster öffnet als Baum und schreibgeschützt:

- `.zip`, sowie `.jar`, `.war` und `.apk`;
- `.7z` (verschlüsselte 7z-Archive werden nicht unterstützt);
- `.tar`, unkomprimiert oder komprimiert mit gzip (`.tar.gz`, `.tgz`), xz
  (`.tar.xz`, `.txz`), bzip2 (`.tar.bz2`, `.tbz2`) oder zstd (`.tar.zst`).

Navigation, [Filespec](filespec.md), Branch, Showall, Markierungen,
[Betrachter](viewer.md), [Ctrl+S](search.md) und [Vergleichen](compare.md)
funktionieren darin wie gewohnt, und [Kopieren](copy-move.md) (auch Ctrl+C und
Alt+C mit Pfaden) entpackt die Dateien. `Esc` im Baum schließt das Archiv und
kehrt zum Ausgangspunkt zurück.

Aus Sicherheitsgründen werden Einträge mit `..` oder einem absoluten Pfad
ignoriert, und symbolische Links innerhalb von Archiven werden niemals
entpackt.

## Ein Archiv erstellen (F5 / Ctrl+F5)

Erstellt eine `.zip`-, `.tar`-, `.tar.gz`- oder `.tgz`-Datei (je nach dem von
Ihnen eingegebenen Namen) mit der aktuellen Datei oder den markierten Dateien;
F2 [wählt den Ordner aus](picker.md). Die Pfade im Archiv sind relativ zur
Wurzel der Liste, genau wie bei Alt+C: In einer Branch-Ansicht bleiben
Unterordner erhalten. Der Fortschritt wird angezeigt und `Esc` bricht ab; das
Archiv wird zunächst in eine temporäre Datei geschrieben und erhält erst am
Ende seinen endgültigen Namen. Symbolische Links werden als Links gespeichert.
Eine bereits vorhandene Datei wird nicht überschrieben.
