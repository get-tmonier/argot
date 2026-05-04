"""Build :class:`EvidenceCorpus` snapshots from a fitted scorer + repo files.

Run once during ``argot calibrate``; the output goes into
``scorer-config.json`` under the ``evidence_corpus`` key. The check-time
loader rehydrates it and hands it back to the scorer so per-reason
collectors can reach for ``common here:`` samples and rarity denominators
without re-tokenising the whole repo on every run.

Scope discipline: this module owns nothing except the corpus-aggregation
math. The data shape is owned by ``evidence/types.py``; per-reason
rendering is owned by ``evidence/formatters.py``.
"""

from __future__ import annotations

import re
from collections import Counter
from collections.abc import Iterable
from pathlib import Path

from argot.scoring.adapters.language_adapter import LanguageAdapter
from argot.scoring.evidence.types import (
    CommonEntry,
    EvidenceCorpus,
    EvidenceCorpusTotals,
)
from argot.scoring.scorers.sequential_import_bpe import (
    SequentialImportBpeScorer,
    _blank_prose_lines,
)

# Source-level identifier extractor: any sequence of identifier characters
# whose first character is a letter or underscore. Used for the repo-wide
# identifier vocabulary. A regex pass over source is faster and more
# accurate than rebuilding identifiers from BPE pieces, and the resulting
# vocabulary is what the user actually sees in their editor.
_IDENTIFIER_RE = re.compile(r"\b[A-Za-z_][A-Za-z0-9_]*\b")


def _count_imports(files: Iterable[Path], adapter: LanguageAdapter) -> Counter[str]:
    """Return per-specifier occurrence counts across all files.

    Each file contributes 1 to the count of every distinct top-level
    specifier it imports. Re-imports inside the same file collapse — what
    matters for orientation is "how many files in this repo touch X",
    not "how many ``import X`` lines exist".
    """
    counts: Counter[str] = Counter()
    for path in files:
        try:
            source = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        counts.update(adapter.extract_imports(source))
    return counts


def _count_identifiers(files: Iterable[Path], adapter: LanguageAdapter) -> Counter[str]:
    """Return per-identifier occurrence counts across all files.

    Counts every textual occurrence (every call to ``useEffect`` adds 1),
    not just the distinct files it appears in — frequency under
    ``common here:`` should reflect "how often you'll see this token", and
    repo-wide identifier counts also serve as the denominator for the
    BPE rarity stat.

    Filters out the language's reserved words / soft keywords / implicit
    identifiers via ``adapter.identifier_noise``, *and* blanks comment /
    docstring lines before regex extraction so prose words (``the``,
    ``a``, ``to``) from JSDoc don't pollute the rendered ``common here:``
    line. The prose blanking mirrors the symmetric treatment BPE scoring
    applies — keeping the two paths in lock-step means the rendered
    "common identifiers" align with what the BPE scorer actually saw.
    """
    noise = adapter.identifier_noise
    counts: Counter[str] = Counter()
    for path in files:
        try:
            source = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        prose = adapter.prose_line_ranges(source)
        clean = _blank_prose_lines(source, prose) if prose else source
        counts.update(t for t in _IDENTIFIER_RE.findall(clean) if t not in noise)
    return counts


def _top_n(counts: Counter[str], n: int) -> list[CommonEntry]:
    """Return the top-``n`` entries by count, ties broken alphabetically.

    Alphabetical tiebreak keeps the output deterministic across calibration
    runs even when the underlying counts are identical (common for tiny
    repos where many identifiers tie on count = 1).
    """
    return [
        CommonEntry(name=name, count=count)
        for name, count in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))[:n]
    ]


def build_evidence_corpus(
    scorer: SequentialImportBpeScorer,
    repo_corpus_files: Iterable[Path],
    *,
    top_n: int = 50,
) -> EvidenceCorpus:
    """Aggregate per-dimension top-``top_n`` samples from a fitted scorer.

    Pulls callees-by-cluster directly from the call-receiver scorer's
    ``cluster_callee_counts`` (already populated at fit time); recomputes
    imports and identifiers from the repo files because the scorer keeps
    only set-shaped derivatives of those.

    ``top_n`` defaults to 50 — large enough that the rendered ``+N more``
    overflow lands on a meaningful number while keeping calibration JSON
    footprint bounded on big repos.
    """
    files_list = list(repo_corpus_files)

    import_counts = _count_imports(files_list, scorer._adapter)  # noqa: SLF001
    identifier_counts = _count_identifiers(files_list, scorer._adapter)  # noqa: SLF001

    callees_by_cluster: dict[int, list[CommonEntry]] = {}
    callees_attested_by_cluster: dict[int, int] = {}
    cr = scorer._call_receiver  # noqa: SLF001
    if cr is not None:
        for cluster_id, callee_counts in cr.cluster_callee_counts.items():
            counter = Counter(callee_counts)
            callees_by_cluster[cluster_id] = _top_n(counter, top_n)
            callees_attested_by_cluster[cluster_id] = len(callee_counts)

    totals = EvidenceCorpusTotals(
        import_specifiers_attested=len(import_counts),
        identifiers_attested=len(identifier_counts),
        callees_attested_by_cluster=callees_attested_by_cluster,
    )

    return EvidenceCorpus(
        imports=_top_n(import_counts, top_n),
        identifiers=_top_n(identifier_counts, top_n),
        callees_by_cluster=callees_by_cluster,
        totals=totals,
    )


__all__ = ["build_evidence_corpus"]
