"""BPE token attribution diagnostic for Phase 14 Experiment #7 Step 2.

Identifies which token dominates the BPE score (4.0668) in routing.py hunks.
Examines 5 hunks spread across the file at different end_line positions.
"""
from __future__ import annotations

import json
import math
from collections import Counter
from pathlib import Path

# ── Configuration ──────────────────────────────────────────────────────────────
REPO_ROOT = Path("/Users/damienmeur/projects/argot")
FASTAPI_ROOT = REPO_ROOT / ".argot/research/repos/fastapi"
ROUTING_PY = FASTAPI_ROOT / "fastapi/routing.py"
BPE_MODEL_B_PATH = REPO_ROOT / "engine/argot/research/reference/generic_tokens_bpe.json"

EPSILON = 1e-7
TARGET_END_LINES = [437, 799, 1527, 2528, 3498]

# Exclusion logic matching _collect_source_files / _is_excluded from random_hunk_sampler
_EXCLUDE_DIRS: frozenset[str] = frozenset({
    "test", "tests", "doc", "docs", "examples", "example",
    "migrations", "migration", "benchmarks", "benchmark",
    "fixtures", "scripts", "build", "dist", "__pycache__", ".git", ".tox", ".eggs",
})


def _is_excluded(path: Path, source_dir: Path) -> bool:
    try:
        rel = path.relative_to(source_dir)
    except ValueError:
        return True
    for part in rel.parts[:-1]:
        if part in _EXCLUDE_DIRS or part.startswith("test"):
            return True
    name = rel.name
    return name.startswith("test_") or name == "conftest.py"


def _collect_source_files(repo_dir: Path) -> list[Path]:
    return sorted(p for p in repo_dir.rglob("*.py") if not _is_excluded(p, repo_dir))

# ── Load tokenizer ─────────────────────────────────────────────────────────────
print("Loading UnixCoder tokenizer...")
from transformers import AutoTokenizer  # noqa: E402

tokenizer = AutoTokenizer.from_pretrained("microsoft/unixcoder-base")
vocab: dict[str, int] = tokenizer.get_vocab()
id_to_token: dict[int, str] = {v: k for k, v in vocab.items()}

# ── Load model B (generic reference) ──────────────────────────────────────────
print("Loading model B...")
raw_b = json.loads(BPE_MODEL_B_PATH.read_text(encoding="utf-8"))
model_b: dict[int, int] = {int(k): v for k, v in raw_b["token_counts"].items()}
total_b: int = raw_b["total_tokens"]

# ── Build model A (fastapi source — matching _collect_source_files exclusions) ─
print("Building model A from fastapi source (with exclusions matching scorer)...")
model_a_files = _collect_source_files(FASTAPI_ROOT)
print(f"  Using {len(model_a_files)} source files (after exclusions)")
counts_a: Counter[int] = Counter()
for py_file in model_a_files:
    try:
        source = py_file.read_text(encoding="utf-8", errors="replace")
    except OSError:
        continue
    ids = tokenizer.encode(source, add_special_tokens=False)
    counts_a.update(ids)
model_a: dict[int, int] = dict(counts_a)
total_a: int = sum(counts_a.values()) or 1
print(f"  Model A: {len(model_a)} unique tokens, {total_a} total")

# ── Read routing.py lines ──────────────────────────────────────────────────────
routing_lines = ROUTING_PY.read_text(encoding="utf-8", errors="replace").splitlines()
file_len = len(routing_lines)
print(f"routing.py has {file_len} lines")


def is_meaningful(token_str: str) -> bool:
    return len(token_str) >= 3 and any(c.isalnum() for c in token_str)


def analyse_hunk(end_line: int) -> dict:
    """Reconstruct extraction (lines 0..end_line) and find top BPE tokens."""
    actual_end = min(file_len, end_line)
    extraction = "\n".join(routing_lines[:actual_end])

    ids = tokenizer.encode(extraction, add_special_tokens=False)
    filtered = [i for i in ids if is_meaningful(id_to_token.get(i, ""))]
    if not filtered:
        filtered = ids

    scored: list[tuple[float, int]] = []
    for tok_id in filtered:
        llr = (
            math.log(model_b.get(tok_id, 0) / total_b + EPSILON)
            - math.log(model_a.get(tok_id, 0) / total_a + EPSILON)
        )
        scored.append((llr, tok_id))

    scored.sort(key=lambda x: -x[0])
    top_token_id = scored[0][1]
    top_llr = scored[0][0]
    top_str = id_to_token.get(top_token_id, "<unk>")

    # Find which line the top token FIRST appears in the extraction.
    # We'll scan lines looking for the raw token string (strip Ġ prefix = space prefix in RoBERTa vocab).
    clean_token = top_str.replace("Ġ", " ").replace("Ċ", "\n").strip()
    first_line = None
    for line_no, line_text in enumerate(routing_lines[:actual_end], start=1):
        if clean_token in line_text:
            first_line = line_no
            break

    # Also find via token-level position — encode each line segment progressively
    # to get the actual char position of first occurrence.
    top20 = [(llr, tok_id, id_to_token.get(tok_id, "<unk>")) for llr, tok_id in scored[:20]]

    return {
        "end_line": end_line,
        "actual_end": actual_end,
        "extraction_lines": actual_end,
        "top_token_id": top_token_id,
        "top_token_str": top_str,
        "top_token_clean": clean_token,
        "top_llr": top_llr,
        "first_line_in_extraction": first_line,
        "top20": top20,
    }


# ── Run analysis ───────────────────────────────────────────────────────────────
print("\n" + "=" * 70)
results = []
for end_line in TARGET_END_LINES:
    print(f"\nAnalysing hunk end_line={end_line}...")
    result = analyse_hunk(end_line)
    results.append(result)

    print(f"  Extraction lines: 1..{result['actual_end']}")
    print(f"  TOP token: '{result['top_token_str']}' (id={result['top_token_id']})")
    print(f"  LLR: {result['top_llr']:.6f}")
    print(f"  First appears at line: {result['first_line_in_extraction']}")
    if result["first_line_in_extraction"]:
        line_content = routing_lines[result["first_line_in_extraction"] - 1]
        print(f"  Line content: {line_content!r}")
    print("  Top 20 tokens by LLR:")
    for rank, (llr, tid, tstr) in enumerate(result["top20"], 1):
        print(f"    {rank:2d}. '{tstr}' (id={tid}) LLR={llr:.6f}")


# ── Summary ────────────────────────────────────────────────────────────────────
print("\n" + "=" * 70)
print("SUMMARY")
print("=" * 70)
top_tokens = [r["top_token_str"] for r in results]
top_token_ids = [r["top_token_id"] for r in results]
all_same = len(set(top_token_ids)) == 1

print(f"Top token per hunk: {top_tokens}")
print(f"All same token: {all_same}")
if all_same:
    tok_str = results[0]["top_token_str"]
    tok_id = results[0]["top_token_id"]
    first_lines = [r["first_line_in_extraction"] for r in results]
    print(f"Dominant token: '{tok_str}' (id={tok_id})")
    print(f"Appears at lines in extractions: {first_lines}")
    print("Context at first occurrence (hunk with end_line=437):")
    fl = results[0]["first_line_in_extraction"]
    if fl:
        context_start = max(0, fl - 3)
        context_end = min(file_len, fl + 2)
        for i in range(context_start, context_end):
            marker = ">>>" if (i + 1) == fl else "   "
            print(f"  {marker} line {i+1:4d}: {routing_lines[i]}")
