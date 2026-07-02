#!/usr/bin/env bash
# Re-render docs/demo/demo.gif for the README.
#
# Reproducible demo: fit argot on the pinned FastAPI benchmark checkout, plant
# the out-of-voice hunk (receipts.py), then record `argot check --staged` with
# VHS. The rendered hit is byte-identical to the sample quoted in the README.
#
# Requirements: vhs (brew install vhs → pulls ttyd + ffmpeg), a release `argot`
# on PATH (or run `just build` first), and network access to clone FastAPI on
# the first run. Everything else is scripted.
set -euo pipefail

FASTAPI_SHA="88021c3dc016d02fe609397cb034648262c270e8"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
ARGOT="${ARGOT_BIN:-$ROOT/target/release/argot}"
WORK="${DEMO_WORKDIR:-$ROOT/target/demo-fastapi}"

command -v vhs >/dev/null || { echo "error: vhs not found (brew install vhs)"; exit 1; }
[ -x "$ARGOT" ] || { echo "error: argot binary not found at $ARGOT (run: just build)"; exit 1; }

if [ ! -d "$WORK/.git" ]; then
  echo "==> cloning FastAPI @ $FASTAPI_SHA"
  git clone --quiet https://github.com/fastapi/fastapi "$WORK"
  git -C "$WORK" checkout --quiet "$FASTAPI_SHA"
fi

echo "==> fitting argot on the checkout (once; seconds)"
git -C "$WORK" checkout --quiet -- . 2>/dev/null || true
git -C "$WORK" clean -fdq -e .argot
( cd "$WORK" && "$ARGOT" extract >/dev/null && "$ARGOT" fit >/dev/null )

echo "==> planting the out-of-voice hunk"
cp "$HERE/receipts.py" "$WORK/fastapi/receipts.py"
git -C "$WORK" add fastapi/receipts.py

echo "==> recording demo.gif"
cp "$HERE/demo.tape" "$WORK/demo.tape"
( cd "$WORK" && PATH="$(dirname "$ARGOT"):$PATH" vhs demo.tape )
mv "$WORK/demo.gif" "$HERE/demo.gif"
git -C "$WORK" restore --staged fastapi/receipts.py 2>/dev/null || true
rm -f "$WORK/fastapi/receipts.py" "$WORK/demo.tape"

echo "==> wrote $HERE/demo.gif"
