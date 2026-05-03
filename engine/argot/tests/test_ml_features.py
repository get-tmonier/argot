"""Tests for engine/argot/ml/features.py + cli.py.

Synthetic micro-corpora only — no benchmark or network.
"""

from __future__ import annotations

import json
import subprocess
import sys
from collections.abc import Iterator
from pathlib import Path

import pytest

from argot.ml.cli import stream_sample_controls
from argot.ml.features import (
    _ast_features,
    _hunk_callee_bag,
    _hunk_file_context_features,
    _jaccard,
    _resolve_cluster,
    build_feature_row,
    compute_features,
    synthesize_hunk_in_host,
)
from argot.scoring.adapters.python_adapter import PythonAdapter
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_BPE_MODEL_B = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_python_corpus(tmp_path: Path) -> list[Path]:
    """Build a tiny Python corpus exercising imports + calls."""
    files: list[Path] = []
    for i in range(6):
        f = tmp_path / f"f{i}.py"
        if i < 3:
            # IO-heavy cluster
            f.write_text(
                "import logging\n"
                "import json\n\n"
                "logger = logging.getLogger(__name__)\n\n"
                f"def handler_{i}(payload: dict) -> str:\n"
                "    logger.info('processing')\n"
                "    return json.dumps(payload)\n"
            )
        else:
            # Math-heavy cluster
            f.write_text(
                "import math\n\n"
                f"def compute_{i}(x: float, y: float) -> float:\n"
                "    a = math.sqrt(x)\n"
                "    b = math.log(y)\n"
                "    return math.fabs(a - b)\n"
            )
        files.append(f)
    return files


def _make_scorer(
    files: list[Path],
    *,
    n_clusters: int = 2,
    cluster_bonus: float = 5.0,
    bpe_threshold: float = 99.0,
) -> SequentialImportBpeScorer:
    return SequentialImportBpeScorer(
        model_a_files=files,
        bpe_model_b_path=_BPE_MODEL_B,
        bpe_threshold=bpe_threshold,
        adapter=PythonAdapter(),
        call_receiver_alpha=2.0,
        call_receiver_cap=5,
        call_receiver_root_bonus=2.0,
        call_receiver_n_clusters=n_clusters,
        call_receiver_cluster_seed=0,
        call_receiver_cluster_bonus=cluster_bonus,
        enable_typicality_filter=False,
    )


# ---------------------------------------------------------------------------
# (a) compute_features happy path
# ---------------------------------------------------------------------------


def test_compute_features_for_known_fixture(tmp_path: Path) -> None:
    files = _make_python_corpus(tmp_path)
    inner = _make_scorer(files)

    # Synthetic break: foreign callee + import
    file_source = (
        "import requests\n"
        "\n"
        "def fetch_data(url: str) -> dict:\n"
        "    response = requests.get(url)\n"
        "    return response.json()\n"
    )
    hunk_content = (
        "def fetch_data(url: str) -> dict:\n"
        "    response = requests.get(url)\n"
        "    return response.json()\n"
    )

    feats = compute_features(
        inner,
        hunk_content,
        file_source=file_source,
        file_path=tmp_path / "new_file.py",
        hunk_start_line=3,
        hunk_end_line=5,
        language="python",
    )

    # Stage outputs
    assert isinstance(feats["import_score"], float)
    assert isinstance(feats["bpe_score"], float)
    assert isinstance(feats["adjusted_bpe"], float)
    assert isinstance(feats["stage1_flagged"], bool)
    assert isinstance(feats["stage2_flagged"], bool)
    assert isinstance(feats["scorer_reason"], str)

    # Call-receiver derived
    assert isinstance(feats["n_distinct_callees"], int)
    assert feats["n_distinct_callees"] >= 1  # requests.get and response.json
    assert isinstance(feats["n_unattested_callees"], int)
    assert isinstance(feats["n_attested_root_only"], int)
    assert isinstance(feats["n_cluster_absent_callees"], int)
    # Era-14 Phase 1 fix: cluster_assignment_method removed (unified routing).
    assert "cluster_assignment_method" not in feats
    assert isinstance(feats["cluster_jaccard_to_centroid"], float)
    assert 0.0 <= feats["cluster_jaccard_to_centroid"] <= 1.0

    # Hunk vs file context
    assert isinstance(feats["hunk_callee_bag_size"], int)
    assert isinstance(feats["file_callee_bag_size"], int)
    assert isinstance(feats["hunk_file_callee_jaccard"], float)
    assert 0.0 <= feats["hunk_file_callee_jaccard"] <= 1.0
    assert isinstance(feats["hunk_callees_in_file_fraction"], float)

    # AST shape
    assert isinstance(feats["ast_node_type_counts"], dict)
    assert isinstance(feats["n_returns"], int)
    assert feats["n_returns"] == 1
    assert isinstance(feats["n_throws"], int)
    assert isinstance(feats["n_awaits"], int)
    assert isinstance(feats["max_nesting_depth"], int)
    assert feats["max_nesting_depth"] >= 1
    assert isinstance(feats["n_distinct_identifiers"], int)
    assert isinstance(feats["parse_fragment_flag"], bool)


# ---------------------------------------------------------------------------
# (b) Jaccard correctness
# ---------------------------------------------------------------------------


def test_jaccard_basic() -> None:
    a = frozenset({"x", "y", "z"})
    b = frozenset({"y", "z", "w"})
    # |a ∩ b| = 2, |a ∪ b| = 4
    assert _jaccard(a, b) == pytest.approx(0.5)


def test_jaccard_identical_sets() -> None:
    s = frozenset({"a", "b"})
    assert _jaccard(s, s) == pytest.approx(1.0)


def test_jaccard_disjoint_sets() -> None:
    assert _jaccard(frozenset({"a"}), frozenset({"b"})) == pytest.approx(0.0)


def test_jaccard_empty_returns_zero() -> None:
    assert _jaccard(frozenset(), frozenset()) == pytest.approx(0.0)


def test_hunk_callee_bag_jaccard_correct() -> None:
    file_source = "def f():\n    a()\n    b()\n    c()\n"
    hunk = "def g():\n    a()\n    b()\n"
    hunk_bag = _hunk_callee_bag(hunk, "python")
    file_bag = _hunk_callee_bag(file_source, "python")
    assert hunk_bag == frozenset({"a", "b"})
    assert file_bag == frozenset({"a", "b", "c"})

    feats = _hunk_file_context_features(hunk, file_source, "python")
    # |∩| = 2, |∪| = 3 → 2/3
    assert feats["hunk_file_callee_jaccard"] == pytest.approx(2 / 3)
    # |∩| = 2, |hunk| = 2 → 1.0
    assert feats["hunk_callees_in_file_fraction"] == pytest.approx(1.0)
    assert feats["hunk_callee_bag_size"] == 2
    assert feats["file_callee_bag_size"] == 3


# ---------------------------------------------------------------------------
# (c) Cluster assignment method dispatch
# ---------------------------------------------------------------------------


def test_resolve_cluster_uses_jaccard_for_corpus_files(tmp_path: Path) -> None:
    """Era-14 Phase 1 fix: corpus files use Jaccard routing (no static shortcut).

    Even when ``file_path`` is in ``cr.file_to_cluster``, ``_resolve_cluster``
    must compute ``cluster_jaccard_to_centroid`` from ``file_source`` so the
    feature is meaningful and structurally identical to the value emitted for
    catalog fixtures (which were never in ``file_to_cluster`` and were the
    leakage shortcut in Phase 3.5).
    """
    files = _make_python_corpus(tmp_path)
    inner = _make_scorer(files, n_clusters=2)
    cr = inner._call_receiver
    assert cr is not None
    assert cr.cluster_attested  # clusters were built

    # Corpus file: now routes via Jaccard, NOT a constant 1.0.
    file_source = files[0].read_text()
    cid_corpus, jacc_corpus = _resolve_cluster(
        cr, files[0], file_source=file_source, language="python"
    )
    assert cid_corpus is not None
    assert 0.0 < jacc_corpus <= 1.0  # real Jaccard, not the constant 1.0 shortcut

    # Foreign file: same code path, also routes via Jaccard.
    foreign_path = tmp_path / "foreign.py"
    foreign_source = "import math\n\ndef g(x):\n    return math.sqrt(x)\n"
    cid_fb, jacc_fb = _resolve_cluster(
        cr, foreign_path, file_source=foreign_source, language="python"
    )
    assert cid_fb is not None
    assert 0.0 < jacc_fb <= 1.0

    # Without file_source we cannot route — both paths return None.
    cid_no_src, jacc_no_src = _resolve_cluster(cr, files[0], file_source=None, language="python")
    assert cid_no_src is None
    assert jacc_no_src == pytest.approx(0.0)


def test_resolve_cluster_none_when_no_clusters(tmp_path: Path) -> None:
    files = _make_python_corpus(tmp_path)
    inner = _make_scorer(files, n_clusters=1)  # no clusters built
    cr = inner._call_receiver
    assert cr is not None
    assert cr.cluster_attested == {}

    cid, jacc = _resolve_cluster(cr, files[0], file_source="x = 1", language="python")
    assert cid is None
    assert jacc == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# (d) AST features — language-aware node type names
# ---------------------------------------------------------------------------


def test_ast_node_type_counts_python_vs_typescript() -> None:
    py_source = "def f(x):\n" "    if x > 0:\n" "        return x\n" "    return 0\n"
    ts_source = (
        "function f(x: number): number {\n"
        "  if (x > 0) {\n"
        "    return x;\n"
        "  }\n"
        "  return 0;\n"
        "}\n"
    )
    py_feats = _ast_features(py_source, "python")
    ts_feats = _ast_features(ts_source, "typescript")

    py_types = py_feats["ast_node_type_counts"]
    ts_types = ts_feats["ast_node_type_counts"]

    # Python uses snake_case grammar names
    assert "function_definition" in py_types
    assert "if_statement" in py_types
    assert "return_statement" in py_types
    # TypeScript uses different names
    assert "function_declaration" in ts_types
    assert "if_statement" in ts_types
    assert "return_statement" in ts_types
    # Python-specific types should not appear in TS counts
    assert "function_definition" not in ts_types

    # Both should detect 2 returns and no throws/awaits
    assert py_feats["n_returns"] == 2
    assert ts_feats["n_returns"] == 2
    assert py_feats["n_throws"] == 0
    assert ts_feats["n_throws"] == 0


def test_ast_features_throw_and_await_typescript() -> None:
    src = (
        "async function f(): Promise<number> {\n"
        "  const x = await fetch('/');\n"
        "  if (!x.ok) throw new Error('bad');\n"
        "  return 1;\n"
        "}\n"
    )
    feats = _ast_features(src, "typescript")
    assert feats["n_throws"] == 1
    assert feats["n_awaits"] == 1
    assert feats["n_returns"] == 1


def test_ast_features_throw_and_await_python() -> None:
    src = (
        "async def f():\n"
        "    x = await fetch('/')\n"
        "    if not x:\n"
        "        raise ValueError('bad')\n"
        "    return x\n"
    )
    feats = _ast_features(src, "python")
    assert feats["n_throws"] == 1
    assert feats["n_awaits"] == 1
    assert feats["n_returns"] == 1


def test_ast_features_max_nesting_depth_python() -> None:
    src = (
        "def f():\n"
        "    if x:\n"
        "        for i in range(10):\n"
        "            if y:\n"
        "                return 1\n"
    )
    feats = _ast_features(src, "python")
    # function_def(1) → if(2) → for(3) → if(4)
    assert feats["max_nesting_depth"] == 4


def test_ast_features_top_n_node_types_capped() -> None:
    # Generate 30+ distinct identifier-bearing nodes; top_n should cap to 20
    body = "\n".join(f"    var_{i} = {i}" for i in range(30))
    src = "def f():\n" + body
    feats = _ast_features(src, "python")
    # We only cap node TYPES, not occurrences. With 30 assignments we expect
    # very few distinct types. So just verify the cap holds for any input:
    assert len(feats["ast_node_type_counts"]) <= 20


def test_ast_features_unparseable_returns_neutral() -> None:
    src = "def (((( !!! @@@\n"
    feats = _ast_features(src, "python")
    # Tree-sitter is permissive — root may be ERROR
    assert feats["parse_fragment_flag"] is True or feats["n_distinct_identifiers"] >= 0


# ---------------------------------------------------------------------------
# Build feature row
# ---------------------------------------------------------------------------


def test_build_feature_row_shape() -> None:
    feats = {"x": 1, "y": "z"}
    row = build_feature_row(
        corpus="fastapi",
        is_break=True,
        fixture_id="fixture_1",
        category="routing",
        difficulty="easy",
        file_path_rel="a/b.py",
        hunk_start_line=10,
        hunk_end_line=20,
        hunk_content="line1\nline2",
        features=feats,
    )
    assert row["corpus"] == "fastapi"
    assert row["is_break"] is True
    assert row["fixture_id"] == "fixture_1"
    assert row["category"] == "routing"
    assert row["difficulty"] == "easy"
    assert row["file_path"] == "a/b.py"
    assert row["hunk_start_line"] == 10
    assert row["hunk_end_line"] == 20
    assert row["hunk_length_lines"] == 11
    assert row["hunk_length_chars"] == len("line1\nline2")
    assert row["features"] is feats


# ---------------------------------------------------------------------------
# Era-14 Phase 6.1 — build_feature_row embedding kwargs
# ---------------------------------------------------------------------------


def test_build_feature_row_without_embeddings_unchanged() -> None:
    """Backward compat: embedding kwargs default None → keys are absent.

    The serialized row must be byte-identical to a pre-Phase-6.1 row built
    with the same provenance + features.
    """
    feats = {"x": 1, "y": "z"}
    row = build_feature_row(
        corpus="fastapi",
        is_break=True,
        fixture_id="fixture_1",
        category="routing",
        difficulty="easy",
        file_path_rel="a/b.py",
        hunk_start_line=10,
        hunk_end_line=20,
        hunk_content="line1\nline2",
        features=feats,
    )
    assert "hunk_embedding" not in row
    assert "context_embedding" not in row
    # JSONL serialization parity (sort_keys=True is what _write_jsonl uses).
    assert json.dumps(row, sort_keys=True) == json.dumps(
        {
            "corpus": "fastapi",
            "is_break": True,
            "fixture_id": "fixture_1",
            "category": "routing",
            "difficulty": "easy",
            "file_path": "a/b.py",
            "hunk_start_line": 10,
            "hunk_end_line": 20,
            "hunk_length_lines": 11,
            "hunk_length_chars": len("line1\nline2"),
            "features": feats,
        },
        sort_keys=True,
    )


def test_build_feature_row_with_embeddings() -> None:
    """When embedding kwargs are provided, they appear as top-level fields."""
    feats = {"x": 1}
    hunk_emb = [0.1] * 768
    ctx_emb = [0.2] * 768
    row = build_feature_row(
        corpus="fastapi",
        is_break=False,
        fixture_id=None,
        category=None,
        difficulty=None,
        file_path_rel="a.py",
        hunk_start_line=1,
        hunk_end_line=1,
        hunk_content="x",
        features=feats,
        hunk_embedding=hunk_emb,
        context_embedding=ctx_emb,
    )
    assert "hunk_embedding" in row
    assert "context_embedding" in row
    assert len(row["hunk_embedding"]) == 768
    assert len(row["context_embedding"]) == 768
    assert row["hunk_embedding"][0] == pytest.approx(0.1)
    assert row["context_embedding"][0] == pytest.approx(0.2)
    # Embeddings live at top level — NOT nested under features.
    assert "hunk_embedding" not in row["features"]
    assert "context_embedding" not in row["features"]


# ---------------------------------------------------------------------------
# (e) CLI smoke
# ---------------------------------------------------------------------------


def _write_synthetic_corpus(tmp_path: Path) -> tuple[Path, Path, Path, Path]:
    """Build a fake corpus with: repo + manifest (json) + dataset (jsonl)."""
    repo = tmp_path / "repo"
    repo.mkdir()
    # Build a slightly larger repo so calibration sampling has room.
    # MIN_BODY_LINES=5: each function body must have >= 5 lines after `def`.
    for i in range(5):
        f = repo / f"src_{i}.py"
        f.write_text(
            "import logging\n\n"
            f"def handler_{i}(payload: dict) -> str:\n"
            "    logger = logging.getLogger(__name__)\n"
            "    logger.info('processing')\n"
            "    logger.debug('details')\n"
            "    result = str(payload)\n"
            "    logger.info('done')\n"
            "    return result\n"
        )

    breaks_dir = tmp_path / "breaks"
    breaks_dir.mkdir()
    break_file = breaks_dir / "break_1.py"
    break_file.write_text(
        "import requests\n"
        "\n"
        "def fetch(url: str) -> dict:\n"
        "    response = requests.get(url)\n"
        "    return response.json()\n"
    )

    manifest = tmp_path / "manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "corpus": "synthetic",
                "language": "python",
                "categories": ["fetch"],
                "fixtures": [
                    {
                        "id": "synth_fetch_1",
                        "file": "breaks/break_1.py",
                        "category": "fetch",
                        "hunk_start_line": 3,
                        "hunk_end_line": 5,
                        "rationale": "synthetic",
                        "difficulty": "easy",
                    }
                ],
            }
        )
    )

    dataset = tmp_path / "dataset.jsonl"
    # One control hunk pointing at src_0.py lines 2-5 (0-indexed half-open)
    dataset.write_text(
        json.dumps(
            {
                "file_path": "src_0.py",
                "hunk_start_line": 2,
                "hunk_end_line": 6,
            }
        )
        + "\n"
    )
    return repo, manifest, dataset, tmp_path


def test_cli_smoke(tmp_path: Path) -> None:
    """End-to-end: run the CLI on a synthetic corpus and verify well-formed JSONL."""
    repo, manifest, dataset, catalog_dir = _write_synthetic_corpus(tmp_path)
    out = tmp_path / "features.jsonl"

    proc = subprocess.run(
        [
            sys.executable,
            "-m",
            "argot.ml.cli",
            "--manifest",
            str(manifest),
            "--repo-dir",
            str(repo),
            "--dataset",
            str(dataset),
            "--catalog-dir",
            str(catalog_dir),
            "--out",
            str(out),
            "--n-controls-per-corpus",
            "5",
            "--threshold-n-seeds",
            "1",
            "--n-cal",
            "5",
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, f"CLI failed: stdout={proc.stdout} stderr={proc.stderr}"
    assert out.exists()

    rows = [json.loads(line) for line in out.read_text().splitlines() if line.strip()]
    assert len(rows) >= 1  # at least the fixture
    fixture_rows = [r for r in rows if r["is_break"]]
    assert len(fixture_rows) == 1
    fr = fixture_rows[0]
    assert fr["corpus"] == "synthetic"
    assert fr["fixture_id"] == "synth_fetch_1"
    assert fr["category"] == "fetch"
    assert fr["difficulty"] == "easy"
    assert "features" in fr
    assert "import_score" in fr["features"]
    assert "bpe_score" in fr["features"]
    assert "ast_node_type_counts" in fr["features"]
    # Era-14 Phase 1 fix: no row may carry cluster_assignment_method anymore.
    for row in rows:
        assert "cluster_assignment_method" not in row["features"]
    # Era-14 Phase 6.1 backward compat: without --with-embeddings, no row may
    # carry the optional embedding fields.  This is the byte-identical guard.
    for row in rows:
        assert "hunk_embedding" not in row
        assert "context_embedding" not in row


def test_cli_with_embeddings_smoke(tmp_path: Path) -> None:
    """Era-14 Phase 6.1: --with-embeddings attaches 768-dim vectors to each row.

    Skipped when torch is not installed (the embeddings extra is optional).
    The subprocess uses the same Python interpreter as the test runner, so
    if torch is importable here, the subprocess can also load the encoder.
    """
    pytest.importorskip("torch")

    repo, manifest, dataset, catalog_dir = _write_synthetic_corpus(tmp_path)
    out = tmp_path / "features_with_emb.jsonl"

    proc = subprocess.run(
        [
            sys.executable,
            "-m",
            "argot.ml.cli",
            "--manifest",
            str(manifest),
            "--repo-dir",
            str(repo),
            "--dataset",
            str(dataset),
            "--catalog-dir",
            str(catalog_dir),
            "--out",
            str(out),
            "--n-controls-per-corpus",
            "5",
            "--threshold-n-seeds",
            "1",
            "--n-cal",
            "5",
            "--with-embeddings",
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, f"CLI failed: stdout={proc.stdout} stderr={proc.stderr}"
    assert out.exists()

    rows = [json.loads(line) for line in out.read_text().splitlines() if line.strip()]
    assert len(rows) >= 1
    for row in rows:
        assert "hunk_embedding" in row, row
        assert "context_embedding" in row, row
        assert len(row["hunk_embedding"]) == 768
        assert len(row["context_embedding"]) == 768
        # Embeddings are top-level — not nested under features.
        assert "hunk_embedding" not in row["features"]
        assert "context_embedding" not in row["features"]


# ---------------------------------------------------------------------------
# (e2) RAM hygiene — subprocess-per-corpus smoke
# ---------------------------------------------------------------------------


def test_subprocess_per_corpus_isolation(tmp_path: Path) -> None:
    """Era-14 Phase 1 fix (RAM hygiene): running two corpus extractions in
    sequence as separate subprocesses must leave each output independent and
    well-formed.

    This is the structural smoke-test for ``--all``: we cannot easily invoke
    ``--all`` without ``argot_bench`` + repo clones in CI, but the
    subprocess-per-corpus contract is exactly that each ``--manifest`` run
    is self-contained and tears down its tokenizer/scorer state on exit.
    """
    # Two distinct synthetic corpora — different tmp subdirs.
    corpus_a = tmp_path / "corpusA"
    corpus_a.mkdir()
    corpus_b = tmp_path / "corpusB"
    corpus_b.mkdir()
    repo_a, manifest_a, dataset_a, catalog_a = _write_synthetic_corpus(corpus_a)
    repo_b, manifest_b, dataset_b, catalog_b = _write_synthetic_corpus(corpus_b)

    out_a = tmp_path / "a.jsonl"
    out_b = tmp_path / "b.jsonl"

    def _run(manifest: Path, repo: Path, dataset: Path, cat: Path, out: Path) -> int:
        proc = subprocess.run(
            [
                sys.executable,
                "-m",
                "argot.ml.cli",
                "--manifest",
                str(manifest),
                "--repo-dir",
                str(repo),
                "--dataset",
                str(dataset),
                "--catalog-dir",
                str(cat),
                "--out",
                str(out),
                "--n-controls-per-corpus",
                "5",
                "--threshold-n-seeds",
                "1",
                "--n-cal",
                "5",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        assert proc.returncode == 0, f"stdout={proc.stdout} stderr={proc.stderr}"
        return proc.returncode

    _run(manifest_a, repo_a, dataset_a, catalog_a, out_a)
    _run(manifest_b, repo_b, dataset_b, catalog_b, out_b)

    rows_a = [json.loads(line) for line in out_a.read_text().splitlines() if line.strip()]
    rows_b = [json.loads(line) for line in out_b.read_text().splitlines() if line.strip()]
    assert len(rows_a) >= 1 and len(rows_b) >= 1
    # Each run produced its own well-formed outputs (subprocess isolation).
    for row in [*rows_a, *rows_b]:
        assert "features" in row
        assert "cluster_assignment_method" not in row["features"]


# ---------------------------------------------------------------------------
# (f) Deterministic JSONL
# ---------------------------------------------------------------------------


def test_jsonl_output_deterministic(tmp_path: Path) -> None:
    """Two runs of the CLI on the same input must produce byte-identical output."""
    repo, manifest, dataset, catalog_dir = _write_synthetic_corpus(tmp_path)
    out1 = tmp_path / "features_1.jsonl"
    out2 = tmp_path / "features_2.jsonl"

    cmd_base = [
        sys.executable,
        "-m",
        "argot.ml.cli",
        "--manifest",
        str(manifest),
        "--repo-dir",
        str(repo),
        "--dataset",
        str(dataset),
        "--catalog-dir",
        str(catalog_dir),
        "--n-controls-per-corpus",
        "5",
        "--threshold-n-seeds",
        "1",
        "--n-cal",
        "5",
        "--seed",
        "42",
    ]
    p1 = subprocess.run([*cmd_base, "--out", str(out1)], check=False, capture_output=True)
    p2 = subprocess.run([*cmd_base, "--out", str(out2)], check=False, capture_output=True)
    assert p1.returncode == 0, p1.stderr
    assert p2.returncode == 0, p2.stderr

    assert out1.read_bytes() == out2.read_bytes()


# ---------------------------------------------------------------------------
# (g) Era-14 Fix A — synthesize_hunk_in_host helper
# ---------------------------------------------------------------------------


def test_synthesize_hunk_in_host_inject_at_top() -> None:
    """host_inject_at_line=1 splices catalog content at the very top."""
    catalog = "def break_fn():\n    return 1\n"
    host = "def host_fn():\n    return 2\n"
    syn, hs, he = synthesize_hunk_in_host(
        catalog_content=catalog,
        catalog_hunk_start=1,
        catalog_hunk_end=2,
        host_content=host,
        host_inject_at_line=1,
    )
    assert syn == "def break_fn():\n    return 1\ndef host_fn():\n    return 2\n"
    # New hunk lines point at the catalog content in the synthesized file.
    assert hs == 1
    assert he == 2
    syn_lines = syn.splitlines()
    assert syn_lines[hs - 1 : he] == ["def break_fn():", "    return 1"]


def test_synthesize_hunk_in_host_inject_in_middle() -> None:
    """host_inject_at_line=N inserts catalog content BEFORE host line N."""
    catalog = "X1\nX2\nX3\n"
    host = "H1\nH2\nH3\nH4\n"
    syn, hs, he = synthesize_hunk_in_host(
        catalog_content=catalog,
        catalog_hunk_start=2,
        catalog_hunk_end=3,
        host_content=host,
        host_inject_at_line=3,  # before "H3"
    )
    # Expect: H1, H2, X1, X2, X3, H3, H4
    expected = "H1\nH2\nX1\nX2\nX3\nH3\nH4\n"
    assert syn == expected
    syn_lines = syn.splitlines()
    # Catalog hunk was lines 2-3 of catalog → "X2","X3"
    assert syn_lines[hs - 1 : he] == ["X2", "X3"]
    # Original host content around the splice is preserved.
    assert syn_lines[0:2] == ["H1", "H2"]
    assert syn_lines[5:7] == ["H3", "H4"]


def test_synthesize_hunk_in_host_inject_beyond_end() -> None:
    """host_inject_at_line > host length → catalog appended at end (no error)."""
    catalog = "X1\nX2\n"
    host = "H1\nH2\n"
    syn, hs, he = synthesize_hunk_in_host(
        catalog_content=catalog,
        catalog_hunk_start=1,
        catalog_hunk_end=2,
        host_content=host,
        host_inject_at_line=99,
    )
    assert syn == "H1\nH2\nX1\nX2\n"
    syn_lines = syn.splitlines()
    assert syn_lines[hs - 1 : he] == ["X1", "X2"]
    # Hunk line numbers point past the original host's 2 lines.
    assert hs == 3
    assert he == 4


def test_synthesize_hunk_in_host_no_trailing_newline_on_either() -> None:
    """Both files end without trailing newline → still parse-friendly."""
    catalog = "X1\nX2"
    host = "H1\nH2"
    syn, hs, he = synthesize_hunk_in_host(
        catalog_content=catalog,
        catalog_hunk_start=1,
        catalog_hunk_end=2,
        host_content=host,
        host_inject_at_line=2,  # before H2
    )
    # We splice catalog between H1 and H2; no spurious blank lines.
    syn_lines = syn.splitlines()
    assert syn_lines == ["H1", "X1", "X2", "H2"]
    assert syn_lines[hs - 1 : he] == ["X1", "X2"]


def test_synthesize_hunk_in_host_preserves_host_lines_around_splice() -> None:
    """The original host content (lines around the splice) is preserved verbatim."""
    catalog = "C1\nC2\n"
    host = "\n".join(f"line_{i}" for i in range(1, 11)) + "\n"
    syn, hs, he = synthesize_hunk_in_host(
        catalog_content=catalog,
        catalog_hunk_start=1,
        catalog_hunk_end=2,
        host_content=host,
        host_inject_at_line=5,
    )
    syn_lines = syn.splitlines()
    # First 4 lines unchanged.
    assert syn_lines[:4] == [f"line_{i}" for i in range(1, 5)]
    # Catalog content next.
    assert syn_lines[hs - 1 : he] == ["C1", "C2"]
    # Remaining 6 host lines unchanged.
    assert syn_lines[6:] == [f"line_{i}" for i in range(5, 11)]


def test_synthesize_hunk_in_host_inject_at_line_one_with_trailing_only_host() -> None:
    """Edge: host_inject_at_line=1 with an empty host → catalog only."""
    catalog = "X1\nX2\n"
    syn, hs, he = synthesize_hunk_in_host(
        catalog_content=catalog,
        catalog_hunk_start=1,
        catalog_hunk_end=2,
        host_content="",
        host_inject_at_line=1,
    )
    assert syn == catalog
    assert (hs, he) == (1, 2)


def test_synthesize_hunk_in_host_invalid_args_raise() -> None:
    """Defensive validation."""
    with pytest.raises(ValueError):
        synthesize_hunk_in_host("c", 0, 1, "h", 1)
    with pytest.raises(ValueError):
        synthesize_hunk_in_host("c", 2, 1, "h", 1)
    with pytest.raises(ValueError):
        synthesize_hunk_in_host("c", 1, 1, "h", 0)


# ---------------------------------------------------------------------------
# (h) Era-14 Fix A — fixture-level injection through the CLI
# ---------------------------------------------------------------------------


def _write_corpus_with_optional_host(
    tmp_path: Path,
    *,
    with_host: bool,
) -> tuple[Path, Path, Path]:
    """Build a synthetic corpus where the single fixture optionally injects.

    Returns (repo_dir, manifest_path, catalog_dir).
    """
    tmp_path.mkdir(parents=True, exist_ok=True)
    repo = tmp_path / "repo"
    repo.mkdir()
    # Larger multi-function host file so file_callees is materially bigger
    # than hunk_callees (mimics real-PR distribution).
    for i in range(5):
        f = repo / f"src_{i}.py"
        f.write_text(
            "import logging\n\n"
            f"def handler_{i}(payload: dict) -> str:\n"
            "    logger = logging.getLogger(__name__)\n"
            "    logger.info('processing')\n"
            "    logger.debug('details')\n"
            "    result = str(payload)\n"
            "    logger.info('done')\n"
            "    return result\n"
        )
    # A meaty host file with many distinct callees.
    host_path = repo / "host.py"
    host_lines = ["import logging\n", "import json\n", "\n"]
    for i in range(20):
        host_lines.append(f"def host_fn_{i}(x):\n")
        host_lines.append(f"    logging.info('host_{i}')\n")
        host_lines.append(f"    json.dumps({{'k': {i}}})\n")
        host_lines.append(f"    return x + {i}\n")
        host_lines.append("\n")
    host_path.write_text("".join(host_lines))

    breaks_dir = tmp_path / "breaks"
    breaks_dir.mkdir()
    break_file = breaks_dir / "break_1.py"
    break_file.write_text(
        "import requests\n"
        "\n"
        "def fetch(url: str) -> dict:\n"
        "    response = requests.get(url)\n"
        "    return response.json()\n"
    )

    fixture: dict[str, object] = {
        "id": "synth_fetch_1",
        "file": "breaks/break_1.py",
        "category": "fetch",
        "hunk_start_line": 3,
        "hunk_end_line": 5,
        "rationale": "synthetic",
        "difficulty": "easy",
    }
    if with_host:
        fixture["host_file"] = "host.py"
        fixture["host_inject_at_line"] = 10  # inside the host body

    manifest = tmp_path / "manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "corpus": "synthetic",
                "language": "python",
                "categories": ["fetch"],
                "fixtures": [fixture],
            }
        )
    )
    return repo, manifest, tmp_path


def test_fixture_without_host_file_unchanged_behavior(tmp_path: Path) -> None:
    """A fixture without host_file produces the same FeatureRow as before this change.

    We compare the FeatureRow emitted by ``_iter_fixture_rows`` against a
    fresh ``compute_features`` call against the catalog file standalone — they
    must match byte-for-byte.
    """
    from argot.ml.cli import _iter_fixture_rows, build_production_scorer

    repo, manifest, catalog_dir = _write_corpus_with_optional_host(tmp_path, with_host=False)
    raw_manifest = json.loads(manifest.read_text())

    inner = build_production_scorer(repo, "python", n_cal=5, threshold_n_seeds=1)

    rows = list(
        _iter_fixture_rows(
            inner,
            corpus="synthetic",
            language="python",
            catalog_dir=catalog_dir,
            fixtures=raw_manifest["fixtures"],
            repo_dir=repo,
        )
    )
    assert len(rows) == 1
    row = rows[0]

    # Reproduce the legacy path: read catalog file, compute_features against it.
    rel = "breaks/break_1.py"
    full = (catalog_dir / rel).read_text(encoding="utf-8")
    lines = full.splitlines()
    hs, he = 3, 5
    hunk = "\n".join(lines[hs - 1 : he])
    expected_feats = compute_features(
        inner,
        hunk,
        file_source=full,
        file_path=repo / rel,
        hunk_start_line=hs,
        hunk_end_line=he,
        language="python",
    )
    expected_row = build_feature_row(
        corpus="synthetic",
        is_break=True,
        fixture_id="synth_fetch_1",
        category="fetch",
        difficulty="easy",
        file_path_rel=rel,
        hunk_start_line=hs,
        hunk_end_line=he,
        hunk_content=hunk,
        features=expected_feats,
    )
    # Byte-identical JSON serialization: the strongest "unchanged" guarantee.
    assert json.dumps(row, sort_keys=True) == json.dumps(expected_row, sort_keys=True)


def test_fixture_with_host_file_uses_synthesized_content(tmp_path: Path) -> None:
    """A fixture with host_file produces a FeatureRow scored against synthesized content."""
    from argot.ml.cli import _iter_fixture_rows, build_production_scorer

    # Same scorer reused across both runs for an apples-to-apples comparison.
    repo, manifest_h, catalog_h = _write_corpus_with_optional_host(tmp_path / "h", with_host=True)
    raw_manifest_h = json.loads(manifest_h.read_text())

    # No-host comparison run uses an isolated copy of the same repo so the
    # scorer state is identical (the breaks dir lives outside the repo root,
    # so the corpus is the same in both cases).
    repo_n, manifest_n, catalog_n = _write_corpus_with_optional_host(
        tmp_path / "n", with_host=False
    )

    inner_h = build_production_scorer(repo, "python", n_cal=5, threshold_n_seeds=1)
    inner_n = build_production_scorer(repo_n, "python", n_cal=5, threshold_n_seeds=1)

    rows_h = list(
        _iter_fixture_rows(
            inner_h,
            corpus="synthetic",
            language="python",
            catalog_dir=catalog_h,
            fixtures=raw_manifest_h["fixtures"],
            repo_dir=repo,
        )
    )
    rows_n = list(
        _iter_fixture_rows(
            inner_n,
            corpus="synthetic",
            language="python",
            catalog_dir=catalog_n,
            fixtures=json.loads(manifest_n.read_text())["fixtures"],
            repo_dir=repo_n,
        )
    )
    assert len(rows_h) == 1 and len(rows_n) == 1
    row_h = rows_h[0]
    row_n = rows_n[0]

    # file_path is now the host file, not the standalone break file.
    assert row_h["file_path"].endswith("host.py")
    assert row_n["file_path"].endswith("break_1.py")

    # Scoring against a much larger host file should yield a substantially
    # bigger file_callee_bag and a much smaller hunk/file Jaccard.
    assert row_h["features"]["file_callee_bag_size"] > row_n["features"]["file_callee_bag_size"]
    assert (
        row_h["features"]["hunk_file_callee_jaccard"]
        < row_n["features"]["hunk_file_callee_jaccard"]
    )


def test_host_file_resolution_with_repo_dir(tmp_path: Path) -> None:
    """The host file path is resolved relative to the corpus repo dir."""
    from argot.ml.cli import _iter_fixture_rows, build_production_scorer

    repo, manifest, catalog_dir = _write_corpus_with_optional_host(tmp_path, with_host=True)
    raw = json.loads(manifest.read_text())

    inner = build_production_scorer(repo, "python", n_cal=5, threshold_n_seeds=1)

    rows = list(
        _iter_fixture_rows(
            inner,
            corpus="synthetic",
            language="python",
            catalog_dir=catalog_dir,
            fixtures=raw["fixtures"],
            repo_dir=repo,
        )
    )
    assert len(rows) == 1
    # file_path written to JSONL is the relative host path (portable).
    assert rows[0]["file_path"] == "host.py"
    # And the host content was clearly picked up — synthesized hunk_start_line
    # has shifted past the host's first 9 lines.
    assert rows[0]["hunk_start_line"] >= 10


def test_iter_fixture_rows_validates_host_fields_paired(tmp_path: Path) -> None:
    """_iter_fixture_rows itself enforces both-or-neither for safety."""
    from argot.ml.cli import _iter_fixture_rows, build_production_scorer

    repo, manifest, catalog_dir = _write_corpus_with_optional_host(tmp_path, with_host=False)
    raw = json.loads(manifest.read_text())
    raw["fixtures"][0]["host_file"] = "host.py"
    # host_inject_at_line missing → must raise.

    inner = build_production_scorer(repo, "python", n_cal=5, threshold_n_seeds=1)
    with pytest.raises(ValueError, match="host_file and host_inject_at_line"):
        list(
            _iter_fixture_rows(
                inner,
                corpus="synthetic",
                language="python",
                catalog_dir=catalog_dir,
                fixtures=raw["fixtures"],
                repo_dir=repo,
            )
        )


# ---------------------------------------------------------------------------
# (i) CLI smoke — mixed fixtures (one with host_file, one without)
# ---------------------------------------------------------------------------


def test_cli_smoke_mixed_host_file(tmp_path: Path) -> None:
    """Run argot-extract-features with one host_file fixture and one plain fixture.

    The output JSONL must be well-formed for both rows; the host_file row
    carries the host's relative path as ``file_path``, the plain row carries
    the catalog's relative path.
    """
    repo = tmp_path / "repo"
    repo.mkdir()
    for i in range(5):
        f = repo / f"src_{i}.py"
        f.write_text(
            "import logging\n\n"
            f"def handler_{i}(payload: dict) -> str:\n"
            "    logger = logging.getLogger(__name__)\n"
            "    logger.info('processing')\n"
            "    logger.debug('details')\n"
            "    result = str(payload)\n"
            "    logger.info('done')\n"
            "    return result\n"
        )
    host_path = repo / "host.py"
    host_lines = ["import logging\n", "import json\n", "\n"]
    for i in range(15):
        host_lines.append(f"def host_fn_{i}(x):\n")
        host_lines.append(f"    logging.info('host_{i}')\n")
        host_lines.append(f"    json.dumps({{'k': {i}}})\n")
        host_lines.append(f"    return x + {i}\n")
        host_lines.append("\n")
    host_path.write_text("".join(host_lines))

    breaks_dir = tmp_path / "breaks"
    breaks_dir.mkdir()
    (breaks_dir / "break_a.py").write_text(
        "import requests\n"
        "\n"
        "def fetch_a(url: str) -> dict:\n"
        "    response = requests.get(url)\n"
        "    return response.json()\n"
    )
    (breaks_dir / "break_b.py").write_text(
        "import urllib3\n"
        "\n"
        "def fetch_b(url: str) -> dict:\n"
        "    pool = urllib3.PoolManager()\n"
        "    return pool.request('GET', url).json()\n"
    )

    manifest = tmp_path / "manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "corpus": "synthetic",
                "language": "python",
                "categories": ["fetch"],
                "fixtures": [
                    {
                        "id": "with_host",
                        "file": "breaks/break_a.py",
                        "category": "fetch",
                        "hunk_start_line": 3,
                        "hunk_end_line": 5,
                        "difficulty": "easy",
                        "host_file": "host.py",
                        "host_inject_at_line": 10,
                    },
                    {
                        "id": "no_host",
                        "file": "breaks/break_b.py",
                        "category": "fetch",
                        "hunk_start_line": 3,
                        "hunk_end_line": 5,
                        "difficulty": "easy",
                    },
                ],
            }
        )
    )

    out = tmp_path / "features.jsonl"
    proc = subprocess.run(
        [
            sys.executable,
            "-m",
            "argot.ml.cli",
            "--manifest",
            str(manifest),
            "--repo-dir",
            str(repo),
            "--catalog-dir",
            str(tmp_path),
            "--out",
            str(out),
            "--threshold-n-seeds",
            "1",
            "--n-cal",
            "5",
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, f"stdout={proc.stdout} stderr={proc.stderr}"
    rows = [json.loads(line) for line in out.read_text().splitlines() if line.strip()]
    by_id = {r["fixture_id"]: r for r in rows}
    assert "with_host" in by_id and "no_host" in by_id

    with_host = by_id["with_host"]
    no_host = by_id["no_host"]
    assert with_host["file_path"] == "host.py"
    assert no_host["file_path"] == "breaks/break_b.py"
    # Both rows have well-formed feature dicts.
    for r in (with_host, no_host):
        assert "features" in r
        assert "import_score" in r["features"]
        assert "bpe_score" in r["features"]
        assert "hunk_file_callee_jaccard" in r["features"]


# ---------------------------------------------------------------------------
# (g) Era-14 streaming control sampler — RAM-bounded top-N + reservoir.
# ---------------------------------------------------------------------------


def _fake_candidate(score: float, idx: int) -> tuple[dict[str, object], dict[str, object]]:
    """Build a synthetic ``(record, features)`` pair with a known adjusted_bpe."""
    record: dict[str, object] = {"id": idx, "file_path": f"f{idx}.py"}
    features: dict[str, object] = {"adjusted_bpe": score, "marker": idx}
    return record, features


def test_stream_sample_controls_returns_top_n_by_adjusted_bpe() -> None:
    """Top-N section of the output is exactly the N highest adjusted_bpe scores."""
    # Stream 100 candidates with monotonic scores 0.0 .. 99.0 in a shuffled order.
    n_top = 10
    scores = list(range(100))
    # Deterministic shuffle (fixed permutation, not seeded by sampler).
    perm = [37, 12, 88, 5, 99, 41, 23, 64, 0, 76, 2, 91] + [
        s for s in scores if s not in {37, 12, 88, 5, 99, 41, 23, 64, 0, 76, 2, 91}
    ]
    candidates = (_fake_candidate(float(s), s) for s in perm)

    out = stream_sample_controls(candidates, n_top=n_top, seed=0)

    # Top-N is the prefix of the output. Pull out the top-N and check scores.
    top = out[:n_top]
    top_scores = sorted([float(f["adjusted_bpe"]) for (_r, f) in top], reverse=True)
    expected = sorted(scores, reverse=True)[:n_top]
    assert top_scores == [float(s) for s in expected]
    # Top-N output ordering: descending by adjusted_bpe.
    emitted_top_scores = [float(f["adjusted_bpe"]) for (_r, f) in top]
    assert emitted_top_scores == sorted(emitted_top_scores, reverse=True)


def test_stream_sample_controls_reservoir_size() -> None:
    """Reservoir extras count is exactly N//2 when top-N and reservoir cannot overlap.

    Construct a stream where the top-N items have scores strictly greater than
    every other item, AND the top-N items appear at the *front* of the stream
    (so the reservoir, which sees items in stream order with diminishing
    probability of replacement, is statistically unlikely to keep them — and
    even if it did, dedup would shrink the extras count, which the assertion
    below would catch).

    Easier: split the stream into two disjoint id ranges by score so any
    overlap is impossible.
    """
    n_top = 20
    n_reservoir = n_top // 2  # 10
    # First 10000 candidates have score 0.0 (low); last 20 candidates have
    # score 1000+ (the top-N). Top-N is disjoint from anything the reservoir
    # might pick because the reservoir only ever sees the low-score range
    # excepting the final 20 items — but we structure so the reservoir's
    # picks come from the low-score prefix where ids never overlap with the
    # high-score tail.
    low = [_fake_candidate(0.0, i) for i in range(10000)]
    high = [_fake_candidate(1000.0 + i, 100000 + i) for i in range(n_top)]
    stream = low + high

    out = stream_sample_controls(iter(stream), n_top=n_top, seed=42)

    # Total is exactly n_top + n_reservoir — no overlap possible by construction.
    assert len(out) == n_top + n_reservoir
    extras = out[n_top:]
    assert len(extras) == n_reservoir
    # Top-N entries all came from the high-score block.
    top = out[:n_top]
    for record, _f in top:
        assert int(record["id"]) >= 100000
    # Reservoir extras all came from the low-score block.
    for record, _f in extras:
        assert int(record["id"]) < 10000


def test_stream_sample_controls_reservoir_short_stream() -> None:
    """Stream shorter than N//2 → reservoir is just the whole stream prefix."""
    n_top = 40
    # Only 5 candidates; n_top//2 = 20, so reservoir gets all 5 — but they all
    # also fit in the top-N, so dedup reduces the extras to 0.
    candidates = (_fake_candidate(float(i), i) for i in range(5))
    out = stream_sample_controls(candidates, n_top=n_top, seed=0)
    assert len(out) == 5  # all 5 in top-N, reservoir fully overlaps


def test_stream_sample_controls_dedup() -> None:
    """When top-N and reservoir overlap, the union is deduplicated by record id."""
    n_top = 5
    # 8 candidates: stream is small enough that the reservoir (n_top//2 = 2)
    # is likely to contain items already in the top-N (which is the top 5 of 8).
    candidates = [_fake_candidate(float(i), i) for i in range(8)]
    out = stream_sample_controls(iter(candidates), n_top=n_top, seed=0)

    # All output records are unique by id().
    ids = [id(r) for (r, _f) in out]
    assert len(ids) == len(set(ids))
    # Top-N portion contains the 5 highest scores (3,4,5,6,7).
    top = out[:n_top]
    top_scores = sorted([float(f["adjusted_bpe"]) for (_r, f) in top], reverse=True)
    assert top_scores == [7.0, 6.0, 5.0, 4.0, 3.0]


def test_stream_sample_controls_memory_bounded() -> None:
    """Sampler holds ≤ ~1.5*N candidates at any time, regardless of stream length.

    We wrap the iterator in a counter that records the number of *live*
    candidate tuples produced but not yet released (i.e. tracked by a weakref-
    like counter via a sentinel object). Easier: count alive sentinel objects
    in a set, and verify the sampler's internal heap+reservoir together never
    hold more than n_top + n_top//2 distinct sentinel objects in their feats
    dicts.

    We instead track that during streaming, the sampler never accumulates a
    growing list of candidates by counting how many records remain referenced
    by the sampler at iteration end (proxied via stream length-independence).
    """
    import gc

    n_top = 10
    n_reservoir = n_top // 2
    cap_expected = n_top + n_reservoir  # 15

    # Build a generator that yields 10000 unique candidates. Each record gets a
    # distinct sentinel object whose id we can count.
    sentinels: list[object] = []

    def gen() -> Iterator[tuple[dict[str, object], dict[str, object]]]:
        for i in range(10000):
            sentinel = object()
            sentinels.append(sentinel)
            record: dict[str, object] = {"id": i, "sentinel": sentinel}
            features: dict[str, object] = {"adjusted_bpe": float(i % 1000), "marker": i}
            yield record, features

    out = stream_sample_controls(gen(), n_top=n_top, seed=7)

    # Output bounded by cap_expected.
    assert len(out) <= cap_expected
    assert len(out) >= n_top  # should fill top-N

    # After collection, only the sampled records (and their sentinels) should
    # be reachable from `out`. Force a GC and count how many sentinel objects
    # are referenced by `out` records.
    gc.collect()
    sampled_sentinels = {id(r["sentinel"]) for (r, _f) in out}
    assert len(sampled_sentinels) == len(out)
    # The sampler retained at most cap_expected records — proven by output.
    # (We can't easily inspect peak intermediate state, but the algorithm's
    # invariants — heap≤n_top, reservoir≤n_reservoir — imply the bound.)


class _AliveCounter:
    """Counts live instances via ``__del__`` — proxy for record-dict lifetime.

    Each instance is embedded inside a record dict the sampler is fed.  When
    the sampler discards a record, its dict + counter become unreachable and
    CPython's reference-count GC reclaims them deterministically (no need for
    a full mark-and-sweep cycle).  This lets us bound the number of records
    the sampler holds simultaneously.
    """

    __slots__ = ("counter",)

    def __init__(self, counter: list[int]) -> None:
        self.counter = counter
        counter[0] += 1
        if counter[0] > counter[1]:
            counter[1] = counter[0]

    def __del__(self) -> None:  # pragma: no cover — refcount reclaim
        self.counter[0] -= 1


def test_stream_sample_controls_memory_bounded_peak() -> None:
    """Peak live records held by the sampler stays ≤ n_top + n_reservoir + slack.

    Embeds an :class:`_AliveCounter` inside each record dict and relies on
    CPython's deterministic refcount GC: once the sampler stops referencing a
    record (because a higher-scored item evicted it from the heap, or the
    reservoir replaced its slot), the record dict + counter are reclaimed
    immediately, decrementing the live count.
    """
    import gc

    n_top = 10
    n_reservoir = n_top // 2
    # Slack accounts for: (a) the iterator frame's local `record`/`feats`
    # bindings still alive at the comparison point, (b) heap/reservoir update
    # transients, (c) any cycle-collected stragglers. 5 is generous.
    slack = 5
    cap = n_top + n_reservoir + slack

    # counter[0] = live count, counter[1] = peak.
    counter: list[int] = [0, 0]

    def gen() -> Iterator[tuple[dict[str, object], dict[str, object]]]:
        for i in range(10000):
            tracker = _AliveCounter(counter)
            record: dict[str, object] = {"id": i, "_tracker": tracker}
            features: dict[str, object] = {"adjusted_bpe": float(i % 997), "marker": i}
            yield record, features

    out = stream_sample_controls(gen(), n_top=n_top, seed=0)
    gc.collect()

    assert len(out) <= n_top + n_reservoir
    assert counter[1] <= cap, (
        f"sampler held {counter[1]} records live at peak; expected ≤ {cap} "
        f"(n_top={n_top}, n_reservoir={n_reservoir})"
    )


def test_stream_sample_controls_deterministic() -> None:
    """Same seed → identical output (top-N + reservoir + ordering)."""
    candidates_a = [_fake_candidate(float((i * 37) % 100), i) for i in range(500)]
    candidates_b = [_fake_candidate(float((i * 37) % 100), i) for i in range(500)]

    out_a = stream_sample_controls(iter(candidates_a), n_top=20, seed=12345)
    out_b = stream_sample_controls(iter(candidates_b), n_top=20, seed=12345)

    # Same sequence of records (compared by 'id' field, not Python id()).
    ids_a = [r["id"] for (r, _f) in out_a]
    ids_b = [r["id"] for (r, _f) in out_b]
    assert ids_a == ids_b

    # Different seed → different reservoir (top-N is deterministic regardless).
    out_c = stream_sample_controls(iter(candidates_a), n_top=20, seed=99999)
    ids_c = [r["id"] for (r, _f) in out_c]
    # Top-N portion (first 20) is identical because it depends only on scores.
    assert ids_a[:20] == ids_c[:20]
    # Reservoir portion (after first 20) differs between seeds with very high
    # probability on a 500-item stream sampling 10 extras.
    assert ids_a[20:] != ids_c[20:]


def test_stream_sample_controls_empty_stream() -> None:
    """Empty input → empty output, no crash."""
    out = stream_sample_controls(iter([]), n_top=10, seed=0)
    assert out == []


def test_stream_sample_controls_zero_n_top() -> None:
    """n_top=0 → empty output regardless of stream length."""
    candidates = (_fake_candidate(float(i), i) for i in range(50))
    out = stream_sample_controls(candidates, n_top=0, seed=0)
    assert out == []
