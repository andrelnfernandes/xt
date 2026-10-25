# The built-in terminal (Alt+X)

Not in XTree or ZTreeWin: your shell (`$SHELL`) running inside Xylem's screen,
in the bottom part, with the tree and the files still visible above it.

- **Alt+X** opens the terminal and goes back to Xylem. The shell keeps running
  while hidden, with its history and whatever it had on screen; `exit` closes
  it.
- **Alt+Z** opens it in full screen; inside, it switches between full screen
  and the panel. Alt+X then reopens it the way it was.
- **The tree follows the shell:** a `cd` in the terminal moves the tree's
  cursor to the same directory (Linux).
- **The shell follows the tree:** when you enter the terminal from another
  directory, it goes there. This only happens when the shell is waiting at the
  prompt; with a program running (vim, less...), nothing is typed.
- **`$XY_TAGGED`** has the tagged files, set when you enter the terminal. It is
  an array: in bash, `"${XY_TAGGED[@]}"` (`cp "${XY_TAGGED[@]}" ~/backup`); in
  zsh and fish, `$XY_TAGGED`. With other shells (sh, dash) there is only the
  `cd`.
- **Alt+O** types the tagged files at the cursor, quoted and relative to the
  shell's directory (`wc -l 'a.txt' 'b c.txt'`). From the tree or the file
  window, it opens the terminal and types them.
- **Shift+PgUp/PgDn** scroll back through what went off the screen.
- On leaving the terminal, the open directories are reread.
- **Quitting Xylem ends the shell** and whatever runs in it. If a program is
  running there (say, `vim`), Q and Alt+Q ask first, even with `confirm_quit`
  off (Linux and macOS).
- Programs that take the whole screen (vim, htop, less) work inside it; all
  keys go to them, except Alt+X, Alt+Z and Alt+O. That is why F1 and Alt+H do
  not open this help inside the terminal.

## Its own history

The commands typed in it go to a shell history of their own, apart from your
other terminals': `~/.local/state/xylem/bash_history` (`zsh_history`...),
through `HISTFILE`; in fish, the `xylem` session. If your rc file sets
`HISTFILE` unconditionally, it wins. Turn it off with F12 (page 3) or
`terminal_history = false`.

## Keys and colors

The keys can be changed in `[keys.any]` (`terminal`, `terminal_full`,
`terminal_tagged`): Alt+T, for instance, is the "Terminal" menu in some
terminals. The colors are chosen with F12 (page 2) or `terminal_theme`:
`native` (those of the terminal Xylem runs in), `xylem` (the background and
text of Xylem's theme), `solarized`, `gruvbox`, `nord`, `dracula` or
`one_dark`.

To enter the terminal and the directory, Xylem writes a small script in a
folder of its own (in `~/.cache/xylem/`, readable only by you) and types
` . script` at the prompt; the leading space keeps the line out of the history
in bash (`ignorespace`) and zsh (`HIST_IGNORE_SPACE`). Inside the terminal,
`XY_TERMINAL=1`, for prompts and scripts that want to know; the terminal on a
[remote machine](remote.md) sets it too (useful, for instance, to keep a
`~/.bashrc` from starting tmux there).
