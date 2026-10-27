# Das integrierte Terminal (Alt+X)

Weder in XTree noch in ZTreeWin vorhanden: Ihre Shell (`$SHELL`) läuft direkt
innerhalb des Xylem-Bildschirms im unteren Bereich, während der Baum und die
Dateien darüber weiterhin sichtbar bleiben.

- **Alt+X** öffnet das Terminal und kehrt wieder zu Xylem zurück. Solange es
  ausgeblendet ist, läuft die Shell im Hintergrund samt Verlauf und
  Bildschirminhalt weiter; `exit` beendet sie.
- **Alt+Z** öffnet es im Vollbildmodus; innerhalb des Terminals wechselt die
  Taste zwischen Vollbild und geteilter Ansicht. Alt+X öffnet es anschließend
  wieder so, wie es zuletzt war.
- **Der Baum folgt der Shell:** Ein `cd` im Terminal bewegt den Cursor des Baums
  in dasselbe Verzeichnis (Linux).
- **Die Shell folgt dem Baum:** Wenn Sie das Terminal aus einem anderen
  Verzeichnis heraus betreten, wechselt die Shell dorthin. Dies geschieht nur,
  wenn die Shell an der Eingabeaufforderung wartet; läuft gerade ein Programm
  (vim, less...), wird nichts eingegeben.
- **`$XY_TAGGED`** enthält die markierten Dateien und wird beim Betreten des
  Terminals gesetzt. Es handelt sich um ein Array: in bash `"${XY_TAGGED[@]}"`
  (`cp "${XY_TAGGED[@]}" ~/backup`), in zsh und fish `$XY_TAGGED`. Bei anderen
  Shells (sh, dash) erfolgt lediglich das `cd`.
- **Alt+O** fügt die markierten Dateien an der Cursorposition ein, in
  Anführungszeichen und relativ zum Verzeichnis der Shell
  (`wc -l 'a.txt' 'b c.txt'`). Aus dem Baum oder dem Dateifenster heraus öffnet
  es das Terminal und fügt sie dort ein.
- **Shift+PgUp/PgDn** scrollen durch die bereits aus dem Bild gelaufene Ausgabe
  zurück.
- Beim Verlassen des Terminals werden die geöffneten Verzeichnisse neu
  eingelesen.
- **Das Beenden von Xylem beendet auch die Shell** und alles, was darin läuft.
  Läuft dort noch ein Programm (etwa `vim`), fragen Q und Alt+Q immer zuerst
  nach, selbst wenn `confirm_quit` ausgeschaltet ist (Linux und macOS).
- Programme, die den gesamten Bildschirm nutzen (vim, htop, less),
  funktionieren im Terminal; alle Tasten werden an sie weitergereicht, mit
  Ausnahme von Alt+X, Alt+Z und Alt+O. Aus diesem Grund öffnen F1 und Alt+H
  innerhalb des Terminals nicht diese Hilfe.

## Eigener Befehlsverlauf

Die im integrierten Terminal eingegebenen Befehle landen in einem eigenen
Shell-Verlauf, getrennt von Ihren anderen Terminals:
`~/.local/state/xylem/bash_history` (`zsh_history`...), gesteuert über
`HISTFILE`; in fish über die Sitzung `xylem`. Falls Ihre rc-Datei `HISTFILE`
bedingungslos überschreibt, hat diese Vorrang. Abschalten lässt sich das über
F12 (Seite 3) oder `terminal_history = false`.

## Tasten und Farben

Die Tasten können unter `[keys.any]` geändert werden (`terminal`,
`terminal_full`, `terminal_tagged`): Alt+T öffnet beispielsweise in manchen
Terminals das Menü „Terminal". Die Farben werden mit F12 (Seite 2) oder
`terminal_theme` gewählt: `native` (die Farben des Terminals, in dem Xylem
läuft), `xylem` (Hintergrund und Text des Xylem-Themas), `solarized`,
`gruvbox`, `nord`, `dracula` oder `one_dark`.

Um beim Betreten des Terminals das Verzeichnis und die Variablen zu setzen,
schreibt Xylem ein kleines Skript in einen eigenen Ordner (unter
`~/.cache/xylem/`, nur für Sie lesbar) und gibt ` . script` an der
Eingabeaufforderung ein; das führende Leerzeichen hält die Zeile in bash
(`ignorespace`) und zsh (`HIST_IGNORE_SPACE`) aus dem Verlauf fern. Innerhalb
des Terminals ist `XY_TERMINAL=1` gesetzt, für Eingabeaufforderungen und
Skripte, die dies abfragen möchten; auch das Terminal auf einem [entfernten
Rechner](remote.md) setzt diese Variable (nützlich etwa, um zu verhindern, dass
eine `~/.bashrc` dort automatisch tmux startet).
