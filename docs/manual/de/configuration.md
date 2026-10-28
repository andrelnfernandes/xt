# Konfiguration

## Der Konfigurationsbildschirm (F12)

**F12** (oder Alt+F10 wie in ZTreeWin; unter GNOME maximiert Alt+F10 meist das
Fenster und erreicht Xylem nicht) öffnet vier Seiten:

| Seite | Optionen |
|---|---|
| 1. Programme und Formate | Editor, Editor im Xylem-Rahmen öffnen, Xylem-Thema im Editor, Hilfesprache, Programm zum Öffnen (Opener), Datumsformat, Zahlenformat, Git-Befehle |
| 2. Anzeige und Sortierung | versteckte Dateien, anfängliches Sortierkriterium und Richtung, Verhalten von Enter im Dateifenster, anfänglicher Anzeigemodus, Syntaxfarben, Zeilenumbruch, Zeilennummern, komprimierte Dateien, die Kodierung alter Texte, das Größenlimit dessen, was der Betrachter liest, Icons und Symbole (Nerd Fonts), Farben des integrierten Terminals |
| 3. Bestätigungen und Verlauf | beim Beenden nachfragen, in den Papierkorb löschen, Prune-Bestätigung, Einträge mit `@` nicht speichern, eigener Verlauf des integrierten Terminals |
| 4. Kopieren, Vergleichen und Aktualisieren | 2 s ignorieren, 1 Std. ignorieren, automatische Aktualisierung, Startpunkt der Pfade bei Alt+C/Alt+M |

Das Größenlimit des Betrachters (**L** auf Seite 2) kostet Speicher und Zeit: Mit `1024` MiB belegt das Öffnen einer gewöhnlichen Textdatei dieser Größe etwa 2,5 GiB RAM und dauert rund 5 s, und Dateien mit sehr kurzen Zeilen kosten mehr pro Byte (siehe [den Betrachter](viewer.md)).

Icons und Symbole (**M** auf Seite 2) erfordern eine mit Nerd Fonts erweiterte Schriftart. Sie benötigen keine bestimmte Schriftart: Jede dicktengleiche Schrift funktioniert (wie Fira Code, JetBrains Mono, Hack, Meslo oder Cascadia Code), solange Sie die jeweilige „Nerd Font“-Variante wählen. Nerd Fonts fügt tausende Standard-Symbole in den privaten Unicode-Bereich ein. Herkömmliche Schriftarten ohne diese Erweiterung (selbst die originale Fira Code) enthalten diese Symbole nicht und zeigen leere Kästchen (`□`). So richten Sie sie ein:
- Laden Sie eine beliebige Nerd Font Ihrer Wahl von nerdfonts.com herunter oder installieren Sie ein Nerd-Fonts-Paket über Ihre Paketverwaltung.
- Wählen Sie diese Schriftart in den Einstellungen Ihres Terminal-Emulators aus.

Verrutschen die Spalten bei eingeschalteten Icons, stellt das Terminal vermutlich Zeichen „mehrdeutiger Breite“ breit dar (eine bei chinesischen, japanischen und koreanischen Schriften übliche Einstellung), während Xylem jedes Icon als eine Zelle zählt. Stellen Sie diese Option auf schmal oder schalten Sie die Icons aus.

Der Buchstabe der Option (oder Enter) ändert den Wert, PgUp/PgDn oder 1–4
wechseln die Seite, **F4** wählt das Thema aus einer Liste (mit sofortiger
Vorschau), **F3** speichert und wendet die Einstellungen an, **Esc** bricht ab
(und fragt bei ungespeicherten Änderungen nach).

## Die Konfigurationsdatei

Alle Einstellungen werden in `~/.config/xylem/config.toml` gespeichert (oder
unter `$XDG_CONFIG_HOME/xylem/`; unter Windows in `%APPDATA%\xylem\`). Die
Datei kann auch von Hand bearbeitet werden: Der Konfigurationsbildschirm lässt
Ihre Kommentare unangetastet. Um mit einer kommentierten Datei samt allen
Standardwerten zu beginnen:

```bash
mkdir -p ~/.config/xylem && xy --default-config > ~/.config/xylem/config.toml
```

- `theme`: `classic` (XTree-Blau), `dark`, `light`, `mono` (die eigenen Farben
  des Terminals), `norton`, `solarized`, `gruvbox`, `nord`, `slate` (im Stil
  von Windows Terminal), `graphite` (macOS im Dunkelmodus), `daylight` (macOS
  im Hellmodus) oder `vivid` (kräftige Farben für Präsentationen und
  Screenshots). F4 zeigt alle Themen mit einer Vorschau an.
- `[options]`: die Optionen des Konfigurationsbildschirms.
- `[colors]`: einzelne Farben aufbauend auf dem Thema, als Namen (`"yellow"`),
  `"#rrggbb"` oder Farbindex von 0 bis 255.
- `[keys.any]`, `[keys.tree]`, `[keys.files]`: Tasten in beiden Fenstern oder
  nur in einem Fenster neu belegen (letzteres hat Vorrang). Beispiel:
  `"f3" = "view"`, `"ctrl+r" = "relog"`; `"none"` deaktiviert eine Taste. Die
  Tastenliste der Hilfe (**K**) zeigt die tatsächlich aktiven Tasten
  einschließlich Ihrer Anpassungen.
- `[editors]`: der Editor für jeden Dateityp (`"*.png *.jpg" = "gimp %f"`,
  `binary = "hexedit"`); siehe [Bearbeiten](execute.md).

Ein Fehler in der Datei hindert Xylem nicht am Starten: Es verwendet dann die
Standardwerte, zeigt den Fehler in der Fußzeile an, und der
Konfigurationsbildschirm speichert erst wieder, wenn die Datei korrigiert
wurde, damit Ihre manuellen Einträge nicht überschrieben werden.

Bis Version 0.1 hieß Xylem noch xt. Beim ersten Start verschiebt es die
Konfiguration, den Verlauf und die Lesezeichen von xt automatisch in seine
eigenen Ordner.
