from __future__ import annotations

import textwrap
from collections import Counter
from pathlib import Path
from typing import Any

import pytest

from argot.research.signal.scorers.ast_contrastive import ContrastiveAstTreeletScorer


def _make_record(source: str, path: str = "test.py") -> dict[str, Any]:
    return {"context_before": [], "hunk_tokens": [{"text": source}], "file_path": path}


def test_smoke_empty_corpus() -> None:
    scorer = ContrastiveAstTreeletScorer()
    scorer.fit([])
    records = [_make_record("def foo(x): return x + 1")]
    scores = scorer.score(records)
    assert len(scores) == 1
    assert isinstance(scores[0], float)


def test_returns_float_after_fit() -> None:
    source = "def foo(x):\n    return x + 1"
    scorer = ContrastiveAstTreeletScorer()
    corpus = [_make_record(source)]
    scorer.fit(corpus)
    scores = scorer.score([_make_record(source)])
    assert len(scores) == 1
    assert isinstance(scores[0], float)


def test_direction_anomaly_high_when_rare_in_model_a() -> None:
    """Hunk with a treelet that is rare in the repo (model_A) but common in
    generic Python (model_B) should receive a HIGH anomaly score — i.e. it
    looks more generic than repo-specific, signalling a potential paradigm break.
    """
    treelet = "d1:AsyncFunctionDef>arguments"
    source = "async def foo(x, y): return x"

    scorer = ContrastiveAstTreeletScorer(epsilon=1.0)
    scorer._model_a = Counter({treelet: 0})  # rare in repo
    scorer._model_b = Counter({treelet: 500})  # common generically

    records = [_make_record(source)]
    scores = scorer.score(records)
    assert len(scores) == 1
    assert scores[0] > 0.0, f"Expected positive anomaly score, got {scores[0]}"


def test_direction_anomaly_low_when_common_in_model_a() -> None:
    """Hunk with a treelet that is common in the repo (model_A) should receive
    a LOW anomaly score — it looks repo-idiomatic, not a break.
    """
    treelet = "d1:AsyncFunctionDef>arguments"
    source = "async def foo(x, y): return x"

    scorer = ContrastiveAstTreeletScorer(epsilon=1.0)
    scorer._model_a = Counter({treelet: 500})  # common in repo
    scorer._model_b = Counter({treelet: 0})  # rare generically

    records = [_make_record(source)]
    scores = scorer.score(records)
    assert len(scores) == 1
    assert scores[0] < 0.0, f"Expected negative anomaly score, got {scores[0]}"


def test_minimum_treelet_returns_zero() -> None:
    scorer = ContrastiveAstTreeletScorer()
    scorer.fit([])
    # SyntaxError source → extract_treelets returns [] which is < 3
    records = [_make_record("def (")]
    scores = scorer.score(records)
    assert scores[0] == 0.0


# ---------------------------------------------------------------------------
# model_a_files kwarg — the fix for corpus-hunk sparsity
# ---------------------------------------------------------------------------


@pytest.fixture()
def two_py_files(tmp_path: Path) -> list[Path]:
    """Two small but complete .py files that ast.parse() can handle."""
    f1 = tmp_path / "a.py"
    f1.write_text(
        textwrap.dedent("""\
        import asyncio

        async def handle(request):
            return await request.json()
    """)
    )
    f2 = tmp_path / "b.py"
    f2.write_text(
        textwrap.dedent("""\
        from typing import List

        def process(items: List[int]) -> int:
            return sum(items)
    """)
    )
    return [f1, f2]


def test_model_a_files_builds_denser_model(two_py_files: list[Path]) -> None:
    """fit(model_a_files=...) should produce more treelet types than the
    lossy corpus-record fallback, which only parses ~14% of hunks."""
    scorer_files = ContrastiveAstTreeletScorer()
    scorer_files.fit([], model_a_files=two_py_files)

    scorer_corpus = ContrastiveAstTreeletScorer()
    # Corpus records with unparseable hunk tokens (the common failure mode)
    bad_records = [
        {"context_before": [], "hunk_tokens": [{"text": '{"key": "value"}'}], "file_path": "f.py"},
        {"context_before": [], "hunk_tokens": [{"text": "- old_line"}], "file_path": "f.py"},
    ]
    scorer_corpus.fit(bad_records)

    assert len(scorer_files._model_a) > len(scorer_corpus._model_a), (
        f"Expected file-based model_A to have more treelet types "
        f"({len(scorer_files._model_a)}) > corpus-based ({len(scorer_corpus._model_a)})"
    )


def test_model_a_files_identity_gives_near_zero_scores(two_py_files: list[Path]) -> None:
    """When model_A == model_B (forced identical counters), every score must
    be ~0.  This guards against sign bugs or asymmetric smoothing."""
    scorer = ContrastiveAstTreeletScorer(epsilon=1.0)
    scorer.fit([], model_a_files=two_py_files)
    # Force model_B to be identical to what we just built
    scorer._model_b = Counter(scorer._model_a)

    source = "def foo(x, y):\n    return x + y\n"
    scores = scorer.score([_make_record(source)])
    assert abs(scores[0]) < 1e-6, f"A=B should give score ≈ 0, got {scores[0]}"


def test_fixture_path_fallback_scores_unparseable_hunk(tmp_path: Path) -> None:
    """When hunk_source is a mid-block fragment (SyntaxError), the scorer
    must fall back to the full fixture file via _fixture_path."""
    full_file = tmp_path / "full.py"
    full_file.write_text(
        textwrap.dedent("""\
        import queue
        import multiprocessing

        def worker():
            q = queue.Queue()
            p = multiprocessing.Process(target=worker)
            p.start()
    """)
    )

    # Hunk is mid-block (indented fragment) — cannot be parsed standalone
    unparseable_record = {
        "context_before": [],
        "hunk_tokens": [{"text": "    q = queue.Queue()"}],
        "file_path": "some/file.py",
        "hunk_source": "    q = queue.Queue()\n    p = multiprocessing.Process()",
        "_fixture_path": str(full_file),
    }

    scorer = ContrastiveAstTreeletScorer()
    scorer.fit([])
    scores = scorer.score([unparseable_record])
    assert len(scores) == 1
    # Full file is parseable → treelets >= 3 → non-zero score
    assert scores[0] != 0.0, "Expected fallback to full fixture file, got 0"


def test_fixture_path_fallback_not_used_when_hunk_parseable(two_py_files: list[Path]) -> None:
    """When the hunk itself is parseable, _fixture_path must not be used
    (the fixture file might contain different patterns)."""
    decoy_file = two_py_files[0]  # parseable but different content
    parseable_hunk = "def foo(x):\n    return x + 1\n"

    record = {
        "context_before": [],
        "hunk_tokens": [],
        "file_path": "some/file.py",
        "hunk_source": parseable_hunk,
        "_fixture_path": str(decoy_file),
    }

    scorer = ContrastiveAstTreeletScorer(epsilon=1e-6)
    # model_A: treelet appears 900/1000 times → freq 90 %
    # model_B: treelet appears 1/1000 times → freq 0.1 %
    # → log(0.001) - log(0.9) is negative (idiomatic in A)
    scorer._model_a = Counter({"d1:FunctionDef>arguments": 900, "_pad": 100})
    scorer._model_b = Counter({"d1:FunctionDef>arguments": 1, "_pad": 999})

    scores_with_fallback = scorer.score([record])

    # Score should be negative (hunk is idiomatic in model_A)
    assert (
        scores_with_fallback[0] < 0.0
    ), "Hunk is parseable; model_A-favoured treelet should drive score negative"


def test_model_a_files_direction_correct(two_py_files: list[Path]) -> None:
    """A hunk that uses asyncio (present in the fixture files / model_A) should
    score LOWER (more idiomatic) than a hunk using only basic arithmetic."""
    scorer = ContrastiveAstTreeletScorer(epsilon=1.0)
    scorer.fit([], model_a_files=two_py_files)

    # Async pattern — common in model_A files
    async_source = "async def handle(req):\n    return await req.json()\n"
    # Generic arithmetic — not particularly present in model_A files
    generic_source = "import queue\nq = queue.Queue()\nq.put(1)\n"

    scores = scorer.score([_make_record(async_source), _make_record(generic_source)])
    async_score, generic_score = scores[0], scores[1]
    assert async_score < generic_score, (
        f"Async pattern (repo-idiomatic) should score lower than generic pattern. "
        f"async={async_score:.4f}, generic={generic_score:.4f}"
    )
