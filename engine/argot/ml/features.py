"""Engineered per-hunk feature extraction (research-only).

Runs the production 3-stage scorer (``SequentialImportBpeScorer``) over a
hunk and emits a feature dictionary suitable for training a downstream ML
classifier. Used by the benchmark / research pipeline; not exercised by
the production scoring path.

Public surface:
    * :class:`FeatureRow`    — typed shape of a single emitted JSONL row.
    * :func:`compute_features` — given a hunk + file context + a built scorer,
      return the per-hunk feature dict.
    * :func:`build_feature_row` — packages provenance + ``compute_features``.
    * :func:`synthesize_hunk_in_host` — splice a catalog hunk into a real
      host file at a chosen line, returning ``(synthesized_content,
      new_hunk_start_line, new_hunk_end_line)``. Used by the bench's
      catalog-fixture scoring routing.

The module deliberately avoids importing anything from ``argot_bench``
to keep the helpers usable from the engine package in isolation;
``cli.py`` does the bench-side wiring.
"""

from __future__ import annotations

from collections import Counter
from collections.abc import Iterable
from pathlib import Path
from typing import Any, Literal, NotRequired, TypedDict

from tree_sitter import Node

from argot.scoring.filters.typicality import _PY_PARSER, _TS_PARSER
from argot.scoring.scorers.call_receiver import (
    CallReceiverScorer,
    extract_callees,
)
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

Language = Literal["python", "typescript"]

# Top-N node types kept in ast_node_type_counts (full distribution would blow
# up the JSONL size and most tail buckets are noise for the downstream model).
_AST_TOP_N_NODE_TYPES: int = 20

# Node types we count explicitly per language (return / throw / await / nesting).
_PY_RETURN_TYPES: frozenset[str] = frozenset({"return_statement"})
_PY_THROW_TYPES: frozenset[str] = frozenset({"raise_statement"})
_PY_AWAIT_TYPES: frozenset[str] = frozenset({"await"})
_PY_NESTING_TYPES: frozenset[str] = frozenset(
    {
        "if_statement",
        "for_statement",
        "while_statement",
        "try_statement",
        "with_statement",
        "function_definition",
        "async_function_definition",
        "class_definition",
    }
)
_PY_IDENTIFIER_TYPES: frozenset[str] = frozenset({"identifier"})

_TS_RETURN_TYPES: frozenset[str] = frozenset({"return_statement"})
_TS_THROW_TYPES: frozenset[str] = frozenset({"throw_statement"})
_TS_AWAIT_TYPES: frozenset[str] = frozenset({"await_expression"})
_TS_NESTING_TYPES: frozenset[str] = frozenset(
    {
        "if_statement",
        "for_statement",
        "for_in_statement",
        "for_of_statement",
        "while_statement",
        "do_statement",
        "try_statement",
        "switch_statement",
        "function_declaration",
        "function_expression",
        "arrow_function",
        "method_definition",
        "class_declaration",
    }
)
_TS_IDENTIFIER_TYPES: frozenset[str] = frozenset(
    {"identifier", "type_identifier", "property_identifier"}
)


class FeatureRow(TypedDict):
    """JSONL row schema. One row per hunk.

    All scalar / engineered keys are always present.  The two embedding keys
    are :class:`typing.NotRequired` and populated only when the extractor is
    invoked with ``--with-embeddings`` (UnixCoder embedding extraction); they are 768-dim
    UnixCoder [CLS] vectors for the hunk and a context window respectively.
    Backward-compat: when omitted, JSONL output is byte-identical to
    pre-Phase-6.1 behaviour.
    """

    corpus: str
    is_break: bool
    fixture_id: str | None
    category: str | None
    difficulty: str | None
    file_path: str
    hunk_start_line: int
    hunk_end_line: int
    hunk_length_lines: int
    hunk_length_chars: int
    features: dict[str, Any]
    hunk_embedding: NotRequired[list[float]]
    context_embedding: NotRequired[list[float]]


def synthesize_hunk_in_host(
    catalog_content: str,
    catalog_hunk_start: int,
    catalog_hunk_end: int,
    host_content: str,
    host_inject_at_line: int,
) -> tuple[str, int, int]:
    """Splice catalog content into host content at an injection point.

    Fix A (host injection): catalog break files are tiny single-function standalone
    files, which makes ``hunk_callees ⊆ file_callees`` by construction
    (Jaccard ≈ 1.0).  Real-PR controls live in large multi-function files
    (Jaccard ≈ 0.05).  This shape mismatch was the strongest discriminator
    in the data and overwhelmed the actual anomaly signal.  By injecting
    each catalog file's content into a real corpus host file at a chosen
    line, the synthesized content shares the data-generating process of
    the controls — the catalog hunk is now scored as if it had been added
    inside a realistic file.

    Args:
        catalog_content: Full source of the catalog break file.
        catalog_hunk_start: 1-indexed line in *catalog_content* where the
            hunk begins.
        catalog_hunk_end: 1-indexed inclusive line in *catalog_content*
            where the hunk ends.
        host_content: Full source of the host file from the corpus.
        host_inject_at_line: 1-indexed line in *host_content* BEFORE which
            the catalog content is inserted.  ``1`` means "splice at the
            top, before any host line".  Values larger than the host line
            count are treated as "append at end" (no error).

    Returns:
        Tuple ``(synthesized_content, new_hunk_start_line, new_hunk_end_line)``
        where the line numbers are 1-indexed positions of the catalog hunk
        within *synthesized_content*.

    Notes on edge cases:
        * Trailing newline preservation: ``str.splitlines(keepends=True)``
          is used so a host or catalog file that ends without ``\\n`` does
          not gain one, and one that ends with ``\\n`` keeps it.
        * If *catalog_content* is empty the synthesized content equals the
          host (no lines spliced); the returned line numbers degenerate
          to ``host_inject_at_line, host_inject_at_line - 1``.
    """
    if catalog_hunk_start < 1:
        raise ValueError(f"catalog_hunk_start must be >= 1 (got {catalog_hunk_start})")
    if catalog_hunk_end < catalog_hunk_start:
        raise ValueError(
            f"catalog_hunk_end ({catalog_hunk_end}) must be >= "
            f"catalog_hunk_start ({catalog_hunk_start})"
        )
    if host_inject_at_line < 1:
        raise ValueError(f"host_inject_at_line must be >= 1 (got {host_inject_at_line})")

    catalog_lines = catalog_content.splitlines(keepends=True)
    host_lines = host_content.splitlines(keepends=True)

    # Clamp injection point — "beyond end" → append.
    inject_idx = min(host_inject_at_line - 1, len(host_lines))

    # Splicing rule: catalog content goes BEFORE host_lines[inject_idx].
    # If the host line immediately preceding the splice does not end with a
    # newline, prepend one to the catalog content so the splice is on a fresh
    # line. Symmetric guard at the catalog/host boundary on the trailing side.
    prefix = host_lines[:inject_idx]
    suffix = host_lines[inject_idx:]

    # Ensure prefix ends in a newline before injection (only if there is
    # something to follow it).
    if prefix and catalog_lines and not prefix[-1].endswith(("\n", "\r")):
        prefix = list(prefix)
        prefix[-1] = prefix[-1] + "\n"

    # Ensure catalog content ends in a newline before host suffix begins
    # (so the catalog's last line is its own line in the synthesized file).
    catalog_lines_adj = list(catalog_lines)
    if catalog_lines_adj and suffix and not catalog_lines_adj[-1].endswith(("\n", "\r")):
        catalog_lines_adj[-1] = catalog_lines_adj[-1] + "\n"

    synthesized_lines = [*prefix, *catalog_lines_adj, *suffix]
    synthesized = "".join(synthesized_lines)

    # Catalog line N (1-indexed) sits at: len(prefix) + N in the synthesized
    # file (1-indexed).  Equivalently: shift by len(prefix).
    shift = len(prefix)
    new_start = shift + catalog_hunk_start
    new_end = shift + catalog_hunk_end
    return synthesized, new_start, new_end


def _parser_for(language: Language) -> Any:
    return _PY_PARSER if language == "python" else _TS_PARSER


def _walk(root: Node) -> Iterable[Node]:
    stack: list[Node] = [root]
    while stack:
        node = stack.pop()
        yield node
        stack.extend(reversed(node.children))


def _max_nesting(root: Node, nesting_types: frozenset[str]) -> int:
    """Iterative DFS that tracks nesting depth of *nesting_types* nodes only."""
    best = 0

    def _dfs(node: Node, depth: int) -> None:
        nonlocal best
        d = depth + 1 if node.type in nesting_types else depth
        if d > best:
            best = d
        for child in node.children:
            _dfs(child, d)

    _dfs(root, 0)
    return best


def _ast_features(source: str, language: Language) -> dict[str, Any]:
    """Compute hunk-shape AST features.

    Returns a dict containing:
        ast_node_type_counts (dict[str, int]) — top-N node types by frequency
        n_returns, n_throws, n_awaits (int)
        max_nesting_depth (int)
        n_distinct_identifiers (int)
        parse_fragment_flag (bool)
    """
    if language == "python":
        return_types = _PY_RETURN_TYPES
        throw_types = _PY_THROW_TYPES
        await_types = _PY_AWAIT_TYPES
        nesting_types = _PY_NESTING_TYPES
        ident_types = _PY_IDENTIFIER_TYPES
    else:
        return_types = _TS_RETURN_TYPES
        throw_types = _TS_THROW_TYPES
        await_types = _TS_AWAIT_TYPES
        nesting_types = _TS_NESTING_TYPES
        ident_types = _TS_IDENTIFIER_TYPES

    parser = _parser_for(language)
    parse_fragment = False
    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:  # noqa: BLE001
        return {
            "ast_node_type_counts": {},
            "n_returns": 0,
            "n_throws": 0,
            "n_awaits": 0,
            "max_nesting_depth": 0,
            "n_distinct_identifiers": 0,
            "parse_fragment_flag": True,
        }

    root = tree.root_node
    parse_fragment = any(child.type == "ERROR" for child in root.children)

    type_counts: Counter[str] = Counter()
    n_returns = 0
    n_throws = 0
    n_awaits = 0
    identifiers: set[str] = set()

    for node in _walk(root):
        # Only count named nodes for type stats, returns/throws/awaits, and idents.
        # Tree-sitter emits anonymous keyword tokens (e.g. ``await``, ``return``)
        # as separate children of the corresponding named expression node; counting
        # both would double the values.
        if not node.is_named:
            continue
        type_counts[node.type] += 1
        if node.type in return_types:
            n_returns += 1
        if node.type in throw_types:
            n_throws += 1
        if node.type in await_types:
            n_awaits += 1
        if node.type in ident_types and not node.children:
            text = node.text.decode("utf-8", errors="replace") if node.text else ""
            if text:
                identifiers.add(text)

    max_nest = _max_nesting(root, nesting_types)

    del tree

    top_types = dict(type_counts.most_common(_AST_TOP_N_NODE_TYPES))

    return {
        "ast_node_type_counts": top_types,
        "n_returns": n_returns,
        "n_throws": n_throws,
        "n_awaits": n_awaits,
        "max_nesting_depth": max_nest,
        "n_distinct_identifiers": len(identifiers),
        "parse_fragment_flag": parse_fragment,
    }


def _hunk_callee_bag(hunk: str, language: Language) -> frozenset[str]:
    return frozenset(c for c in extract_callees(hunk, language) if c is not None)


def _file_callee_bag(file_source: str, language: Language) -> frozenset[str]:
    return frozenset(c for c in extract_callees(file_source, language) if c is not None)


def _jaccard(a: frozenset[str], b: frozenset[str]) -> float:
    union = a | b
    if not union:
        return 0.0
    return len(a & b) / len(union)


def _resolve_cluster(
    call_receiver: CallReceiverScorer,
    file_path: Path | None,  # noqa: ARG001 — kept for signature stability with prior callers
    file_source: str | None,
    language: Language,  # noqa: ARG001 — kept for signature stability with prior callers
) -> tuple[int | None, float]:
    """Resolve cluster for ML feature emission via unified Jaccard routing.

    Routing-leak fix: always uses the Jaccard fallback path regardless of
    whether ``file_path`` is in ``call_receiver.file_to_cluster``. This
    eliminates the routing-leak between catalog fixtures (paths NOT in
    repo_corpus_files → previously fallback) and real-PR controls (paths IN
    repo_corpus_files → previously static lookup), which was a near-perfect
    proxy for ``is_break`` (AUC 0.95).

    Returns ``(cluster_id, jaccard_to_centroid)``. Both are ``None`` / ``0.0``
    when no clusters were built, file_source is None, or the file's callee
    bag is empty.
    """
    if not call_receiver.cluster_attested:
        return None, 0.0
    if file_source is None:
        return None, 0.0
    result = call_receiver.nearest_cluster_for_source(file_source)
    if result is None:
        return None, 0.0
    return result


def _call_receiver_features(
    inner: SequentialImportBpeScorer,
    hunk_content: str,
    file_path: Path | None,
    file_source: str | None,
    language: Language,
) -> dict[str, Any]:
    """Compute call-receiver-derived counts and cluster features.

    When the production scorer was built with ``call_receiver_alpha=0`` the
    inner ``_call_receiver`` is None and we emit zero/None values for these
    fields so the schema stays stable.
    """
    cr = inner._call_receiver  # noqa: SLF001 — internal access; production scorer owns this
    if cr is None:
        return {
            "n_distinct_callees": 0,
            "n_unattested_callees": 0,
            "n_attested_root_only": 0,
            "n_cluster_absent_callees": 0,
            "cluster_id": None,
            "cluster_jaccard_to_centroid": 0.0,
        }

    callees = extract_callees(hunk_content, language)
    distinct: list[str] = []
    seen: set[str] = set()
    for c in callees:
        if c is None or c in seen:
            continue
        seen.add(c)
        distinct.append(c)

    n_unattested = sum(1 for c in distinct if c not in cr.attested)
    n_root_only = sum(
        1 for c in distinct if c not in cr.attested and c.split(".", 1)[0] in cr.attested_roots
    )

    # Routing-leak fix: unified Jaccard routing — always compute via
    # cluster_attested + file_source, never short-circuit to static lookup.
    # This makes catalog fixtures and real-PR controls take the same path,
    # eliminating the cluster_assignment_method routing-leak shortcut.
    cluster_id, jaccard = _resolve_cluster(cr, file_path, file_source, language)
    cluster_set = cr.cluster_attested.get(cluster_id) if cluster_id is not None else None
    if cluster_set is None:
        n_cluster_absent = 0
    else:
        n_cluster_absent = sum(1 for c in distinct if c in cr.attested and c not in cluster_set)

    return {
        "n_distinct_callees": len(distinct),
        "n_unattested_callees": n_unattested,
        "n_attested_root_only": n_root_only,
        "n_cluster_absent_callees": n_cluster_absent,
        "cluster_id": cluster_id,
        "cluster_jaccard_to_centroid": jaccard,
    }


def _hunk_file_context_features(
    hunk_content: str,
    file_source: str | None,
    language: Language,
) -> dict[str, Any]:
    """Compute hunk-vs-file callee context features.

    Returns ``hunk_callee_bag_size``, ``file_callee_bag_size``, the Jaccard,
    and the in-file fraction.  When ``file_source`` is None, file-level fields
    fall back to mirroring the hunk (Jaccard=1, fraction=1) — controls without
    file context still carry usable hunk-only signal.
    """
    hunk_bag = _hunk_callee_bag(hunk_content, language)
    if file_source is None:
        return {
            "hunk_callee_bag_size": len(hunk_bag),
            "file_callee_bag_size": len(hunk_bag),
            "hunk_file_callee_jaccard": 1.0 if hunk_bag else 0.0,
            "hunk_callees_in_file_fraction": 1.0 if hunk_bag else 0.0,
        }
    file_bag = _file_callee_bag(file_source, language)
    inter = hunk_bag & file_bag
    fraction = len(inter) / len(hunk_bag) if hunk_bag else 0.0
    return {
        "hunk_callee_bag_size": len(hunk_bag),
        "file_callee_bag_size": len(file_bag),
        "hunk_file_callee_jaccard": _jaccard(hunk_bag, file_bag),
        "hunk_callees_in_file_fraction": fraction,
    }


def compute_features(
    inner: SequentialImportBpeScorer,
    hunk_content: str,
    *,
    file_source: str | None,
    file_path: Path | None,
    hunk_start_line: int,
    hunk_end_line: int,
    language: Language,
) -> dict[str, Any]:
    """Run the 3-stage scorer on the hunk and pack all engineered features.

    Mirrors the call signature used by ``BenchScorer.score_hunk`` so the
    extractor can be wired into the existing fixture/control iteration paths.

    Returns a dict matching the ``features`` field of :class:`FeatureRow`.
    """
    raw = inner.score_hunk(
        hunk_content,
        file_source=file_source,
        hunk_start_line=hunk_start_line,
        hunk_end_line=hunk_end_line,
        file_path=file_path,
    )

    import_score = raw.stages.import_score
    bpe_score = raw.stages.bpe_score
    flagged = raw.flagged
    reason = raw.reason

    # adjusted_bpe is what stage 2.5 compares against the threshold; the
    # call-receiver contribution is already cached on the StageScores
    # payload so we don't need to re-run weighted_contribution here.
    adjusted_bpe = bpe_score + raw.stages.call_receiver_contribution

    stage1_flagged = flagged and reason == "import"
    stage2_flagged = flagged and reason in ("bpe", "call_receiver")

    features: dict[str, Any] = {
        "import_score": import_score,
        "bpe_score": bpe_score,
        "adjusted_bpe": adjusted_bpe,
        "stage1_flagged": stage1_flagged,
        "stage2_flagged": stage2_flagged,
        "scorer_reason": reason,
    }
    features.update(_call_receiver_features(inner, hunk_content, file_path, file_source, language))
    features.update(_hunk_file_context_features(hunk_content, file_source, language))
    features.update(_ast_features(hunk_content, language))
    return features


def build_feature_row(
    *,
    corpus: str,
    is_break: bool,
    fixture_id: str | None,
    category: str | None,
    difficulty: str | None,
    file_path_rel: str,
    hunk_start_line: int,
    hunk_end_line: int,
    hunk_content: str,
    features: dict[str, Any],
    hunk_embedding: list[float] | None = None,
    context_embedding: list[float] | None = None,
) -> FeatureRow:
    """Pack provenance + features into a single JSONL row.

    ``file_path_rel`` is the path relative to the repo root (or the catalog
    dir for fixtures); we store the relative form so rows are portable across
    machines.

    ``hunk_embedding`` / ``context_embedding`` are optional UnixCoder embedding extraction
    additions: 768-dim UnixCoder [CLS] vectors for the hunk and a context
    window centred on the hunk.  When ``None`` (the default — equivalent to
    running without ``--with-embeddings``), the keys are **omitted** from the
    returned dict so the JSONL output is byte-identical to pre-Phase-6.1
    behaviour.  When provided, they are added as **top-level** fields
    (deliberately not nested under ``features.*`` to keep the engineered
    feature dict scalar-only and to make the high-dimensional vectors easy
    to skip in downstream consumers that only need engineered features).
    """
    row: FeatureRow = {
        "corpus": corpus,
        "is_break": is_break,
        "fixture_id": fixture_id,
        "category": category,
        "difficulty": difficulty,
        "file_path": file_path_rel,
        "hunk_start_line": hunk_start_line,
        "hunk_end_line": hunk_end_line,
        "hunk_length_lines": hunk_end_line - hunk_start_line + 1,
        "hunk_length_chars": len(hunk_content),
        "features": features,
    }
    if hunk_embedding is not None:
        row["hunk_embedding"] = hunk_embedding
    if context_embedding is not None:
        row["context_embedding"] = context_embedding
    return row
