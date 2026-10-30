# Filtering lines

In the viewer's text mode, `&` shows only the lines that match. The lines are
hidden, not changed: with line numbers on (`#`), each line keeps its number in
the file, so the gaps show the filter at work. The header tells the result:
`[filter error: 24 of 850 lines]`.

| Key at the prompt | What it does |
|---|---|
| **F1** | this help |
| **F2** | match case or ignore it |
| **F3** | invert: keep the lines that do not match |
| **F4** | plain text or regular expression |
| ↑ ↓ | earlier filters |
| Enter | apply |
| Esc | cancel |

## Regular expressions

The term may match anywhere in the line, unless you anchor it. Case is ignored
unless you press **F2**; accents are not ignored here.

| Write | To match |
|---|---|
| `.` | any character |
| `\d` `\D` | a digit, anything but a digit |
| `\w` `\W` | a letter, digit or `_`, anything else |
| `\s` `\S` | a space, anything else |
| `[abc]` `[^abc]` | one of these, anything but these |
| `[a-z0-9]` | one in these ranges |
| `^` `$` | the start, the end of the line |
| `\b` | the edge of a word |
| `a*` `a+` `a?` | zero or more, one or more, zero or one |
| `a{3}` `a{2,5}` | exactly 3, from 2 to 5 |
| `a+?` | one or more, as few as possible |
| `(abc)` | a group, for repeating or alternatives |
| `(?P<name>abc)` | a group with a name |
| `(?i)` `(?-i)` | ignore case or match it from here on |

The vertical bar separates alternatives: `error|fail|fatal`, or
`disk (full|error)` inside a group.

To match one of the special characters `. * + ? ( ) [ ] { } ^ $ \` or the bar
itself, put a `\` before it: `\.log$`, `\(1\)`.

## Examples

- `^\d{4}-\d{2}-\d{2}`: lines that start with a date
- `\b(error|fatal)\b`: these whole words, in any case
- `^\s*$` with **F3**: hides the blank lines
- `^\s*#` with **F3**: hides comments in a shell script
- ` 50\d `: HTTP errors 500 to 509 in an access log
- `\b\d{1,3}(\.\d{1,3}){3}\b`: lines with an IPv4 address
- `(?-i)TODO`: only in capitals, even with case ignored

Backreferences (`\1`) and lookaround (`(?=...)`) do not exist here, on purpose:
without them the time grows only with the size of the text, so no file or
expression can hang the viewer with a pattern such as `(a+)+$`.

## Plain text

The same rules as the search: ignoring case, accents count too (`AÇÃO` finds
`ação`), and a `*` in the middle matches anything on the same line
(`error*disk`).

## While a filter is on

Press `&` again to change the filter. **Enter on an empty prompt turns it off**,
and the line at the top of the screen stays there. A filter that matches nothing
or is invalid changes nothing, and the footer says why.

While a filter is on, scrolling and [searching](viewer.md) go through the kept
lines only. `g` takes a line number of the file; if that line is hidden, it goes
to the next one kept. The filter stays after editing the file (`E`) or changing
the encoding (`O`), and ends when the viewer closes.

Only what the viewer read is filtered: if the header says `(showing first N
MiB)`, the rest of the file is not looked at. The filter sees the lines as they
are shown, so a tab is spaces (use `\s`, not `\t`) and a control character is
`·`. In table mode, `&` opens the SQL filter.

See also: [the viewer](viewer.md).
