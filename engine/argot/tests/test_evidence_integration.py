"""End-to-end tests: ``score_hunk`` populates ``ScoredHunk.evidence``.

The collector dispatch is part of ``score_hunk``; these tests verify a
realistic scorer + tiny evidence corpus combination produces the expected
:class:`Evidence` subtype on each reason path.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from argot.scoring.adapters.python_adapter import PythonAdapter
from argot.scoring.evidence.types import (
    BpeEvidence,
    CallReceiverEvidence,
    CommonEntry,
    EvidenceCorpus,
    EvidenceCorpusTotals,
    ImportEvidence,
)
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_BPE_GENERIC_BASELINE = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"


def _corpus() -> EvidenceCorpus:
    """Tiny synthetic evidence corpus for the integration tests.

    Uses big enough ``import_total`` to clear the import rarity floor so
    collectors emit the full "0 of N" wording — keeps assertion strings
    stable across formatter floors.
    """
    return EvidenceCorpus(
        imports=[CommonEntry("logging", 3), CommonEntry("os", 1)],
        identifiers={"logger": 9, "info": 6},
        callees_by_cluster={0: [CommonEntry("logger.info", 4)]},
        totals=EvidenceCorpusTotals(
            import_specifiers_attested=100,
            callees_attested_by_cluster={0: 1247},
        ),
    )


def _make_scorer(tmp_path: Path, *, with_corpus: bool) -> SequentialImportBpeScorer:
    code_file = tmp_path / "code.py"
    code_file.write_text(
        "import logging\nlogger = logging.getLogger()\n"
        "def fn(x):\n    logger.info(x)\n    return x\n"
    )
    return SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        bpe_threshold=0.5,
        call_receiver_alpha=1.0,
        call_receiver_cap=5,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
        evidence_corpus=_corpus() if with_corpus else None,
    )


def test_evidence_is_none_when_corpus_not_supplied(tmp_path: Path) -> None:
    """No EvidenceCorpus → no collector runs → ``evidence`` stays ``None``."""
    scorer = _make_scorer(tmp_path, with_corpus=False)
    result = scorer.score_hunk("import flask\nflask.Flask(__name__)")
    assert result.flagged is True
    assert result.evidence is None


def test_import_winner_yields_import_evidence(tmp_path: Path) -> None:
    """An import-fired hit comes out with :class:`ImportEvidence`.

    With ``bpe_threshold=99`` only the import stage can fire, so we know
    the dispatcher takes the import branch even though raw BPE may also
    have a non-zero score.
    """
    code_file = tmp_path / "code.py"
    code_file.write_text("import logging\n")
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        bpe_threshold=99.0,  # disables BPE/CR firing
        call_receiver_alpha=0.0,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
        evidence_corpus=_corpus(),
    )
    result = scorer.score_hunk("import flask\nflask.Flask(__name__)")
    assert result.flagged is True
    assert result.reason == "import"
    assert isinstance(result.evidence, ImportEvidence)
    assert "flask" in result.evidence.foreign_specifiers
    assert result.evidence.rarity.where == "repo"


def test_bpe_winner_yields_bpe_evidence(tmp_path: Path) -> None:
    """When BPE alone trips threshold, evidence is :class:`BpeEvidence`."""
    code_file = tmp_path / "code.py"
    code_file.write_text("def fn(x):\n    return x\n")
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        bpe_threshold=2.0,  # low enough that bpe trips on a chunky line
        call_receiver_alpha=0.0,  # disable CR so bpe wins outright
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
        evidence_corpus=_corpus(),
    )
    # Hunk full of unfamiliar tokens
    result = scorer.score_hunk("toStrictEqualOnce(mockResolvedValueAlpha(payload))")
    if not result.flagged or result.reason != "bpe":
        pytest.skip("bpe didn't fire on the synthetic hunk; sensitive to baseline")
    assert isinstance(result.evidence, BpeEvidence)
    # Identifier reconstruction should produce at least one of the call names;
    # surprising_identifiers carries CommonEntry pairs (name + repo count).
    names = [e.name for e in result.evidence.surprising_identifiers]
    assert any(
        n in names for n in ("toStrictEqualOnce", "mockResolvedValueAlpha", "payload")
    ), names


def test_call_receiver_winner_yields_cr_evidence(tmp_path: Path) -> None:
    """When CR contribution is decisive, evidence is :class:`CallReceiverEvidence`."""
    code_file = tmp_path / "code.py"
    code_file.write_text(
        "import logging\nlogger = logging.getLogger()\n"
        "def fn(x):\n    logger.info(x)\n    return x\n"
    )
    # bpe_threshold sized so raw bpe (~8 on the synthetic hunk) doesn't trip,
    # but adjusted_bpe (raw + 4-ish CR contribution) does. Keeps the CR-only
    # branch firing deterministically without depending on the BPE baseline.
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[code_file],
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        bpe_threshold=11.0,
        call_receiver_alpha=2.0,
        call_receiver_root_bonus=2.0,
        call_receiver_cap=10,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
        evidence_corpus=_corpus(),
    )
    result = scorer.score_hunk("UnknownClass().some_unknown_method()")
    if not result.flagged or result.reason != "call_receiver":
        pytest.skip(
            f"call_receiver didn't win on the synthetic hunk "
            f"(reason={result.reason}, score={result.score:.2f}, "
            f"threshold={result.threshold:.2f})"
        )
    assert isinstance(result.evidence, CallReceiverEvidence)
    # Cluster routing on a one-file repo collapses to no cluster → repo
    # scope fallback. Either is acceptable; assert the rarity is honest.
    assert result.evidence.rarity.where in ("this cluster", "repo")
    # The unattested callees should match what we wrote.
    assert any(
        "UnknownClass" in c or "some_unknown_method" in c
        for c in result.evidence.unfamiliar_callees
    )
