from __future__ import annotations

import json
import tempfile
from datetime import UTC
from pathlib import Path

import pytest

from argot.scoring.adapters.registry import adapter_for_files
from argot.scoring.adapters.typescript import TypeScriptAdapter
from argot.scoring.calibration import load_config
from argot.scoring.calibration.random_hunk_sampler import collect_candidates, sample_hunks
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_CATALOG = Path(__file__).parent.parent / "acceptance" / "catalog"
_FASTAPI_FIXTURES = _CATALOG / "fastapi" / "fixtures" / "default"
_BPE_MODEL_B = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"
_CONTROL_FILES = sorted(_FASTAPI_FIXTURES.glob("control_*.py"))


def _scorer_with_cal(seed: int, n: int = 5) -> SequentialImportBpeScorer:
    hunks = sample_hunks(_FASTAPI_FIXTURES, n, seed)
    return SequentialImportBpeScorer(
        model_a_files=_CONTROL_FILES,
        bpe_model_b_path=_BPE_MODEL_B,
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


def test_scorer_config_json_roundtrip(tmp_path: Path) -> None:
    """Write scorer-config.json then read it back with load_config."""
    scorer = _scorer_with_cal(seed=7)
    import pygit2

    try:
        repo = pygit2.Repository(str(Path(__file__).parent.parent.parent.parent))
        repo_sha = str(repo.head.target)
    except Exception:
        repo_sha = "unknown"

    from datetime import datetime

    config = {
        "version": 1,
        "threshold": scorer.bpe_threshold,
        "calibration": {
            "n_cal": scorer.n_calibration,
            "seed": 7,
            "repo_sha": repo_sha,
            "timestamp_utc": datetime.now(tz=UTC).isoformat(),
        },
    }
    out = tmp_path / "scorer-config.json"
    out.write_text(json.dumps(config))

    loaded = load_config(out)
    assert loaded["version"] == 1
    threshold = loaded["threshold"]
    assert isinstance(threshold, float)
    assert threshold == pytest.approx(scorer.bpe_threshold, abs=1e-10)
    cal = loaded["calibration"]
    assert isinstance(cal, dict)
    assert cal["seed"] == 7


def test_load_config_rejects_unknown_version(tmp_path: Path) -> None:
    """load_config raises ValueError for unknown version numbers."""
    bad_config = tmp_path / "scorer-config.json"
    bad_config.write_text(json.dumps({"version": 99, "threshold": 1.0}))
    with pytest.raises(ValueError, match="Unsupported scorer-config.json version"):
        load_config(bad_config)


def test_n_calibration_matches_hunks() -> None:
    """Scorer n_calibration equals the number of calibration hunks passed."""
    hunks = sample_hunks(_FASTAPI_FIXTURES, 8, seed=3)
    scorer = SequentialImportBpeScorer(
        model_a_files=_CONTROL_FILES,
        bpe_model_b_path=_BPE_MODEL_B,
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


def test_calibration_cli_threshold_iqr_k(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """--threshold-iqr-k flag is accepted and produces a valid threshold."""
    import sys
    from argot.scoring.calibration import main

    model_a_path = tmp_path / "model_a.txt"
    model_a_path.write_text(
        "\n".join(str(p) for p in _CONTROL_FILES)
    )
    (tmp_path / "model_b.json").write_text(
        (_BPE_MODEL_B).read_text()
    )
    out = tmp_path / "scorer-config.json"
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "argot-calibrate",
            "--repo", str(_FASTAPI_FIXTURES),
            "--model-a", str(model_a_path),
            "--model-b", str(tmp_path / "model_b.json"),
            "--output", str(out),
            "--n-cal", "5",
            "--threshold-iqr-k", "2.5",
        ],
    )
    main()
    assert out.exists()
    cfg = json.loads(out.read_text())
    assert "threshold" in cfg
    assert isinstance(cfg["threshold"], float)
