#!/bin/sh
# Builds the .deb and the .rpm of Xylem in target/packages/, each with a
# .sha256 next to it, from the release binary already built for the target
# given, e.g.:
#   cargo build --release --locked --target x86_64-unknown-linux-musl   (needs musl-tools)
#   sh packaging/linux/packages.sh x86_64-unknown-linux-musl
# What goes in them is in [package.metadata.deb] and
# [package.metadata.generate-rpm] in Cargo.toml. Used by
# .github/workflows/release.yml and ci.yml.
set -eu
target=$1
cd "$(dirname "$0")/../.."

# Pinned, like cargo-audit in CI; cargo install does nothing when that
# version is already there.
cargo install cargo-deb --locked --version 3.8.0
cargo install cargo-generate-rpm --locked --version 0.21.0

dir=target/$target/release
"$dir/xy" --man | gzip -9n > "$dir/xy.1.gz"
out=target/packages
rm -rf "$out"
mkdir -p "$out"
cargo deb --no-build --no-strip --target "$target" -o "$out/"
cargo generate-rpm --target "$target" -o "$out/"
cd "$out"
for f in *.deb *.rpm; do
    sha256sum "$f" > "$f.sha256"
done
