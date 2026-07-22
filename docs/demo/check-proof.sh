#!/usr/bin/env bash
# Regenerate into a temporary copy and compare every committed audit receipt.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
ARGOT="${ARGOT_BIN:-$ROOT/target/debug/argot}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/argot-proof-check.XXXXXX")"

cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

cp -R "$HERE" "$WORK/demo"
ARGOT_BIN="$ARGOT" "$WORK/demo/rebuild-proof.sh"
(cd "$WORK/demo" && sha256sum --check proof/checksums.sha256)
cmp "$HERE/proof/checksums.sha256" "$WORK/demo/proof/checksums.sha256"

# VHS/FFmpeg does not produce stable GIF bytes on this runner. Re-render it to
# prove the recording command works, but keep the deterministic checksum gate
# scoped to the JSON, Markdown, and HTML receipts documented in proof/README.
RECORD_AUDIT=1 ARGOT_BIN="$ARGOT" "$WORK/demo/rebuild-proof.sh"
test -s "$WORK/demo/proof/audit.gif"

echo "proof receipts are reproducible"
