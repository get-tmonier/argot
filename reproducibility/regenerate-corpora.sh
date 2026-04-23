#!/usr/bin/env bash
# regenerate-corpora.sh — rebuild pinned argot training corpora from targets.yaml
#
# Requires: uv (project's standard Python manager) — PyYAML ships in the project venv.
# This script uses `uv run python3 -c "import yaml"` to parse targets.yaml.
#
# Expected disk footprint (conservative estimate):
#   Python repos (~1 checkout each):   ~300 MB (fastapi ~50M, rich ~50M, faker ~30M)
#   TypeScript repos (5 PRs each):     ~600 MB (hono ~100M, ink ~50M, faker-js ~100M × 5)
#   Generated JSONL corpora:           ~500 MB total
#   Total:                             ~1.4 GB
#
# Expected runtime on M-series Mac:
#   Clone phase (all 6 targets, network-bound): ~5–15 minutes
#   Extract phase (per PR, CPU-bound):          ~2–5 minutes per target-PR pair
#   Total for all 6 targets × ~3 PRs avg:       ~1–2 hours

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_TARGETS="${SCRIPT_DIR}/targets.yaml"

DATA_DIR=""
TARGETS_FILE="${DEFAULT_TARGETS}"
TARGETS_FILTER=""
DRY_RUN=false

usage() {
  echo "Usage: $0 --data-dir <path> [--targets <targets.yaml>] [--targets-filter <name[,name,...]>] [--dry-run]"
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --data-dir)    DATA_DIR="$2"; shift 2 ;;
    --targets)     TARGETS_FILE="$2"; shift 2 ;;
    --targets-filter) TARGETS_FILTER="$2"; shift 2 ;;
    --dry-run)     DRY_RUN=true; shift ;;
    *) echo "Unknown argument: $1"; usage ;;
  esac
done

[[ -z "${DATA_DIR}" ]] && { echo "Error: --data-dir is required"; usage; }

if ! uv run python3 -c "import yaml" 2>/dev/null; then
  echo "Error: PyYAML not found in uv environment. Run: uv add pyyaml" >&2
  exit 1
fi

run_or_print() {
  if [[ "${DRY_RUN}" == "true" ]]; then
    echo "[dry-run] $*"
  else
    "$@"
  fi
}

# Parse targets.yaml into tab-separated lines: name\turl\tpr\tsha
# Each (target, pr) pair becomes one line.
parse_targets() {
  uv run python3 - "${TARGETS_FILE}" <<'PYEOF'
import sys, yaml
targets_file = sys.argv[1]
with open(targets_file) as f:
    data = yaml.safe_load(f)
for target in data.get("targets", []):
    name = target["name"]
    url  = target["url"]
    for pr_entry in target.get("prs", []):
        pr  = pr_entry["pr"]
        sha = pr_entry["sha"]
        print(f"{name}\t{url}\t{pr}\t{sha}")
PYEOF
}

# Build comma-padded filter string for portable substring matching (bash 3.2 compatible)
FILTER_PADDED=""
if [[ -n "${TARGETS_FILTER}" ]]; then
  FILTER_PADDED=",${TARGETS_FILTER},"
fi

mkdir -p "${DATA_DIR}"

PREV_TARGET=""

while IFS=$'\t' read -r target_name target_url pr_num sha; do
  # Apply filter (portable: check comma-padded string)
  if [[ -n "${FILTER_PADDED}" && "${FILTER_PADDED}" != *",${target_name},"* ]]; then
    continue
  fi

  REPO_DIR="${DATA_DIR}/${target_name}/.repo"
  OUTPUT_FILE="${DATA_DIR}/${target_name}/${pr_num}/dataset.jsonl"

  # Clone or fetch — once per target
  if [[ "${target_name}" != "${PREV_TARGET}" ]]; then
    if [[ -d "${REPO_DIR}/.git" ]]; then
      echo "[${target_name}] repo exists — fetching"
      run_or_print git -C "${REPO_DIR}" fetch --quiet
    else
      echo "[${target_name}] cloning from ${target_url}"
      run_or_print git clone --filter=blob:none --quiet "${target_url}" "${REPO_DIR}"
    fi
    PREV_TARGET="${target_name}"
  fi

  # Skip if output already present
  if [[ -f "${OUTPUT_FILE}" ]]; then
    echo "[${target_name}] PR ${pr_num} @ ${sha:0:8} — already present, skipping"
    continue
  fi

  echo "[${target_name}] PR ${pr_num} @ ${sha:0:8} — checking out"
  run_or_print git -C "${REPO_DIR}" checkout --quiet --detach "${sha}"

  echo "[${target_name}] PR ${pr_num} — extracting corpus"
  run_or_print mkdir -p "$(dirname "${OUTPUT_FILE}")"
  run_or_print uv run argot-extract "${REPO_DIR}" --out "${OUTPUT_FILE}"

  echo "[${target_name}] PR ${pr_num} — written to ${OUTPUT_FILE}"

done < <(parse_targets)

echo "Done."
