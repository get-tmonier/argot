#!/usr/bin/env bash
# Build a deterministic git repo for `check` parity testing: enough real
# source files that calibration finds sampleable candidates, plus a final
# commit that introduces an anomaly (a foreign import + unusual call) which
# `argot check HEAD~1..HEAD` should flag.
set -euo pipefail
DEST="${1:?usage: build_check_repo.sh <target_dir>}"
rm -rf "$DEST"; mkdir -p "$DEST"; cd "$DEST"
export GIT_AUTHOR_NAME="Argot Fixture" GIT_AUTHOR_EMAIL="fixture@argot.test"
export GIT_COMMITTER_NAME="Argot Fixture" GIT_COMMITTER_EMAIL="fixture@argot.test"
git init -q -b main; git config core.autocrlf false; git config commit.gpgsign false
commit() { export GIT_AUTHOR_DATE="$2" GIT_COMMITTER_DATE="$2"; git add -A; git commit -q -m "$1"; }

cat > stats.py <<'PY'
from __future__ import annotations
import math


def mean(values):
    xs = list(values)
    if not xs:
        return 0.0
    return math.fsum(xs) / len(xs)


def variance(values):
    xs = list(values)
    if not xs:
        return 0.0
    m = mean(xs)
    return math.fsum((x - m) ** 2 for x in xs) / len(xs)
PY

cat > text.py <<'PY'
from __future__ import annotations


def normalize(line):
    stripped = line.strip()
    if not stripped:
        return ""
    return " ".join(stripped.split())


def wrap(text, width):
    words = text.split()
    lines = []
    current = ""
    for word in words:
        if len(current) + len(word) + 1 > width:
            lines.append(current)
            current = word
        else:
            current = f"{current} {word}".strip()
    if current:
        lines.append(current)
    return lines
PY

cat > store.py <<'PY'
from __future__ import annotations
import json
from pathlib import Path


class Store:
    def __init__(self, path):
        self.path = Path(path)
        self.data = {}

    def load(self):
        if self.path.exists():
            self.data = json.loads(self.path.read_text())
        return self.data

    def save(self):
        self.path.write_text(json.dumps(self.data))
PY

cat > graph.py <<'PY'
from __future__ import annotations
from collections import defaultdict


def build_adjacency(edges):
    adj = defaultdict(list)
    for a, b in edges:
        adj[a].append(b)
        adj[b].append(a)
    return adj


def neighbors(adj, node):
    return sorted(adj.get(node, []))
PY

commit "initial modules" "2021-01-01T00:00:00 +0000"

cat > pipeline.py <<'PY'
from __future__ import annotations
from stats import mean
from text import normalize


def summarize(rows):
    values = [len(normalize(r)) for r in rows]
    return {"count": len(rows), "avg_len": mean(values)}
PY
commit "add pipeline" "2021-01-02T00:00:00 +0000"

# Anomalous change: a foreign import + unusual construct.
cat > integration.py <<'PY'
from __future__ import annotations
import sqlalchemy
from sqlalchemy.orm import sessionmaker


def connect(url):
    engine = sqlalchemy.create_engine(url)
    Session = sessionmaker(bind=engine)
    return Session()
PY
commit "add sqlalchemy integration" "2021-01-03T00:00:00 +0000"
echo "built check fixture at $DEST"
git log --oneline
