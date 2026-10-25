#!/bin/sh
# Assembles the website in _site/ (or in the folder given): the pages in
# site/ plus the images from docs/, so they are not kept twice. Used by
# .github/workflows/pages.yml; to look at it locally:
#   sh site/build.sh && python3 -m http.server -d _site
set -eu
cd "$(dirname "$0")/.."
out=${1:-_site}
rm -rf "$out"
mkdir -p "$out/assets/screenshots"
cp -r site/. "$out/"
rm "$out/build.sh"
cp docs/demo.gif docs/social-preview.png docs/icons/xylem.svg docs/icons/xylem-32.png "$out/assets/"
cp docs/screenshots/*.png "$out/assets/screenshots/"
