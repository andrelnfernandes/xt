# Markierungen und Gruppenoperationen

Markierte Dateien (gekennzeichnet mit `■`) sind das Ziel der
Gruppenoperationen: bearbeitet werden die markierten Dateien **der aktuellen
Liste** (des Verzeichnisses oder der geöffneten Branch- bzw. Showall-Ansicht).

## Markieren

| Taste | Baum | Dateifenster |
|---|---|---|
| `T` / `U` | Dateien des Verzeichnisses markieren / Markierung aufheben | Datei markieren / Markierung aufheben und nach unten gehen |
| Ctrl+T / Ctrl+U | alles Eingelesene (passend zum Filespec) markieren / Markierung aufheben | gesamte Liste markieren / Markierung aufheben |
| Ctrl+S | Showall nur der markierten Dateien | |
| `I` | Markierungen des Verzeichnisses umkehren | Markierungen der Liste umkehren |
| Ctrl+F7 / F8 / F9 | eingelesene Dateien des Zweigs markieren / Markierung aufheben / umkehren | |

## Operationen auf markierten Dateien

| Taste | Funktion |
|---|---|
| Ctrl+C / Ctrl+M | in das Zielverzeichnis [kopieren / verschieben](copy-move.md) |
| Alt+C / Alt+M | kopieren / verschieben, **unter Wiederherstellung ihrer Pfade** |
| Ctrl+D | [löschen](delete.md) |
| Ctrl+A | [Berechtigungen](attributes.md) ändern |
| Ctrl+N | [Datum](attributes.md) ändern |
| Ctrl+R | mit einer Maske [umbenennen](rename.md) |
| Ctrl+B | [Batch](execute.md): ein Skript oder eine Liste mit einer Zeile pro Datei |
| Ctrl+F5 | in ein [Archiv](archives.md) packen |
| Ctrl+S / Ctrl+V | [nach Text durchsuchen / betrachten](search.md) |
| Ctrl+O | im Standardprogramm öffnen |
| Ctrl+Y | ihre Pfade in die Zwischenablage kopieren |
| Alt+O | in das [integrierte Terminal](terminal.md) einfügen |

Bei eXecute steht `%t` für die markierten Dateien ([Ausführen](execute.md)).

## Ctrl- und Alt-Menüs (F9)

Wie in ZTree zeigt die Fußzeile standardmäßig nur die Befehle mit einer
einzelnen Taste. **F9** schaltet zum Ctrl-Menü um, dann zum Alt-Menü und wieder
zurück. Ist eines dieser Menüs geöffnet, zählt der nächste Buchstabe als
Ctrl+Buchstabe bzw. Alt+Buchstabe (`F9` `M` = Ctrl+M); `Esc` bricht ab. In
Terminals mit dem Kitty-Tastaturprotokoll genügt es, **Ctrl oder Alt gedrückt
zu halten**, um das entsprechende Menü anzuzeigen.

**Ctrl+M:** In den meisten Terminals wird Ctrl+M genau wie Enter übertragen.
Xylem aktiviert das Kitty-Tastaturprotokoll, wenn das Terminal es unterstützt
(kitty, WezTerm, foot, Ghostty, aktuelles Alacritty, iTerm2), sodass Ctrl+M die
markierten Dateien wie in ZTree verschiebt. Verwenden Sie in VTE-basierten
Terminals (GNOME Terminal, Ptyxis, Terminator, Tilix) stattdessen **F9 M**
(oder Alt+M und F6 in der Eingabezeile).
