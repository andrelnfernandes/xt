# Markierte Dateien durchsuchen

## Ctrl+S

Sucht nach Text in allen markierten Dateien der Liste und **hebt die Markierung
bei jenen auf, die ihn nicht enthalten**, mit Fortschrittsanzeige und `Esc` zum
Abbrechen. In der Eingabezeile schaltet **F2** die Beachtung der
Groß-/Kleinschreibung um (wird sie ignoriert, werden auch Akzente
berücksichtigt: `AÇÃO` findet `ação`) und **F4** wechselt zwischen Text und Hex
(Byte-Paare wie `4d 5a`). Ein `*` in der Mitte des Suchbegriffs passt auf
beliebigen Text in derselben Zeile (`error*disk`); ein Suchbegriff erstreckt
sich niemals über einen Zeilenumbruch hinweg. Dies funktioniert auch innerhalb
von [Archiven](archives.md).

## Ctrl+V

Öffnet die markierten Dateien nacheinander im [Betrachter](viewer.md): **N**ext
und **P**rev wechseln die Datei (Alt+Home und Alt+End: erste und letzte) und
**U** hebt die Markierung der aktuellen Datei auf. Der Suchbegriff wird mit
Ctrl+S geteilt: Nach einem Ctrl+S springt **Space** in Ctrl+V direkt zum
nächsten Treffer und wechselt am Dateiende automatisch zur nächsten markierten
Datei.

Ein typischer Ablauf: Dateien markieren (`T` oder Ctrl+T in einer
Branch-Ansicht), mit Ctrl+S nach einem Begriff filtern, Ctrl+V öffnen und mit
Space durch die Treffer über alle Dateien hinweg springen.
