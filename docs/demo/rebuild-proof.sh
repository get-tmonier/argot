#!/usr/bin/env bash
# Rebuild the authored check and audit proof receipts. See README.md.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
ARGOT="${ARGOT_BIN:-$ROOT/target/debug/argot}"
EXPECTED_VERSION="argot 0.2.89"
OUT="$HERE/proof"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/argot-proof.XXXXXX")"
RECORD_AUDIT="${RECORD_AUDIT:-1}"

cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

[ -x "$ARGOT" ] || { echo "error: argot binary not found: $ARGOT" >&2; exit 1; }
[ "$($ARGOT --version)" = "$EXPECTED_VERSION" ] || {
  echo "error: expected $EXPECTED_VERSION; got $($ARGOT --version)" >&2
  exit 1
}

repo="$WORK/authored-proof"
git init --quiet "$repo"
git -C "$repo" config user.name 'Argot proof fixture'
git -C "$repo" config user.email 'proof@example.invalid'
mkdir -p "$repo/src"

cat > "$repo/src/receipt.py" <<'PY'
def render_receipt(value):
    return {"value": value}
PY

git -C "$repo" add src/receipt.py
GIT_AUTHOR_DATE='2026-01-01T00:00:00Z' GIT_COMMITTER_DATE='2026-01-01T00:00:00Z' \
  git -C "$repo" commit --quiet -m baseline

(cd "$repo" && "$ARGOT" init >/dev/null)

cp "$HERE/receipts.py" "$repo/src/receipt.py"
git -C "$repo" add src/receipt.py
(cd "$repo" && "$ARGOT" check --staged --format json > "$WORK/authored-check.json") || check_status=$?
if [ "${check_status:-0}" -ne 1 ]; then
  echo "error: authored fixture must return Argot finding exit 1" >&2
  exit 1
fi

GIT_AUTHOR_DATE='2026-01-02T00:00:00Z' GIT_COMMITTER_DATE='2026-01-02T00:00:00Z' \
  git -C "$repo" commit --quiet -m 'add authored foreign import'

"$ARGOT" audit --repo "$repo" --commits 1 --format json > "$WORK/audit.json"
"$ARGOT" audit --repo "$repo" --commits 1 --format markdown > "$WORK/audit.md"
"$ARGOT" audit --repo "$repo" --commits 1 --format html > "$WORK/audit.html"

if [ "$RECORD_AUDIT" = 1 ]; then
  command -v vhs >/dev/null || { echo "error: vhs not found" >&2; exit 1; }
  cp "$HERE/audit.tape" "$repo/audit.tape"
  (cd "$repo" && PATH="$(dirname "$ARGOT"):$PATH" vhs audit.tape)
fi

mkdir -p "$OUT"
cp "$WORK/authored-check.json" "$OUT/authored-check.json"
cp "$WORK/audit.json" "$OUT/audit.json"
cp "$WORK/audit.md" "$OUT/audit.md"
cp "$WORK/audit.html" "$OUT/audit.html"
if [ "$RECORD_AUDIT" = 1 ]; then
  cp "$repo/audit.gif" "$OUT/audit.gif"
  [ -s "$OUT/audit.gif" ] || { echo "error: audit recording is empty" >&2; exit 1; }
fi
(cd "$HERE" && sha256sum proof/authored-check.json proof/audit.json proof/audit.md proof/audit.html > proof/checksums.sha256)

echo "wrote authored check and audit receipts to $OUT"
