#!/usr/bin/env bash
# Deterministic no-state audit-to-habit receipt. The semantic model stays
# offline so this fixture records the no-network branch without downloading it.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
binary=${ARGOT_BIN:-"$root/target/debug/argot"}
fixture=${1:-"$root/.github/fixtures/release-journey/app.py"}
work=$(mktemp -d "${TMPDIR:-/tmp}/argot-release-journey.XXXXXX")
repo="$work/repo"
receipt="$work/receipt.txt"
trap 'rm -rf "$work"' EXIT

test -x "$binary"
mkdir -p "$repo"
cp "$fixture" "$repo/app.py"
git -C "$repo" init -q
git -C "$repo" config user.name 'Argot release fixture'
git -C "$repo" config user.email 'release-fixture@example.invalid'
git -C "$repo" add app.py
git -C "$repo" commit -qm baseline
printf '# release journey fixture\n' > "$repo/README.md"
git -C "$repo" add README.md
git -C "$repo" commit -qm 'fixture history'

printf 'network: offline (ARGOT_OFFLINE=1; no model download)\n' > "$receipt"
test ! -e "$repo/.argot"
"$binary" audit --repo "$repo" --commits 1 --format json > "$work/audit.json"
ARGOT_OFFLINE=1 "$binary" init --repo "$repo" > "$work/init.txt"
test -f "$repo/.argot/scorer-config.json"
printf 'mutation: init created .argot/scorer-config.json\n' >> "$receipt"

# The candidate is intentionally a locally installed binary in CI, not the
# developer's copy. Inspecting the uninstall plan in an empty user home proves
# that this journey never takes ownership of the runner's existing state.
home="$work/home"
mkdir -p "$home"
HOME="$home" XDG_CONFIG_HOME="$home/.config" XDG_CACHE_HOME="$home/.cache" \
  "$binary" uninstall --dry-run > "$work/uninstall.txt"
grep -F 'raw binary (no receipt, not npm)' "$work/uninstall.txt" > /dev/null
printf 'ownership: uninstall dry run leaves authored source untouched\n' >> "$receipt"

printf '\nimport requests\n' >> "$repo/app.py"
set +e
ARGOT_OFFLINE=1 "$binary" check --repo "$repo" --format json > "$work/finding.json"
check_status=$?
set -e
test "$check_status" -eq 1
hash=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["hits"][0]["hash"])' "$work/finding.json")
test -n "$hash"
printf 'exit: finding check=%s\n' "$check_status" >> "$receipt"

(
  cd "$repo"
  "$binary" mute "$hash" --reason 'release journey fixture' > "$work/mute.txt"
)
test -f "$repo/argot.toml"
printf 'mutation: mute appended an auditable reason to argot.toml\n' >> "$receipt"
ARGOT_OFFLINE=1 "$binary" check --repo "$repo" --format json > "$work/suppressed.json"
python3 -c '
import json, sys
result = json.load(open(sys.argv[1]))["result"]
raise SystemExit(not (result["exit_code"] == 0 and result["suppressed_hits"] == 1))
' "$work/suppressed.json"
printf 'exit: suppressed rerun=0\n' >> "$receipt"

cat "$receipt"
