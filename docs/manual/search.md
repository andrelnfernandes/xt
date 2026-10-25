# Searching the tagged files

## Ctrl+S

Searches for text in all the tagged files of the list and **untags those that
do not contain it**, with progress and `Esc` to stop. At the prompt, **F2**
toggles case sensitivity (ignoring case, accents count too: `AÇÃO` finds
`ação`) and **F4** switches between text and hex (byte pairs such as `4d 5a`).
A `*` in the middle of the term matches anything on the same line
(`error*disk`); a term never crosses a line break. It also works inside
[archives](archives.md).

## Ctrl+V

Opens the tagged files in the [viewer](viewer.md), one after another: **N**ext
and **P**rev switch files (Alt+Home and Alt+End: first and last) and **U**
untags the current one. The term is shared with Ctrl+S: after a Ctrl+S,
**Space** in Ctrl+V goes straight to the next match, moving on to the next
tagged file when the file ends.

A common round: tag files (`T`, or Ctrl+T in a Branch), Ctrl+S a term, Ctrl+V,
and Space to walk the matches across the files.
