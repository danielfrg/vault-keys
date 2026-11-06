#!/usr/bin/env bash
# End-to-end check of packaging, the installer and the Pages build for this machine.
set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
version=0.0.0-test.1
fail() { echo "FAIL: $*" >&2; exit 1; }

bash "$repo_dir/scripts/package.sh" "v$version" "" "$work/dist" >/dev/null
cat "$work"/dist/*.sha256 | LC_ALL=C sort > "$work/dist/SHA256SUMS"
printf 'v%s\n' "$version" > "$work/dist/VERSION"
rm "$work"/dist/*.sha256

echo "== installer"
KEYS_BASE_URL="file://$work/dist" KEYS_VERSION="$version" KEYS_INSTALL_DIR="$work/bin" \
    bash "$repo_dir/scripts/install.sh"
[ "$("$work/bin/keys" --version)" = "keys $version" ] || fail "unexpected version: $("$work/bin/keys" --version)"

echo "== installer rejects a tampered archive"
archive=$(ls "$work"/dist/keys_*.tar.gz)
printf 'x' >> "$archive"
if KEYS_BASE_URL="file://$work/dist" KEYS_VERSION="$version" KEYS_INSTALL_DIR="$work/bin2" \
    bash "$repo_dir/scripts/install.sh" 2>"$work/err"; then fail "tampered archive was installed"; fi
grep -q 'checksum mismatch' "$work/err" || fail "expected checksum error, got: $(cat "$work/err")"
[ ! -e "$work/bin2/keys" ] || fail "binary installed despite checksum failure"

echo "== pages"
# pages must refuse the tampered archive too
if bash "$repo_dir/scripts/build-pages.sh" "v$version" "$(printf 'a%.0s' {1..40})" "$work/dist" "$work/site" >/dev/null 2>&1; then
    fail "pages accepted a tampered archive"
fi
bash "$repo_dir/scripts/package.sh" "v$version" "" "$work/dist2" >/dev/null
cat "$work"/dist2/*.sha256 > "$work/dist2/SHA256SUMS"; printf 'v%s\n' "$version" > "$work/dist2/VERSION"; rm "$work"/dist2/*.sha256
bash "$repo_dir/scripts/build-pages.sh" "v$version" "$(printf 'a%.0s' {1..40})" "$work/dist2" "$work/site"
for f in index.html install.sh .nojekyll main/VERSION main/SHA256SUMS; do [ -e "$work/site/$f" ] || fail "missing $f in site"; done
grep -q "$version" "$work/site/index.html" || fail "version not in landing page"
grep -q '@VERSION@' "$work/site/index.html" && fail "placeholder left in landing page"
echo "OK"
