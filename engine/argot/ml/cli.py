"""``argot-extract-features`` — emit per-hunk feature vectors as JSONL.

Research infrastructure: runs the production 3-stage scorer
(``SequentialImportBpeScorer`` with the shipping config) over a corpus
and writes one JSON row per hunk to ``--out``. Used by the benchmark
and the research scripts under ``engine/scripts/``; not part of the
production scoring path.

Two operating modes
-------------------

``--manifest`` mode (always available; no ``argot_bench`` dep)
    Pass an explicit ``--manifest <manifest.yaml>`` (or ``manifest.json``),
    ``--repo-dir <path>``, and ``--dataset <dataset.jsonl>``.  The CLI
    parses the manifest, scores every fixture, then samples N controls
    from the dataset.  Useful for tests and ad-hoc runs.

``--corpus`` mode (requires the workspace ``argot_bench`` package)
    Pass ``--corpus <name>`` and the CLI reads the bench-side
    ``targets.yaml`` + ``catalogs/<name>/manifest.yaml``, clones the repo,
    checks out the primary PR sha, and runs the extract pipeline to
    produce a controls dataset.

Sampling strategy for controls
------------------------------

Real-PR control sets are large (255k for faker-js).  We sample
``N = --n-controls-per-corpus`` (default 200) by:

* Top-N highest-scoring controls by ``adjusted_bpe`` — these sit closest
  to the threshold and are the most informative negative-class examples
  for downstream ML investigation.
* PLUS an additional ``N // 2`` random controls (deterministic seed) for
  negative-class diversity.

Excluded reasons (``atypical``, ``atypical_file``, ``excluded_path``,
``auto_generated``) are dropped before sampling because downstream
classifiers only fire when stages 1-3 emit ``flagged=False`` AND the
hunk is not short-circuited.  Sampling these is wasted budget.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import TYPE_CHECKING, Any, Literal, cast

from argot.ml.features import (
    FeatureRow,
    build_feature_row,
    compute_features,
    synthesize_hunk_in_host,
)

if TYPE_CHECKING:
    from argot.ml.embeddings import UnixCoderEmbedder
from argot.scoring.adapters.python_adapter import PythonAdapter
from argot.scoring.calibration.random_hunk_sampler import (
    DEFAULT_EXCLUDE_DIRS,
    is_excluded_path,
)
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

Language = Literal["python", "typescript"]

_BPE_MODEL_B = (
    Path(__file__).resolve().parent.parent / "scoring" / "bpe" / "generic_tokens_bpe.json"
)


# ---------------------------------------------------------------------------
# Catalog/manifest loading (dependency-free for the manifest path)
# ---------------------------------------------------------------------------


def _load_manifest(manifest_path: Path) -> dict[str, Any]:
    """Load a manifest from YAML or JSON.

    YAML support is via the optional pyyaml package (declared by argot_bench
    in the workspace); JSON works dependency-free.  Raises ``RuntimeError``
    if the manifest is YAML and pyyaml is unavailable.
    """
    text = manifest_path.read_text(encoding="utf-8")
    if manifest_path.suffix in {".json"}:
        return cast(dict[str, Any], json.loads(text))
    try:
        import yaml  # type: ignore[import-untyped]
    except ImportError as e:  # pragma: no cover — workspace always has yaml
        raise RuntimeError(
            f"YAML manifest {manifest_path} requires pyyaml — install argot-bench "
            "or convert the manifest to JSON."
        ) from e
    return cast(dict[str, Any], yaml.safe_load(text))


def _adapter_for_language(language: Language) -> Any:
    if language == "python":
        return PythonAdapter()
    from argot.scoring.adapters.typescript import TypeScriptAdapter  # local import for laziness

    return TypeScriptAdapter()


def _source_files(repo_dir: Path, adapter: Any) -> list[Path]:
    out: list[Path] = []
    for ext in sorted(adapter.file_extensions):
        out.extend(sorted(repo_dir.rglob(f"*{ext}")))
    return out


# ---------------------------------------------------------------------------
# Scorer construction (mirrors argot_bench.score.build_scorer for the
# shipping config — K=8, cluster_bonus=5.0)
# ---------------------------------------------------------------------------


def build_production_scorer(
    repo_dir: Path,
    language: Language,
    *,
    n_cal: int = 100,
    seed: int = 0,
    threshold_n_seeds: int = 7,
    bpe_model_b: Path | None = None,
    enable_typicality_filter: bool = True,
    call_receiver_alpha: float = 2.0,
    call_receiver_cap: int = 5,
    call_receiver_root_bonus: float = 2.0,
    call_receiver_n_clusters: int = 8,
    call_receiver_cluster_seed: int = 0,
    call_receiver_cluster_bonus: float = 5.0,
    call_receiver_force_jaccard_routing: bool = True,
    threshold_percentile: float | None = 100.0,
) -> SequentialImportBpeScorer:
    """Build the production-config scorer (shipping: K=8, CB=5.0).

    Defaults match ``argot_bench.score.build_scorer`` so the feature
    extractor sees byte-identical scoring, EXCEPT
    ``call_receiver_force_jaccard_routing`` defaults to True here so
    catalog fixtures and real-PR controls take the same routing path
    (eliminates the catalog-vs-control routing-leak shortcut).
    """
    adapter = _adapter_for_language(language)
    files = _source_files(repo_dir, adapter)
    if not files:
        raise ValueError(f"No {language} source files found in {repo_dir}")

    from argot.scoring.calibration import calibrate_multi_seed

    median_threshold = calibrate_multi_seed(
        base_seed=seed,
        n_seeds=threshold_n_seeds,
        n_cal=n_cal,
        repo_dir=repo_dir,
        model_a_files=files,
        adapter=adapter,
        bpe_model_b_path=bpe_model_b or _BPE_MODEL_B,
        threshold_percentile=threshold_percentile,
        threshold_iqr_k=None,
        call_receiver_alpha=call_receiver_alpha,
        call_receiver_cap=call_receiver_cap,
        call_receiver_root_bonus=call_receiver_root_bonus,
        call_receiver_n_clusters=call_receiver_n_clusters,
        call_receiver_cluster_seed=call_receiver_cluster_seed,
        call_receiver_cluster_bonus=call_receiver_cluster_bonus,
        enable_typicality_filter=enable_typicality_filter,
    )
    return SequentialImportBpeScorer(
        model_a_files=files,
        bpe_model_b_path=bpe_model_b or _BPE_MODEL_B,
        bpe_threshold=median_threshold,
        adapter=adapter,
        repo_root=repo_dir,
        enable_typicality_filter=enable_typicality_filter,
        call_receiver_alpha=call_receiver_alpha,
        call_receiver_cap=call_receiver_cap,
        call_receiver_root_bonus=call_receiver_root_bonus,
        call_receiver_n_clusters=call_receiver_n_clusters,
        call_receiver_cluster_seed=call_receiver_cluster_seed,
        call_receiver_cluster_bonus=call_receiver_cluster_bonus,
        call_receiver_force_jaccard_routing=call_receiver_force_jaccard_routing,
        threshold_percentile=threshold_percentile,
        threshold_iqr_k=None,
    )


# ---------------------------------------------------------------------------
# Embedder helper (UnixCoder embedding extraction)
# ---------------------------------------------------------------------------


def _maybe_embed(
    embedder: UnixCoderEmbedder | None,
    *,
    hunk_content: str,
    file_source: str | None,
    hunk_start_line: int,
    hunk_end_line: int,
) -> tuple[list[float] | None, list[float] | None]:
    """Compute (hunk_embedding, context_embedding) when *embedder* is set.

    Returns ``(None, None)`` when ``embedder is None`` so the caller can
    pass the result straight through to ``build_feature_row`` and get
    backward-compatible JSONL.

    Context-window source: when *file_source* is available the embedder
    embeds a 512-token window centred on the hunk via
    :meth:`UnixCoderEmbedder.embed_context_window`.  When it is ``None`` (a
    rare control path), we degrade gracefully and reuse the hunk embedding
    as the context embedding so the output schema stays uniform.
    """
    if embedder is None:
        return None, None
    hunk_emb = embedder.embed(hunk_content)
    if file_source is None:
        ctx_emb = list(hunk_emb)  # copy so consumers can mutate independently
    else:
        ctx_emb = embedder.embed_context_window(
            file_source,
            hunk_start_line=hunk_start_line,
            hunk_end_line=hunk_end_line,
        )
    return hunk_emb, ctx_emb


# ---------------------------------------------------------------------------
# Fixture iteration (manifest mode)
# ---------------------------------------------------------------------------


def _iter_fixture_rows(
    inner: SequentialImportBpeScorer,
    *,
    corpus: str,
    language: Language,
    catalog_dir: Path,
    fixtures: list[dict[str, Any]],
    repo_dir: Path | None,
    embedder: UnixCoderEmbedder | None = None,
) -> Iterator[FeatureRow]:
    for fx in fixtures:
        rel = str(fx["file"])
        full = (catalog_dir / rel).read_text(encoding="utf-8")
        hs = int(fx["hunk_start_line"])
        he = int(fx["hunk_end_line"])
        lines = full.splitlines()
        hunk = "\n".join(lines[hs - 1 : he])

        # Fix A (host injection): optional host-file injection.
        # When the manifest specifies host_file + host_inject_at_line, the
        # ML feature extractor scores the catalog hunk inside a real corpus
        # file rather than the standalone catalog file.  This eliminates
        # the catalog fixture-shape leak (jaccard ≈ 1.0 standalone vs ≈ 0.05
        # for real-PR controls).  Both fields must be present together; the
        # bench-side validator (argot_bench.fixtures._parse_fixture) already
        # enforces this — we still re-check here defensively because the
        # manifest dict may be loaded outside that validator (direct mode).
        host_file_rel = fx.get("host_file")
        host_inject_at_line = fx.get("host_inject_at_line")
        if (host_file_rel is None) != (host_inject_at_line is None):
            raise ValueError(
                f"fixture {fx.get('id')!r}: host_file and host_inject_at_line must be "
                "specified together (both or neither)."
            )

        if host_file_rel is not None and host_inject_at_line is not None and repo_dir is not None:
            host_path = repo_dir / str(host_file_rel)
            host_content = host_path.read_text(encoding="utf-8")
            synthesized, new_hs, new_he = synthesize_hunk_in_host(
                catalog_content=full,
                catalog_hunk_start=hs,
                catalog_hunk_end=he,
                host_content=host_content,
                host_inject_at_line=int(host_inject_at_line),
            )
            scored_file_source: str | None = synthesized
            scored_file_path: Path | None = host_path
            scored_hs = new_hs
            scored_he = new_he
            row_file_path_rel = str(host_file_rel)
            # Recompute hunk text from the synthesized content so length
            # stats reflect what was actually scored.
            syn_lines = synthesized.splitlines()
            row_hunk = "\n".join(syn_lines[new_hs - 1 : new_he])
        else:
            scored_file_source = full
            scored_file_path = (repo_dir / rel) if repo_dir is not None else None
            scored_hs = hs
            scored_he = he
            row_file_path_rel = rel
            row_hunk = hunk

        feats = compute_features(
            inner,
            row_hunk,
            file_source=scored_file_source,
            file_path=scored_file_path,
            hunk_start_line=scored_hs,
            hunk_end_line=scored_he,
            language=language,
        )
        hunk_emb, ctx_emb = _maybe_embed(
            embedder,
            hunk_content=row_hunk,
            file_source=scored_file_source,
            hunk_start_line=scored_hs,
            hunk_end_line=scored_he,
        )
        yield build_feature_row(
            corpus=corpus,
            is_break=True,
            fixture_id=str(fx["id"]),
            category=str(fx.get("category")) if fx.get("category") is not None else None,
            difficulty=(str(fx.get("difficulty")) if fx.get("difficulty") is not None else None),
            file_path_rel=row_file_path_rel,
            hunk_start_line=scored_hs,
            hunk_end_line=scored_he,
            hunk_content=row_hunk,
            features=feats,
            hunk_embedding=hunk_emb,
            context_embedding=ctx_emb,
        )


# ---------------------------------------------------------------------------
# Control sampling + iteration
# ---------------------------------------------------------------------------


def _iter_dataset(dataset_path: Path) -> Iterator[dict[str, Any]]:
    """Stream JSONL records one at a time — never materializes the full file."""
    with dataset_path.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                yield cast(dict[str, Any], json.loads(line))


def _hunk_content_from_record(
    record: dict[str, Any], repo_dir: Path
) -> tuple[str, str, str] | None:
    """Return (file_path_rel, file_source, hunk_content) for a dataset record.

    Returns None for records that are unreadable, malformed, or whose file
    falls under an excluded directory (test/, docs/, etc.) — same gate the
    bench harness uses before scoring.
    """
    file_path_rel = record.get("file_path")
    hs = record.get("hunk_start_line")
    he = record.get("hunk_end_line")
    if not (isinstance(file_path_rel, str) and isinstance(hs, int) and isinstance(he, int)):
        return None
    file_abs = repo_dir / file_path_rel
    if is_excluded_path(file_abs, repo_dir, DEFAULT_EXCLUDE_DIRS):
        return None
    try:
        file_source = file_abs.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None
    lines = file_source.splitlines()
    hunk_content = "\n".join(lines[hs:he])
    return file_path_rel, file_source, hunk_content


def stream_sample_controls(
    candidates: Iterator[tuple[dict[str, Any], dict[str, Any]]],
    n_top: int,
    seed: int,
) -> list[tuple[dict[str, Any], dict[str, Any]]]:
    """Stream candidates; return top-N by adjusted_bpe + N//2 reservoir extras.

    Research RAM fix: replaces the previous ``_stratified_sample_controls``
    helper which materialized the entire candidate list (256k×few-KB on
    faker-js → >20 GB RSS, OOM).

    Two single-pass samples are maintained side by side as the iterator is
    drained:

    * **Top-N by ``adjusted_bpe``** — a min-heap of size ``n_top``. Each
      incoming candidate is pushed when the heap is not yet full, otherwise
      compared to the current min and swapped in if larger.
    * **N//2 reservoir** — Algorithm R (Vitter): keep the first ``n_top//2``
      items; for item ``k`` (1-indexed, ``k > n_top//2``) replace a random
      reservoir slot with probability ``(n_top//2) / k``.

    Memory footprint: ``O(n_top + n_top//2)`` candidates simultaneously,
    independent of stream length. Discarded candidates' feature dicts are
    eligible for GC immediately.

    Output ordering: top-N first (sorted by ``adjusted_bpe`` descending),
    then reservoir extras (sorted by stream index for determinism).
    Deduplicated by ``id(record)`` — overlap between top-N and reservoir is
    common when the stream is short.
    """
    import heapq

    import numpy as np

    if n_top <= 0:
        return []

    rng = np.random.default_rng(seed)
    n_reservoir = n_top // 2

    # Min-heap entries: (adjusted_bpe, tie_breaker, record, features)
    # tie_breaker is the stream index — guarantees a total ordering even
    # when adjusted_bpe ties, and gives deterministic eviction behaviour.
    heap: list[tuple[float, int, dict[str, Any], dict[str, Any]]] = []

    # Reservoir entries: (stream_index, record, features). Stream index is
    # retained so we can sort the reservoir output deterministically.
    reservoir: list[tuple[int, dict[str, Any], dict[str, Any]]] = []

    for stream_idx, (record, feats) in enumerate(candidates):
        score = float(feats.get("adjusted_bpe", 0.0))

        # --- top-N min-heap update ---
        if len(heap) < n_top:
            heapq.heappush(heap, (score, stream_idx, record, feats))
        else:
            # Cheaper than push+pop; only mutate if strictly better.
            if score > heap[0][0]:
                heapq.heapreplace(heap, (score, stream_idx, record, feats))

        # --- N//2 reservoir update (Algorithm R) ---
        if n_reservoir > 0:
            if len(reservoir) < n_reservoir:
                reservoir.append((stream_idx, record, feats))
            else:
                # k = stream_idx + 1 (1-indexed item count).
                # Probability of keeping current item = n_reservoir / k.
                j = int(rng.integers(0, stream_idx + 1))
                if j < n_reservoir:
                    reservoir[j] = (stream_idx, record, feats)

    # --- assemble output ---
    # Top-N sorted by adjusted_bpe descending (heap is min-first; a sort
    # gives the canonical output ordering callers expect).
    top_sorted = sorted(heap, key=lambda x: x[0], reverse=True)
    top_pairs = [(rec, fts) for (_score, _idx, rec, fts) in top_sorted]

    # Reservoir sorted by original stream index for deterministic output.
    reservoir_sorted = sorted(reservoir, key=lambda x: x[0])

    # Dedupe by record identity — top and reservoir may overlap on short streams.
    seen_ids = {id(rec) for rec in (pair[0] for pair in top_pairs)}
    extras: list[tuple[dict[str, Any], dict[str, Any]]] = []
    for _idx, rec, fts in reservoir_sorted:
        if id(rec) not in seen_ids:
            extras.append((rec, fts))
            seen_ids.add(id(rec))

    return top_pairs + extras


def _iter_control_rows(
    inner: SequentialImportBpeScorer,
    *,
    corpus: str,
    language: Language,
    dataset_path: Path,
    repo_dir: Path,
    n_controls: int,
    seed: int,
    embedder: UnixCoderEmbedder | None = None,
) -> Iterator[FeatureRow]:
    """Score all candidate controls then sample N + N/2.

    Excluded reasons (atypical, etc.) are dropped because the research ML
    stage only fires when stages 1-3 do not short-circuit.

    Research RAM fix: candidates are streamed through ``stream_sample_controls``
    (top-N min-heap + N/2 reservoir) instead of being materialized.  Memory
    is bounded to ~``n_controls + n_controls//2`` records regardless of
    dataset size.  The dataset JSONL is also read line-by-line (see
    :func:`_iter_dataset`) so the raw record list is never held either.
    """
    excluded_reasons = {"atypical", "atypical_file", "auto_generated"}

    def _candidate_stream() -> Iterator[tuple[dict[str, Any], dict[str, Any]]]:
        for record in _iter_dataset(dataset_path):
            ctx = _hunk_content_from_record(record, repo_dir)
            if ctx is None:
                continue
            file_path_rel, file_source, hunk_content = ctx
            hs = int(record["hunk_start_line"])
            he = int(record["hunk_end_line"])
            feats = compute_features(
                inner,
                hunk_content,
                file_source=file_source,
                file_path=repo_dir / file_path_rel,
                hunk_start_line=hs + 1,  # extract dataset is 0-indexed half-open
                hunk_end_line=he,
                language=language,
            )
            if feats["scorer_reason"] in excluded_reasons:
                continue
            yield record, feats

    sampled = stream_sample_controls(_candidate_stream(), n_controls, seed)
    for record, feats in sampled:
        file_path_rel = str(record["file_path"])
        hs = int(record["hunk_start_line"])
        he = int(record["hunk_end_line"])
        # Recover hunk content for length stats (cheap second read; alternative
        # is to plumb it through the candidate list, which doubles memory)
        ctx = _hunk_content_from_record(record, repo_dir)
        hunk_content = ctx[2] if ctx is not None else ""
        file_source_for_emb = ctx[1] if ctx is not None else None
        hunk_emb, ctx_emb = _maybe_embed(
            embedder,
            hunk_content=hunk_content,
            file_source=file_source_for_emb,
            hunk_start_line=hs + 1,
            hunk_end_line=he,
        )
        yield build_feature_row(
            corpus=corpus,
            is_break=False,
            fixture_id=None,
            category=None,
            difficulty=None,
            file_path_rel=file_path_rel,
            hunk_start_line=hs + 1,
            hunk_end_line=he,
            hunk_content=hunk_content,
            features=feats,
            hunk_embedding=hunk_emb,
            context_embedding=ctx_emb,
        )


# ---------------------------------------------------------------------------
# JSONL output
# ---------------------------------------------------------------------------


def _write_jsonl(rows: Iterator[FeatureRow], out_path: Path) -> int:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    n = 0
    with out_path.open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, sort_keys=True))
            f.write("\n")
            n += 1
    return n


# ---------------------------------------------------------------------------
# Argparse + entry point
# ---------------------------------------------------------------------------


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="argot-extract-features",
        description=(
            "Emit engineered per-hunk feature vectors as JSONL (research-only).\n\n"
            "Sampling strategy for controls: top N by adjusted_bpe (closest to threshold) "
            "PLUS N/2 random for diversity.  Excluded reasons (atypical, auto_generated, "
            "excluded_path) are dropped before sampling."
        ),
    )
    p.add_argument("--out", type=Path, required=True, help="Path to write JSONL output.")
    p.add_argument(
        "--n-controls-per-corpus",
        type=int,
        default=200,
        metavar="N",
        help=(
            "Number of top-by-adjusted_bpe controls to keep; an additional N/2 random "
            "controls are appended for negative-class diversity. Default 200."
        ),
    )
    p.add_argument("--seed", type=int, default=0, help="Sampling seed (default 0).")

    mode = p.add_argument_group("mode (one required)")
    mode.add_argument("--corpus", type=str, default=None, help="Corpus name (uses argot_bench).")
    mode.add_argument(
        "--all", action="store_true", help="Run every corpus listed in argot_bench targets.yaml."
    )

    direct = p.add_argument_group("direct mode (no argot_bench dep)")
    direct.add_argument("--manifest", type=Path, default=None, help="Path to manifest.yaml/.json.")
    direct.add_argument("--repo-dir", type=Path, default=None, help="Path to source repo.")
    direct.add_argument("--dataset", type=Path, default=None, help="Path to dataset.jsonl.")
    direct.add_argument(
        "--language",
        choices=("python", "typescript"),
        default=None,
        help="Source language (required in direct mode if manifest doesn't carry it).",
    )
    direct.add_argument(
        "--corpus-name",
        type=str,
        default=None,
        help="Corpus label written to each row in direct mode.",
    )
    direct.add_argument(
        "--catalog-dir",
        type=Path,
        default=None,
        help=(
            "Catalog dir holding fixture files (defaults to the manifest's parent). "
            "Fixture file paths in the manifest are resolved relative to this dir."
        ),
    )

    p.add_argument(
        "--n-cal",
        type=int,
        default=100,
        help="Calibration hunks per seed (default 100, current shipping).",
    )
    p.add_argument(
        "--threshold-n-seeds",
        type=int,
        default=7,
        help="Multi-seed median threshold K (default 7).",
    )
    p.add_argument(
        "--with-embeddings",
        action="store_true",
        help=(
            "UnixCoder embedding extraction: attach UnixCoder [CLS] embeddings (768-dim) "
            "for the hunk and a 512-token context window centred on the "
            "hunk to every emitted JSONL row, as top-level "
            "`hunk_embedding` / `context_embedding` fields.  Loads the "
            "encoder once per subprocess (~500 MB resident).  Requires "
            "the optional 'embeddings' extra (PyTorch); fails with a "
            "helpful error if torch is missing.  Default off — when the "
            "flag is absent the JSONL output is byte-identical to the "
            "pre-Phase-6.1 schema."
        ),
    )
    return p


def _maybe_build_embedder(with_embeddings: bool) -> UnixCoderEmbedder | None:
    """Construct a :class:`UnixCoderEmbedder` once if requested, else ``None``.

    Lazy import isolates the torch dependency: code paths that don't ask for
    embeddings never touch ``argot.ml.embeddings`` and therefore don't try
    to import torch.
    """
    if not with_embeddings:
        return None
    from argot.ml.embeddings import UnixCoderEmbedder

    print("loading UnixCoder encoder (one-time, ~500MB)...", flush=True)
    return UnixCoderEmbedder()


def _run_direct(args: argparse.Namespace) -> int:
    if args.manifest is None or args.repo_dir is None:
        print(
            "direct mode requires --manifest and --repo-dir (--dataset optional).",
            file=sys.stderr,
        )
        return 2

    manifest = _load_manifest(args.manifest)
    catalog_dir: Path = args.catalog_dir or args.manifest.parent
    corpus = args.corpus_name or str(manifest.get("corpus", catalog_dir.name))
    language: Language = cast(Language, args.language or str(manifest.get("language", "python")))
    fixtures = list(manifest.get("fixtures", []))

    inner = build_production_scorer(
        args.repo_dir,
        language,
        n_cal=args.n_cal,
        threshold_n_seeds=args.threshold_n_seeds,
    )
    embedder = _maybe_build_embedder(args.with_embeddings)

    def _all_rows() -> Iterator[FeatureRow]:
        yield from _iter_fixture_rows(
            inner,
            corpus=corpus,
            language=language,
            catalog_dir=catalog_dir,
            fixtures=fixtures,
            repo_dir=args.repo_dir,
            embedder=embedder,
        )
        if args.dataset is not None:
            yield from _iter_control_rows(
                inner,
                corpus=corpus,
                language=language,
                dataset_path=args.dataset,
                repo_dir=args.repo_dir,
                n_controls=args.n_controls_per_corpus,
                seed=args.seed,
                embedder=embedder,
            )

    n = _write_jsonl(_all_rows(), args.out)
    print(f"wrote {n} feature rows to {args.out}")
    return 0


def _run_corpus(args: argparse.Namespace, corpus_name: str) -> int:
    """Run a named corpus end-to-end via argot_bench machinery.

    Lazy-imports argot_bench so the CLI still works in --manifest mode when
    the bench package is unavailable.
    """
    try:
        from argot_bench.clone import (  # type: ignore[import-untyped]
            ensure_clone,
            ensure_sha_checked_out,
        )
        from argot_bench.extract import ensure_extracted  # type: ignore[import-untyped]
        from argot_bench.targets import load_targets  # type: ignore[import-untyped]
    except ImportError as e:
        print(f"--corpus mode requires argot_bench: {e}", file=sys.stderr)
        return 2

    bench_root = Path(__file__).resolve().parent.parent.parent.parent / "benchmarks"
    targets_yaml = bench_root / "targets.yaml"
    catalogs_dir = bench_root / "catalogs"
    data_dir = bench_root / "data"

    targets = load_targets(targets_yaml)
    by_name = {t.name: t for t in targets}
    if corpus_name not in by_name:
        print(f"unknown corpus: {corpus_name}", file=sys.stderr)
        return 2
    target = by_name[corpus_name]

    catalog_dir = catalogs_dir / corpus_name
    manifest = _load_manifest(catalog_dir / "manifest.yaml")
    fixtures = list(manifest.get("fixtures", []))
    language = cast(Language, target.language)

    repo = ensure_clone(data_dir, target.name, target.url)
    primary_sha = target.prs[0].sha
    ensure_sha_checked_out(repo, primary_sha)
    dataset = ensure_extracted(repo, data_dir / target.name / primary_sha / "dataset.jsonl")

    inner = build_production_scorer(
        repo,
        language,
        n_cal=args.n_cal,
        threshold_n_seeds=args.threshold_n_seeds,
    )
    embedder = _maybe_build_embedder(args.with_embeddings)

    def _all_rows() -> Iterator[FeatureRow]:
        yield from _iter_fixture_rows(
            inner,
            corpus=corpus_name,
            language=language,
            catalog_dir=catalog_dir,
            fixtures=fixtures,
            repo_dir=repo,
            embedder=embedder,
        )
        yield from _iter_control_rows(
            inner,
            corpus=corpus_name,
            language=language,
            dataset_path=dataset,
            repo_dir=repo,
            n_controls=args.n_controls_per_corpus,
            seed=args.seed,
            embedder=embedder,
        )

    out_path = args.out
    n = _write_jsonl(_all_rows(), out_path)
    print(f"[{corpus_name}] wrote {n} feature rows to {out_path}")
    return 0


def _run_all(args: argparse.Namespace) -> int:
    """Run every corpus listed in ``targets.yaml`` — each in a fresh subprocess.

    RAM-hygiene fix: the in-process loop accumulated the
    BPE tokenizer + scorer state across corpora, pushing peak RSS to ~22 GB
    on the full 6-corpus run. Spawning a subprocess per corpus keeps peak
    RSS bounded to a single corpus's footprint; process teardown frees all
    transformer / tokenizer / sklearn state deterministically. The
    re-import overhead per corpus (~5-10s) is small relative to extraction
    time per corpus (minutes).
    """
    import subprocess

    try:
        from argot_bench.targets import load_targets
    except ImportError as e:
        print(f"--all mode requires argot_bench: {e}", file=sys.stderr)
        return 2

    bench_root = Path(__file__).resolve().parent.parent.parent.parent / "benchmarks"
    targets = load_targets(bench_root / "targets.yaml")
    args.out.mkdir(parents=True, exist_ok=True)
    rc = 0
    for t in targets:
        out_path = args.out / f"{t.name}.jsonl"
        cmd = [
            sys.executable,
            "-m",
            "argot.ml.cli",
            "--corpus",
            t.name,
            "--out",
            str(out_path),
            "--n-controls-per-corpus",
            str(args.n_controls_per_corpus),
            "--seed",
            str(args.seed),
            "--n-cal",
            str(args.n_cal),
            "--threshold-n-seeds",
            str(args.threshold_n_seeds),
        ]
        if args.with_embeddings:
            cmd.append("--with-embeddings")
        print(f"[--all] spawning subprocess for corpus={t.name}", flush=True)
        proc = subprocess.run(cmd, check=False)
        if proc.returncode != 0:
            print(
                f"[--all] corpus {t.name} failed (rc={proc.returncode})",
                file=sys.stderr,
                flush=True,
            )
            rc |= proc.returncode
    return rc


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.all:
        if not args.out.is_dir() and args.out.exists():
            print("--all requires --out to be a directory.", file=sys.stderr)
            return 2
        return _run_all(args)
    if args.corpus is not None:
        return _run_corpus(args, args.corpus)
    return _run_direct(args)


if __name__ == "__main__":
    raise SystemExit(main())
