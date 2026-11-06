#!/bin/bash
# Install keys on macOS or Linux (amd64/arm64).
#
#   curl -fsSL https://danielfrg.github.io/vault-keys/install.sh | bash
#
# Options (environment variables):
#   KEYS_CHANNEL=main        newest build from the main branch instead of the latest release
#   KEYS_VERSION=1.2.3       a specific release
#   KEYS_INSTALL_DIR=DIR     install location (default: /usr/local/bin if writable, else ~/.local/bin)
#   KEYS_BASE_URL=URL        download from URL (needs KEYS_VERSION); for mirrors and tests

set -euo pipefail

REPO=danielfrg/vault-keys
PAGES=https://danielfrg.github.io/vault-keys

die() { printf 'keys: %s\n' "$*" >&2; exit 1; }

for cmd in curl uname tar mktemp awk; do
    command -v "$cmd" >/dev/null 2>&1 || die "missing dependency: $cmd"
done
if command -v sha256sum >/dev/null 2>&1; then sha256() { sha256sum "$1"; }
elif command -v shasum >/dev/null 2>&1; then sha256() { shasum -a 256 "$1"; }
else die 'missing dependency: sha256sum or shasum'; fi

case "$(uname -s)" in
    Darwin) os=darwin ;;
    Linux) os=linux ;;
    *) die "unsupported OS: $(uname -s) (macOS and Linux only)" ;;
esac
case "$(uname -m)" in
    x86_64|amd64) arch=amd64 ;;
    aarch64|arm64) arch=arm64 ;;
    *) die "unsupported architecture: $(uname -m)" ;;
esac

channel=${KEYS_CHANNEL:-stable}
if [ -n "${KEYS_BASE_URL:-}" ]; then
    [ -n "${KEYS_VERSION:-}" ] || die 'KEYS_BASE_URL needs KEYS_VERSION'
    version=${KEYS_VERSION#v}
    base_url=$KEYS_BASE_URL
elif [ "$channel" = main ]; then
    [ -z "${KEYS_VERSION:-}" ] || die 'KEYS_CHANNEL=main cannot be combined with KEYS_VERSION'
    base_url=$PAGES/main
    version=$(curl -fsSL "$base_url/VERSION") || die "cannot read $base_url/VERSION"
    version=${version#v}
elif [ "$channel" = stable ]; then
    if [ -n "${KEYS_VERSION:-}" ]; then
        version=${KEYS_VERSION#v}
    else
        final=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest") \
            || die "cannot find the latest release (see https://github.com/$REPO/releases)"
        case "$final" in
            */releases/tag/*) version=${final##*/}; version=${version#v} ;;
            *) die "no release published yet; try KEYS_CHANNEL=main" ;;
        esac
    fi
    base_url=https://github.com/$REPO/releases/download/v$version
else
    die "invalid KEYS_CHANNEL: $channel (expected stable or main)"
fi
[[ $version =~ ^[0-9A-Za-z.+-]+$ ]] || die "invalid version: $version"

if [ -n "${KEYS_INSTALL_DIR:-}" ]; then
    install_dir=$KEYS_INSTALL_DIR
elif [ -d /usr/local/bin ] && [ -w /usr/local/bin ]; then
    install_dir=/usr/local/bin
else
    install_dir=$HOME/.local/bin
fi
mkdir -p "$install_dir" || die "cannot create $install_dir"
[ -w "$install_dir" ] || die "$install_dir is not writable; set KEYS_INSTALL_DIR"

work=$(mktemp -d "${TMPDIR:-/tmp}/keys-install.XXXXXX")
tmp_bin=''
trap 'rm -rf "$work"; [ -z "$tmp_bin" ] || rm -f "$tmp_bin"' EXIT

asset=keys_${version}_${os}_${arch}.tar.gz
echo "Downloading keys $version for $os/$arch..."
curl -fsSL "$base_url/$asset" -o "$work/$asset" || die "cannot download $base_url/$asset"
curl -fsSL "$base_url/SHA256SUMS" -o "$work/SHA256SUMS" || die 'cannot download SHA256SUMS'

expected=$(awk -v f="$asset" '$2 == f { n++; h = $1 } END { if (n == 1) print h; else exit 1 }' "$work/SHA256SUMS") \
    || die "no single checksum for $asset in SHA256SUMS"
actual=$(sha256 "$work/$asset"); actual=${actual%% *}
[ "$actual" = "$expected" ] || die "checksum mismatch for $asset"

mkdir "$work/x"
tar -xzf "$work/$asset" -C "$work/x" ./keys 2>/dev/null || tar -xzf "$work/$asset" -C "$work/x" keys \
    || die 'archive does not contain keys'
[ -f "$work/x/keys" ] && [ ! -L "$work/x/keys" ] || die 'archive keys is not a regular file'

# Stage next to the destination so the final rename is atomic.
tmp_bin=$(mktemp "$install_dir/.keys.XXXXXX")
cp "$work/x/keys" "$tmp_bin"
chmod 755 "$tmp_bin"
mv -f "$tmp_bin" "$install_dir/keys"
tmp_bin=''

echo "Installed keys $version to $install_dir/keys"
case ":$PATH:" in
    *":$install_dir:"*) ;;
    *) printf 'Add it to your PATH:\n  export PATH=%q:"$PATH"\n' "$install_dir" ;;
esac
echo 'Get started: export VAULT_ADDR=<your vault>; keys login --user <name>'
