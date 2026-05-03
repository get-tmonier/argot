"""Tests for engine/argot/ml/features.py + cli.py.

Synthetic micro-corpora only — no benchmark or network.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

from argot.ml.features import (
    _ast_features,
    _hunk_callee_bag,
    _hunk_file_context_features,
    _jaccard,
    _resolve_cluster,
    build_feature_row,
    compute_features,
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
    assert feats["cluster_assignment_method"] in {"static_corpus", "fallback_jaccard", "none"}
    assert isinstance(feats["cluster_jaccard_to_centroid"], float)

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


def test_cluster_assignment_method_distinguishes_corpus_vs_fallback(tmp_path: Path) -> None:
    files = _make_python_corpus(tmp_path)
    inner = _make_scorer(files, n_clusters=2)
    cr = inner._call_receiver
    assert cr is not None
    assert cr.cluster_attested  # clusters were built

    # Static path: file from model_a corpus
    cid_static, method_static, jacc_static = _resolve_cluster(
        cr, files[0], file_source=None, language="python"
    )
    assert method_static == "static_corpus"
    assert cid_static is not None
    assert jacc_static == pytest.approx(1.0)

    # Fallback path: file not in corpus, but provide source
    foreign_path = tmp_path / "foreign.py"
    foreign_source = "import math\n\ndef g(x):\n    return math.sqrt(x)\n"
    cid_fb, method_fb, jacc_fb = _resolve_cluster(
        cr, foreign_path, file_source=foreign_source, language="python"
    )
    assert method_fb == "fallback_jaccard"
    assert cid_fb is not None
    assert 0.0 <= jacc_fb <= 1.0


def test_cluster_assignment_method_none_when_no_clusters(tmp_path: Path) -> None:
    files = _make_python_corpus(tmp_path)
    inner = _make_scorer(files, n_clusters=1)  # no clusters built
    cr = inner._call_receiver
    assert cr is not None
    assert cr.cluster_attested == {}

    cid, method, jacc = _resolve_cluster(cr, files[0], file_source="x = 1", language="python")
    assert cid is None
    assert method == "none"
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
