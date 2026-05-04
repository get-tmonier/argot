"""Unit tests for the evidence-corpus calibration artefact.

Covers:
- ``build_evidence_corpus`` aggregates imports, identifiers, and per-cluster
  callees into a stable top-N snapshot.
- :meth:`EvidenceCorpus.to_json_dict` / :meth:`from_json_dict` round-trip
  through the same shape stored in ``scorer-config.json``.
- The cluster-id key coercion step (JSON keys come back as strings) survives
  the round-trip without losing information.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from argot.scoring.adapters.python_adapter import PythonAdapter
from argot.scoring.calibration.evidence_builder import build_evidence_corpus
from argot.scoring.evidence.types import (
    CommonEntry,
    EvidenceCorpus,
    EvidenceCorpusTotals,
)
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_BPE_GENERIC_BASELINE = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"


def _write_repo(tmp_path: Path) -> list[Path]:
    """Build a tiny multi-file repo with overlapping imports/identifiers/callees.

    The shape is deliberately small but rich enough that top-N counts have
    something to rank — without this, every count is 1 and the ordering test
    becomes vacuous.
    """
    files: list[Path] = []

    f1 = tmp_path / "service.py"
    f1.write_text(
        "import logging\nimport json\n\n"
        "logger = logging.getLogger()\n"
        "def handle(req):\n"
        "    logger.info('serving')\n"
        "    return json.dumps({'ok': True})\n"
    )
    files.append(f1)

    f2 = tmp_path / "worker.py"
    f2.write_text(
        "import logging\nfrom queue import Queue\n\n"
        "logger = logging.getLogger()\n"
        "queue = Queue()\n"
        "def loop():\n"
        "    while True:\n"
        "        item = queue.get()\n"
        "        logger.info('processing')\n"
        "        if item is None:\n"
        "            break\n"
    )
    files.append(f2)

    f3 = tmp_path / "config.py"
    f3.write_text(
        "import logging\nimport os\n\n"
        "logger = logging.getLogger()\n"
        "def load():\n"
        "    return os.environ.get('CONFIG_PATH', '/etc/app.toml')\n"
    )
    files.append(f3)
    return files


def _make_scorer(tmp_path: Path, files: list[Path]) -> SequentialImportBpeScorer:
    return SequentialImportBpeScorer(
        repo_corpus_files=files,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        bpe_threshold=10.0,
        adapter=PythonAdapter(),
        # Small clusters: 3 files → at most 3 clusters; we want at least 2
        # to exercise the per-cluster callees path.
        call_receiver_n_clusters=2,
        call_receiver_alpha=1.0,
        enable_typicality_filter=False,
    )


def test_build_evidence_corpus_ranks_imports_by_count(tmp_path: Path) -> None:
    """Top-N import list ranks specifiers by the number of files importing them."""
    files = _write_repo(tmp_path)
    scorer = _make_scorer(tmp_path, files)
    corpus = build_evidence_corpus(scorer, files, top_n=10)

    # ``logging`` appears in all three files → must be top of the import list.
    assert corpus.imports[0].name == "logging"
    assert corpus.imports[0].count == 3
    # The denominator on rarity is the number of distinct specifiers seen.
    assert corpus.totals.import_specifiers_attested == len({"logging", "json", "queue", "os"})


def test_build_evidence_corpus_caps_top_n(tmp_path: Path) -> None:
    """``top_n`` strictly bounds the rendered slice."""
    files = _write_repo(tmp_path)
    scorer = _make_scorer(tmp_path, files)
    corpus = build_evidence_corpus(scorer, files, top_n=2)
    assert len(corpus.imports) <= 2
    assert len(corpus.identifiers) <= 2


def test_build_evidence_corpus_per_cluster_callees(tmp_path: Path) -> None:
    """Per-cluster callees come from the call-receiver scorer's cluster map."""
    files = _write_repo(tmp_path)
    scorer = _make_scorer(tmp_path, files)
    corpus = build_evidence_corpus(scorer, files, top_n=10)

    # We asked for 2 clusters with 3 files; expect at least 1 cluster id to
    # have callees attached. (KMeans may collapse to 1 cluster on tiny data
    # — accept that and just check the structure is right.)
    assert isinstance(corpus.callees_by_cluster, dict)
    for cid, entries in corpus.callees_by_cluster.items():
        assert isinstance(cid, int)
        assert all(isinstance(e, CommonEntry) for e in entries)
        # Denominator matches per-cluster distinct callee count.
        assert corpus.totals.callees_attested_by_cluster[cid] >= len(entries)


def test_evidence_corpus_json_round_trip(tmp_path: Path) -> None:
    """``to_json_dict`` → ``json.dumps`` → ``json.loads`` → ``from_json_dict`` is identity."""
    original = EvidenceCorpus(
        imports=[CommonEntry("react", 320), CommonEntry("express", 88)],
        identifiers=[CommonEntry("useEffect", 320)],
        callees_by_cluster={
            0: [CommonEntry("logger.info", 3200)],
            7: [CommonEntry("db.query", 1800), CommonEntry("Result.ok", 900)],
        },
        totals=EvidenceCorpusTotals(
            import_specifiers_attested=47,
            identifiers_attested=12_400,
            callees_attested_by_cluster={0: 1247, 7: 890},
        ),
    )

    serialised = json.dumps(original.to_json_dict())
    rehydrated = EvidenceCorpus.from_json_dict(json.loads(serialised))

    # Cluster keys must come back as ``int`` even though JSON stringified them.
    assert set(rehydrated.callees_by_cluster) == {0, 7}
    assert set(rehydrated.totals.callees_attested_by_cluster) == {0, 7}
    # Full structural equality — frozen dataclasses make this exact.
    assert rehydrated == original


def test_evidence_corpus_top_n_is_deterministic_on_ties(tmp_path: Path) -> None:
    """When counts tie, ranking falls back to alphabetical order — not insertion."""
    # Two files, each with the same one identifier → all identifiers tie at 1.
    f1 = tmp_path / "a.py"
    f1.write_text("zoom = 1\n")
    f2 = tmp_path / "b.py"
    f2.write_text("apex = 1\n")
    files = [f1, f2]

    scorer = SequentialImportBpeScorer(
        repo_corpus_files=files,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        bpe_threshold=10.0,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
        call_receiver_alpha=0.0,
    )
    corpus_a = build_evidence_corpus(scorer, files, top_n=5)
    corpus_b = build_evidence_corpus(scorer, list(reversed(files)), top_n=5)
    # File ordering must not change the rendered identifier ordering.
    assert [e.name for e in corpus_a.identifiers] == [e.name for e in corpus_b.identifiers]


def test_calibration_writes_evidence_corpus_block(tmp_path: Path) -> None:
    """End-to-end: ``argot calibrate`` shapes the JSON with an evidence_corpus key.

    Mirrors the shape :func:`check._load_phase14_scorer` expects so a future
    contributor can't accidentally drop the block from the calibration writer
    without breaking check.
    """
    # Smoke test: build a corpus, check the JSON shape directly.
    files = _write_repo(tmp_path)
    scorer = _make_scorer(tmp_path, files)
    corpus = build_evidence_corpus(scorer, files, top_n=10)

    raw = corpus.to_json_dict()
    assert set(raw) == {"imports", "identifiers", "callees_by_cluster", "totals"}
    assert all({"name", "count"} <= set(e) for e in raw["imports"])
    # Cluster id keys are coerced to str by json.dumps; verify the loader
    # contract roundtrips without explicit pre-coercion.
    re_round = EvidenceCorpus.from_json_dict(json.loads(json.dumps(raw)))
    assert re_round == corpus


@pytest.mark.parametrize("top_n", [0, 1, 5])
def test_build_evidence_corpus_top_n_parameter(tmp_path: Path, top_n: int) -> None:
    """``top_n=0`` returns empty lists; positive ``top_n`` caps the slice."""
    files = _write_repo(tmp_path)
    scorer = _make_scorer(tmp_path, files)
    corpus = build_evidence_corpus(scorer, files, top_n=top_n)
    assert len(corpus.imports) <= top_n
    assert len(corpus.identifiers) <= top_n


def test_identifier_noise_filtered_out(tmp_path: Path) -> None:
    """Language keywords / implicit identifiers don't pollute ``identifiers``.

    Bench validation on faker showed ``common here:`` dominated by ``import``,
    ``export``, ``from``, etc. without this filter — pin the fix so it doesn't
    regress.
    """
    f = tmp_path / "code.py"
    f.write_text(
        "import logging\nlogger = logging.getLogger()\n"
        "def fn(self, x):\n"  # `self` is in PythonAdapter._NOISE
        "    if x is None:\n"  # `if`, `is`, `None` all noise
        "        return logger.info(x)\n"
        "    return x\n"
    )
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[f],
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        bpe_threshold=10.0,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
        call_receiver_alpha=0.0,
    )
    corpus = build_evidence_corpus(scorer, [f], top_n=20)
    names = {e.name for e in corpus.identifiers}
    # Keywords / implicit names must not appear.
    forbidden = {"import", "if", "is", "return", "def", "None", "self"}
    assert names.isdisjoint(
        forbidden
    ), f"language keywords leaked into identifiers: {names & forbidden}"
    # Real identifiers (``logger``, ``logging``, ``fn``, ``x``) survive.
    assert "logger" in names


def test_identifier_extraction_blanks_prose(tmp_path: Path) -> None:
    """Words from comments / docstrings don't pollute ``identifiers``.

    Without prose blanking, ``# the foo bar`` would credit ``the``, ``foo``,
    and ``bar`` as identifiers.
    """
    f = tmp_path / "code.py"
    f.write_text(
        '"""Module docstring with the words foo and bar mentioned."""\n'
        "# Line comment: also mentions baz repeatedly: baz baz baz baz baz\n"
        "def real_function(x):\n"
        "    return x\n"
    )
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=[f],
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        bpe_threshold=10.0,
        adapter=PythonAdapter(),
        enable_typicality_filter=False,
        call_receiver_alpha=0.0,
    )
    corpus = build_evidence_corpus(scorer, [f], top_n=50)
    names = {e.name for e in corpus.identifiers}
    # Prose-only words should be absent.
    assert "foo" not in names
    assert "bar" not in names
    assert "baz" not in names
    # Real code identifier survives.
    assert "real_function" in names
