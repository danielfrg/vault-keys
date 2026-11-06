#!/usr/bin/env bash
# Build a release archive: keys_<version>_<os>_<arch>.tar.gz (+ .sha256).
# Usage: package.sh VERSION [TARGET_TRIPLE] [OUTPUT_DIR]
set -euo pipefail

version=${1:?Usage: $0 VERSION [TARGET_TRIPLE] [OUTPUT_DIR]}
target=${2:-$(rustc -vV | sed -n 's/^host: //p')}
repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
out_dir=${3:-"$repo_dir/dist"}
version=${version#v}

case "$target" in
    aarch64-apple-darwin) os=darwin arch=arm64 ;;
    x86_64-apple-darwin) os=darwin arch=amd64 ;;
    aarch64-unknown-linux-gnu) os=linux arch=arm64 ;;
    x86_64-unknown-linux-gnu) os=linux arch=amd64 ;;
    *) echo "Unsupported target: $target" >&2; exit 1 ;;
esac

mkdir -p "$out_dir"
out_dir=$(cd "$out_dir" && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

(cd "$repo_dir" && KEYS_VERSION="$version" cargo build --release --locked --target "$target")
cp "$repo_dir/target/$target/release/keys" "$stage/keys"
cp "$repo_dir/README.md" "$stage/"
[ ! -f "$repo_dir/LICENSE.txt" ] || cp "$repo_dir/LICENSE.txt" "$stage/"

archive="keys_${version}_${os}_${arch}.tar.gz"
COPYFILE_DISABLE=1 tar -czf "$out_dir/$archive" -C "$stage" .
if command -v sha256sum >/dev/null 2>&1; then
    digest=$(sha256sum "$out_dir/$archive")
else
    digest=$(shasum -a 256 "$out_dir/$archive")
fi
printf '%s  %s\n' "${digest%% *}" "$archive" > "$out_dir/$archive.sha256"
echo "Built $out_dir/$archive"
