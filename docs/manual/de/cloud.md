# Cloud-Speicher (rclone)

Xylem besitzt keinen eigenen Client für Google Drive oder S3, braucht aber auch
keinen: [rclone](https://rclone.org) hängt diese Dienste (und rund 70 weitere:
OneDrive, Dropbox, Backblaze B2, Cloudflare R2, MinIO...) als Ordner in das
Dateisystem ein, und Xylem arbeitet mit diesem Ordner wie mit jedem anderen.
Dort funktioniert alles: Einlesen, Branch, Markierungen, Betrachter, Kopieren
und Verschieben in beide Richtungen, Umbenennen, Verzeichnis erstellen und
Löschen.

## Warum rclone

Wir haben uns entschieden, mit rclone zusammenzuarbeiten, statt direkt mit
Google Drive und Amazon S3 zu kommunizieren. Eigener Code für jeden Dienst wäre
eine weitere Fehlerquelle, und Google Drive bräuchte zudem die Einhaltung der
Google-Richtlinien: Eine Anwendung, die alle Dateien eines Nutzers lesen kann,
erfordert eine Verifizierung durch Google sowie eine jährliche
Sicherheitsprüfung – andernfalls zeigt sie eine Warnung vor einer „nicht
überprüften App" und ist auf wenige Testnutzer beschränkt. Für S3 ist die
offizielle Bibliothek sehr umfangreich und würde erhebliche Komplexität
mitbringen, die Xylem konzeptionell nach Möglichkeit vermeidet. rclone erledigt
all das bereits zuverlässig für viele Dienste gleichzeitig.

## Einrichtung (einmalig)

Installieren Sie rclone (`apt install rclone`, `dnf install rclone`,
`brew install rclone` oder von [rclone.org](https://rclone.org/downloads/)) und
richten Sie mit `rclone config` eine Gegenstelle (Remote) ein. Sie vergeben
einen Namen, der in den folgenden Befehlen vor dem Doppelpunkt steht.

- **Google Drive:** Speichertyp `drive`; die Standardwerte genügen. rclone
  öffnet den Browser, damit Sie sich bei Google anmelden und den Zugriff
  erlauben können. Auf einem Rechner ohne Browser (etwa über SSH) antworten Sie
  auf die Browser-Frage mit *no*: rclone zeigt dann einen Befehl mit
  `rclone authorize` an, den Sie auf einem Rechner mit Browser ausführen.
- **Amazon S3:** Speichertyp `s3`, Anbieter `AWS`, anschließend entweder
  Access-Key und Secret eingeben oder *env_auth* wählen, um die vorhandenen
  Zugangsdaten der AWS-Werkzeuge zu nutzen (`~/.aws/credentials`,
  Umgebungsvariablen, IAM-Rolle der Instanz). Weitere S3-kompatible Dienste
  (R2, MinIO, Wasabi...) finden sich als eigene Anbieter in derselben Liste.

`rclone lsd gdrive:` (oder `rclone lsd s3:`) listet die obersten Ordner auf (in
S3 die Buckets): Wenn das klappt, ist der Remote einsatzbereit.

## Einhängen und Öffnen

Unter Linux, mit einem Remote namens `gdrive`:

```bash
mkdir -p ~/gdrive
rclone mount gdrive: ~/gdrive --vfs-cache-mode writes --daemon
xy ~/gdrive
```

Für S3 gilt dasselbe mit `s3:` (alle Buckets) oder `s3:mein-bucket` (ein
einzelner Bucket). `fusermount -u ~/gdrive` hängt den Ordner wieder aus.

- `--vfs-cache-mode writes` hält beim Schreiben eine lokale Kopie der Dateien
  vor, was Editoren (`E`) und manche Programme benötigen.
- `--dir-cache-time 1h` behält Verzeichnislisten länger im Zwischenspeicher als
  die voreingestellten 5 Minuten: Ein erneutes Einlesen (F3) geht dadurch
  schneller, aber an anderer Stelle vorgenommene Änderungen werden erst später
  sichtbar.

Unter **macOS** benötigt `rclone mount` [macFUSE](https://osxfuse.github.io);
`rclone nfsmount gdrive: ~/gdrive` kommt ohne Zusatzsoftware aus. Unter
**Windows** wird [WinFsp](https://winfsp.dev) benötigt und als
Laufwerksbuchstabe eingehängt: `rclone mount gdrive: X:` und anschließend
`xy X:\`.

Um den Ordner jederzeit griffbereit zu haben, speichern Sie ihn als
[Lesezeichen](bookmarks.md) (Alt+B).

Unter Linux erkennt Xylem von rclone eingehängte Ordner: Im Feld DISK wird der
Dienst angezeigt (Google Drive, Amazon S3, OneDrive...) und darunter der Remote
(`rclone gdrive:`). Info (Alt+I) und Avail (`A`) zeigen diese Angaben ebenfalls
an. Der Dienst wird aus der rclone-Konfigurationsdatei ermittelt
(`~/.config/rclone/rclone.conf` oder `$RCLONE_CONFIG`); ist diese
verschlüsselt, steht im Feld lediglich rclone.

## Was dort anders ist

- **Alles läuft über das Netzwerk.** Jedes gelesene Verzeichnis ist eine
  Netzwerkanfrage: Auf einem großen Drive oder Bucket können `*` (gesamten
  Zweig einlesen) und Branch (`B`) eine Weile dauern. Das gezielte Öffnen nur
  der benötigten Verzeichnisse (→ oder `+`) geht hingegen schnell, da sie erst
  beim Öffnen gelesen werden.
- **Anderswo vorgenommene Änderungen** (im Browser, auf dem Smartphone)
  erscheinen erst nach F3 und nachdem der Verzeichnis-Cache abgelaufen ist
  (`--dir-cache-time`).
- **Löschen:** Antworten Sie mit `P` (endgültig) statt mit `T`: Unter Linux ist
  der Papierkorb eines eingehängten Ordners ein `.Trash-`-Verzeichnis (mit
  Ihrer Benutzer-ID) auf dessen oberster Ebene, sodass die Datei im
  Cloud-Speicher verbleiben und weiter Platz belegen würde. Google Drive
  bewahrt von rclone gelöschte Dateien ohnehin im eigenen Drive-Papierkorb auf;
  auf S3 ist eine gelöschte Datei endgültig weg, sofern der Bucket keine
  Versionierung nutzt.
- **Berechtigungen und Eigentümer** werden nicht gespeichert: Das eingehängte
  Dateisystem zeigt für jede Datei dieselben Werte an, und `A` kann sie nicht
  ändern. Zeitstempel bleiben erhalten.
- **Google Docs, Sheets und Slides** sind keine normalen Dateien: rclone zeigt
  sie als exportierte Dokumente (`.docx`, `.xlsx`, `.pptx`) mit der Größe 0 an.
  `--drive-skip-gdocs` blendet sie aus.
- **S3 kennt keine echten Verzeichnisse**, sondern nur Dateinamen mit `/`: Ein
  leeres Verzeichnis kann nach dem Aushängen verschwinden, und beim Umbenennen
  oder Verschieben eines Verzeichnisses wird jede enthaltene Datei einzeln
  kopiert und gelöscht.

## Herausfinden, was Ihr Google Drive füllt

Wenn Google Drive ständig nach einem größeren Speicherplan fragt, kann Xylem
Ihnen zeigen, wohin der Platz geht, bevor Sie für mehr Speicher bezahlen.
Hängen Sie das Drive ein, wechseln Sie in sein Wurzelverzeichnis, lesen Sie mit
`*` alles ein (auf einem großen Drive dauert das etwas) und wählen Sie Alt+K →
**Percent** ([Verzeichnisgrößen](info.md)): Jeder Ordner zeigt nun an, welchen
Anteil des übergeordneten Ordners er belegt, samt Balken im Vergleich zum
größten Geschwisterordner. Folgen Sie den längsten Balken hinab zu alten
Backups, Videos und Duplikaten.

Die [Speicherplatzbelegung](usage.md) (Alt+W an der Wurzel des Drives) geht
noch weiter: Sie listet Dateien und Ordner gemeinsam auf, die größten zuerst,
sodass eine einzelne riesige Datei genauso sofort ins Auge fällt wie ein voller
Ordner. Und Branch (`B`) listet alle Dateien eines gesamten Ordners auf:
[Sortiert](sort.md) nach Größe, die größten zuerst (Alt+S, dann `O` für
absteigend und `S` für Größe), stehen die größten Dateien ganz oben.

Einige Dinge zählen zum Speicherplan hinzu, ohne im eingehängten Ordner
aufzutauchen:

- **Der Papierkorb von Google Drive** belegt Platz, bis er geleert wird (oder
  30 Tage vergangen sind). `rclone about gdrive:` zeigt Gesamtplatz, belegten
  Platz und den Umfang des Papierkorbs an; ein Einhängen mit
  `--drive-trashed-only` zeigt ausschließlich den Inhalt des Papierkorbs.
- **Gmail und Google Fotos** teilen sich denselben Speicherplatz, während im
  eingehängten Ordner nur die Dateien aus Google Drive erscheinen. Wenn
  `rclone about` deutlich mehr belegten Platz meldet als die Summe im
  eingehängten Ordner, steckt der Rest wahrscheinlich dort.
- **Google Docs, Sheets und Slides** werden mit der Größe 0 angezeigt, neuere
  Dokumente zählen jedoch trotzdem zum Speicherverbrauch.
- **Für Sie freigegebene Dateien** zählen nicht zu Ihrem Kontingent: Sie liegen
  im Speicher ihres jeweiligen Eigentümers.
