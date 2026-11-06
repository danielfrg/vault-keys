#!/usr/bin/env bash
# Assemble the GitHub Pages site: landing page, install.sh and the main channel.
# Usage: build-pages.sh VERSION ASSET_DIR OUTPUT_DIR
#   ASSET_DIR holds keys_*.tar.gz, SHA256SUMS and VERSION (see ci.yml "assets").
set -euo pipefail

die() { printf '%s\n' "$*" >&2; exit 1; }
[ "$#" -eq 3 ] || die "Usage: $0 VERSION ASSET_DIR OUTPUT_DIR"
version=$1 assets=$2 out=$3
repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

[ -d "$assets" ] || die "missing asset directory: $assets"
[ -f "$assets/SHA256SUMS" ] || die 'missing SHA256SUMS'
compgen -G "$assets/keys_*.tar.gz" >/dev/null || die 'no keys_*.tar.gz archives found'
[ "$(cat "$assets/VERSION")" = "$version" ] || die 'VERSION does not match'

# Every archive must match its checksum before it is published.
(cd "$assets" && if command -v sha256sum >/dev/null 2>&1; then sha256sum --check --quiet SHA256SUMS; else shasum -a 256 --check --quiet SHA256SUMS; fi)

mkdir -p "$out/main"
cp "$assets"/keys_*.tar.gz "$assets/SHA256SUMS" "$assets/VERSION" "$out/main/"
cp "$repo_dir/scripts/install.sh" "$out/install.sh"
: > "$out/.nojekyll"
cp "$repo_dir/site/index.html" "$out/index.html"
echo "Built Pages site for $version in $out"
