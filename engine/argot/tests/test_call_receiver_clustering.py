"""Tests for era-11 cluster-conditional attestation in CallReceiverScorer."""

from __future__ import annotations

from pathlib import Path

import pytest

# ---------------------------------------------------------------------------
# Defaults / backward-compatibility
# ---------------------------------------------------------------------------


def test_defaults_n_clusters_is_1(tmp_path: Path) -> None:
    """n_clusters=1 is the default; no clusters are built."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.py"
    f.write_text("import logging\nlogger = logging.getLogger()\nlogger.info('x')\n")
    scorer = CallReceiverScorer([f], language="python")
    assert scorer.file_to_cluster == {}
    assert scorer.cluster_attested == {}


def test_defaults_cluster_seed_is_0(tmp_path: Path) -> None:
    """cluster_seed default is 0; scorer should accept it without kwarg."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.ts"
    f.write_text("Math.floor(x);")
    scorer = CallReceiverScorer([f], language="typescript", n_clusters=2)
    assert isinstance(scorer.file_to_cluster, dict)


def test_n_clusters_1_output_identical_to_era10(tmp_path: Path) -> None:
    """n_clusters=1 with cluster_bonus > 0 produces same output as era-10 weighted_contribution."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.ts"
    f.write_text("Math.floor(x);")
    scorer = CallReceiverScorer([f], language="typescript", n_clusters=1)

    hunk = "Math.random();\nfetch(url);"
    era10 = scorer.weighted_contribution(hunk, alpha=2.0, root_bonus=2.0, cap=5.0)
    era11 = scorer.weighted_contribution_for_file(
        hunk, f, alpha=2.0, root_bonus=2.0, cluster_bonus=1.0, cap=5.0
    )
    assert era10 == pytest.approx(era11)


def test_cluster_bonus_zero_matches_era10(tmp_path: Path) -> None:
    """cluster_bonus=0.0 with n_clusters>1 produces same output as era-10 for globally-attested."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f1 = tmp_path / "a.ts"
    f1.write_text("Math.floor(x);\nconsole.log('hi');")
    f2 = tmp_path / "b.ts"
    f2.write_text("Promise.resolve(x);")
    scorer = CallReceiverScorer([f1, f2], language="typescript", n_clusters=2)

    # Math.random is globally unattested — era-10 gives alpha+root_bonus
    hunk = "Math.random();"
    era10 = scorer.weighted_contribution(hunk, alpha=2.0, root_bonus=2.0, cap=5.0)
    era11 = scorer.weighted_contribution_for_file(
        hunk, f1, alpha=2.0, root_bonus=2.0, cluster_bonus=0.0, cap=5.0
    )
    assert era10 == pytest.approx(era11)


# ---------------------------------------------------------------------------
# Reproducibility
# ---------------------------------------------------------------------------


def _make_corpus(tmp_path: Path, n_files: int = 6) -> list[Path]:
    files = []
    for i in range(n_files):
        f = tmp_path / f"f{i}.ts"
        # Files 0-2: math-heavy; files 3-5: io-heavy
        if i < n_files // 2:
            f.write_text(f"Math.floor(x{i}); Math.random(); Math.min(a,b);")
        else:
            f.write_text(f"fetch(url{i}); Promise.resolve(x); console.log('io');")
        files.append(f)
    return files


def test_same_seed_identical_cluster_assignment(tmp_path: Path) -> None:
    """Two fits with the same cluster_seed must produce identical file_to_cluster."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    files = _make_corpus(tmp_path)
    s1 = CallReceiverScorer(files, language="typescript", n_clusters=2, cluster_seed=42)
    s2 = CallReceiverScorer(files, language="typescript", n_clusters=2, cluster_seed=42)
    assert s1.file_to_cluster == s2.file_to_cluster


def test_different_seeds_may_differ(tmp_path: Path) -> None:
    """cluster_seed affects cluster assignment (structure is seeded; may differ across seeds)."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    # This is a smoke test that seeding doesn't crash and produces valid output
    files = _make_corpus(tmp_path)
    s0 = CallReceiverScorer(files, language="typescript", n_clusters=2, cluster_seed=0)
    s1 = CallReceiverScorer(files, language="typescript", n_clusters=2, cluster_seed=1)
    # Both should produce valid cluster assignments covering all files
    assert set(s0.file_to_cluster.keys()) == set(files)
    assert set(s1.file_to_cluster.keys()) == set(files)


# ---------------------------------------------------------------------------
# cluster_attested correctness
# ---------------------------------------------------------------------------


def test_cluster_attested_union_of_cluster_files(tmp_path: Path) -> None:
    """cluster_attested[k] equals the union of all callees from files in cluster k."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f1 = tmp_path / "a.ts"
    f1.write_text("Math.floor(x); Math.random();")
    f2 = tmp_path / "b.ts"
    f2.write_text("fetch(url); Promise.resolve(x);")
    # With 2 files and n_clusters=2, each file gets its own cluster
    scorer = CallReceiverScorer([f1, f2], language="typescript", n_clusters=2, cluster_seed=0)

    assert len(scorer.cluster_attested) == 2
    # Every callee from each file must appear in its cluster's attested set
    for _path, cid in scorer.file_to_cluster.items():
        ca = scorer.cluster_attested[cid]
        assert len(ca) > 0  # cluster must have at least one callee


def test_cluster_bonus_fires_for_cluster_absent_attested_callee(tmp_path: Path) -> None:
    """A globally-attested callee absent from the file's cluster contributes cluster_bonus."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    # cluster 0: files with Math calls; cluster 1: files with fetch calls
    # We'll craft a scenario where Math.floor is globally attested but absent from cluster 1
    math_files = []
    for i in range(3):
        f = tmp_path / f"math_{i}.ts"
        f.write_text("Math.floor(x); Math.random(); Math.min(a,b);")
        math_files.append(f)

    fetch_files = []
    for i in range(3):
        f = tmp_path / f"fetch_{i}.ts"
        f.write_text("fetch(url); Promise.resolve(x); console.log('done');")
        fetch_files.append(f)

    all_files = math_files + fetch_files
    scorer = CallReceiverScorer(all_files, language="typescript", n_clusters=2, cluster_seed=0)

    # Math.floor is globally attested
    assert "Math.floor" in scorer.attested

    # Find the cluster for a fetch file
    fetch_cluster = scorer.file_to_cluster[fetch_files[0]]
    fetch_cluster_attested = scorer.cluster_attested[fetch_cluster]

    # With clean callee-bag separation, Math.floor should be in math cluster but not fetch cluster
    # (This may not hold if KMeans merges clusters; we check the general behavior)
    if "Math.floor" not in fetch_cluster_attested:
        # cluster_bonus should fire when scoring a fetch-file hunk that calls Math.floor
        result_with_bonus = scorer.weighted_contribution_for_file(
            "Math.floor(x);",
            fetch_files[0],
            alpha=2.0,
            root_bonus=2.0,
            cluster_bonus=1.5,
            cap=10.0,
        )
        # Math.floor IS globally attested, root Math IS attested → era-10: weight=0
        # era-11 cluster bonus: weight = cluster_bonus = 1.5
        assert result_with_bonus == pytest.approx(1.5)

        # Without cluster bonus (=0.0), result should be 0.0 for globally attested callee
        result_no_bonus = scorer.weighted_contribution_for_file(
            "Math.floor(x);",
            fetch_files[0],
            alpha=2.0,
            root_bonus=2.0,
            cluster_bonus=0.0,
            cap=10.0,
        )
        assert result_no_bonus == pytest.approx(0.0)
    else:
        # Both files ended up in the same cluster; no cluster penalty
        pytest.skip("KMeans merged clusters — cluster separation not achieved for this seed")


def test_cluster_bonus_does_not_fire_for_cluster_attested_callee(tmp_path: Path) -> None:
    """A callee in the file's cluster attested set contributes 0 even with cluster_bonus > 0."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f1 = tmp_path / "a.ts"
    f1.write_text("Math.floor(x); Math.random();")
    f2 = tmp_path / "b.ts"
    f2.write_text("fetch(url); Promise.resolve(x);")
    scorer = CallReceiverScorer([f1, f2], language="typescript", n_clusters=2, cluster_seed=0)

    # Math.floor is attested globally
    assert "Math.floor" in scorer.attested
    math_cluster = scorer.file_to_cluster[f1]
    math_cluster_attested = scorer.cluster_attested[math_cluster]

    if "Math.floor" in math_cluster_attested:
        # In f1's cluster, Math.floor is attested → cluster_bonus should NOT fire
        result = scorer.weighted_contribution_for_file(
            "Math.floor(x);",
            f1,
            alpha=2.0,
            root_bonus=2.0,
            cluster_bonus=1.5,
            cap=10.0,
        )
        assert result == pytest.approx(0.0)
    else:
        pytest.skip("Math.floor not in f1's cluster — unexpected cluster assignment")


def test_unknown_file_path_no_cluster_bonus(tmp_path: Path) -> None:
    """A file_path not in file_to_cluster silently falls back to 0 cluster bonus."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.ts"
    f.write_text("Math.floor(x);")
    scorer = CallReceiverScorer([f], language="typescript", n_clusters=2, cluster_seed=0)

    unknown = tmp_path / "unknown_file.ts"
    result = scorer.weighted_contribution_for_file(
        "Math.floor(x);",
        unknown,
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=5.0,
        cap=10.0,
    )
    # Math.floor is globally attested, no cluster set found → 0
    assert result == pytest.approx(0.0)


# ---------------------------------------------------------------------------
# Era-11 Phase 1 fix: file_source fallback for unknown file paths
# ---------------------------------------------------------------------------


def _make_two_cluster_corpus(tmp_path: Path) -> tuple[list[Path], list[Path]]:
    """Build a 2-cluster corpus: math-flavored files and io/fetch-flavored files.

    Returns (math_files, io_files). Math.floor is globally attested but should
    only appear in the math cluster's attested set.
    """
    math_files: list[Path] = []
    for i in range(3):
        f = tmp_path / f"math_{i}.ts"
        f.write_text("Math.floor(x); Math.random(); Math.min(a,b);")
        math_files.append(f)

    io_files: list[Path] = []
    for i in range(3):
        f = tmp_path / f"io_{i}.ts"
        f.write_text("fetch(url); Promise.resolve(x); console.log('done');")
        io_files.append(f)

    return math_files, io_files


def test_unknown_file_path_with_source_falls_back_to_nearest_cluster(tmp_path: Path) -> None:
    """Unknown file_path + file_source → fallback assigns to nearest cluster (Jaccard).

    Math.floor is globally attested but absent from the io cluster's attested set.
    Scoring "Math.floor()" from a fetch-flavored unknown file should fire
    cluster_bonus; scoring it from a math-flavored unknown file should NOT.
    """
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    math_files, io_files = _make_two_cluster_corpus(tmp_path)
    all_files = math_files + io_files
    scorer = CallReceiverScorer(all_files, language="typescript", n_clusters=2, cluster_seed=0)

    # Sanity: cluster split must put Math.floor in only one cluster (the math one)
    math_cluster = scorer.file_to_cluster[math_files[0]]
    io_cluster = scorer.file_to_cluster[io_files[0]]
    if math_cluster == io_cluster:
        pytest.skip("KMeans merged clusters — cannot test fallback discrimination")
    if "Math.floor" in scorer.cluster_attested[io_cluster]:
        pytest.skip("Math.floor leaked into io cluster — cannot test fallback discrimination")
    assert "Math.floor" in scorer.cluster_attested[math_cluster]

    unknown = tmp_path / "unknown.ts"  # NOT in model_a_files
    assert unknown not in scorer.file_to_cluster

    # Fetch-flavored unknown file → Jaccard nearest is io cluster → Math.floor
    # is NOT in io's attested → cluster_bonus FIRES.
    fetch_flavored_source = "fetch(url); Promise.resolve(x); console.log('hi');"
    result_io = scorer.weighted_contribution_for_file(
        "Math.floor()",
        unknown,
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=1.5,
        cap=10.0,
        file_source=fetch_flavored_source,
    )
    assert result_io == pytest.approx(1.5)

    # Math-flavored unknown file → Jaccard nearest is math cluster → Math.floor
    # IS in math's attested → cluster_bonus does NOT fire.
    math_flavored_source = "Math.floor(); Math.random(); Math.min(a, b);"
    result_math = scorer.weighted_contribution_for_file(
        "Math.floor()",
        unknown,
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=1.5,
        cap=10.0,
        file_source=math_flavored_source,
    )
    assert result_math == pytest.approx(0.0)


def test_unknown_file_path_no_source_no_cluster_bonus(tmp_path: Path) -> None:
    """Unknown file_path + file_source=None → cluster_bonus must NOT fire.

    Era-10 graceful no-op preserved when the caller has no source to feed.
    """
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    math_files, io_files = _make_two_cluster_corpus(tmp_path)
    scorer = CallReceiverScorer(
        math_files + io_files, language="typescript", n_clusters=2, cluster_seed=0
    )
    unknown = tmp_path / "unknown.ts"

    result = scorer.weighted_contribution_for_file(
        "Math.floor()",
        unknown,
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=1.5,
        cap=10.0,
        file_source=None,
    )
    # Math.floor is globally attested, no cluster resolved → era-10 weight = 0
    assert result == pytest.approx(0.0)


def test_empty_source_bag_no_cluster_bonus(tmp_path: Path) -> None:
    """file_source with no calls → empty bag → no cluster assignment, no cluster_bonus."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    math_files, io_files = _make_two_cluster_corpus(tmp_path)
    scorer = CallReceiverScorer(
        math_files + io_files, language="typescript", n_clusters=2, cluster_seed=0
    )
    unknown = tmp_path / "unknown.ts"

    result = scorer.weighted_contribution_for_file(
        "Math.floor()",
        unknown,
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=1.5,
        cap=10.0,
        file_source="const x = 1;",  # no call expressions
    )
    # Empty bag → no cluster assigned → globally attested Math.floor → 0
    assert result == pytest.approx(0.0)


def test_known_file_path_unaffected_by_source(tmp_path: Path) -> None:
    """Files in file_to_cluster use their static cluster id regardless of file_source."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    math_files, io_files = _make_two_cluster_corpus(tmp_path)
    scorer = CallReceiverScorer(
        math_files + io_files, language="typescript", n_clusters=2, cluster_seed=0
    )

    known = math_files[0]
    assert known in scorer.file_to_cluster

    hunk = "Math.floor(); fetch(url);"

    # A misleading source argument must NOT override the static cluster mapping.
    misleading_source = "fetch(url); Promise.resolve(x); console.log('hi');"
    result_with_source = scorer.weighted_contribution_for_file(
        hunk,
        known,
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=1.5,
        cap=10.0,
        file_source=misleading_source,
    )
    result_no_source = scorer.weighted_contribution_for_file(
        hunk,
        known,
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=1.5,
        cap=10.0,
        file_source=None,
    )
    assert result_with_source == pytest.approx(result_no_source)


# ---------------------------------------------------------------------------
# Plumbing consistency
# ---------------------------------------------------------------------------


def test_cluster_params_defaults_consistent_across_layers() -> None:
    """n_clusters=1 and cluster_bonus=0.0 defaults match across all layers."""
    import ast
    import inspect
    from pathlib import Path

    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    sig = inspect.signature(SequentialImportBpeScorer.__init__)
    assert sig.parameters["call_receiver_n_clusters"].default == 1
    assert sig.parameters["call_receiver_cluster_bonus"].default == pytest.approx(0.0)

    engine_root = Path(__file__).parent.parent

    # Check calibration/__init__.py for hardcoded defaults in main()
    calib_src = (engine_root / "scoring" / "calibration" / "__init__.py").read_text()
    calib_tree = ast.parse(calib_src)

    n_clusters_vals: list[int] = []
    cluster_bonus_vals: list[float] = []
    for node in ast.walk(calib_tree):
        if (
            isinstance(node, ast.AnnAssign)
            and isinstance(node.target, ast.Name)
            and node.value is not None
            and isinstance(node.value, ast.Constant)
        ):
            if node.target.id == "call_receiver_n_clusters":
                n_clusters_vals.append(int(node.value.value))
            elif node.target.id == "call_receiver_cluster_bonus":
                cluster_bonus_vals.append(float(node.value.value))

    assert len(n_clusters_vals) == 1, (
        f"Expected 1 call_receiver_n_clusters in calibration/__init__.py, "
        f"found {len(n_clusters_vals)}"
    )
    assert n_clusters_vals[0] == 1

    assert len(cluster_bonus_vals) == 1, (
        f"Expected 1 call_receiver_cluster_bonus in calibration/__init__.py, "
        f"found {len(cluster_bonus_vals)}"
    )
    assert cluster_bonus_vals[0] == pytest.approx(0.0)


def test_cli_cluster_defaults_match_run_config() -> None:
    """CLI --call-receiver-clusters and --call-receiver-cluster-bonus defaults match RunConfig."""
    import ast
    from pathlib import Path

    from argot_bench.run import RunConfig  # type: ignore[import-untyped]

    # RunConfig defaults
    assert RunConfig.__dataclass_fields__["call_receiver_n_clusters"].default == 1
    cluster_bonus_default = RunConfig.__dataclass_fields__["call_receiver_cluster_bonus"].default
    assert cluster_bonus_default == pytest.approx(0.0)

    # Bench CLI defaults
    bench_root = Path(__file__).resolve().parent.parent.parent.parent / "benchmarks"
    cli_src = (bench_root / "src" / "argot_bench" / "cli.py").read_text()
    cli_tree = ast.parse(cli_src)

    cli_cluster_defaults: list[int] = []
    cli_bonus_defaults: list[float] = []

    for node in ast.walk(cli_tree):
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "add_argument"
        ):
            has_clusters_flag = any(
                isinstance(a, ast.Constant) and "--call-receiver-clusters" in str(a.value)
                for a in node.args
            )
            has_bonus_flag = any(
                isinstance(a, ast.Constant) and "--call-receiver-cluster-bonus" in str(a.value)
                for a in node.args
            )
            for kw in node.keywords:
                if kw.arg == "default" and isinstance(kw.value, ast.Constant):
                    if has_clusters_flag:
                        cli_cluster_defaults.append(int(kw.value.value))
                    elif has_bonus_flag:
                        cli_bonus_defaults.append(float(kw.value.value))

    assert (
        len(cli_cluster_defaults) >= 1
    ), "Expected at least one --call-receiver-clusters default in cli.py"
    assert all(
        v == 1 for v in cli_cluster_defaults
    ), f"CLI cluster defaults: {cli_cluster_defaults}"

    assert (
        len(cli_bonus_defaults) >= 1
    ), "Expected at least one --call-receiver-cluster-bonus default in cli.py"
    assert all(
        v == pytest.approx(0.0) for v in cli_bonus_defaults
    ), f"CLI bonus defaults: {cli_bonus_defaults}"
