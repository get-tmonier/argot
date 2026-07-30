#!/usr/bin/env bash
set -euo pipefail

repo="$1"
scenario="$2"

mkdir -p "$repo"
cd "$repo"
git init -q
git config user.email "action-smoke@example.invalid"
git config user.name "Action smoke"

cat > sample.py <<'EOF'
from pathlib import Path


def read_name(path: str) -> str:
    return Path(path).name
EOF

git add sample.py
git commit -qm "base fixture"

# The Action is intentionally a pure consumer. CI fixtures therefore create a
# reviewed baseline locally before adding the commit that is scored. The caller
# supplies Argot's just-built binary so this script never downloads or fits in
# the Action under test.
if [ -n "${ARGOT_FIXTURE_ARGOT:-}" ]; then
  "$ARGOT_FIXTURE_ARGOT" init --repo "$repo"
  git add argot.toml .argot
  git commit -qm "fit argot snapshot"
fi

case "$scenario" in
  clean)
    git commit --allow-empty -qm "clean fixture"
    ;;
  finding)
    cat >> sample.py <<'EOF'

import requests
EOF
    git add sample.py
    git commit -qm "foreign import fixture"
    ;;
  *)
    echo "unknown action smoke scenario: $scenario" >&2
    exit 64
    ;;
esac
