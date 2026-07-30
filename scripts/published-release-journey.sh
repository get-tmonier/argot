#!/usr/bin/env bash
# Exercise the installers and lifecycle commands that users receive from a
# published GitHub release. This intentionally starts with a blank home and
# install prefix, so it never reads or modifies runner-owned Argot state.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
release_tag=${ARGOT_RELEASE_TAG:-latest}
fixture=${1:-"$root/.github/fixtures/release-journey/app.py"}
work=$(mktemp -d "${TMPDIR:-/tmp}/argot-published-release.XXXXXX")
home="$work/home"
prefix="$work/install"
config="$home/config"
cache="$home/cache"
repo="$work/repo"
trap 'rm -rf "$work"' EXIT

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) windows=1 ;;
    *) windows=0 ;;
esac

native_path() {
    if [ "$windows" -eq 1 ] && command -v cygpath > /dev/null 2>&1; then
        cygpath -w "$1"
    else
        printf '%s\n' "$1"
    fi
}

mkdir -p "$home" "$prefix" "$config" "$cache" "$repo"
if [ "$windows" -eq 1 ]; then
    export HOME="$(native_path "$home")"
    export USERPROFILE="$HOME"
    export LOCALAPPDATA="$(native_path "$home/localappdata")"
    export XDG_CONFIG_HOME="$(native_path "$config")"
    export XDG_CACHE_HOME="$(native_path "$cache")"
else
    export HOME="$home"
    export XDG_CONFIG_HOME="$config"
    export XDG_CACHE_HOME="$cache"
fi

case "$release_tag" in
    latest) download_base='https://github.com/get-tmonier/argot/releases/latest/download' ;;
    v*) download_base="https://github.com/get-tmonier/argot/releases/download/$release_tag" ;;
    *) download_base="https://github.com/get-tmonier/argot/releases/download/v$release_tag" ;;
esac

if [ "$windows" -eq 1 ]; then
    installer="$work/argot-installer.ps1"
    curl --proto '=https' --tlsv1.2 -fsSL "$download_base/argot-installer.ps1" -o "$installer"
    ARGOT_INSTALL_DIR="$(native_path "$prefix")" ARGOT_NO_MODIFY_PATH=1 \
        pwsh -NoProfile -ExecutionPolicy Bypass -File "$(native_path "$installer")"
    binary="$prefix/bin/argot.exe"
else
    installer="$work/argot-installer.sh"
    curl --proto '=https' --tlsv1.2 -fsSL "$download_base/argot-installer.sh" -o "$installer"
    chmod +x "$installer"
    ARGOT_INSTALL_DIR="$prefix" ARGOT_NO_MODIFY_PATH=1 "$installer"
    binary="$prefix/bin/argot"
fi

test -x "$binary"
"$binary" --version
test -f "$config/argot/argot-receipt.json"

cp "$fixture" "$repo/app.py"
git -C "$repo" init -q
git -C "$repo" config user.name 'Argot release fixture'
git -C "$repo" config user.email 'release-fixture@example.invalid'
git -C "$repo" add app.py
git -C "$repo" commit -qm baseline
printf '# published release journey fixture\n' > "$repo/README.md"
git -C "$repo" add README.md
git -C "$repo" commit -qm 'fixture history'

ARGOT_OFFLINE=1 "$binary" audit --repo "$repo" --commits 1 --format json > "$work/audit.json"
ARGOT_OFFLINE=1 "$binary" init --repo "$repo" > "$work/init.txt"
test -f "$repo/.argot/scorer-config.json"
# The semantic index proves the embedded model works in a fresh, air-gapped
# installation; no separate fetch/model-management command exists anymore.
test -f "$repo/.argot/semantic-index.json"
printf '\nimport requests\n' >> "$repo/app.py"
set +e
ARGOT_OFFLINE=1 "$binary" check --repo "$repo" --format json > "$work/finding.json"
check_status=$?
set -e
test "$check_status" -eq 1
python3 -c 'import json, sys; assert json.load(open(sys.argv[1]))["hits"]' "$work/finding.json"

# A receipt-backed install must be eligible for an update. On a release event
# it is normally already current; a manually selected older release updates.
"$binary" update > "$work/update.txt" 2>&1
grep -E 'Already up to date\.|Updated to argot ' "$work/update.txt" > /dev/null
test -x "$binary"

"$binary" uninstall --dry-run > "$work/uninstall-dry-run.txt"
grep -F 'shell installer (curl / powershell)' "$work/uninstall-dry-run.txt" > /dev/null
"$binary" uninstall --yes > "$work/uninstall.txt"
test ! -e "$config/argot/argot-receipt.json"
if [ "$windows" -eq 1 ] && test -e "$binary"; then
    # Windows cannot unlink the running executable. The command supplies the
    # manual final step; remove that remaining owned file to close the receipt.
    grep -F 'delete the binary itself' "$work/uninstall.txt" > /dev/null
    rm -f "$binary"
fi
test ! -e "$binary"

printf 'published release journey passed for %s\n' "$release_tag"
