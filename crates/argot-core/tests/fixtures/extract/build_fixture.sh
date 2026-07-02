#!/usr/bin/env bash
# Build a deterministic git repo for extract parity testing.
#
# Fixed author/committer identity and dates make the commit SHAs
# reproducible, so a golden dataset.jsonl captured from the Python engine
# stays valid across rebuilds. Usage: build_fixture.sh <target_dir>
set -euo pipefail

DEST="${1:?usage: build_fixture.sh <target_dir>}"
# CARGO_TARGET_TMPDIR is a backslash path on Windows; MSYS bash mangles
# backslashes in `cd`, so normalize to forward slashes.
DEST="${DEST//\\//}"
rm -rf "$DEST"
mkdir -p "$DEST"
cd "$DEST"

export GIT_AUTHOR_NAME="Argot Fixture"
export GIT_AUTHOR_EMAIL="fixture@argot.test"
export GIT_COMMITTER_NAME="Argot Fixture"
export GIT_COMMITTER_EMAIL="fixture@argot.test"

git init -q -b main
git config core.autocrlf false
git config commit.gpgsign false

commit() {
  # $1 = message, $2 = ISO date
  export GIT_AUTHOR_DATE="$2"
  export GIT_COMMITTER_DATE="$2"
  git add -A
  git commit -q -m "$1"
}

# --- commit 1: initial python module ---
cat > calc.py <<'PY'
from __future__ import annotations

import math
from typing import Iterable


def mean(values: Iterable[float]) -> float:
    """Arithmetic mean of a sequence."""
    xs = list(values)
    if not xs:
        return 0.0
    return math.fsum(xs) / len(xs)
PY
commit "add calc" "2020-01-01T00:00:00 +0000"

# --- commit 2: add a typescript file ---
cat > util.ts <<'TS'
import { readFileSync } from "node:fs";

export function loadLines(path: string): string[] {
  const text = readFileSync(path, "utf8");
  return text.split("\n").filter((l) => l.length > 0);
}
TS
commit "add util" "2020-01-02T00:00:00 +0000"

# --- commit 3: modify the python module (produces a mid-file hunk) ---
cat > calc.py <<'PY'
from __future__ import annotations

import math
from typing import Iterable


def mean(values: Iterable[float]) -> float:
    """Arithmetic mean of a sequence."""
    xs = list(values)
    if not xs:
        return 0.0
    return math.fsum(xs) / len(xs)


def variance(values: Iterable[float]) -> float:
    """Population variance."""
    xs = list(values)
    if not xs:
        return 0.0
    m = mean(xs)
    return math.fsum((x - m) ** 2 for x in xs) / len(xs)
PY
commit "add variance" "2020-01-03T00:00:00 +0000"

echo "built fixture at $DEST"
git log --oneline
