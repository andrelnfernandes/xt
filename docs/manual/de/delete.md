# Löschen, Papierkorb und Prune

## Löschen

`D` oder Del löscht die Datei unter dem Cursor (im Baum das Verzeichnis, nur
wenn es leer ist); Ctrl+D löscht die markierten Dateien. Xylem fragt immer
zuerst nach.

Mit `delete_to_trash = true` in der [Konfiguration](configuration.md) wandern
Dateien und leere Verzeichnisse in den System-Papierkorb (FreeDesktop Trash
unter Linux, Papierkorb unter Windows), anstatt entfernt zu werden. Bei der
Rückfrage verschiebt **T** sie in den Papierkorb und **P** löscht sie
endgültig, unabhängig von der Einstellung. **Shift+Del** löscht immer
endgültig.

## Oops

**Oops** (`O` im Baum oder Ctrl+Z in beiden Fenstern) stellt das zuletzt in den
Papierkorb verschobene Element an seinem ursprünglichen Ort wieder her. Wurde
in dieser Sitzung noch nichts in den Papierkorb verschoben, wird das neueste
Element im System-Papierkorb angeboten.

## Prune (Alt+P im Baum)

Löscht das Verzeichnis samt seinem gesamten Inhalt. Xylem liest den gesamten
Zweig ein, zeigt an, wie viele Dateien, Verzeichnisse und Bytes entfernt
werden, und fordert Sie auf, **PRUNE einzugeben** (oder den Namen des
Verzeichnisses bzw. Y, abhängig von `prune_confirmation`). Es löscht zuerst die
Dateien und dann die Verzeichnisse von der tiefsten Ebene nach oben, mit
Fortschrittsanzeige und `Esc` zum Abbrechen. Symbolische Links werden entfernt,
ohne ihnen zu folgen, und **andere Einhängepunkte innerhalb des Zweigs werden
übersprungen** (ein dort eingehängter USB-Stick wird nicht gelöscht). Die
Wurzel des Baums kann nicht mit Prune gelöscht werden. Prune löscht immer
endgültig.
