# Speicherplatzbelegung (Alt+W)

Wohin der Speicherplatz geht. **Alt+W** liest den gesamten Zweig des aktuellen
Verzeichnisses ein und listet dessen Ordner und Dateien nach dem belegten Platz
auf, die größten zuerst, mit einem Balken und ihrem Anteil am Verzeichnis.
Enter wechselt in einen Ordner; Ordner für Ordner gelangen Sie so zu dem, was
die Festplatte füllt.

```
       32.9 GiB ████████████████   45%   wsl-ubuntu-backup.tar
       18.9 GiB █████████▏         26%  /projetos
 .      5.5 GiB ██▋               7.6%  /.local
 >    154.6 MiB                      -  /boot
```

- **Der Balken** wird im Verhältnis zum größten Element des Verzeichnisses
  gezeichnet, sodass die Unterschiede zwischen den Elementen auf einer Spur so
  breit wie das größte Element deutlich werden; **der Prozentsatz** ist der
  Anteil des Elements am Verzeichnis. Die Farbe zeigt an, wie hoch dieser
  Anteil ist: bis 10 % (grün in den meisten Themen), bis 40 %
  (gelb/bernsteinfarben), darüber (rot). Jedes Thema hat seine eigenen Farben
  (`bar_low`, `bar_mid` und `bar_high` unter `[colors]`).
- **Ordner und Dateien sind gemischt**, sortiert nach Größe: Eine einzelne
  große Datei (ein altes Backup, ein ISO-Abbild) erscheint direkt zwischen den
  Ordnern. Ordner beginnen mit `/`.
- **Die Zeile am unteren Rand** zeigt die Summen des Verzeichnisses: auf dem
  Datenträger, scheinbare Größe, die Anzahl der Dateien sowie deren Anteil am
  belegten Platz und am gesamten Datenträger. An der Wurzel eines Datenträgers
  gibt sie an, wie viel vom belegten Platz gefunden wurde: Was fehlt, steckt in
  Ordnern ohne Leseberechtigung, in den eigenen Datenstrukturen des
  Dateisystems oder in gelöschten, aber noch geöffneten Dateien.
- **Das Feld rechts** zeigt das Element unter dem Cursor: Platz auf dem
  Datenträger und scheinbare Größe, Dateianzahl, Datum sowie seinen Anteil am
  belegten Platz und am Datenträger.

## Auf dem Datenträger oder scheinbar

Standardmäßig werden Größen als **Platz auf dem Datenträger** angegeben, so wie
`du` ihn zählt: Eine Datei belegt ganze Blöcke (meist 4 KiB), sodass eine
kleine Datei mehr Platz als ihre eigentliche Größe einnimmt und eine
Sparse-Datei (etwa ein Abbild einer virtuellen Maschine) oder komprimierte
Datei weniger. **A** schaltet auf die **scheinbare** Größe um, also die eigene
Byte-Größe der Dateien. Die untere Zeile zeigt immer beide Werte an.

## Was die Analyse zählt

- **Jeweils ein Datenträger.** Ein anderes Dateisystem, das innerhalb des
  Zweigs eingehängt ist (eine weitere Festplatte, `/boot`, ein USB-Stick, ein
  mit rclone eingehängter Ordner, `/proc`), wird weder eingelesen noch
  mitgezählt: Es erscheint mit `>` und dem von diesem Dateisystem belegten
  Platz; Enter startet eine Analyse dieses Dateisystems.
- **Hardlinks zählen einmal.** Zwei Namen für dieselbe Datei belegen den Platz
  nur einmal: Der zuerst gefundene zählt, die weiteren erscheinen mit `H` und
  ohne Größenangabe.
- **Versteckte Dateien zählen mit**, selbst wenn der Baum sie ausblendet, damit
  die Summen stimmen.
- Symbolische Links (`@`) zählen nur mit der Größe des Links selbst; ihnen wird
  nicht gefolgt.

## Kennzeichen

| Zeichen | Bedeutung |
|---|---|
| `>` | anderer Datenträger, nicht mitgezählt |
| `!` | der Ordner konnte nicht gelesen werden (meist fehlende Berechtigung) |
| `.` | etwas darin konnte nicht gelesen werden: belegt möglicherweise mehr |
| `H` | weiterer Hardlink einer bereits gezählten Datei |
| `@` | symbolischer Link oder Spezialdatei |
| `e` | leerer Ordner |

## Tasten

| Taste | Funktion |
|---|---|
| ↑ ↓ PgUp PgDn Home End | bewegen |
| Enter oder → | in den Ordner wechseln (auf `>`: diesen Datenträger analysieren) |
| Backspace oder ← | zurück zum übergeordneten Ordner |
| `\` | Ansicht schließen und im Baum auf dem angezeigten Ordner landen |
| `A` | Platz auf dem Datenträger oder scheinbare Größe |
| `C` | Dateianzahl ein- oder ausblenden |
| `S` | nach Größe, Name, Dateianzahl oder Datum sortieren |
| `D` | im gezeigten Ordner nach [Duplikaten](duplicates.md) suchen |
| F3 | angezeigten Ordner neu einlesen |
| Esc oder Alt+W | schließen |

Tab, F8, Alt+X (das Terminal) und F12 funktionieren auch bei geöffneter Ansicht
wie gewohnt.

Die Analyse eines großen Datenträgers liest jeden Ordner in den Speicher ein:
Das dauert einen Moment (die Fußzeile zeigt den Fortschritt und Esc bricht ab)
und benötigt etwas Arbeitsspeicher, rund ein paar hundert Megabyte für
Millionen von Dateien. Als normaler Benutzer können einige Systemordner nicht
gelesen werden (`!`), weshalb die Summen von `/` niedriger ausfallen: Die Zeile
am unteren Rand zeigt die Differenz an.
