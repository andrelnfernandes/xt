# Duplikate (Alt+Y)

Dateien mit gleichem Inhalt, egal mit welchen Namen und in welchen Ordnern, und
der Platz, den die Kopien verschwenden. **Alt+Y** sucht im Zweig des aktuellen
Verzeichnisses; **D** in der Belegungsansicht sucht im dort gezeigten Ordner.

Was am meisten freigibt, steht zuerst. Nichts wird markiert, bevor Sie es
verlangen, und eine Kopie jeder Gruppe bleibt immer: Xylem lässt die letzte nicht
markieren.

## Auswählen, was geht

| Taste | Wirkung |
|---|---|
| ↑ ↓ PgUp PgDn Home End | bewegen |
| `T` oder Leertaste | markiert die Kopie oder hebt die Markierung auf |
| `K` | behält diese Kopie: markiert die anderen der Gruppe |
| `A` | in jeder Gruppe alle bis auf eine markieren; erneut `A` ändert, welche bleibt: die älteste, die neueste, der kürzeste Pfad |
| `F` | in jeder Gruppe mit einer Kopie im Ordner dieser Kopie jene behalten |
| `U` | alle Markierungen aufheben |
| `D`, Entf oder Strg+D | löscht die markierten, wie der Befehl Delete |
| `M` oder Strg+M | verschiebt die markierten Dateien |
| `G` | Dateien oder ganze Ordner |
| `V` oder Enter | zeigt die Datei |
| `E` | schreibt die Liste in eine Datei (nie über eine vorhandene) |
| `\` | schließt die Ansicht, mit dem Baum auf diesem Ordner |
| F3 | liest die Ordner neu und vergleicht erneut |
| Esc oder Alt+Y | schließt |

Vor dem Löschen oder Verschieben wird jede Gruppe erneut geprüft: Hat sich eine
Kopie seit der Analyse geändert oder fehlt die, die bleiben sollte, wird von
dieser Gruppe nichts angerührt.

## Ganze Ordner

**G** zeigt die Ordner mit genau gleichem Inhalt: gleiche Namen, gleicher Inhalt
jeder Datei und gleiche Links. Ein Ordner gilt nie als Kopie, wenn etwas darin
nicht verglichen wurde: ein unlesbarer Ordner, ein darin eingehängtes anderes
Laufwerk, eine Spezialdatei oder ein ignorierter Ordner.

## Wie Dateien verglichen werden

Gelesen werden nur Dateien, deren Größe mit einer anderen übereinstimmt: zuerst
beide Enden, dann die Mitte, in Teilen, mit SHA-256. Das läuft im Hintergrund, und
Esc bricht ab. Außen vor bleiben Dateien kleiner als `duplicates_min_size`
(standardmäßig 1 KiB), leere Dateien, Links, weitere Hardlinks derselben Datei,
andere Laufwerke und die Ordner aus `duplicates_ignore` (`.git` und
`node_modules`).

In einem Netzwerkordner wird zuerst nach der zu lesenden Menge gefragt. Auf einem
entfernten Rechner oder in einem Archiv ist es noch nicht verfügbar.

## In der Cloud (rclone)

In einem mit `rclone mount` eingehängten Ordner fragt Xylem rclone nach den
Prüfsummen, die der Dienst bereits speichert (`rclone lsjson --hash`), und lädt
nichts herunter. rclone muss im PATH sein; andernfalls, oder wenn das Remote keine
Prüfsummen liefert, nennt Xylem den Grund und fragt, bevor es die Dateien über die
Einhängung liest. Ganze Ordner werden nur auf lokalen Laufwerken verglichen.

Bei Google Drive: Eine Datei in zwei Ordnern ist eine Datei; Docs, Sheets und
Slides haben keine Prüfsumme; was andere geteilt haben, erscheint mit `s` und wird
von „alle bis auf eine“ nicht markiert; zwei gleiche Namen in einem Ordner löst
`rclone dedupe`; und Löschen schickt in den Papierkorb des Drive, den Xylem nicht
leert: Es zeigt den Befehl `rclone cleanup gdrive:`.

Die Prüfsumme des Dienstes wird vertrauensvoll übernommen, und MD5 lässt sich
absichtlich fälschen.
