"""Phase 14 Exp #7 Step 5.5 — Single-flag token attribution diagnostic.

Procedure:
  1. Load the flagged record from the fix3 JSONL.
  2. Extract current-HEAD hunk at recorded line numbers.
  3. Re-score to confirm bpe_score matches the JSONL value.
  4. Per-token LLR attribution on the hunk.
  5. Cross-reference dominant token with the diff_content field.
  6. Label verdict vs §5 report claim.

Usage:
    uv run python engine/argot/research/signal/phase14/experiments/single_flag_attribution_2026_04_22.py
"""

from __future__ import annotations

import json
import math
from collections import Counter
from pathlib import Path

from argot.research.signal.phase14.calibration.random_hunk_sampler import (
    _DEFAULT_EXCLUDE_DIRS,
    _is_excluded,
    sample_hunks,
)
from argot.research.signal.phase14.scorers.sequential_import_bpe_scorer import (
    SequentialImportBpeScorer,
    _is_meaningful_token,
)

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent
_PROJECT_ROOT = _ARGOT_PKG.parent.parent
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_RESEARCH_DIR = Path(__file__).parent.parent.parent.parent
_BPE_MODEL_B_PATH = _RESEARCH_DIR / "reference" / "generic_tokens_bpe.json"
_SCRIPT_DIR = Path(__file__).parent
_HUNKS_JSONL = _SCRIPT_DIR / "real_pr_base_rate_hunks_fix3_2026_04_22.jsonl"
_FASTAPI_REPO = _REPOS_DIR / "fastapi"

_EPSILON = 1e-7
_SCORE_TOLERANCE = 1e-4
_REPORT_CLAIM = "_normalize_errors"


def _collect_source_files(repo_dir: Path) -> list[Path]:
    return sorted(
        p for p in repo_dir.rglob("*.py") if not _is_excluded(p, repo_dir, _DEFAULT_EXCLUDE_DIRS)
    )


def _extract_hunk(file_path: Path, start_line: int, end_line: int) -> str | None:
    if not file_path.exists():
        return None
    lines = file_path.read_text(encoding="utf-8", errors="replace").splitlines()
    lo = max(0, start_line - 1)
    hi = min(len(lines), end_line)
    return "\n".join(lines[lo:hi])


def main() -> None:
    # ── Step 1: load the flagged record ────────────────────────────────────────
    print("=" * 70)
    print("STEP 1 — Load flagged record")
    print("=" * 70)
    record: dict | None = None
    with _HUNKS_JSONL.open(encoding="utf-8") as fh:
        for line in fh:
            r = json.loads(line.strip())
            if r.get("flagged") and not r.get("is_test"):
                record = r
                break

    if record is None:
        print("ERROR: no flagged non-test record found in JSONL")
        return

    pr_number = record["pr_number"]
    file_path = record["file_path"]
    hunk_start = record["hunk_start_line"]
    hunk_end = record["hunk_end_line"]
    jsonl_bpe = record["bpe_score"]
    jsonl_threshold = record["bpe_threshold"]
    diff_content = record.get("diff_content", "")

    print(f"  pr_number      : {pr_number}")
    print(f"  file_path      : {file_path}")
    print(f"  hunk_start_line: {hunk_start}")
    print(f"  hunk_end_line  : {hunk_end}")
    print(f"  bpe_score      : {jsonl_bpe:.6f}")
    print(f"  bpe_threshold  : {jsonl_threshold:.6f}")
    print(f"  margin         : +{jsonl_bpe - jsonl_threshold:.6f}")
    assert pr_number == 14609, f"Expected PR 14609, got {pr_number}"
    assert file_path == "fastapi/routing.py", f"Unexpected file_path: {file_path}"
    print("  Assertions passed: pr_number==14609, file_path==fastapi/routing.py")

    # ── Step 2: extract current-HEAD hunk ─────────────────────────────────────
    print("\n" + "=" * 70)
    print("STEP 2 — Extract current-HEAD hunk")
    print("=" * 70)
    abs_path = _FASTAPI_REPO / file_path
    hunk_content = _extract_hunk(abs_path, hunk_start, hunk_end)
    if hunk_content is None:
        print(f"ERROR: {abs_path} does not exist")
        return

    print(f"  Lines {hunk_start}–{hunk_end} from current HEAD:")
    print("  ```")
    for line in hunk_content.splitlines():
        print(f"  {line}")
    print("  ```")
    normalize_in_hunk = _REPORT_CLAIM in hunk_content
    print(f"\n  '{_REPORT_CLAIM}' in hunk: {normalize_in_hunk}")

    # ── Step 3: re-score to confirm ────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("STEP 3 — Re-score in isolation")
    print("=" * 70)
    print("Initializing SequentialImportBpeScorer (seed=0, n_cal=100)...")
    model_a_files = _collect_source_files(_FASTAPI_REPO)
    cal_hunks = sample_hunks(_FASTAPI_REPO, 100, 0)
    file_text = abs_path.read_text(encoding="utf-8", errors="replace")
    scorer = SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=cal_hunks,
    )
    print(f"  model_a_files={len(model_a_files)}, n_cal={scorer.n_calibration}")
    print(f"  bpe_threshold={scorer.bpe_threshold:.6f}")

    result = scorer.score_hunk(hunk_content, file_source=file_text)
    rescored_bpe = result["bpe_score"]
    print(f"  re-scored bpe_score={rescored_bpe:.6f}")
    print(f"  JSONL    bpe_score={jsonl_bpe:.6f}")
    delta = abs(rescored_bpe - jsonl_bpe)
    reproducible = delta < _SCORE_TOLERANCE
    print(f"  delta={delta:.8f}  reproducible={reproducible}")
    if not reproducible:
        print("  STOP: score is NOT reproducible — block downstream work")
        return

    # ── Step 4: per-token LLR attribution ─────────────────────────────────────
    print("\n" + "=" * 70)
    print("STEP 4 — Per-token LLR attribution on hunk")
    print("=" * 70)
    # Replicate scorer internals
    tokenizer = scorer._tokenizer
    id_to_token = scorer._id_to_token
    model_a = scorer._model_a
    total_a = scorer._total_a
    model_b = scorer._model_b
    total_b = scorer._total_b

    ids: list[int] = tokenizer.encode(hunk_content, add_special_tokens=False)
    filtered = [i for i in ids if _is_meaningful_token(id_to_token.get(i, ""))]
    if not filtered:
        filtered = ids

    llr_by_id: list[tuple[float, int]] = []
    for tok_id in filtered:
        llr = (
            math.log(model_b.get(tok_id, 0) / total_b + _EPSILON)
            - math.log(model_a.get(tok_id, 0) / total_a + _EPSILON)
        )
        llr_by_id.append((llr, tok_id))

    llr_by_id.sort(key=lambda x: -x[0])
    max_llr, max_id = llr_by_id[0]
    max_str = id_to_token.get(max_id, "<unk>")

    # Find all lines in hunk where the dominant token occurs (by string match)
    clean_token = max_str.replace("Ġ", " ").replace("Ċ", "\n").strip()
    hunk_lines = hunk_content.splitlines()
    token_lines = [
        hunk_start + i for i, ln in enumerate(hunk_lines) if clean_token in ln
    ]

    print(f"  Dominant token: '{max_str}' (id={max_id})")
    print(f"  LLR: {max_llr:.6f}")
    print(f"  Clean form: '{clean_token}'")
    print(f"  Appears in hunk at absolute lines: {token_lines or '(none — token not found by string match)'}")

    print(f"\n  Top 10 tokens by LLR:")
    print(f"  {'Rank':>4}  {'Token':<30}  {'ID':>7}  {'LLR':>10}")
    print(f"  {'-'*4}  {'-'*30}  {'-'*7}  {'-'*10}")
    for rank, (llr, tid) in enumerate(llr_by_id[:10], 1):
        tstr = id_to_token.get(tid, "<unk>")
        print(f"  {rank:4d}  {repr(tstr):<30}  {tid:7d}  {llr:10.6f}")

    # Count occurrences of the dominant token in the hunk token stream
    from collections import Counter as _Counter
    tok_counts: dict[int, int] = _Counter(filtered)
    tok_occurrences = tok_counts.get(max_id, 0)
    print(f"\n  Occurrences of dominant token in filtered token stream: {tok_occurrences}")

    # ── Step 5: cross-reference with diff ─────────────────────────────────────
    print("\n" + "=" * 70)
    print("STEP 5 — Cross-reference with diff_content")
    print("=" * 70)
    print("  diff_content from JSONL record:")
    for dl in diff_content.splitlines():
        print(f"    {dl}")

    # Check where token appears in diff lines
    diff_lines = diff_content.splitlines()
    added_match = any(
        dl.startswith("+") and not dl.startswith("+++") and clean_token in dl
        for dl in diff_lines
    )
    removed_match = any(
        dl.startswith("-") and not dl.startswith("---") and clean_token in dl
        for dl in diff_lines
    )
    context_match = any(
        dl.startswith(" ") and clean_token in dl
        for dl in diff_lines
    )
    header_match = clean_token in diff_content and not (added_match or removed_match or context_match)

    print(f"\n  Dominant token '{clean_token}' in diff:")
    print(f"    added lines   (+): {added_match}")
    print(f"    removed lines (-): {removed_match}")
    print(f"    context lines ( ): {context_match}")
    print(f"    nowhere in diff  : {not (added_match or removed_match or context_match or header_match)}")

    # ── Step 6: verdict ────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("STEP 6 — Verdict")
    print("=" * 70)

    report_claimed_token = _REPORT_CLAIM
    dominant_is_claimed = report_claimed_token in clean_token or clean_token in report_claimed_token
    claimed_in_hunk = report_claimed_token in hunk_content

    if dominant_is_claimed and claimed_in_hunk:
        verdict = "§5 CONFIRMED"
        interp = (
            f"The dominant token is '{clean_token}', which matches the report's claim "
            f"('{report_claimed_token}'). It is present in the current-HEAD hunk content. "
            "The fix3 mechanism story stands."
        )
    elif not claimed_in_hunk and not (added_match or removed_match or context_match):
        verdict = "§5 WRONG (line drift)"
        interp = (
            f"The report claimed '{report_claimed_token}' drives the flag, but that token is "
            f"absent from the current-HEAD hunk at lines {hunk_start}–{hunk_end}. "
            f"The dominant token is '{clean_token}' (LLR={max_llr:.4f}), which also does not "
            "appear anywhere in the diff_content. This is a line-number drift: the hunk "
            f"header says lines {hunk_start}–{hunk_end} post-merge, but fastapi HEAD has "
            "since moved; those line numbers now point to unrelated code. Every old-PR "
            "result from fix3 (and fix1, exp #5) that relies on static HEAD line numbers "
            "is suspect and should be considered invalid."
        )
    elif not dominant_is_claimed:
        verdict = "§5 WRONG (different token)"
        interp = (
            f"The dominant token is '{clean_token}' (LLR={max_llr:.4f}), not "
            f"'{report_claimed_token}'. The §5 mechanism story has the wrong driver. "
            "The scorer produced the correct score, but the causal explanation in §5 "
            "must be rewritten with the correct attribution before moving on."
        )
    else:
        verdict = "§5 WRONG (line drift)"
        interp = (
            f"'{report_claimed_token}' is absent from current-HEAD lines {hunk_start}–{hunk_end}. "
            f"The dominant token is '{clean_token}' (LLR={max_llr:.4f}). "
            "Line-number drift from a post-merge diff applied to a moving HEAD."
        )

    print(f"\n  VERDICT: {verdict}")
    print(f"\n  {interp}")

    print("\n" + "=" * 70)
    print("DONE")
    print("=" * 70)


if __name__ == "__main__":
    main()
