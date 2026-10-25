# Remote machines (SSH)

Xylem can show the files of another machine, reached with SSH, as a tree:
alone, or with this machine next to it in [split screen](split.md).

## Opening one

Type the address in `L` ([Log](treespec.md)), or give it to `xy` on the
command line:

- `user@host:` — the home directory; `user@host:/srv/www` — that directory;
- `ssh://[user@]host[:port][/path]` — with a port, or without a user (as in
  your `~/.ssh/config`).

Xylem runs your own `ssh`, so what you use in a terminal works the same:
`~/.ssh/config` (host aliases, ports, ProxyJump), keys, the agent and
`known_hosts`. Nothing needs installing on the other machine: Xylem talks
SFTP, which every SSH server has (Linux, BSD, macOS, Windows).

## Passwords and questions

When ssh asks for something, the question shows in the footer:

- a **password** or a key's **passphrase**: typed hidden; Enter sends it,
  Esc gives up the connection. Xylem passes it to ssh and keeps nothing.
- an **unknown host**: the host and its key's fingerprint, to check before
  answering `yes` (ssh then remembers it in `known_hosts`, as usual).

`Esc` while connecting cancels. With a key and the agent, nothing is asked.

## What works

The tree, the file window, [Filespec](filespec.md), Branch, Showall, tags,
[the viewer](viewer.md), [Ctrl+S and Ctrl+V](search.md), Autoview, Info
(Alt+I), the display modes (owner shows user and group ids, which is what SFTP
sends), [Compare](compare.md) and Open (`O` downloads a copy and opens it
here).

**Copy** brings files here, with their permissions and dates (to the second,
as SFTP keeps them); with the screen split, it suggests this machine's window
as the destination. Alt+C (with paths) works too, with paths starting at the
remote window's root.

For now the remote side is **read-only**: Move, Delete, Rename, Makedir,
Attributes, dates, Edit, eXecute and copying *to* the remote machine are
not there yet. F3 rereads a directory; there is no automatic refresh.

**F8** in a remote window opens a window on this machine (the folder Xylem
started in), so you have both side by side.

## The terminal on the remote machine

**Alt+X** in a remote window opens a shell on that machine (`ssh -t`),
starting in the window's directory; in a local window, the local shell, as
before (each one keeps running while the other is on screen). On Linux and
macOS the remote shell reuses the connection already made, so the password is
not asked again; on Windows, ssh asks for it in the terminal.

The tree follows the remote shell's `cd` when the shell reports its directory
with the OSC 7 sequence. Many systems do it already; for bash, add this to
`~/.bashrc` **on the remote machine**:

```bash
PROMPT_COMMAND='printf "\033]7;file://%s%s\007" "$HOSTNAME" "$PWD"'
```

Xylem never types into a remote shell (it cannot see whether a program is
running there), and Alt+O and `$XY_TAGGED` are for local terminals only.
Quitting with a remote terminal open asks first.

## tmux or screen started on login

Some people start tmux (or screen) automatically when they log in over SSH,
from `~/.bashrc` on the remote machine, so that a dropped connection (a
flaky Wi-Fi, a VPN that reconnects) does not kill what was running: logging
in again resumes the same session. For example:

```bash
if [ -n "$SSH_CONNECTION" ] && [ -z "$TMUX" ]; then
    tmux attach-session -t remote 2>/dev/null || tmux new-session -s remote
fi
```

Then Xylem's remote terminal also opens inside tmux. Everything works, except
that **the tree does not follow the shell's `cd`**: tmux sits between the
shell and Xylem, keeps the OSC 7 sequence for itself and does not pass it on.

Xylem's remote shell starts with `XY_TERMINAL=1` set, as the local one does,
so the login can tell the two apart. Adding that test to the condition keeps
tmux for your usual `ssh` and leaves it out when Xylem connects:

```bash
if [ -n "$SSH_CONNECTION" ] && [ -z "$TMUX" ] && [ -z "$XY_TERMINAL" ]; then
    tmux attach-session -t remote 2>/dev/null || tmux new-session -s remote
fi
```

Inside Xylem the shell does not need tmux to survive: it keeps running while
Xylem is open, and if the connection drops, the remote window goes with it
anyway.

For Google Drive, S3 and other cloud storage, see [Cloud storage](cloud.md).
