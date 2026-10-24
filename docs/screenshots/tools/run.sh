#!/bin/sh
# Regenerates the README screenshots (Linux): runs Xylem in an emulated
# 120x34 terminal (pyte) on a sample tree and saves each screen as a PNG,
# with Windows Terminal's "Campbell" colors and DejaVu Sans Mono.
#
# The sample tree is mounted at /home, so that paths read
# /home/user/projects, only inside a private namespace (unshare): nothing
# is created outside your home directory.
#
# Needs: unshare (util-linux), git, DejaVu Sans Mono and a Python with pyte
# and Pillow, e.g.
#   python3 -m venv ~/.cache/xylem-screenshots/venv
#   ~/.cache/xylem-screenshots/venv/bin/pip install pyte pillow
#   PYTHON=~/.cache/xylem-screenshots/venv/bin/python docs/screenshots/tools/run.sh
#
# Usage: run.sh [shot...]   (default: all; e.g. run.sh tree terminal demo)
# The PNGs go to docs/screenshots/, the animation (demo) to docs/demo.gif.
set -e
TOOLS=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$TOOLS/../../.." && pwd)
WORK="${XDG_CACHE_HOME:-$HOME/.cache}/xylem-screenshots"
PYTHON=$(command -v "${PYTHON:-python3}")
mkdir -p "$WORK"
(cd "$REPO" && cargo build --release -q)
"$TOOLS/fixture.sh" "$WORK/root" "$REPO/src"
exec unshare -rm sh -c '
set -e
real="$1"; TOOLS="$2"; REPO="$3"; WORK="$4"; PYTHON="$5"; shift 5
# Paths under the home directory, seen from inside (where /home is the
# sample tree): the home directory is also mounted at /mnt/real.
inside() { case "$1" in "$real"/*) echo "/mnt/real${1#$real}";; *) echo "$1";; esac; }
mount -t tmpfs none /mnt
mkdir /mnt/real /mnt/tmp
mount --bind "$real" /mnt/real
mount --bind "$WORK/root/home" /home
T=$(inside "$TOOLS"); X=$(inside "$REPO"); W=$(inside "$WORK"); PY=$(inside "$PYTHON")
D="\033[B"; U="\033[A"
down() { n=$1; s=""; while [ $n -gt 0 ]; do s="$s$D"; n=$((n-1)); done; printf %s "$s"; }
up() { n=$1; s=""; while [ $n -gt 0 ]; do s="$s$U"; n=$((n-1)); done; printf %s "$s"; }
WANT="$*"
run() { name=$1; cfg=$2; shift 2
  [ -z "$WANT" ] || case " $WANT " in *" $name "*) ;; *) return 0;; esac
  case $name in demo) file=docs/demo.gif;; *) file=docs/screenshots/$name.png;; esac
  HOME=/home/user SHELL=/bin/bash XDG_CONFIG_HOME=$W/root/config/$cfg \
  XDG_STATE_HOME=/mnt/tmp/state XDG_CACHE_HOME=/mnt/tmp/cache \
  "$PY" -I "$T/shot.py" "$X/$file" "$X/target/release/xy" /home/user/projects "$@"
  echo "$file"; }
run tree classic "*" sleep:1 "$(down 12)"
run theme-gruvbox gruvbox "*" sleep:1 "$(down 12)"
run branch-tagged classic "*" sleep:1 "$(down 3)" b "$D" t "$D$D" t t "$D$D"
run git-menu git "*" sleep:1 "$(down 12)" "\r" "$(down 7)" "\007"
run split classic "*" sleep:1 "$(down 12)" "\033[19~" "$(up 8)"
run terminal classic "*" sleep:1 "$(down 12)" "\033x" sleep:1.5 "cd model && ls\r" sleep:1
# The animation at the top of the README: the built-in terminal with the
# tree following a cd; then Branch, tags, and Alt+O typing the tagged files.
run demo classic hold:1.5 "*" hold:1 "\033x" hold:1 "type:cd xylem/src" hold:.4 "\r" hold:2.5 \
  "\033x" hold:1 "$(up 9)" hold:.8 b hold:1.2 "$D" hold:.4 t hold:.5 "$D" hold:.3 "$D" hold:.4 \
  t hold:.5 t hold:1.5 "send:\033x" "\014" hold:.6 "type:wc -c " hold:.4 "\033o" hold:1 \
  "\r" hold:2.5 "\033x" hold:3
run terminal-tagged classic "*" sleep:1 "$(down 3)" b "$D" t "$D$D" t t "\033x" sleep:1.5 "\014" "wc -c " "\033o" "\r" sleep:1
' sh "$HOME" "$TOOLS" "$REPO" "$WORK" "$PYTHON" "$@"
