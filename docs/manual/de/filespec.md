# Filespec: der Dateifilter

`F` legt fest, welche Dateien im Dateifenster angezeigt werden (jedes Fenster
hat seinen eigenen Filter). Der aktuelle Filespec erscheint im Fenstertitel.
Angaben werden durch Leerzeichen, `;` oder `,` getrennt und gehören wie in
ZTreeWin zu einer von drei Arten:

- **Name:** ein Glob-Muster (`*.rs`, `a?c*`, `[abc]*`), ohne Beachtung der
  Groß-/Kleinschreibung; in Anführungszeichen darf ein Name Leerzeichen
  enthalten (`"meine datei*"`);
- **Datum:** `<`, `=`, `>`, `<=`, `>=` oder `<>` und ein Datum in der
  Reihenfolge des konfigurierten Formats, mit `/`, `.` oder `-` und einer 2-
  oder 4-stelligen Jahreszahl (`>01/05/2026`; `2026-05-01` funktioniert immer),
  oder `TODAY` und `TODAY-n` (`>TODAY-7`: die letzten 7 Tage);
- **Größe:** dasselbe mit einem `s` nach dem Operator (`>=s200000`); `K`, `M`
  und `G` funktionieren ebenfalls (`>s100M`).

Ein vorangestelltes `-` negiert die Angabe (`-*.bak`; `-<s1K` entspricht
`>=s1K`). Innerhalb jeder Art werden positive Angaben mit ODER verknüpft, außer
wenn auf `>a` unmittelbar `<b` folgt, was für den Bereich zwischen beiden steht
(`>=01/05/2026,<=31/05/2026`: Mai); `<>` schließt aus. Zwischen verschiedenen
Arten gilt UND: `*.txt,=TODAY,<s1000` sind alle `.txt`-Dateien von heute, die
kleiner als 1000 Bytes sind.

`*` (oder eine leere Eingabezeile) zeigt alles an. **↑** ruft frühere Filespecs
auf.

Siehe auch: [das Dateifenster](files.md), [Markierungen](tagging.md) (Ctrl+T im
Baum markiert alles, was zum Filespec passt).
