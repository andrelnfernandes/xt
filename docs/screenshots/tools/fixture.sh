#!/bin/sh
# Builds the sample tree for the README screenshots: $1/home/user/projects.
set -e
out="$1"; src="$2"   # src: Xylem's source folder
rm -rf "$out"; mkdir -p "$out"
P="$out/home/user/projects"
mkdir -p "$P"/backup "$P"/notes "$P"/photos/2024 "$P"/photos/2025/trip \
         "$P"/website/css "$P"/website/img "$P"/website/js "$P"/xylem/src/model "$P"/xylem/src/ops
# Real bytes, not sparse files: the Usage view shows the space on the disk.
mk() { head -c "$2" /dev/zero > "$1"; touch -d "$3" "$1"; }
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
for theme in classic dark light mono norton solarized gruvbox nord slate graphite daylight vivid; do
  for git in false true; do
    name=$theme
    [ $git = false ] || { [ $theme = classic ] && name=git || name=git-$theme; }
    mkdir -p "$out/config/$name/xylem"
    printf 'theme = "%s"\n[options]\nconfirm_quit = false\nshow_hidden = false\ngit = %s\n' \
      $theme $git > "$out/config/$name/xylem/config.toml"
  done
done
# The built-in terminal's shell (terminal shots).
# The rest of the home directory, for the Usage view (usage shot): an old
# backup taking the most, as so often happens.
H="$out/home/user"
mkdir -p "$H/Videos" "$H/Downloads" "$H/Documents" "$H/Templates" \
         "$H/.cache/pip" "$H/.cache/thumbnails" "$H/.cache/browser" "$H/.local/share/Trash"
mk "$H/vm-backup-2023.tar" 75497472 "2023-03-18 22:10"
mk "$H/Videos/trip-2025.mp4" 18874368 "2025-08-02 19:30"
mk "$H/Videos/birthday.mp4" 10485760 "2024-11-10 15:12"
mk "$H/Downloads/ubuntu-26.04-desktop.iso" 15728640 "2026-04-23 09:40"
mk "$H/Downloads/setup.zip" 2097152 "2026-02-11 14:05"
mk "$H/Documents/taxes-2025.pdf" 524288 "2026-04-28 20:15"
mk "$H/Documents/notes.txt" 8192 "2026-10-01 08:30"
mk "$H/.cache/pip/wheels.bin" 3145728 "2026-09-12 10:00"
mk "$H/.cache/thumbnails/cache.db" 1572864 "2026-10-20 18:00"
mk "$H/.cache/browser/data" 6291456 "2026-10-24 23:00"
mk "$H/.local/share/Trash/old-photos.zip" 4194304 "2025-12-30 11:11"
cat > "$out/home/user/.bashrc" <<'RC'
PS1='\[\e[1;32m\]user@host\[\e[0m\]:\[\e[1;34m\]\w\[\e[0m\]$ '
alias ls='ls --color=auto'
HISTCONTROL=ignoreboth
RC
# The remote machine (remote shots): deploy@webserver's home, served by
# fake_ssh.py.
R="$out/home/deploy"
mkdir -p "$R/backups" "$R/logs" "$R/scripts" "$R/site/css" "$R/site/img" "$R/site/js"
for d in 21 22 23 24 25 26 27; do
  mk "$R/backups/db-2026-10-$d.sql.gz" $((3500000 + d * 41000)) "2026-10-$d 03:00"
done
mk "$R/logs/access.log" 2411520 "2026-10-28 09:41"
mk "$R/logs/access.log.1" 5830656 "2026-10-27 23:59"
mk "$R/logs/error.log" 18342 "2026-10-28 08:12"
mk "$R/site/index.html" 7104 "2026-10-20 18:22"
mk "$R/site/about.html" 4388 "2026-10-20 18:22"
mk "$R/site/css/style.css" 9216 "2026-10-20 18:22"
mk "$R/site/js/app.js" 14592 "2026-10-20 18:22"
mk "$R/site/img/logo.svg" 3127 "2026-09-02 11:05"
mk "$R/site/img/hero.jpg" 486210 "2026-09-02 11:05"
cat > "$R/scripts/backup.sh" <<'SH'
#!/bin/sh
# Nightly database backup, kept for a week.
set -e
pg_dump shop | gzip > ~/backups/db-$(date +%F).sql.gz
find ~/backups -name 'db-*.sql.gz' -mtime +7 -delete
SH
cat > "$R/scripts/deploy.sh" <<'SH'
#!/bin/sh
# Publishes the site built on the laptop.
set -e
rsync -a --delete ~/site/ /var/www/shop/
systemctl reload nginx
SH
cat > "$R/docker-compose.yml" <<'YML'
services:
  db:
    image: postgres:17
    volumes: [ "pgdata:/var/lib/postgresql/data" ]
  web:
    image: nginx:1.27
    ports: [ "80:80" ]
volumes:
  pgdata:
YML
touch -d "2026-10-26 22:15" "$R/scripts/backup.sh" "$R/scripts/deploy.sh" "$R/docker-compose.yml"
chmod 755 "$R/scripts/backup.sh" "$R/scripts/deploy.sh"
cat > "$R/.profile" <<'RC'
. ~/.bashrc
RC
# The prompt says where it is, and the tree follows its cd (OSC 7).
cat > "$R/.bashrc" <<'RC'
PS1='\[\e[1;33m\]deploy@webserver\[\e[0m\]:\[\e[1;34m\]\w\[\e[0m\]$ '
PROMPT_COMMAND='printf "\033]7;file://webserver%s\007" "$PWD"'
alias ls='ls --color=auto'
RC
find "$R" -type d -exec chmod 755 {} +
