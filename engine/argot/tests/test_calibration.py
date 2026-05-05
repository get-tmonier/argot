from __future__ import annotations

import json
import sys
import tempfile
from datetime import UTC
from pathlib import Path

import pytest

from argot.scoring.adapters.registry import adapter_for_files
from argot.scoring.adapters.typescript import TypeScriptAdapter
from argot.scoring.calibration import (
    _partition_corpus_by_language,
    language_for_extension,
    load_config,
    main,
)
from argot.scoring.calibration.random_hunk_sampler import collect_candidates, sample_hunks
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_CATALOG = Path(__file__).parent.parent / "acceptance" / "catalog"
_FASTAPI_FIXTURES = _CATALOG / "fastapi" / "fixtures" / "default"
_BPE_GENERIC_BASELINE = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"
_CONTROL_FILES = sorted(_FASTAPI_FIXTURES.glob("control_*.py"))


def _scorer_with_cal(seed: int, n: int = 5) -> SequentialImportBpeScorer:
    hunks = sample_hunks(_FASTAPI_FIXTURES, n, seed)
    return SequentialImportBpeScorer(
        repo_corpus_files=_CONTROL_FILES,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        calibration_hunks=hunks,
    )


def test_calibration_determinism() -> None:
    """Same seed produces identical threshold on two independent runs."""
    s1 = _scorer_with_cal(seed=42)
    s2 = _scorer_with_cal(seed=42)
    assert s1.bpe_threshold == pytest.approx(s2.bpe_threshold, abs=0.0)


def test_different_seeds_may_differ() -> None:
    """Different seeds produce potentially different thresholds (probabilistic)."""
    s0 = _scorer_with_cal(seed=0)
    s1 = _scorer_with_cal(seed=99)
    # Not guaranteed to differ, but thresholds are independent
    assert isinstance(s0.bpe_threshold, float)
    assert isinstance(s1.bpe_threshold, float)


def test_empty_corpus_raises() -> None:
    """sample_hunks raises ValueError when source dir has no qualifying hunks."""
    with (
        tempfile.TemporaryDirectory() as tmp,
        pytest.raises(ValueError, match="Only 0 qualifying hunks"),
    ):
        sample_hunks(Path(tmp), n=1, seed=0)


def test_thin_pool_caps_gracefully() -> None:
    """sample_hunks caps at available pool size when n > pool and emits a warning."""
    import warnings

    candidates_available = len(collect_candidates(_FASTAPI_FIXTURES))
    # Request more than the pool — should cap, not raise
    oversized_n = candidates_available + 50
    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        hunks = sample_hunks(_FASTAPI_FIXTURES, oversized_n, seed=0)
    assert len(hunks) == candidates_available
    assert any("capping" in str(warning.message).lower() for warning in w)


def _make_v2_config(scorer: SequentialImportBpeScorer, lang: str = "python") -> dict[str, object]:
    """Build a minimal valid v2 scorer-config dict for tests."""
    from datetime import datetime

    try:
        import pygit2

        repo = pygit2.Repository(str(Path(__file__).parent.parent.parent.parent))
        repo_sha = str(repo.head.target)
    except Exception:
        repo_sha = "unknown"

    lang_entry: dict[str, object] = {
        "threshold": scorer.bpe_threshold,
        "call_receiver_alpha": 2.0,
        "call_receiver_cap": 5,
        "call_receiver_root_bonus": 2.0,
        "call_receiver_n_clusters": 8,
        "call_receiver_cluster_seed": 0,
        "call_receiver_cluster_bonus": 5.0,
        "call_receiver_cluster_rare_threshold": 0,
        "call_receiver_cluster_size_min": 0,
        "import_modules": [],
        "import_module_prefixes": [],
        "calibration": {
            "n_cal": scorer.n_calibration,
            "seed": 7,
            "n_seeds": 1,
            "repo_sha": repo_sha,
            "timestamp_utc": datetime.now(tz=UTC).isoformat(),
        },
        # Minimal valid evidence_corpus matching EvidenceCorpus.from_json_dict.
        "evidence_corpus": {
            "imports": [],
            "identifiers": {},
            "callees_by_cluster": {},
            "totals": {
                "import_specifiers_attested": 0,
                "callees_attested_by_cluster": {},
            },
        },
    }
    return {"version": 2, "languages": {lang: lang_entry}}


def test_scorer_config_json_roundtrip(tmp_path: Path) -> None:
    """Write v2 scorer-config.json then read it back with load_config."""
    scorer = _scorer_with_cal(seed=7)
    config = _make_v2_config(scorer, lang="python")
    out = tmp_path / "scorer-config.json"
    out.write_text(json.dumps(config))

    loaded = load_config(out)
    assert loaded["version"] == 2
    langs = loaded["languages"]
    assert isinstance(langs, dict)
    assert "python" in langs
    py_cfg = langs["python"]
    assert isinstance(py_cfg, dict)
    assert py_cfg["threshold"] == pytest.approx(scorer.bpe_threshold, abs=1e-10)
    cal = py_cfg["calibration"]
    assert isinstance(cal, dict)
    assert cal["seed"] == 7


def test_load_config_rejects_v1(tmp_path: Path) -> None:
    """load_config raises ValueError for v1 (flat single-language) configs."""
    v1_config = tmp_path / "scorer-config.json"
    v1_config.write_text(json.dumps({"version": 1, "threshold": 4.7}))
    with pytest.raises(ValueError, match="Regenerate via"):
        load_config(v1_config)


def test_load_config_rejects_versionless(tmp_path: Path) -> None:
    """load_config raises ValueError for configs missing the version key."""
    old_config = tmp_path / "scorer-config.json"
    old_config.write_text(json.dumps({"threshold": 4.7}))
    with pytest.raises(ValueError, match="Regenerate via"):
        load_config(old_config)


def test_load_config_rejects_unknown_version(tmp_path: Path) -> None:
    """load_config raises ValueError for unknown version numbers."""
    bad_config = tmp_path / "scorer-config.json"
    bad_config.write_text(json.dumps({"version": 99, "threshold": 1.0}))
    with pytest.raises(ValueError, match="Regenerate via"):
        load_config(bad_config)


def test_n_calibration_matches_hunks() -> None:
    """Scorer n_calibration equals the number of calibration hunks passed."""
    hunks = sample_hunks(_FASTAPI_FIXTURES, 8, seed=3)
    scorer = SequentialImportBpeScorer(
        repo_corpus_files=_CONTROL_FILES,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        calibration_hunks=hunks,
    )
    assert scorer.n_calibration == 8
    assert len(scorer.cal_scores) == 8


# --- TypeScript path tests ---

_TS_FILE_A = """\
import { readFile } from "fs/promises";
import { join } from "path";

export function parseConfig(raw: string): Record<string, unknown> {
    const lines = raw.split("\\n");
    const result: Record<string, unknown> = {};
    for (const line of lines) {
        const [key, value] = line.split("=");
        if (key && value) {
            result[key.trim()] = value.trim();
        }
    }
    return result;
}

export async function loadFile(p: string): Promise<string> {
    const full = join(process.cwd(), p);
    const buf = await readFile(full);
    const text = buf.toString("utf-8");
    return text;
}
"""

_TS_FILE_B = """\
import { EventEmitter } from "events";

export class MessageBus extends EventEmitter {
    private readonly queue: string[] = [];

    enqueue(msg: string): void {
        this.queue.push(msg);
        this.emit("message", msg);
    }

    drain(): string[] {
        const out = [...this.queue];
        this.queue.length = 0;
        return out;
    }
}

export function formatMessage(id: number, body: string): string {
    const prefix = `[${id}]`;
    const trimmed = body.trim();
    const result = `${prefix} ${trimmed}`;
    return result;
}
"""

_TS_FILE_C = """\
import type { RequestInit } from "node-fetch";

export interface RetryOptions {
    maxAttempts: number;
    delayMs: number;
}

export async function fetchWithRetry(
    url: string,
    opts: RequestInit,
    retry: RetryOptions,
): Promise<Response> {
    let last: Error = new Error("no attempts");
    for (let i = 0; i < retry.maxAttempts; i++) {
        try {
            const res = await fetch(url, opts);
            return res;
        } catch (err) {
            last = err as Error;
            await new Promise((r) => setTimeout(r, retry.delayMs));
        }
    }
    throw last;
}
"""


@pytest.fixture()
def ts_source_dir(tmp_path: Path) -> Path:
    """A tmp directory containing three .ts files with sampleable top-level declarations."""
    (tmp_path / "parser.ts").write_text(_TS_FILE_A, encoding="utf-8")
    (tmp_path / "bus.ts").write_text(_TS_FILE_B, encoding="utf-8")
    (tmp_path / "fetch.ts").write_text(_TS_FILE_C, encoding="utf-8")
    return tmp_path


def test_adapter_for_files_returns_typescript(ts_source_dir: Path) -> None:
    """adapter_for_files selects TypeScriptAdapter when all paths are .ts files."""
    paths = [str(p) for p in sorted(ts_source_dir.glob("*.ts"))]
    adapter = adapter_for_files(paths)
    assert isinstance(adapter, TypeScriptAdapter)


def test_collect_candidates_typescript(ts_source_dir: Path) -> None:
    """collect_candidates returns at least one hunk from a TypeScript source tree."""
    paths = [str(p) for p in sorted(ts_source_dir.glob("*.ts"))]
    ts_adapter = adapter_for_files(paths)
    candidates = collect_candidates(ts_source_dir, adapter=ts_adapter)
    assert len(candidates) > 0


def test_sample_hunks_typescript(ts_source_dir: Path) -> None:
    """sample_hunks returns the requested number of hunks from TypeScript sources."""
    paths = [str(p) for p in sorted(ts_source_dir.glob("*.ts"))]
    ts_adapter = adapter_for_files(paths)
    candidates = collect_candidates(ts_source_dir, adapter=ts_adapter)
    n = min(2, len(candidates))
    hunks = sample_hunks(ts_source_dir, n=n, seed=0, adapter=ts_adapter)
    assert len(hunks) == n
    for hunk in hunks:
        assert isinstance(hunk, str)
        assert len(hunk) > 0


def test_exclude_dirs_and_is_excluded_path_are_public() -> None:
    from argot.scoring.calibration.random_hunk_sampler import (
        DEFAULT_EXCLUDE_DIRS,
        is_excluded_path,
    )

    assert "tests" in DEFAULT_EXCLUDE_DIRS
    assert "node_modules" in DEFAULT_EXCLUDE_DIRS or "build" in DEFAULT_EXCLUDE_DIRS
    assert callable(is_excluded_path)


def test_collect_candidates_filters_data_dominant_file(tmp_path: Path) -> None:
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.calibration.random_hunk_sampler import collect_candidates

    repo = tmp_path / "repo"
    repo.mkdir()
    # Normal code file.
    (repo / "normal.py").write_text(
        "\n".join(
            [
                "def fn(value, registry):",
                "    items = registry.lookup(value)",
                "    if not items:",
                "        return None",
                "    out = []",
                "    for item in items:",
                "        out.append(item.transform(value))",
                "    return out",
            ]
        )
    )
    # Data-dominant file.
    (repo / "data.py").write_text(
        "DATA = {\n" + "\n".join(f'    "k{i}": "v{i}",' for i in range(120)) + "\n}"
    )

    candidates = collect_candidates(repo, adapter=PythonAdapter())
    # Should include fn from normal.py but not DATA from data.py.
    assert any("def fn" in h for h in candidates)
    assert not any("DATA" in h for h in candidates)


def test_calibrate_multi_seed_equals_median_of_individual() -> None:
    """calibrate_multi_seed returns exact median of K individually-computed thresholds."""
    import statistics

    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.calibration import calibrate_multi_seed

    adapter = PythonAdapter()
    n_cal = 5
    base_seed = 10
    n_seeds = 3
    threshold_percentile: float | None = None  # max formula

    # Compute individual thresholds for comparison
    individual = []
    for k in range(n_seeds):
        hunks = sample_hunks(_FASTAPI_FIXTURES, n_cal, base_seed + k)
        s = SequentialImportBpeScorer(
            repo_corpus_files=_CONTROL_FILES,
            bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
            calibration_hunks=hunks,
            threshold_percentile=threshold_percentile,
        )
        individual.append(s.bpe_threshold)

    expected = statistics.median(individual)

    # Disable cluster-conditional calibration path (era-11 default) so this test compares
    # the same scoring path as the individual scorers built above (which do not use clusters).
    result = calibrate_multi_seed(
        base_seed=base_seed,
        n_seeds=n_seeds,
        n_cal=n_cal,
        repo_dir=_FASTAPI_FIXTURES,
        repo_corpus_files=list(_CONTROL_FILES),
        adapter=adapter,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        threshold_percentile=threshold_percentile,
        call_receiver_alpha=2.0,
        call_receiver_cap=5,
        call_receiver_n_clusters=1,
        call_receiver_cluster_bonus=0.0,
    )
    assert result == pytest.approx(expected, abs=1e-10)


# ---------------------------------------------------------------------------
# apply_optional_contributions_to_cal tests
# ---------------------------------------------------------------------------

# Template for a Python source file with a unique rare callee and a common callee.
# The function body is 5 lines so it clears the MIN_BODY_LINES=5 gate in the sampler.
_RARE_CALLEE_FILE_TEMPLATE = """\
def compute_{n}(a, b):
    out = common_work(a)
    tmp = unique_op_{n}(b)
    if out > 0:
        return out + tmp
    return tmp - out
"""


@pytest.fixture()
def rare_callee_corpus(tmp_path: Path) -> tuple[Path, list[Path]]:
    """20 Python files, each with a unique callee (file_count=1 within its cluster).

    unique_op_N appears in exactly file N → cluster_callee_counts[c]["unique_op_N"] = 1.
    With cluster_rare_threshold=2, 1 ≤ 2 → rare branch fires on every sampled cal hunk.
    With n_clusters=2 and 20 files, each cluster holds ~10 files (≥ min_cluster_size=10
    for shape primitives if needed by test (c)).
    """
    for i in range(20):
        (tmp_path / f"module_{i:02d}.py").write_text(
            _RARE_CALLEE_FILE_TEMPLATE.format(n=i), encoding="utf-8"
        )
    files = sorted(tmp_path.glob("*.py"))
    return tmp_path, files


def test_apply_optional_contributions_default_is_noop() -> None:
    """Flag default=False with rare_threshold=0 produces bit-identical results.

    Era-13 status quo has cluster_rare_threshold=0. With nothing optional to
    suppress, apply_optional_contributions_to_cal=False is indistinguishable
    from not passing the flag or passing it True. Confirms the default is a
    strict no-op under the production config.
    """
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.calibration import calibrate_multi_seed

    adapter = PythonAdapter()
    without_flag = calibrate_multi_seed(
        base_seed=10,
        n_seeds=3,
        n_cal=5,
        repo_dir=_FASTAPI_FIXTURES,
        repo_corpus_files=list(_CONTROL_FILES),
        adapter=adapter,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        call_receiver_n_clusters=1,
        call_receiver_cluster_bonus=0.0,
        enable_typicality_filter=False,
    )
    with_false = calibrate_multi_seed(
        base_seed=10,
        n_seeds=3,
        n_cal=5,
        repo_dir=_FASTAPI_FIXTURES,
        repo_corpus_files=list(_CONTROL_FILES),
        adapter=adapter,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        call_receiver_n_clusters=1,
        call_receiver_cluster_bonus=0.0,
        enable_typicality_filter=False,
        apply_optional_contributions_to_cal=False,
    )
    with_true = calibrate_multi_seed(
        base_seed=10,
        n_seeds=3,
        n_cal=5,
        repo_dir=_FASTAPI_FIXTURES,
        repo_corpus_files=list(_CONTROL_FILES),
        adapter=adapter,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        call_receiver_n_clusters=1,
        call_receiver_cluster_bonus=0.0,
        enable_typicality_filter=False,
        apply_optional_contributions_to_cal=True,
    )
    # All three must be bit-identical: no optional contributions to suppress or add.
    assert without_flag == pytest.approx(with_false, abs=0.0)
    assert without_flag == pytest.approx(with_true, abs=0.0)


def test_apply_optional_contributions_false_drops_threshold_by_cluster_bonus(
    rare_callee_corpus: tuple[Path, list[Path]],
) -> None:
    """apply_optional_contributions_to_cal=False drops the threshold by exactly cluster_bonus.

    Corpus design guarantees every sampled cal hunk has unique_op_N (file_count=1 ≤
    cluster_rare_threshold=2), so the rare branch fires on every cal hunk in symmetric
    mode (flag=True). With flag=False the cal scorer uses rare_threshold=0, suppressing
    all rare-callee contributions.

    Because every cal hunk fires:
        cal_score_sym[i]  = raw_bpe[i] + cluster_bonus
        cal_score_asym[i] = raw_bpe[i]
        T_sym  = max(raw_bpe) + cluster_bonus = T_asym + cluster_bonus
    """
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.calibration import calibrate_multi_seed

    corpus_dir, corpus_files = rare_callee_corpus
    cluster_bonus = 5.0
    adapter = PythonAdapter()

    t_sym = calibrate_multi_seed(
        base_seed=0,
        n_seeds=1,
        n_cal=20,
        repo_dir=corpus_dir,
        repo_corpus_files=corpus_files,
        adapter=adapter,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        threshold_percentile=None,  # max(cal_scores) ensures exact delta
        call_receiver_n_clusters=2,
        call_receiver_cluster_bonus=cluster_bonus,
        call_receiver_cluster_rare_threshold=2,
        call_receiver_cluster_size_min=0,
        enable_typicality_filter=False,
        apply_optional_contributions_to_cal=True,
    )
    t_asym = calibrate_multi_seed(
        base_seed=0,
        n_seeds=1,
        n_cal=20,
        repo_dir=corpus_dir,
        repo_corpus_files=corpus_files,
        adapter=adapter,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        threshold_percentile=None,
        call_receiver_n_clusters=2,
        call_receiver_cluster_bonus=cluster_bonus,
        call_receiver_cluster_rare_threshold=2,
        call_receiver_cluster_size_min=0,
        enable_typicality_filter=False,
        apply_optional_contributions_to_cal=False,
    )
    # Symmetric mode inflates threshold by exactly cluster_bonus because every
    # cal hunk fires the rare rule once (unique_op_N, file_count=1 ≤ threshold=2).
    assert t_sym - t_asym == pytest.approx(cluster_bonus, abs=1e-10)


def test_apply_optional_contributions_false_suppresses_shape_primitives(
    rare_callee_corpus: tuple[Path, list[Path]],
) -> None:
    """apply_optional_contributions_to_cal=False suppresses shape primitives on the cal path.

    Invariant: flag=False with a non-empty primitive list is equivalent to flag=True
    with an empty primitive list — both produce a cal scorer with no primitives,
    so the thresholds are identical. When ExceptReturnRaiseRatio fires on cal hunks
    in the symmetric+ERR run, the delta t_sym_with_prim - t_asym equals the ERR
    cal contribution; this assertion verifies the suppression mechanism regardless
    of whether ERR happens to fire on any hunk in this corpus.
    """
    import argot.scoring.scorers.shape_primitive_registrations  # noqa: F401 — registers ERR
    from argot.scoring.adapters.python_adapter import PythonAdapter
    from argot.scoring.calibration import calibrate_multi_seed

    corpus_dir, corpus_files = rare_callee_corpus
    adapter = PythonAdapter()

    # Asymmetric mode: ExceptReturnRaiseRatio passed but suppressed on cal path.
    t_asym_with_prim = calibrate_multi_seed(
        base_seed=0,
        n_seeds=1,
        n_cal=20,
        repo_dir=corpus_dir,
        repo_corpus_files=corpus_files,
        adapter=adapter,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        threshold_percentile=None,
        call_receiver_n_clusters=2,
        call_receiver_cluster_bonus=5.0,
        call_receiver_cluster_rare_threshold=0,
        call_receiver_cluster_size_min=0,
        call_receiver_shape_primitive_names=("except_return_raise_ratio",),
        enable_typicality_filter=False,
        apply_optional_contributions_to_cal=False,
    )
    # Symmetric mode: no primitives at all (nothing to apply on cal).
    t_sym_no_prim = calibrate_multi_seed(
        base_seed=0,
        n_seeds=1,
        n_cal=20,
        repo_dir=corpus_dir,
        repo_corpus_files=corpus_files,
        adapter=adapter,
        bpe_generic_baseline_path=_BPE_GENERIC_BASELINE,
        threshold_percentile=None,
        call_receiver_n_clusters=2,
        call_receiver_cluster_bonus=5.0,
        call_receiver_cluster_rare_threshold=0,
        call_receiver_cluster_size_min=0,
        call_receiver_shape_primitive_names=(),
        enable_typicality_filter=False,
        apply_optional_contributions_to_cal=True,
    )
    # Both cal scorers see empty primitives: flag=False suppresses ERR,
    # flag=True with no primitives has nothing to apply. Thresholds must match.
    assert t_asym_with_prim == pytest.approx(t_sym_no_prim, abs=1e-10)


def test_calibration_cli_threshold_iqr_k(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """--threshold-iqr-k flag is accepted and produces a valid threshold."""
    repo_corpus_path = tmp_path / "repo-corpus.txt"
    repo_corpus_path.write_text("\n".join(str(p) for p in _CONTROL_FILES))
    generic_baseline_path = tmp_path / "generic-baseline.json"
    generic_baseline_path.write_text((_BPE_GENERIC_BASELINE).read_text())
    out = tmp_path / "scorer-config.json"
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "argot-calibrate",
            "--repo",
            str(_FASTAPI_FIXTURES),
            "--repo-corpus",
            str(repo_corpus_path),
            "--generic-baseline",
            str(generic_baseline_path),
            "--output",
            str(out),
            "--n-cal",
            "5",
            "--threshold-iqr-k",
            "2.5",
        ],
    )
    main()
    assert out.exists()
    cfg = json.loads(out.read_text())
    assert cfg.get("version") == 2
    langs = cfg.get("languages")
    assert isinstance(langs, dict)
    assert len(langs) >= 1
    # The corpus is all-Python so exactly one entry.
    assert "python" in langs
    py_cfg = langs["python"]
    assert isinstance(py_cfg, dict)
    assert "threshold" in py_cfg
    assert isinstance(py_cfg["threshold"], float)


# ---------------------------------------------------------------------------
# v2 schema — language_for_extension + _partition_corpus_by_language
# ---------------------------------------------------------------------------


def test_language_for_extension_known() -> None:
    """Known extensions map to the expected language names."""
    assert language_for_extension(".py") == "python"
    assert language_for_extension(".ts") == "typescript"
    assert language_for_extension(".tsx") == "typescript"
    assert language_for_extension(".js") == "typescript"
    assert language_for_extension(".jsx") == "typescript"


def test_language_for_extension_unknown() -> None:
    """Unknown extensions return None."""
    assert language_for_extension(".md") is None
    assert language_for_extension(".json") is None
    assert language_for_extension("") is None


def test_partition_corpus_single_language(tmp_path: Path) -> None:
    """Single-language corpus produces one entry under its language key."""
    py_a = tmp_path / "a.py"
    py_b = tmp_path / "b.py"
    py_a.write_text("x = 1")
    py_b.write_text("y = 2")
    result = _partition_corpus_by_language([py_a, py_b])
    assert list(result.keys()) == ["python"]
    assert set(result["python"]) == {py_a, py_b}


def test_partition_corpus_multi_language(tmp_path: Path) -> None:
    """Mixed Py+TS corpus partitions into two entries with the right files."""
    py_file = tmp_path / "mod.py"
    ts_file = tmp_path / "comp.ts"
    tsx_file = tmp_path / "app.tsx"
    py_file.write_text("x = 1")
    ts_file.write_text("const x = 1;")
    tsx_file.write_text("export default () => null;")
    result = _partition_corpus_by_language([py_file, ts_file, tsx_file])
    assert set(result.keys()) == {"python", "typescript"}
    assert result["python"] == [py_file]
    assert set(result["typescript"]) == {ts_file, tsx_file}


def test_partition_corpus_drops_unknown_extensions(tmp_path: Path) -> None:
    """Files with unsupported extensions are silently dropped."""
    py_file = tmp_path / "mod.py"
    json_file = tmp_path / "data.json"
    py_file.write_text("x = 1")
    json_file.write_text("{}")
    result = _partition_corpus_by_language([py_file, json_file])
    assert list(result.keys()) == ["python"]
    assert result["python"] == [py_file]


def test_calibration_cli_v2_multi_language(
    tmp_path: Path,
    ts_source_dir: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """argot-calibrate emits a v2 config with both python and typescript entries
    when the corpus contains both .py and .ts files.
    """
    # Mix Python control files with the TypeScript fixture files.
    all_files = list(_CONTROL_FILES) + sorted(ts_source_dir.glob("*.ts"))
    repo_corpus_path = tmp_path / "repo-corpus.txt"
    repo_corpus_path.write_text("\n".join(str(p) for p in all_files))
    generic_baseline_path = tmp_path / "generic-baseline.json"
    generic_baseline_path.write_text((_BPE_GENERIC_BASELINE).read_text())
    out = tmp_path / "scorer-config.json"

    # Use a mixed source dir: copy the TS files alongside the Python fixtures.
    mixed_dir = tmp_path / "mixed"
    mixed_dir.mkdir()
    for src in _CONTROL_FILES:
        (mixed_dir / src.name).write_text(src.read_text())
    for src in sorted(ts_source_dir.glob("*.ts")):
        (mixed_dir / src.name).write_text(src.read_text())

    monkeypatch.setattr(
        sys,
        "argv",
        [
            "argot-calibrate",
            "--repo",
            str(mixed_dir),
            "--repo-corpus",
            str(repo_corpus_path),
            "--generic-baseline",
            str(generic_baseline_path),
            "--output",
            str(out),
            "--n-cal",
            "5",
            "--threshold-n-seeds",
            "1",
            "--no-auto-select-asym-cal",
        ],
    )
    main()
    assert out.exists()
    cfg = json.loads(out.read_text())
    assert cfg.get("version") == 2
    langs = cfg.get("languages")
    assert isinstance(langs, dict)
    assert "python" in langs, f"Expected 'python' key in {list(langs.keys())}"
    assert "typescript" in langs, f"Expected 'typescript' key in {list(langs.keys())}"
    for lang, entry in langs.items():
        assert isinstance(entry, dict), f"{lang} entry should be a dict"
        assert isinstance(entry.get("threshold"), float), f"{lang} missing float threshold"
        assert "evidence_corpus" in entry, f"{lang} missing evidence_corpus"
        assert "calibration" in entry, f"{lang} missing calibration block"
