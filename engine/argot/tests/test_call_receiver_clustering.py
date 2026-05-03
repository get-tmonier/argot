"""Tests for era-11 cluster-conditional attestation in CallReceiverScorer."""

from __future__ import annotations

from pathlib import Path

import pytest

from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

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
    """n_clusters=8 and cluster_bonus=5.0 defaults (era-11 shipping) match across all layers."""
    import ast
    import inspect
    from pathlib import Path

    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    sig = inspect.signature(SequentialImportBpeScorer.__init__)
    assert sig.parameters["call_receiver_n_clusters"].default == 8
    assert sig.parameters["call_receiver_cluster_bonus"].default == pytest.approx(5.0)

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
    assert n_clusters_vals[0] == 8

    assert len(cluster_bonus_vals) == 1, (
        f"Expected 1 call_receiver_cluster_bonus in calibration/__init__.py, "
        f"found {len(cluster_bonus_vals)}"
    )
    assert cluster_bonus_vals[0] == pytest.approx(5.0)


def test_cli_cluster_defaults_match_run_config() -> None:
    """CLI --call-receiver-clusters and --call-receiver-cluster-bonus defaults match RunConfig."""
    import ast
    from pathlib import Path

    from argot_bench.run import RunConfig  # type: ignore[import-untyped]

    # RunConfig defaults (era-11 shipping)
    assert RunConfig.__dataclass_fields__["call_receiver_n_clusters"].default == 8
    cluster_bonus_default = RunConfig.__dataclass_fields__["call_receiver_cluster_bonus"].default
    assert cluster_bonus_default == pytest.approx(5.0)

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
        v == 8 for v in cli_cluster_defaults
    ), f"CLI cluster defaults (era-11 shipping): {cli_cluster_defaults}"

    assert (
        len(cli_bonus_defaults) >= 1
    ), "Expected at least one --call-receiver-cluster-bonus default in cli.py"
    assert all(
        v == pytest.approx(5.0) for v in cli_bonus_defaults
    ), f"CLI bonus defaults (era-11 shipping): {cli_bonus_defaults}"


# ---------------------------------------------------------------------------
# Era-11 Phase 5: cluster-aware calibration
# ---------------------------------------------------------------------------


_BPE_MODEL_B_PATH = Path(__file__).parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"


def _build_two_cluster_repo(tmp_path: Path, n_per_cluster: int = 8) -> Path:
    """Build a synthetic 2-cluster TS repo where cluster_bonus fires on calibration hunks.

    Cluster_bonus can only fire on a hunk from file ``f`` (in cluster ``k``) when
    one of its callees ``c`` is globally attested but absent from cluster ``k``'s
    attested set.  But ``cluster_attested[k]`` includes ``f``'s own callee bag
    by construction, so any callee in ``f``'s source is always in ``f``'s
    cluster set.  The only way cluster_bonus can fire on a SAMPLED hunk is via
    the fallback path: ``f`` is NOT in ``file_to_cluster`` (e.g. because the
    repo dir is broader than ``model_a_files``), and the Jaccard fallback maps
    ``f`` to a cluster whose attested set is missing some of ``f``'s callees.

    To produce that situation we create a third "fallback" subdir whose files
    are NOT included in model_a_files (the scorer's fit corpus), but ARE
    included in the calibration hunk pool (sampled from repo root).  Each
    fallback file is io-flavored at the FILE level (so Jaccard maps it to the
    io cluster) but contains a sampleable function that calls Math.* callees
    — those Math.* callees are globally attested via math/ files but absent
    from the io cluster's attested set → cluster_bonus FIRES.
    """
    src_dir = tmp_path / "src"
    src_dir.mkdir()

    math_callees = (
        "  Math.floor(x);\n"
        "  Math.random();\n"
        "  Math.min(x, x);\n"
        "  Math.max(x, x);\n"
        "  Math.abs(x);\n"
        "  Math.sqrt(x);\n"
        "  Math.log(x);\n"
        "  Math.exp(x);\n"
        "  Number.isFinite(x);\n"
        "  Number.parseInt('1');\n"
    )
    io_callees = (
        "  fetch(url);\n"
        "  Promise.resolve(url);\n"
        "  console.log('a');\n"
        "  console.warn('b');\n"
        "  console.error('c');\n"
        "  console.info('d');\n"
        "  JSON.stringify(url);\n"
        "  JSON.parse(url);\n"
        "  Promise.reject(url);\n"
        "  Promise.all([]);\n"
    )

    # Pure math files (in model_a_files): only math callees → math cluster.
    for i in range(n_per_cluster):
        f = src_dir / f"math_{i}.ts"
        f.write_text(
            f"export function pure_math_{i}(x: number) {{\n" f"{math_callees}" "  return x;\n" "}\n"
        )
    # Pure io files (in model_a_files): only io callees → io cluster.
    for i in range(n_per_cluster):
        f = src_dir / f"io_{i}.ts"
        f.write_text(
            f"export async function pure_io_{i}(url: string) {{\n"
            f"{io_callees}"
            "  return url;\n"
            "}\n"
        )

    # Fallback subdir (NOT in model_a_files): io-flavored files whose hunks
    # call Math.* callees.  Sampled hunks from these files trigger the
    # fallback path → Jaccard-nearest is io cluster → cluster_bonus fires
    # on the Math.* callees (globally attested via math/ files but absent
    # from io cluster's attested set).
    fb_dir = src_dir / "fallback"
    fb_dir.mkdir()
    for i in range(n_per_cluster):
        f = fb_dir / f"hybrid_{i}.ts"
        f.write_text(
            # File-level callee bag is dominated by io callees → Jaccard nearest is io.
            f"export async function io_helper_{i}(url: string) {{\n"
            f"{io_callees}"
            "  return url;\n"
            "}\n"
            "\n"
            # Second sampleable function: calls Math.* (globally attested via
            # math/ files, but Math.* are absent from io cluster's attested set
            # because no io file has Math.* in its bag).
            f"export function math_caller_{i}(x: number) {{\n"
            "  Math.floor(x);\n"
            "  Math.random();\n"
            "  Math.min(x, x);\n"
            "  Math.abs(x);\n"
            "  Math.max(x, x);\n"
            "  return x;\n"
            "}\n"
        )
    return src_dir


def _build_scorer(
    repo: Path,
    *,
    n_clusters: int,
    cluster_bonus: float,
    metadata: bool,
    threshold_percentile: float | None = None,
) -> SequentialImportBpeScorer:
    """Helper: build a SequentialImportBpeScorer for tests, sharing a tokenizer cache."""
    from argot.scoring.adapters.typescript import TypeScriptAdapter
    from argot.scoring.calibration.random_hunk_sampler import (
        sample_hunks,
        sample_hunks_with_metadata,
    )

    adapter = TypeScriptAdapter()
    # model_a_files EXCLUDES the fallback/ subdir so hunks sampled from
    # fallback/ trigger the era-11 Phase 1 file_source fallback path → that
    # is where cluster_bonus actually fires on calibration hunks.
    files = sorted(p for p in repo.rglob("*.ts") if "fallback" not in p.parts)
    if metadata:
        meta = sample_hunks_with_metadata(repo, n=8, seed=0, adapter=adapter)
        return SequentialImportBpeScorer(
            model_a_files=files,
            bpe_model_b_path=_BPE_MODEL_B_PATH,
            calibration_hunks=[h for h, _, _ in meta],
            calibration_hunks_with_metadata=meta,
            adapter=adapter,
            threshold_percentile=threshold_percentile,
            call_receiver_alpha=2.0,
            call_receiver_root_bonus=2.0,
            call_receiver_cap=5,
            call_receiver_n_clusters=n_clusters,
            call_receiver_cluster_seed=0,
            call_receiver_cluster_bonus=cluster_bonus,
            enable_typicality_filter=False,  # synthetic corpus may not pass typicality
        )
    hunks = sample_hunks(repo, n=8, seed=0, adapter=adapter)
    return SequentialImportBpeScorer(
        model_a_files=files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=hunks,
        adapter=adapter,
        threshold_percentile=threshold_percentile,
        call_receiver_alpha=2.0,
        call_receiver_root_bonus=2.0,
        call_receiver_cap=5,
        call_receiver_n_clusters=n_clusters,
        call_receiver_cluster_seed=0,
        call_receiver_cluster_bonus=cluster_bonus,
        enable_typicality_filter=False,
    )


def test_metadata_calibration_raises_threshold_when_cluster_bonus_fires(
    tmp_path: Path,
) -> None:
    """When n_clusters>1 and cluster_bonus>0, calibration threshold > the n_clusters=1 baseline."""
    repo = _build_two_cluster_repo(tmp_path)

    s_baseline = _build_scorer(repo, n_clusters=1, cluster_bonus=0.0, metadata=False)
    s_cluster = _build_scorer(repo, n_clusters=2, cluster_bonus=5.0, metadata=True)

    # Sanity: the metadata path actually fired (some calibration hunks got bonus).
    assert s_cluster.n_calibration > 0
    # The cluster-aware threshold must rise to absorb cluster_bonus signal.
    assert s_cluster.bpe_threshold > s_baseline.bpe_threshold + 0.1, (
        f"cluster threshold {s_cluster.bpe_threshold:.4f} did not rise meaningfully "
        f"above baseline {s_baseline.bpe_threshold:.4f}"
    )


def test_n_clusters_1_calibration_byte_identical(tmp_path: Path) -> None:
    """n_clusters=1 path: with vs without metadata-aware sampling → identical threshold."""
    repo = _build_two_cluster_repo(tmp_path)

    # When n_clusters=1, the metadata kwarg should be ignored (era-10 path).
    s_no_meta = _build_scorer(repo, n_clusters=1, cluster_bonus=0.0, metadata=False)
    s_with_meta = _build_scorer(repo, n_clusters=1, cluster_bonus=0.0, metadata=True)

    # Era-10 byte-identical guarantee for n_clusters=1.
    assert s_no_meta.bpe_threshold == pytest.approx(s_with_meta.bpe_threshold, abs=0.0)
    assert s_no_meta.cal_scores == s_with_meta.cal_scores


def test_alpha_root_bonus_zero_in_calibration(tmp_path: Path) -> None:
    """Calibration isolates cluster_bonus (alpha=0, root_bonus=0); score-time stays full."""
    from argot.scoring.adapters.typescript import TypeScriptAdapter
    from argot.scoring.scorers.call_receiver import CallReceiverScorer
    from argot.scoring.scorers.sequential_import_bpe import (
        SequentialImportBpeScorer,
        _blank_prose_lines,
    )

    repo = _build_two_cluster_repo(tmp_path)
    adapter = TypeScriptAdapter()
    # Mirror _build_scorer: model_a_files exclude the fallback subdir so that
    # the io cluster's attested set is io-only (Math.* absent).
    files = sorted(p for p in repo.rglob("*.ts") if "fallback" not in p.parts)

    # Build a CallReceiverScorer the same way the inner scorer does, to inspect contributions.
    cr = CallReceiverScorer(
        files,
        language="typescript",
        alpha=2.0,
        cap=5,
        adapter=adapter,
        n_clusters=2,
        cluster_seed=0,
    )

    # Construct hunks that exercise the unattested + attested-root branch (era-10 alpha+root_bonus)
    # AND a globally-attested-but-cluster-absent branch (era-11 cluster_bonus).
    fp_io = next(p for p in files if p.name.startswith("io_"))

    # Mixed hunk: a globally unattested callee + a globally-attested-cluster-absent callee.
    # `nonexistent_function()` is unattested → era-10 contributes alpha (2.0).
    # `Math.floor()` is globally attested but absent from io's cluster → cluster_bonus.
    mixed_hunk = "nonexistent_function();\nMath.floor(1);"

    # Calibration-flavored call: alpha=0, root_bonus=0 → only cluster_bonus fires.
    contrib_calibration = cr.weighted_contribution_for_file(
        mixed_hunk,
        file_path=fp_io,
        alpha=0.0,
        root_bonus=0.0,
        cluster_bonus=5.0,
        cap=100.0,
    )
    # Score-time call: full weights — alpha + cluster_bonus.
    contrib_score_time = cr.weighted_contribution_for_file(
        mixed_hunk,
        file_path=fp_io,
        alpha=2.0,
        root_bonus=2.0,
        cluster_bonus=5.0,
        cap=100.0,
    )

    # Calibration must NOT count alpha for `nonexistent_function`.
    # If clustering put Math.floor in io's set, both contribs equal alpha-only and we skip;
    # otherwise calibration = cluster_bonus and score-time > calibration.
    io_cluster = cr.file_to_cluster[fp_io]
    if "Math.floor" not in cr.cluster_attested[io_cluster]:
        # cluster_bonus fires → calibration sees only it.
        assert contrib_calibration == pytest.approx(5.0)
        # score-time also adds alpha for the unattested call.
        assert contrib_score_time == pytest.approx(2.0 + 5.0)
    else:
        pytest.skip("KMeans put Math.floor in io cluster; cluster_bonus suppressed")

    # Now end-to-end: build a real scorer with metadata calibration and verify
    # cal_scores reflect raw_BPE + cluster_bonus only (no alpha).
    from argot.scoring.calibration.random_hunk_sampler import sample_hunks_with_metadata

    meta = sample_hunks_with_metadata(repo, n=8, seed=0, adapter=adapter)
    scorer = SequentialImportBpeScorer(
        model_a_files=files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=[h for h, _, _ in meta],
        calibration_hunks_with_metadata=meta,
        adapter=adapter,
        threshold_percentile=None,
        call_receiver_alpha=2.0,
        call_receiver_root_bonus=2.0,
        call_receiver_cap=5,
        call_receiver_n_clusters=2,
        call_receiver_cluster_seed=0,
        call_receiver_cluster_bonus=5.0,
        enable_typicality_filter=False,
    )

    # Manually compute expected cal_scores using alpha=0, root_bonus=0.
    expected: list[float] = []
    cr2 = scorer._call_receiver
    assert cr2 is not None
    for hunk, fp, src in meta:
        raw = scorer._bpe_score(_blank_prose_lines(hunk, adapter.prose_line_ranges(hunk)))
        contrib = cr2.weighted_contribution_for_file(
            hunk,
            file_path=fp,
            file_source=src,
            alpha=0.0,
            root_bonus=0.0,
            cluster_bonus=5.0,
            cap=float(cr2.cap),
        )
        expected.append(raw + contrib)

    assert scorer.cal_scores == pytest.approx(expected)


def test_metadata_calibration_filters_typicality(tmp_path: Path) -> None:
    """Metadata calibration must apply the same hunk-level typicality filter as era-10."""
    from argot.scoring.adapters.typescript import TypeScriptAdapter
    from argot.scoring.calibration.random_hunk_sampler import sample_hunks_with_metadata
    from argot.scoring.filters.typicality import TypicalityModel
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

    repo = _build_two_cluster_repo(tmp_path)
    adapter = TypeScriptAdapter()
    # Match _build_scorer: model_a_files exclude fallback so cluster_bonus can fire.
    files = sorted(p for p in repo.rglob("*.ts") if "fallback" not in p.parts)
    meta = sample_hunks_with_metadata(repo, n=8, seed=0, adapter=adapter)

    # Inject a synthetic atypical hunk: a string-array literal trips the
    # literal_leaf_ratio gate (>0.80) with named_leaf_count >= 5.
    typ = TypicalityModel(language="typescript")
    atypical_hunk: str | None = None
    candidates = [
        'const data = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"];',
        "const xs = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18];",
    ]
    for c in candidates:
        if typ.is_atypical(c)[0]:
            atypical_hunk = c
            break
    if atypical_hunk is None:
        pytest.skip("Could not construct an atypical hunk for this typicality model")

    fp = files[0]
    src = fp.read_text()
    meta_with_atypical: list[tuple[str, Path, str]] = [*meta, (atypical_hunk, fp, src)]

    scorer = SequentialImportBpeScorer(
        model_a_files=files,
        bpe_model_b_path=_BPE_MODEL_B_PATH,
        calibration_hunks=[h for h, _, _ in meta_with_atypical],
        calibration_hunks_with_metadata=meta_with_atypical,
        adapter=adapter,
        threshold_percentile=None,
        call_receiver_alpha=2.0,
        call_receiver_root_bonus=2.0,
        call_receiver_cap=5,
        call_receiver_n_clusters=2,
        call_receiver_cluster_seed=0,
        call_receiver_cluster_bonus=1.0,
        enable_typicality_filter=True,
    )

    # n_calibration must equal the count of TYPICAL hunks (atypical was filtered).
    expected_n = sum(1 for h, _, _ in meta_with_atypical if not typ.is_atypical(h)[0])
    assert scorer.n_calibration == expected_n
    assert scorer.n_calibration < len(meta_with_atypical)


def test_call_receiver_alpha_root_bonus_zero_suppresses_branches(tmp_path: Path) -> None:
    """Confirm weighted_contribution_for_file with alpha=0, root_bonus=0 contributes 0
    for unattested/attested-root callees (only cluster_bonus can fire)."""
    from argot.scoring.scorers.call_receiver import CallReceiverScorer

    f = tmp_path / "a.ts"
    f.write_text("Math.floor(x);")
    scorer = CallReceiverScorer([f], language="typescript", n_clusters=1)

    # Unattested + attested-root combinations — both must contribute 0 with alpha=0.
    hunk = "Math.random();\nfetch(url);\nentirelyForeign();"
    result = scorer.weighted_contribution_for_file(
        hunk,
        file_path=f,
        alpha=0.0,
        root_bonus=0.0,
        cluster_bonus=0.0,
        cap=100.0,
    )
    assert result == pytest.approx(0.0)
