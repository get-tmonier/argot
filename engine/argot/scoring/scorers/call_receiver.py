"""Call-receiver scorer — Stage 1.5.

Presence-based scorer: tracks distinct call-expression callees in the
model-A corpus and counts unattested callees in a hunk.  Used by
SequentialImportBpeScorer to apply a soft additive BPE penalty:

    adjusted_bpe = raw_bpe + alpha * min(count_unattested(hunk), cap)

Cluster-conditional attestation (when ``n_clusters > 1``): files are grouped
by callee-bag similarity (MinHash + KMeans). Callees globally attested but
absent from the hunk-file's cluster's attested set contribute ``cluster_bonus``
as an additive penalty on top of the global-attestation logic.

Optional frequency-aware attestation (``cluster_rare_threshold > 0``): a
callee technically present in a cluster but in only a few files is treated
as cluster-absent, so ``cluster_bonus`` fires for "rare-but-present"
callees too.

Reuses module-level parsers from filters.typicality to avoid the linear
memory growth that occurs when TsParser is instantiated per-hunk.
"""

from __future__ import annotations

import hashlib
from collections.abc import Iterator
from pathlib import Path
from typing import Any, Literal, Protocol

from tree_sitter import Node

from argot.scoring.filters.typicality import _PY_PARSER, _TS_PARSER
from argot.scoring.scorers.shape_primitive import ShapePrimitive

Language = Literal["python", "typescript"]

_PY_CALL_TYPES: frozenset[str] = frozenset({"call"})
_PY_MEMBER_TYPES: frozenset[str] = frozenset({"attribute"})
_PY_IDENTIFIER_TYPES: frozenset[str] = frozenset({"identifier"})

_TS_CALL_TYPES: frozenset[str] = frozenset({"call_expression", "new_expression"})
_TS_MEMBER_TYPES: frozenset[str] = frozenset({"member_expression"})
_TS_IDENTIFIER_TYPES: frozenset[str] = frozenset({"identifier", "type_identifier"})

_MINHASH_N_PERMS: int = 128
# Mersenne prime 2^31-1; fits in int32 after mod, safe for int64 arithmetic
_MINHASH_PRIME: int = (1 << 31) - 1


class _DataDominantAdapter(Protocol):
    def is_data_dominant(self, source: str, threshold: float = 0.65) -> bool: ...


def _walk_nodes(root: Node) -> Iterator[Node]:
    stack: list[Node] = [root]
    while stack:
        node = stack.pop()
        yield node
        stack.extend(reversed(node.children))


def _text(node: Node) -> str:
    return node.text.decode("utf-8", errors="replace") if node.text else ""


def _extract_python_callee(call_node: Node) -> str | None:
    callee = call_node.child_by_field_name("function")
    if callee is None:
        return None
    parts: list[str] = []
    while callee.type in _PY_MEMBER_TYPES:
        attr = callee.child_by_field_name("attribute")
        obj = callee.child_by_field_name("object")
        if attr is None or obj is None:
            return None
        parts.insert(0, _text(attr))
        callee = obj
    if callee.type in _PY_IDENTIFIER_TYPES:
        parts.insert(0, _text(callee))
        return ".".join(parts)
    if callee.type in _PY_CALL_TYPES:
        parts.insert(0, "<call>")
        return ".".join(parts)
    return None


def _extract_typescript_callee(call_node: Node) -> str | None:
    field_name = "constructor" if call_node.type == "new_expression" else "function"
    callee = call_node.child_by_field_name(field_name)
    if callee is None:
        return None
    parts: list[str] = []
    while callee.type in _TS_MEMBER_TYPES:
        prop = callee.child_by_field_name("property")
        obj = callee.child_by_field_name("object")
        if prop is None or obj is None:
            return None
        parts.insert(0, _text(prop))
        callee = obj
    if callee.type in _TS_IDENTIFIER_TYPES:
        parts.insert(0, _text(callee))
        return ".".join(parts)
    if callee.type in _TS_CALL_TYPES:
        parts.insert(0, "<call>")
        return ".".join(parts)
    return None


def _has_root_error(source: str, language: Language) -> bool:
    """Return True if any direct child of the parse tree root is an ERROR node.

    Hunk slices extracted out of file context (docstring bodies, method-shorthand
    definitions without their enclosing object literal) produce root-level ERROR
    nodes.  Callee extraction from such fragments is unreliable and should be
    skipped to avoid false positives.
    """
    parser = _PY_PARSER if language == "python" else _TS_PARSER
    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:  # noqa: BLE001
        return True
    has_error = any(child.type == "ERROR" for child in tree.root_node.children)
    del tree
    return has_error


def extract_callees(source: str, language: Language) -> list[str | None]:
    """Return dotted-callee signatures for every call-expression in *source*.

    Each call-expression maps to a dotted string (``"Math.random"``, ``"app.route"``,
    ``"fetch"``), a canonical ``"<call>.X"`` string when the callee chain root is itself
    a call expression (e.g., ``Router().route('/x')`` → ``"<call>.route"``), or ``None``
    when the callee bottoms out at a subscript or parenthesised expression.  ``None``
    entries are included for auditing but excluded from set membership.

    Returns ``[]`` on parse error or empty source.
    """
    if not source.strip():
        return []
    if language == "python":
        parser = _PY_PARSER
        call_types = _PY_CALL_TYPES
        extractor = _extract_python_callee
    elif language == "typescript":
        parser = _TS_PARSER
        call_types = _TS_CALL_TYPES
        extractor = _extract_typescript_callee
    else:
        raise ValueError(f"unsupported language: {language!r}")

    try:
        tree = parser.parse(source.encode("utf-8"))
    except Exception:  # noqa: BLE001
        return []
    out: list[str | None] = []
    for node in _walk_nodes(tree.root_node):
        if node.type in call_types:
            out.append(extractor(node))
    del tree
    return out


def _minhash_signature(
    callee_bag: frozenset[str],
    a_params: list[int],
    b_params: list[int],
) -> list[int]:
    """Return a 128-element MinHash signature for *callee_bag*.

    Uses universal hash family h(x) = (a*x + b) mod P where P is a Mersenne prime.
    Files with similar callee bags will tend to share more matching signature values.
    """
    n = len(a_params)
    sig = [_MINHASH_PRIME] * n
    for callee in callee_bag:
        h = int.from_bytes(hashlib.md5(callee.encode()).digest()[:8], "little") % _MINHASH_PRIME
        for i in range(n):
            v = (a_params[i] * h + b_params[i]) % _MINHASH_PRIME
            if v < sig[i]:
                sig[i] = v
    return sig


def _build_clusters(
    file_bags: list[tuple[Path, frozenset[str]]],
    n_clusters: int,
    seed: int,
) -> tuple[
    dict[Path, int],
    dict[int, frozenset[str]],
    dict[int, dict[str, int]],
    dict[int, int],
]:
    """Cluster files by callee-bag similarity using MinHash signatures + KMeans.

    Returns:
        file_to_cluster: maps resolved file path → cluster id (0-indexed).
        cluster_attested: maps cluster id → union of all callees in cluster files
            (boolean attestation set; default behaviour).
        cluster_callee_counts: maps cluster id → {callee → number of cluster
            files that contain this callee}. Used by frequency-aware
            attestation: a callee in <= ``cluster_rare_threshold`` files is
            treated as cluster-absent for scoring purposes, even though it
            is technically present in the union.
        cluster_sizes: maps cluster id → number of files in the cluster.
    """
    import numpy as np
    from sklearn.cluster import KMeans

    rng = np.random.default_rng(seed)
    a_params = [int(x) for x in rng.integers(1, _MINHASH_PRIME, size=_MINHASH_N_PERMS)]
    b_params = [int(x) for x in rng.integers(0, _MINHASH_PRIME, size=_MINHASH_N_PERMS)]

    paths = [p for p, _ in file_bags]
    bags = [bag for _, bag in file_bags]

    # Compute MinHash signatures; empty bags get all-zero signature
    raw_sigs: list[list[int]] = []
    for bag in bags:
        if bag:
            raw_sigs.append(_minhash_signature(bag, a_params, b_params))
        else:
            raw_sigs.append([0] * _MINHASH_N_PERMS)

    sigs = np.array(raw_sigs, dtype=np.float64) / _MINHASH_PRIME  # normalize to [0, 1]

    effective_k = min(n_clusters, len(bags))
    if effective_k <= 1:
        labels: list[int] = [0] * len(bags)
    else:
        km = KMeans(n_clusters=effective_k, random_state=seed, n_init=10)
        labels = list(km.fit_predict(sigs))

    file_to_cluster: dict[Path, int] = {p: int(labels[i]) for i, p in enumerate(paths)}

    cluster_attested: dict[int, frozenset[str]] = {}
    cluster_callee_counts: dict[int, dict[str, int]] = {}
    cluster_sizes: dict[int, int] = {}
    for cid in range(effective_k):
        counts: dict[str, int] = {}
        n_files_in_cluster = 0
        for i, bag in enumerate(bags):
            if labels[i] == cid:
                n_files_in_cluster += 1
                for callee in bag:
                    counts[callee] = counts.get(callee, 0) + 1
        cluster_attested[cid] = frozenset(counts.keys())
        cluster_callee_counts[cid] = counts
        cluster_sizes[cid] = n_files_in_cluster

    return file_to_cluster, cluster_attested, cluster_callee_counts, cluster_sizes


class CallReceiverScorer:
    """BPE scorer call-receiver component.

    Fit: scan *repo_corpus_files*, union all non-None callees into a frozenset.
    Score: count distinct unattested callees in a hunk (0 if parse fragment).
    Used by SequentialImportBpeScorer to compute adjusted_bpe.

    When n_clusters > 1, files are clustered by callee-bag similarity. Use
    weighted_contribution_for_file() to apply the additive cluster_bonus for
    globally-attested callees absent from the hunk-file's cluster attested set
    (and optionally for cluster-rare callees when cluster_rare_threshold > 0).
    """

    def __init__(
        self,
        repo_corpus_files: list[Path],
        *,
        language: Language,
        alpha: float = 1.0,
        cap: int = 5,
        adapter: _DataDominantAdapter | None = None,
        n_clusters: int = 1,
        cluster_seed: int = 0,
        force_jaccard_routing: bool = False,
        cluster_rare_threshold: int = 0,
        cluster_size_min: int = 0,
        shape_primitives: list[ShapePrimitive[Any]] | None = None,
    ) -> None:
        if not repo_corpus_files:
            raise ValueError("repo_corpus_files must be non-empty")
        self._language: Language = language
        self.alpha: float = alpha
        self.cap: int = cap
        # When True, weighted_contribution_for_file ALWAYS routes via the Jaccard
        # fallback path (using file_source) regardless of whether file_path is in
        # file_to_cluster. This eliminates the routing-leak between catalog
        # fixtures (unknown paths → fallback) and real-PR controls (known paths
        # → static lookup) for ML feature extraction. Default False preserves
        # production scoring's static-lookup fast path.
        self.force_jaccard_routing: bool = force_jaccard_routing
        # Frequency-aware cluster attestation. A callee is treated as
        # cluster-absent (cluster_bonus fires) when it appears in at most
        # this many files of the hunk's cluster — even though it's technically
        # present in the union. 0 (default) preserves boolean-union behaviour.
        # Catches "rare-but-attested" callees like a network primitive that
        # shows up in one build script but never in production modules.
        self.cluster_rare_threshold: int = cluster_rare_threshold
        # Size-conditional rare attestation. The rare-threshold rule only fires
        # on clusters with at least this many files. In small clusters
        # ("1 of 24") rare counts conflate with "uncommon callee" and the rule
        # fires symmetrically on calibration hunks too, defeating its purpose.
        # The size floor decouples "genuinely anomalous in a large cluster"
        # from "small-sample noise". 0 (default) disables the floor.
        self.cluster_size_min: int = cluster_size_min

        # Swappable AST-shape primitives. Each primitive computes a
        # per-cluster baseline at fit time and an additive scalar
        # contribution per hunk at score time. Empty list (default) is
        # a true no-op — the dispatch in weighted_contribution_for_file
        # adds 0.0 to the existing sum.
        self.shape_primitives: list[ShapePrimitive[Any]] = list(shape_primitives or [])

        attested: set[str] = set()
        skipped: int = 0
        # (path, callee_bag) pairs for cluster building — collected iff n_clusters > 1
        file_bags: list[tuple[Path, frozenset[str]]] = []
        # (path, source) pairs for shape-primitive baseline fitting.
        # Only collected when shape_primitives is non-empty AND n_clusters > 1
        # (clusters are required for per-cluster baselines).
        primitive_files: list[tuple[Path, str]] = []

        for path in repo_corpus_files:
            try:
                src = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            if adapter is not None and adapter.is_data_dominant(src):
                skipped += 1
                continue
            callees = [c for c in extract_callees(src, language) if c is not None]
            for callee in callees:
                attested.add(callee)
            if n_clusters > 1:
                file_bags.append((path, frozenset(callees)))
                if self.shape_primitives:
                    primitive_files.append((path, src))

        self.attested: frozenset[str] = frozenset(attested)
        self.attested_roots: frozenset[str] = frozenset(c.split(".", 1)[0] for c in self.attested)
        self.n_skipped_data_dominant: int = skipped

        self.file_to_cluster: dict[Path, int] = {}
        self.cluster_attested: dict[int, frozenset[str]] = {}
        self.cluster_callee_counts: dict[int, dict[str, int]] = {}
        self.cluster_sizes: dict[int, int] = {}
        # Counts how many times the cluster-rare branch fires in
        # weighted_contribution_for_file. Observable after calibration and
        # after fixture scoring to distinguish plumbing bugs from masking.
        # Callee-level: each rare callee in each hunk increments by 1.
        self.rare_branch_fire_count: int = 0
        # Counts how many distinct hunks fired the cluster-rare branch at
        # least once. Per-hunk fire rate is robust to "many fires per hunk
        # vs few fires per hunk" — used by build_scorer's auto-detect.
        self.rare_branch_hunks_fired: int = 0
        self.hunks_scored: int = 0
        # Per-primitive per-cluster baseline payload, populated only when
        # shape_primitives is non-empty. Outer key: primitive.name. Inner
        # key: cluster_id. Value: primitive-defined baseline payload (4a
        # stores (mean, std), 4c stores a histogram, etc.) or None when
        # the primitive abstains on that cluster (e.g. wrong language).
        self.primitive_baselines: dict[str, dict[int, object]] = {}
        # Per-primitive fire-count, observable from the bench's stderr
        # log to distinguish plumbing bugs from "primitive doesn't fire
        # because data shape doesn't match".
        self.primitive_fire_count: dict[str, int] = {p.name: 0 for p in self.shape_primitives}

        if n_clusters > 1 and file_bags:
            (
                self.file_to_cluster,
                self.cluster_attested,
                self.cluster_callee_counts,
                self.cluster_sizes,
            ) = _build_clusters(file_bags, n_clusters, cluster_seed)

        # Fit per-cluster baselines for each shape primitive after
        # cluster assignments are known. Each primitive sees only the
        # files in its target cluster, so it can compute a baseline
        # statistic across that cluster's distribution.
        if self.shape_primitives and self.file_to_cluster:
            cluster_files: dict[int, list[tuple[Path, str]]] = {}
            for path, src in primitive_files:
                cid = self.file_to_cluster.get(path)
                if cid is None:
                    continue
                cluster_files.setdefault(cid, []).append((path, src))
            for primitive in self.shape_primitives:
                per_cluster: dict[int, object] = {}
                for cid, files_in in cluster_files.items():
                    baseline = primitive.fit_cluster_baseline(files_in, language)
                    if baseline is not None:
                        per_cluster[cid] = baseline
                self.primitive_baselines[primitive.name] = per_cluster

    def _get_distinct_unattested(self, hunk_content: str) -> list[str]:
        if _has_root_error(hunk_content, self._language):
            return []
        callees = extract_callees(hunk_content, self._language)
        seen: set[str] = set()
        deduped: list[str] = []
        for c in callees:
            if c is not None and c not in self.attested and c not in seen:
                seen.add(c)
                deduped.append(c)
        return deduped

    def distinct_unattested(self, hunk_content: str) -> list[str]:
        """Public façade over :meth:`_get_distinct_unattested`.

        Lets the per-reason evidence collector ask for the same unattested
        list the scoring path computed without reaching into a private name
        — keeps the cross-module surface explicit for the linter
        (``noqa: SLF001`` would otherwise pile up at every collector site).
        """
        return self._get_distinct_unattested(hunk_content)

    def cluster_id_for_hunk_file(
        self, file_path: Path | None, file_source: str | None
    ) -> int | None:
        """Resolve the cluster id used by ``weighted_contribution_for_file``.

        Mirrors the lookup logic in :meth:`weighted_contribution_for_file`
        so the call-receiver evidence collector can scope its
        ``common here:`` and rarity samples to the same cluster the scorer
        used. Returns ``None`` when neither the static path nor the Jaccard
        fallback can name a cluster — the collector then falls back to a
        repo-wide framing rather than printing a wrong cluster's data.
        """
        if file_path is not None and file_path in self.file_to_cluster:
            return self.file_to_cluster[file_path]
        if file_source is not None and self.cluster_attested:
            return self._nearest_cluster_for_source(file_source)
        return None

    def count_unattested(self, hunk_content: str) -> int:
        """Return count of distinct unattested callees in *hunk_content*.

        Returns 0 if the hunk has root-level ERROR nodes (parse fragment).
        """
        return len(self._get_distinct_unattested(hunk_content))

    def weighted_contribution(
        self,
        hunk_content: str,
        *,
        alpha: float = 2.0,
        root_bonus: float = 2.0,
        cap: float = 5.0,
    ) -> float:
        """Return weighted penalty for unattested callees; 0.0 on parse fragment.

        No cluster-conditional logic — use this when file_path is unavailable.
        For cluster-conditional scoring, use weighted_contribution_for_file().
        """
        if _has_root_error(hunk_content, self._language):
            return 0.0
        callees = extract_callees(hunk_content, self._language)
        weights: list[float] = []
        seen: set[str] = set()
        for c in callees:
            if c is None or c in seen:
                continue
            seen.add(c)
            if c in self.attested:
                continue
            root = c.split(".", 1)[0]
            if root in self.attested_roots:
                weights.append(alpha + root_bonus)
            else:
                weights.append(alpha)
        return min(sum(weights), cap)

    def weighted_contribution_for_file(
        self,
        hunk_content: str,
        file_path: Path,
        *,
        alpha: float = 2.0,
        root_bonus: float = 2.0,
        cluster_bonus: float = 0.0,
        cap: float = 5.0,
        file_source: str | None = None,
    ) -> float:
        """Return weighted penalty with optional cluster-conditional bonus.

        Identical to weighted_contribution() when n_clusters=1 (no clusters built,
        cluster_bonus has no effect). When n_clusters > 1 and cluster_bonus > 0:
        for each globally-attested callee absent from file_path's cluster attested
        set, adds cluster_bonus as an additional penalty term.

        Scoring per callee c (distinct, after dedup):
          - c not in attested AND root in attested_roots → alpha + root_bonus
          - c not in attested                            → alpha
          - c in attested AND absent from cluster set   → cluster_bonus
          - c in attested AND in cluster but ≤ rare-threshold files → cluster_bonus
          - c in attested AND present in cluster set    → 0
        Returns min(sum(weights), cap).

        Cluster lookup:
          - Static path: ``file_path`` is in ``self.file_to_cluster`` → use that cluster.
          - Fallback path: ``file_path`` is unknown AND ``file_source`` is provided
            AND ``self.cluster_attested`` is non-empty → compute the file's callee
            bag and assign it to the cluster whose attested set has the highest
            Jaccard similarity (ties broken by smallest cluster id). When the
            file's bag is empty, no cluster is assigned.
          - When neither path resolves, ``cluster_set`` is ``None`` and
            ``cluster_bonus`` does not fire.

        The fallback assignment is computed on the fly and NOT cached on the
        scorer, so repeated calls with the same arguments are deterministic but
        do not mutate the static cluster maps.
        """
        self.hunks_scored += 1
        if _has_root_error(hunk_content, self._language):
            return 0.0
        callees = extract_callees(hunk_content, self._language)
        weights: list[float] = []
        seen: set[str] = set()
        _hunk_fired_rare = False

        if self.force_jaccard_routing:
            # ML-feature mode: always use Jaccard fallback path so catalog
            # fixtures and real-PR controls take the same code path.
            if file_source is not None and self.cluster_attested:
                cluster_id = self._nearest_cluster_for_source(file_source)
            else:
                cluster_id = None
        else:
            cluster_id = self.file_to_cluster.get(file_path)
            if cluster_id is None and file_source is not None and self.cluster_attested:
                cluster_id = self._nearest_cluster_for_source(file_source)
        cluster_set = self.cluster_attested.get(cluster_id) if cluster_id is not None else None
        cluster_counts = (
            self.cluster_callee_counts.get(cluster_id) if cluster_id is not None else None
        )

        for c in callees:
            if c is None or c in seen:
                continue
            seen.add(c)
            if c not in self.attested:
                root = c.split(".", 1)[0]
                if root in self.attested_roots:
                    weights.append(alpha + root_bonus)
                else:
                    weights.append(alpha)
            elif cluster_set is not None and c not in cluster_set:
                # Cluster-absent attested callee.
                weights.append(cluster_bonus)
            elif (
                self.cluster_rare_threshold > 0
                and cluster_id is not None
                and cluster_counts is not None
                and cluster_counts.get(c, 0) <= self.cluster_rare_threshold
                and self.cluster_sizes.get(cluster_id, 0) >= self.cluster_size_min
            ):
                # Cluster-rare attested callee: present in ≤ threshold
                # cluster files within a cluster of size ≥ cluster_size_min.
                # Treated as effectively cluster-absent.
                self.rare_branch_fire_count += 1
                _hunk_fired_rare = True
                weights.append(cluster_bonus)

        # Shape-primitive dispatch. Each primitive contributes
        # an additive scalar; the final clip at ``cap`` continues to
        # bound the total contribution to cluster_bonus. Empty
        # primitive list (default) is a true no-op.
        if self.shape_primitives and cluster_id is not None:
            cluster_size = self.cluster_sizes.get(cluster_id, 0)
            for primitive in self.shape_primitives:
                baseline = self.primitive_baselines.get(primitive.name, {}).get(cluster_id)
                contribution = primitive.score(
                    hunk_content,
                    baseline=baseline,
                    cluster_size=cluster_size,
                )
                if contribution > 0.0:
                    self.primitive_fire_count[primitive.name] += 1
                    weights.append(contribution)

        if _hunk_fired_rare:
            self.rare_branch_hunks_fired += 1
        return min(sum(weights), cap)

    def _nearest_cluster_for_source(self, file_source: str) -> int | None:
        """Return the cluster id whose attested set is closest (Jaccard) to
        the callee bag of *file_source*.

        Returns None when the file's callee bag is empty (no signal to match).
        Ties on Jaccard are broken by smallest cluster id (deterministic).
        """
        result = self.nearest_cluster_for_source(file_source)
        return None if result is None else result[0]

    def nearest_cluster_for_source(self, file_source: str) -> tuple[int, float] | None:
        """Public Jaccard-nearest cluster lookup for an arbitrary file source.

        Returns ``(cluster_id, jaccard_to_centroid)`` where ``jaccard_to_centroid``
        is the Jaccard similarity between the file's callee bag and the chosen
        cluster's attested set. Returns ``None`` when no clusters were built
        (n_clusters=1) or the file's callee bag is empty.

        Used by the ML feature extractor to compute uniform
        ``cluster_id`` and ``cluster_jaccard_to_centroid`` features for every
        hunk regardless of whether the file is in ``file_to_cluster``.

        Ties on Jaccard are broken by smallest cluster id (deterministic).
        """
        if not self.cluster_attested:
            return None
        bag: frozenset[str] = frozenset(
            c for c in extract_callees(file_source, self._language) if c is not None
        )
        if not bag:
            return None

        best_cid: int | None = None
        best_jaccard: float = -1.0
        for cid in sorted(self.cluster_attested.keys()):
            attested = self.cluster_attested[cid]
            union = bag | attested
            jaccard = 0.0 if not union else len(bag & attested) / len(union)
            if jaccard > best_jaccard:
                best_jaccard = jaccard
                best_cid = cid
        if best_cid is None:
            return None
        return best_cid, best_jaccard
