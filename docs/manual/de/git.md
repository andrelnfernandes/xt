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
- **G**raph: `git log --graph --decorate --oneline --all`, die
  letzten 1000 Commits aller Zweige, in den Farben von Git selbst (eine pro
  Zweiglinie), ebenfalls im Betrachter;
- **A**dd (vormerken) und **U**nstage (Vormerkung aufheben);
- **C**ommit öffnet den eigenen Editor von Git, genau wie `git commit`.

Die Befehle wirken auf die markierten Dateien der Liste, andernfalls auf die
aktuelle Datei; im Baum auf das aktuelle Verzeichnis. Status und Graph umfassen immer
das gesamte Repository. Für push, pull und andere Befehle, die das Netzwerk nutzen,
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

## Repositorys, die Sie nicht selbst erstellt haben

Das `.git` eines Repositorys kann Befehle nennen, die Git ausführt, und ein heruntergeladener oder entpackter Ordner kommt mit seinem `.git` (ein `git clone` bringt es nicht mit). Darum führen die Dateimarkierungen und Status, Log, Graph, Diff und Blame keinen davon aus: keine Hooks, kein `core.fsmonitor`, keine externen Diffs, Textkonvertierungen oder Signaturprüfungen. Das kostet Folgendes:

- Ein Repository mit eigenen Filtern, wie bei `git-crypt`, hat hier keine Markierungen, kein Status, Diff oder Blame, mit einer Meldung, die den Grund nennt. Log und Graph funktionieren. Filter aus Ihrer globalen Konfiguration, etwa Git LFS, gelten als vertrauenswürdig.
- Ein Submodul mit geänderten Dateien, aber ohne neuen Commit, erscheint nicht als geändert.

Add, Unstage und Commit sind Ihre eigenen Aufträge und folgen der Konfiguration des Repositorys: Sie können ausführen, was sie nennt.

### Vertrauenswürdige Ordner

Standardmäßig ist nichts vertrauenswürdig: Jedes Repository gilt als feindlich. Damit die Repositorys eines Ordners mit ihrer ganzen Konfiguration arbeiten, etwa mit `git-crypt` oder einem eigenen Diff-Werkzeug, tragen Sie den Ordner unter **Trusted Git folders** ein (**K** auf Seite 1 von **F12**), mit `;` zwischen den Ordnern:

```
~/projekte; /arbeit/code
```

In der Datei ist es eine Liste: `git_trusted = ["~/projekte", "/arbeit/code"]`. Ein Ordner ist ein vollständiger Pfad oder einer unter `~` und gilt für alles darin. Ein Link, der nach außen zeigt, nimmt das Vertrauen nicht mit.

**Gefahr:** In einem vertrauenswürdigen Ordner genügt es, ein Repository zu betreten, damit Git mit Ihren Rechten ausführt, was es nennt. Entpacken oder laden Sie dort nie etwas von anderen, und vertrauen Sie weder `~` noch `/`, die enthalten, was Sie herunterladen.
