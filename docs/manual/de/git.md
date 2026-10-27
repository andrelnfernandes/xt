# Git

Git gab es weder in XTree noch in ZTreeWin, daher ist es standardmäßig
deaktiviert: Schalten Sie es mit F12 (Seite 1, „Git-Befehle") oder über
`git = true` unter `[options]` in der [Konfiguration](configuration.md) ein.
Das Programm `git` muss installiert sein.

## Das Git-Menü (Ctrl+G)

Ctrl+G öffnet das Git-Menü in der Fußzeile; der nächste Buchstabe wählt den
Befehl:

- **S**tatus, **L**og, **D**iff (gegenüber HEAD: vorgemerkte und nicht
  vorgemerkte Änderungen zusammen, farbig dargestellt) und **B**lame werden im
  [Betrachter](viewer.md) mit Suchfunktion geöffnet;
- **A**dd (vormerken) und **U**nstage (Vormerkung aufheben);
- **C**ommit öffnet den eigenen Editor von Git, genau wie `git commit`.

Die Befehle wirken auf die markierten Dateien der Liste, andernfalls auf die
aktuelle Datei; im Baum auf das aktuelle Verzeichnis. Status umfasst immer das
gesamte Repository. Für push, pull und andere Befehle, die das Netzwerk nutzen,
verwenden Sie [Ausführen](execute.md) (`X`), z. B. `git push`.

## Status in der Dateiliste

Die Dateiliste zeigt vor dem Namen jeder Datei deren Status mit den zwei
Zeichen von `git status --short` an: `M ` vorgemerkt (staged), ` M` geändert,
aber nicht vorgemerkt, `A ` hinzugefügt, `??`unversioniert, `R ` umbenannt.
Unveränderte Dateien zeigen Leerzeichen. Der Status wird nach eigenen
Operationen von Xylem, nach F3 sowie alle 10 Sekunden neu eingelesen (um
Commits zu erfassen, die in einem anderen Terminal ausgeführt wurden).

## Diff im Betrachter

Im Betrachter zeigt **D** an, was sich in der Datei seit dem letzten Commit
geändert hat (ihr `git diff HEAD`, farbig hervorgehoben), und ein erneutes
**D** kehrt an dieselbe Stelle der Datei zurück. „Diff" erscheint in der
Fußzeile nur bei Dateien mit Änderungen; **E** bearbeitet weiterhin die Datei,
und das Diff wird bei der Rückkehr aktualisiert.
