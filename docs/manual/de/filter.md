# Zeilen filtern

Im Textmodus des Betrachters zeigt `&` nur die passenden Zeilen. Die Zeilen werden
ausgeblendet, nicht verändert: Mit eingeschalteten Zeilennummern (`#`) behält jede
Zeile ihre Nummer in der Datei, und die Lücken zeigen den Filter bei der Arbeit.
Die Kopfzeile nennt das Ergebnis: `[filter error: 24 of 850 lines]`.

| Taste in der Eingabe | Was sie tut |
|---|---|
| **F1** | diese Hilfe |
| **F2** | Groß-/Kleinschreibung beachten oder ignorieren |
| **F3** | umkehren: die nicht passenden Zeilen behalten |
| **F4** | einfacher Text oder regulärer Ausdruck |
| ↑ ↓ | frühere Filter |
| Enter | anwenden |
| Esc | abbrechen |

## Reguläre Ausdrücke

Der Ausdruck darf überall in der Zeile passen, außer Sie verankern ihn.
Groß-/Kleinschreibung wird ignoriert, außer Sie drücken **F2**; Akzente zählen
hier.

| Schreiben Sie | Um zu finden |
|---|---|
| `.` | ein beliebiges Zeichen |
| `\d` `\D` | eine Ziffer, alles außer einer Ziffer |
| `\w` `\W` | Buchstabe, Ziffer oder `_`, alles andere |
| `\s` `\S` | ein Leerzeichen, alles andere |
| `[abc]` `[^abc]` | eines davon, alles außer diesen |
| `[a-z0-9]` | eines aus diesen Bereichen |
| `^` `$` | Anfang, Ende der Zeile |
| `\b` | die Grenze eines Wortes |
| `a*` `a+` `a?` | null oder mehr, eins oder mehr, null oder eins |
| `a{3}` `a{2,5}` | genau 3, von 2 bis 5 |
| `a+?` | eins oder mehr, so wenige wie möglich |
| `(abc)` | eine Gruppe, zum Wiederholen oder für Alternativen |
| `(?P<name>abc)` | eine Gruppe mit Namen |
| `(?i)` `(?-i)` | ab hier Groß-/Kleinschreibung ignorieren oder beachten |

Der senkrechte Strich trennt Alternativen: `fehler|fatal`, oder
`platte (voll|fehler)` innerhalb einer Gruppe.

Um eines der Sonderzeichen `. * + ? ( ) [ ] { } ^ $ \` oder den Strich selbst zu
finden, setzen Sie ein `\` davor: `\.log$`, `\(1\)`.

## Beispiele

- `^\d{4}-\d{2}-\d{2}`: Zeilen, die mit einem Datum beginnen
- `\b(fehler|fatal)\b`: diese ganzen Wörter, in beliebiger Schreibung
- `^\s*$` mit **F3**: blendet leere Zeilen aus
- `^\s*#` mit **F3**: blendet Kommentare eines Shell-Skripts aus
- ` 50\d `: HTTP-Fehler 500 bis 509 in einem Zugriffsprotokoll
- `\b\d{1,3}(\.\d{1,3}){3}\b`: Zeilen mit einer IPv4-Adresse
- `(?-i)TODO`: nur in Großbuchstaben, auch wenn die Schreibung ignoriert wird

Rückverweise (`\1`) und Lookaround (`(?=...)`) gibt es hier absichtlich nicht:
Ohne sie wächst die Zeit nur mit der Größe des Textes, und keine Datei und kein
Ausdruck kann den Betrachter mit einem Muster wie `(a+)+$` zum Hängen bringen.

## Einfacher Text

Dieselben Regeln wie bei der Suche: Beim Ignorieren der Groß-/Kleinschreibung
zählen auch Akzente nicht (`AÇÃO` findet `ação`), und ein `*` in der Mitte passt
auf alles in derselben Zeile (`fehler*platte`).

## Bei aktivem Filter

Drücken Sie erneut `&`, um den Filter zu ändern. **Enter bei leerer Eingabe
schaltet ihn aus**, und die Zeile oben auf dem Bildschirm bleibt dort. Ein Filter,
der nichts findet oder ungültig ist, ändert nichts, und die Fußzeile nennt den
Grund.

Bei aktivem Filter laufen Blättern und [Suche](viewer.md) nur durch die behaltenen
Zeilen. `g` nimmt eine Zeilennummer der Datei; ist diese Zeile ausgeblendet, geht
es zur nächsten behaltenen. Der Filter bleibt nach dem Bearbeiten der Datei (`E`)
oder dem Wechsel der Kodierung (`O`) bestehen und endet, wenn der Betrachter
schließt.

Gefiltert wird nur, was der Betrachter gelesen hat: Steht in der Kopfzeile
`(showing first N MiB)`, wird der Rest der Datei nicht untersucht. Der Filter
sieht die Zeilen so, wie sie angezeigt werden: Ein Tabulator besteht aus
Leerzeichen (`\s` verwenden, nicht `\t`), und ein Steuerzeichen ist ein `·`. Im
Tabellenmodus öffnet `&` den SQL-Filter.

Siehe auch: [Der Betrachter](viewer.md).
