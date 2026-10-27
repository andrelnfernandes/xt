# Treespec und Log

## Treespec (`\` oder `/`)

Öffnet eine Eingabezeile mit dem aktuellen Pfad. Während Sie tippen, springt
der Baum-Cursor zum ersten Verzeichnis, das mit Ihrer Eingabe beginnt (ohne
Beachtung der Groß-/Kleinschreibung):

- `/` oder `\` vervollständigen den Namen und gehen eine Ebene tiefer;
- Tab und Shift+Tab vervollständigen und wechseln zum nächsten bzw. vorherigen
  Verzeichnis auf derselben Ebene;
- Backspace geht Ebenen nach oben zurück;
- Enter bestätigt und Esc kehrt zum Ausgangspunkt zurück;
- ↑ ruft frühere Pfade auf (bleiben zwischen Programmstarts erhalten).

Ein Pfad außerhalb des Baums wird beim Bestätigen wie ein Log geöffnet. In
Branch und Showall springt `\` direkt zum Verzeichnis der ausgewählten Datei
und hebt diese hervor.

## Log (L)

Öffnet ein anderes Verzeichnis im Fenster und ersetzt den Baum (ein Verzeichnis
innerhalb des aktuellen Baums bewegt lediglich den Cursor dorthin). Eine
Adresse wie `user@host:` öffnet einen [entfernten Rechner](remote.md). Der Pfad
kann absolut, relativ zum aktuellen Verzeichnis sein oder mit `~` beginnen;
**F2** [wählt ihn aus einem Baum aus](picker.md) und ↑ ruft frühere Pfade auf.
Neue Fenster behalten die aktuelle [Sortierreihenfolge](sort.md) und den
Anzeigemodus bei.

Siehe auch: [das Baumfenster](tree.md), [Lesezeichen](bookmarks.md).
