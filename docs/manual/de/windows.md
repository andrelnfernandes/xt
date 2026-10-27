# Windows

Xylem lässt sich auch für Windows kompilieren (Windows Terminal oder klassische
Konsole). Es besteht dort die automatisierten Tests, wird aber im Alltag noch
wenig genutzt – rechnen Sie also noch mit kleineren Ecken und Kanten. Was sich
gegenüber Linux ändert:

- **Berechtigungen / Attribute** (`A`) bearbeitet R, H, S und A wie in
  ZTreeWin: `RA` setzt genau diese Attribute, `+H -A` schaltet einzelne ein
  oder aus. In der Dateiliste erscheinen sie als `r-sa`.
- **Versteckte** Dateien und Verzeichnisse sind solche mit dem Attribut H (ein
  mit einem Punkt beginnender Name ist unter Windows ein gewöhnlicher Name).
- **Ausführen (eXecute)** und **Bearbeiten (Edit)** laufen über `cmd`, mit
  Dateinamen in doppelten Anführungszeichen. Ein `%NAME%` innerhalb eines
  Dateinamens wird von `cmd` dennoch als Umgebungsvariable interpretiert.
- Der Standardeditor ist `notepad`, `O` verwendet `start`, und Ctrl+B schlägt
  `xy-batch.bat` vor (beginnend mit `@echo off`).
- Das Löschen einer schreibgeschützten Datei entfernt diese ohne
  Sonderbehandlung, genau wie `rm` unter Linux.
- Das Kopieren symbolischer Links erfordert den Entwicklermodus oder
  Administratorrechte.
- Der Anzeigemodus für Eigentümer (Alt+F) zeigt `-` an.
- Die Konfiguration liegt in `%APPDATA%\xylem\config.toml`, der Verlauf in
  `%LOCALAPPDATA%\xylem\`.
- Das integrierte Terminal (Alt+X) startet `cmd` (`%COMSPEC%`). Beim Betreten
  wechselt die Shell zwar in das Verzeichnis des Baums, der Baum folgt jedoch
  nicht den `cd`-Befehlen der Shell, und `$XY_TAGGED` steht nicht zur
  Verfügung.

Für Alt+Q in PowerShell siehe [Beenden](quit.md).
