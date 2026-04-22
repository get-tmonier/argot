# engine/argot/research/signal/phase14/calibration/test_random_hunk_sampler.py
"""Tests for random_hunk_sampler — seed reproducibility, n correctness, exclusions."""

from __future__ import annotations

import textwrap
from pathlib import Path

import pytest

from argot.research.signal.phase14.calibration.random_hunk_sampler import (
    MIN_BODY_LINES,
    collect_candidates,
    sample_hunks,
    sample_hunks_disjoint,
)


def _write_py(directory: Path, name: str, source: str) -> Path:
    p = directory / name
    p.write_text(textwrap.dedent(source))
    return p


@pytest.fixture()
def mini_corpus(tmp_path: Path) -> Path:
    """Small corpus: 2 qualifying defs in module_a, 1 in module_b, 1 too-short def."""
    _write_py(
        tmp_path,
        "module_a.py",
        """\
        def short():
            pass

        def long_func():
            x = 1
            y = 2
            z = 3
            w = 4
            return x + y + z + w

        class BigClass:
            def method(self) -> int:
                a = 1
                b = 2
                c = 3
                d = 4
                return a + b + c + d
        """,
    )
    _write_py(
        tmp_path,
        "module_b.py",
        """\
        def another_long() -> int:
            a = 1
            b = 2
            c = 3
            d = 4
            e = 5
            return a
        """,
    )
    return tmp_path


def test_collect_candidates_count(mini_corpus: Path) -> None:
    # short() excluded (1 body line); long_func, BigClass, another_long qualify
    candidates = collect_candidates(mini_corpus)
    assert len(candidates) == 3


def test_min_body_lines_satisfied(mini_corpus: Path) -> None:
    candidates = collect_candidates(mini_corpus)
    for hunk in candidates:
        assert hunk.count("\n") + 1 >= MIN_BODY_LINES


def test_seed_reproducibility(mini_corpus: Path) -> None:
    result_a = sample_hunks(mini_corpus, n=2, seed=42)
    result_b = sample_hunks(mini_corpus, n=2, seed=42)
    assert result_a == result_b


def test_different_seeds_produce_different_samples(mini_corpus: Path) -> None:
    samples = {sample_hunks(mini_corpus, n=1, seed=s)[0] for s in range(10)}
    assert len(samples) > 1, "Different seeds should produce different samples"


def test_n_correctness(mini_corpus: Path) -> None:
    result = sample_hunks(mini_corpus, n=2, seed=0)
    assert len(result) == 2


def test_sample_too_large_raises(mini_corpus: Path) -> None:
    with pytest.raises(ValueError, match="cannot sample"):
        sample_hunks(mini_corpus, n=999, seed=0)


def test_excludes_test_directory(tmp_path: Path) -> None:
    tests_dir = tmp_path / "tests"
    tests_dir.mkdir()
    _write_py(
        tests_dir,
        "test_foo.py",
        """\
        def test_something() -> None:
            x = 1
            y = 2
            z = 3
            w = 4
            assert x + y + z + w == 10
        """,
    )
    _write_py(
        tmp_path,
        "real_module.py",
        """\
        def real_func() -> int:
            x = 1
            y = 2
            z = 3
            w = 4
            return x + y + z + w
        """,
    )
    candidates = collect_candidates(tmp_path)
    assert len(candidates) == 1


def test_disjoint_no_overlap(mini_corpus: Path) -> None:
    cal, ctrl = sample_hunks_disjoint(mini_corpus, n_cal=2, n_ctrl=1, seed=0)
    assert set(cal) & set(ctrl) == set(), "cal and ctrl must be disjoint"


def test_disjoint_correct_counts(mini_corpus: Path) -> None:
    cal, ctrl = sample_hunks_disjoint(mini_corpus, n_cal=2, n_ctrl=1, seed=0)
    assert len(cal) == 2
    assert len(ctrl) == 1


def test_disjoint_reproducibility(mini_corpus: Path) -> None:
    cal_a, ctrl_a = sample_hunks_disjoint(mini_corpus, n_cal=2, n_ctrl=1, seed=7)
    cal_b, ctrl_b = sample_hunks_disjoint(mini_corpus, n_cal=2, n_ctrl=1, seed=7)
    assert cal_a == cal_b
    assert ctrl_a == ctrl_b


def test_disjoint_different_seeds_differ(mini_corpus: Path) -> None:
    results = {
        frozenset(sample_hunks_disjoint(mini_corpus, n_cal=1, n_ctrl=1, seed=s)[0])
        for s in range(10)
    }
    assert len(results) > 1, "Different seeds should yield different cal sets"


def test_disjoint_too_large_raises(mini_corpus: Path) -> None:
    with pytest.raises(ValueError, match="cannot sample"):
        sample_hunks_disjoint(mini_corpus, n_cal=2, n_ctrl=2, seed=0)


def test_excludes_test_prefixed_files(tmp_path: Path) -> None:
    _write_py(
        tmp_path,
        "test_module.py",
        """\
        def test_func() -> None:
            x = 1
            y = 2
            z = 3
            w = 4
            assert x
        """,
    )
    candidates = collect_candidates(tmp_path)
    assert len(candidates) == 0


def test_excludes_auto_generated_files_by_default(tmp_path: Path) -> None:
    _write_py(
        tmp_path,
        "generated_data.py",
        """\
        # auto-generated — do not edit
        DATA = {
            "a": 1,
            "b": 2,
            "c": 3,
            "d": 4,
        }

        def lookup(key: str) -> int:
            v = DATA.get(key, 0)
            w = v * 2
            x = w + 1
            y = x - 1
            return y
        """,
    )
    _write_py(
        tmp_path,
        "real_module.py",
        """\
        def real_func() -> int:
            a = 1
            b = 2
            c = 3
            d = 4
            return a + b + c + d
        """,
    )
    # Default: auto-generated file excluded → only real_module hunks remain
    candidates = collect_candidates(tmp_path)
    assert len(candidates) == 1
    assert "real_func" in candidates[0]


def test_include_auto_generated_when_opted_out(tmp_path: Path) -> None:
    _write_py(
        tmp_path,
        "generated_data.py",
        """\
        # auto-generated — do not edit
        DATA = {}

        def lookup(key: str) -> int:
            a = 1
            b = 2
            c = 3
            d = 4
            return a + b + c + d
        """,
    )
    # Opt-out: auto-generated file included
    candidates = collect_candidates(tmp_path, exclude_auto_generated=False)
    assert len(candidates) == 1


def test_excludes_data_dominant_files_by_default(tmp_path: Path) -> None:
    # Data-dominant file: overwhelmingly top-level tuple/list assignments.
    _write_py(
        tmp_path,
        "locale_data.py",
        """\
        CITIES = (
            "city_alpha",
            "city_beta",
            "city_gamma",
            "city_delta",
            "city_epsilon",
            "city_zeta",
            "city_eta",
            "city_theta",
            "city_iota",
            "city_kappa",
        )
        STREETS = [
            "street_one",
            "street_two",
            "street_three",
            "street_four",
            "street_five",
            "street_six",
            "street_seven",
            "street_eight",
            "street_nine",
            "street_ten",
        ]
        DISTRICTS = (
            "district_a",
            "district_b",
            "district_c",
            "district_d",
            "district_e",
            "district_f",
            "district_g",
            "district_h",
        )


        def locale() -> str:
            return "xx_XX"
        """,
    )
    # Normal module with qualifying hunks.
    _write_py(
        tmp_path,
        "real_module.py",
        """\
        def real_func() -> int:
            a = 1
            b = 2
            c = 3
            d = 4
            return a + b + c + d
        """,
    )
    # Default: data-dominant file excluded → only real_module hunks remain.
    candidates = collect_candidates(tmp_path)
    assert len(candidates) == 1
    assert "real_func" in candidates[0]


def test_include_data_dominant_when_opted_out(tmp_path: Path) -> None:
    _write_py(
        tmp_path,
        "locale_data.py",
        """\
        CITIES = (
            "city_alpha",
            "city_beta",
            "city_gamma",
            "city_delta",
            "city_epsilon",
            "city_zeta",
            "city_eta",
            "city_theta",
            "city_iota",
            "city_kappa",
        )
        STREETS = [
            "street_one",
            "street_two",
            "street_three",
            "street_four",
            "street_five",
            "street_six",
            "street_seven",
            "street_eight",
            "street_nine",
            "street_ten",
        ]
        DISTRICTS = (
            "district_a",
            "district_b",
            "district_c",
            "district_d",
            "district_e",
            "district_f",
            "district_g",
            "district_h",
        )


        def locale_info() -> dict[str, str]:
            return {
                "code": "xx_XX",
                "language": "unknown",
                "country": "unknown",
                "encoding": "utf-8",
                "direction": "ltr",
            }
        """,
    )
    _write_py(
        tmp_path,
        "real_module.py",
        """\
        def real_func() -> int:
            a = 1
            b = 2
            c = 3
            d = 4
            return a + b + c + d
        """,
    )
    # Opt-out: data-dominant file included — locale_info() and real_func both qualify.
    candidates = collect_candidates(tmp_path, exclude_data_dominant=False)
    assert len(candidates) == 2


def _write_ts(directory: Path, name: str, source: str) -> Path:
    p = directory / name
    p.write_text(textwrap.dedent(source))
    return p


def test_collect_candidates_typescript_adapter(tmp_path: Path) -> None:
    """Passing a TypeScriptAdapter must yield TS hunks, not Python ones."""
    from argot.research.signal.phase14.adapters.typescript_adapter import TypeScriptAdapter

    _write_ts(
        tmp_path,
        "utils.ts",
        """\
        export function add(a: number, b: number): number {
          const sum = a + b;
          const result = sum;
          const check = result > 0;
          const out = check ? result : 0;
          return out;
        }

        export const multiply = (a: number, b: number): number => {
          const product = a * b;
          const result = product;
          const check = result > 0;
          const out = check ? result : 0;
          return out;
        };
        """,
    )
    candidates = collect_candidates(tmp_path, adapter=TypeScriptAdapter())
    assert len(candidates) == 2


def test_sample_hunks_typescript_adapter(tmp_path: Path) -> None:
    """sample_hunks with TypeScriptAdapter samples from TS files."""
    from argot.research.signal.phase14.adapters.typescript_adapter import TypeScriptAdapter

    _write_ts(
        tmp_path,
        "a.ts",
        """\
        export function one(x: number): number {
          const a = x + 1;
          const b = a + 1;
          const c = b + 1;
          const d = c + 1;
          return d;
        }
        export function two(x: number): number {
          const a = x + 2;
          const b = a + 2;
          const c = b + 2;
          const d = c + 2;
          return d;
        }
        export function three(x: number): number {
          const a = x + 3;
          const b = a + 3;
          const c = b + 3;
          const d = c + 3;
          return d;
        }
        """,
    )
    result = sample_hunks(tmp_path, n=2, seed=0, adapter=TypeScriptAdapter())
    assert len(result) == 2
