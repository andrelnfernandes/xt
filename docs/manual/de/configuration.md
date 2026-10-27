# Konfiguration

## Der Konfigurationsbildschirm (F12)

**F12** (oder Alt+F10 wie in ZTreeWin; unter GNOME maximiert Alt+F10 meist das
Fenster und erreicht Xylem nicht) öffnet vier Seiten:

| Seite | Optionen |
|---|---|
| 1. Programme und Formate | Editor, Editor im Xylem-Rahmen öffnen, Xylem-Thema im Editor, Hilfesprache, Programm zum Öffnen (Opener), Datumsformat, Zahlenformat, Git-Befehle |
| 2. Anzeige und Sortierung | versteckte Dateien, anfängliches Sortierkriterium und Richtung, Verhalten von Enter im Dateifenster, anfänglicher Anzeigemodus, Syntaxfarben, Zeilenumbruch und Zeilennummern im Betrachter, Farben des integrierten Terminals |
| 3. Bestätigungen und Verlauf | beim Beenden nachfragen, in den Papierkorb löschen, Prune-Bestätigung, Einträge mit `@` nicht speichern, eigener Verlauf des integrierten Terminals |
| 4. Kopieren, Vergleichen und Aktualisieren | 2 s ignorieren, 1 Std. ignorieren, automatische Aktualisierung, Startpunkt der Pfade bei Alt+C/Alt+M |

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
