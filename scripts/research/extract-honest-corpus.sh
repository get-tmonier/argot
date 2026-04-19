#!/usr/bin/env bash
set -euo pipefail

WORK="${ARGOT_WORK:-$HOME/argot-research}"
OUT=".argot/research/datasets-v2"
LOG=".argot/research/datasets-v2/SHAS.md"

mkdir -p "$WORK" "$OUT"

# Pairs: "name url" — compatible with bash 3.2 (no associative arrays)
REPOS=(
  "httpx https://github.com/encode/httpx"
  "requests https://github.com/psf/requests"
  "ky https://github.com/sindresorhus/ky"
  "zod https://github.com/colinhacks/zod"
  "fastapi https://github.com/tiangolo/fastapi"
  "flask https://github.com/pallets/flask"
  "vite https://github.com/vitejs/vite"
  "typescript-eslint https://github.com/typescript-eslint/typescript-eslint"
  "pydantic https://github.com/pydantic/pydantic"
  "django https://github.com/django/django"
  "effect https://github.com/Effect-TS/effect"
  "angular https://github.com/angular/angular"
)

echo "# Pinned SHAs" > "$LOG"
echo "" >> "$LOG"
echo "| repo | sha | default branch |" >> "$LOG"
echo "|:-----|:----|:---------------|" >> "$LOG"

for pair in "${REPOS[@]}"; do
  name="${pair%% *}"
  url="${pair#* }"
  out_jsonl="$OUT/$name.jsonl"
  repo_dir="$WORK/$name"

  if [[ ! -d "$repo_dir" ]]; then
    echo "==> cloning $name"
    git clone --depth 5000 "$url" "$repo_dir"
  fi

  branch=$(git -C "$repo_dir" symbolic-ref --short HEAD || echo "HEAD")
  sha=$(git -C "$repo_dir" rev-parse HEAD)
  echo "| $name | \`$sha\` | $branch |" >> "$LOG"

  if [[ -s "$out_jsonl" ]]; then
    echo "==> $name already extracted at $out_jsonl — skipping"
    continue
  fi

  echo "==> extracting $name → $out_jsonl"
  uv run --package argot-engine python -m argot.extract "$repo_dir" \
    --out "$out_jsonl" \
    --repo-name "$name"
done

echo ""
echo "SHA log written to $LOG"
