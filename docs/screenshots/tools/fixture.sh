#!/bin/sh
# Builds the sample tree for the README screenshots: $1/home/user/projects.
set -e
out="$1"; src="$2"   # src: Xylem's source folder
rm -rf "$out"; mkdir -p "$out"
P="$out/home/user/projects"
mkdir -p "$P"/backup "$P"/notes "$P"/photos/2024 "$P"/photos/2025/trip \
         "$P"/website/css "$P"/website/img "$P"/website/js "$P"/xylem/src/model "$P"/xylem/src/ops
mk() { truncate -s "$2" "$1"; touch -d "$3" "$1"; }
mk "$P/backup/projects-2026-09.tar.gz" 18874368 "2026-09-30 23:00"
mk "$P/backup/projects-2026-10.tar.gz" 19922944 "2026-10-01 23:00"
mk "$P/notes/ideas.md" 4211 "2026-10-02 09:14"
mk "$P/notes/todo.md" 1873 "2026-10-12 18:40"
mk "$P/website/index.html" 6342 "2026-08-20 21:05"
mk "$P/website/css/site.css" 8120 "2026-08-20 21:05"
mk "$P/website/img/logo.png" 23417 "2026-08-19 16:30"
mk "$P/website/js/app.js" 12880 "2026-08-20 21:05"
i=0
for f in "20240201 1266120 2025-09-09 12:35" "20240210 797200 2025-07-16 19:42" \
         "20240302 651880 2025-05-23 03:49" "20240311 683880 2025-03-03 10:56" \
         "20240403 1111960 2025-01-10 17:04" "20240412 1395920 2025-08-17 01:11" \
         "20240504 650320 2025-06-24 08:18" "20240513 727560 2025-04-04 15:25" \
         "20240605 1084680 2025-02-11 22:32" "20240614 524920 2025-09-18 06:39" \
         "20240706 1274600 2025-07-25 13:46" "20240807 761680 2025-05-05 20:53"; do
  set -- $f; mk "$P/photos/2024/IMG_$1.jpg" "$2" "$3 $4"
done
for f in "01 1777400 2025-03-12 04:01" "02 1017500 2025-01-19 11:08" "03 619550 2025-08-26 18:15" \
         "04 1400150 2025-06-06 02:22" "05 578250 2025-04-13 09:29" "06 1794500 2025-02-20 16:36" \
         "07 1601350 2025-09-27 00:43" "08 2136950 2025-07-07 07:50" "09 1235050 2025-05-14 14:57"; do
  set -- $f; mk "$P/photos/2025/trip/DSC_$1.jpg" "$2" "$3 $4"
done
# Xylem's own sources, as a Git repository with a few changes (git-menu shot).
cp "$src"/*.rs "$P/xylem/src/"; cp "$src"/model/*.rs "$P/xylem/src/model/"; cp "$src"/ops/*.rs "$P/xylem/src/ops/"
rm -f "$P/xylem/src/git.rs" "$P/xylem/src/testing.rs"
find "$P/xylem" -type f -exec touch -d "2026-10-12 13:02" {} +
g() { git -C "$P/xylem" -c user.name=user -c user.email=user@example.com "$@"; }
g init -q -b main; g add -A; g commit -q -m "xylem"
cp "$src/git.rs" "$P/xylem/src/git.rs"; g add src/git.rs
echo "// staged change" >> "$P/xylem/src/viewer.rs"; g add src/viewer.rs
echo "// unstaged change" >> "$P/xylem/src/keymap.rs"
touch -d "2026-10-13 13:44" "$P/xylem/src/git.rs" "$P/xylem/src/viewer.rs"
touch -d "2026-10-13 13:45" "$P/xylem/src/keymap.rs"
chmod 644 $(find "$P" -type f -not -path '*/.git/*')
find "$P" -type d -not -path "*/.git*" -exec chmod 755 {} +
# Configurations: one per theme (classic is the defaults), and each of them
# again with Git on (git = classic, git-dark, ...).
for theme in classic dark light mono norton solarized gruvbox nord slate graphite daylight; do
  for git in false true; do
    name=$theme
    [ $git = false ] || { [ $theme = classic ] && name=git || name=git-$theme; }
    mkdir -p "$out/config/$name/xylem"
    printf 'theme = "%s"\n[options]\nconfirm_quit = false\nshow_hidden = false\ngit = %s\n' \
      $theme $git > "$out/config/$name/xylem/config.toml"
  done
done
# The built-in terminal's shell (terminal shots).
cat > "$out/home/user/.bashrc" <<'RC'
PS1='\[\e[1;32m\]user@host\[\e[0m\]:\[\e[1;34m\]\w\[\e[0m\]$ '
alias ls='ls --color=auto'
HISTCONTROL=ignoreboth
RC
