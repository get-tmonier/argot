"""One-shot diagnostic: classify v2 typicality slipthroughs as H1 (size gate) or H2 (parse recovery)."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from argot_bench.typicality import TypicalityModel, compute_features

CASES = [
    ("rich",     "python",     "rich/_emoji_codes.py",                       878, 890),
    ("faker-js", "typescript", "src/locales/en/word/verb.ts",                608, 620),
    ("faker-js", "typescript", "src/locales/en/word/adjective.ts",           229, 324),
    ("faker-js", "typescript", "src/locales/ru/internet/domain_suffix.ts",     2,  14),
    ("faker-js", "typescript", "src/locales/ru/hacker/abbreviation.ts",        0,  32),
    ("faker-js", "typescript", "src/locales/el/hacker/abbreviation.ts",        0,  31),
]

REPO_ROOT = Path(__file__).parent.parent / "data"


def classify(f) -> str:
    if f.literal_leaf_ratio == 0.0 and f.named_leaf_count == 0:
        return "H2 (NEUTRAL — parse recovery bailed)"
    if f.literal_leaf_ratio > 0.80 and f.named_leaf_count <= 30:
        return f"H1 (size gate: ratio={f.literal_leaf_ratio:.3f} but named_leaves={f.named_leaf_count} ≤ 30)"
    if f.literal_leaf_ratio <= 0.80:
        return f"THIRD MODE (ratio={f.literal_leaf_ratio:.3f} < 0.80 — feature doesn't separate)"
    return "FLAGGED (should not appear here)"


for corpus, lang, fp, start, end in CASES:
    repo = REPO_ROOT / corpus / ".repo"
    full = repo / fp
    if not full.exists():
        print(f"[MISSING] {corpus}/{fp}")
        continue

    lines = full.read_text(encoding="utf-8").splitlines()
    hunk = "\n".join(lines[start:end])
    features = compute_features(hunk, lang)  # type: ignore[arg-type]
    model = TypicalityModel(language=lang)  # type: ignore[arg-type]
    is_atyp, _, _ = model.is_atypical(hunk)

    print(f"\n{'='*70}")
    print(f"corpus={corpus}  file={fp}  lines={start}-{end}")
    print(f"hunk prefix: {repr(hunk[:200])}")
    print(f"features: {features}")
    print(f"is_atypical: {is_atyp}")
    print(f"classification: {classify(features)}")
