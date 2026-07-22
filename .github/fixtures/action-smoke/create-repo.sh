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
