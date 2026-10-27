# Informationen

- **Info (Alt+I):** Typ, Größe, Berechtigungen, Eigentümer und Gruppe,
  Zeitstempel (Änderung, Zugriff und Statusänderung), Inode, Links und Gerät
  der Datei oder des Verzeichnisses; bei einem Link das Ziel; bei einem
  Verzeichnis außerdem, was in ihm und in seinem Zweig eingelesen ist.
- **Avail (`A` im Baum):** die eingehängten Dateisysteme mit Gesamtplatz,
  belegtem und freiem Platz sowie Belegung in %, ähnlich wie `df`.
- **Statistiken (`?`):** was das Fenster eingelesen hat, die aktuelle Liste,
  die markierten Dateien und der Datenträger (Einhängepunkt, frei und belegt);
  darunter der Rechner (System und Kernel, CPU, Laufzeit, Auslastung,
  Arbeitsspeicher und Swap, Shell, Terminal), das Netzwerk (Adressen jeder
  Schnittstelle, Gateway und DNS) sowie Xylem selbst (Version, PID,
  Speicherverbrauch, Programmdatei und Konfigurationsdatei).

## Verzeichnisgrößen (Alt+K)

Zeigt rechts neben jedem Verzeichnis im Baum die **S**ize (Gesamtgröße in K),
den **C**ount (Dateianzahl) oder **P**ercent (Prozentsatz); **N**one schaltet
die Anzeige aus. Vor der Auswahl legen **A**ll oder **T**agged fest, ob alle
oder nur die markierten Dateien gezählt werden, und **B**ranch oder
**D**irectory, ob der gesamte Zweig oder nur das einzelne Verzeichnis summiert
wird. Noch nicht eingelesene Verzeichnisse zeigen `?`. Jedes Fenster hat seine
eigene Einstellung.

**Percent** zeigt an, welchen Anteil des übergeordneten Verzeichnisses jeder
Zweig einnimmt, zusammen mit einem Balken: Das größte Verzeichnis unter seinen
Geschwistern füllt ihn aus, und die übrigen werden proportional gezeichnet,
sodass der Platzfresser sofort auffällt. Seine Farbe gibt an, wie groß der
Anteil am übergeordneten Verzeichnis ist: bis 10 % (grün in den meisten
Themen), bis 40 % (gelb/bernsteinfarben) oder mehr (rot). Hierbei werden immer
ganze Zweige summiert. Die Dateien des übergeordneten Verzeichnisses zählen zu
dessen Gesamtwert, weshalb die Anteile seiner Unterverzeichnisse zusammen
weniger als 100 % ergeben können. Um herauszufinden, was eine Festplatte füllt,
lesen Sie den Zweig zuerst mit `*` ein: Ein noch nicht eingelesenes Verzeichnis
gilt als leer. Ist Google Drive oder S3 über rclone eingehängt, finden Sie so
heraus, was dort den Platz belegt: siehe [Cloud-Speicher](cloud.md). Für einen
ganzen Datenträger leistet die [Speicherplatzbelegung](usage.md) (Alt+W) noch
mehr: Dateien und Ordner gemeinsam, tatsächlicher Platz auf dem Datenträger und
einfache Zählung von Hardlinks.
