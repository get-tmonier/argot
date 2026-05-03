"""Call-receiver scorer — Stage 1.5 (production port of era-6 research scorer).

Presence-based scorer: tracks distinct call-expression callees in the
model-A corpus and counts unattested callees in a hunk.  Used by
SequentialImportBpeScorer to apply a soft additive BPE penalty:

    adjusted_bpe = raw_bpe + alpha * min(count_unattested(hunk), cap)

Era-11 adds cluster-conditional attestation: when n_clusters > 1, files are
grouped by callee-bag similarity (MinHash + KMeans). Callees globally attested
but absent from the hunk-file's cluster's attested set contribute cluster_bonus
as an additive penalty on top of all era-10 logic.

Reuses module-level parsers from filters.typicality to avoid the linear
memory growth that occurs when TsParser is instantiated per-hunk.
"""

from __future__ import annotations

import hashlib
from collections.abc import Iterator
from pathlib import Path
from typing import Literal, Protocol

from tree_sitter import Node

from argot.scoring.filters.typicality import _PY_PARSER, _TS_PARSER

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
) -> tuple[dict[Path, int], dict[int, frozenset[str]]]:
    """Cluster files by callee-bag similarity using MinHash signatures + KMeans.

    Returns:
        file_to_cluster: maps resolved file path → cluster id (0-indexed).
        cluster_attested: maps cluster id → union of all callees in cluster files.
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
    for cid in range(effective_k):
        union: set[str] = set()
        for i, bag in enumerate(bags):
            if labels[i] == cid:
                union.update(bag)
        cluster_attested[cid] = frozenset(union)

    return file_to_cluster, cluster_attested


class CallReceiverScorer:
    """Stage-1.5 call-receiver scorer.

    Fit: scan *model_a_files*, union all non-None callees into a frozenset.
    Score: count distinct unattested callees in a hunk (0 if parse fragment).
    Used by SequentialImportBpeScorer to compute adjusted_bpe.

    Era-11: when n_clusters > 1, files are clustered by callee-bag similarity.
    Use weighted_contribution_for_file() to apply the additive cluster_bonus for
    globally-attested callees absent from the hunk-file's cluster attested set.
    """

    def __init__(
        self,
        model_a_files: list[Path],
        *,
        language: Language,
        alpha: float = 1.0,
        cap: int = 5,
        adapter: _DataDominantAdapter | None = None,
        n_clusters: int = 1,
        cluster_seed: int = 0,
    ) -> None:
        if not model_a_files:
            raise ValueError("model_a_files must be non-empty")
        self._language: Language = language
        self.alpha: float = alpha
        self.cap: int = cap

        attested: set[str] = set()
        skipped: int = 0
        # (path, callee_bag) pairs for cluster building — collected iff n_clusters > 1
        file_bags: list[tuple[Path, frozenset[str]]] = []

        for path in model_a_files:
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

        self.attested: frozenset[str] = frozenset(attested)
        self.attested_roots: frozenset[str] = frozenset(c.split(".", 1)[0] for c in self.attested)
        self.n_skipped_data_dominant: int = skipped

        self.file_to_cluster: dict[Path, int] = {}
        self.cluster_attested: dict[int, frozenset[str]] = {}

        if n_clusters > 1 and file_bags:
            self.file_to_cluster, self.cluster_attested = _build_clusters(
                file_bags, n_clusters, cluster_seed
            )

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

        Era-10 behavior: no cluster-conditional logic. Use this when file_path
        is unavailable. For cluster-conditional scoring, use
        weighted_contribution_for_file().
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
          - c in attested AND absent from cluster set   → cluster_bonus  (era-11 NEW)
          - c in attested AND present in cluster set    → 0
        Returns min(sum(weights), cap).

        Cluster lookup (era-11 Phase 1 fix):
          - Static path: ``file_path`` is in ``self.file_to_cluster`` → use that cluster.
          - Fallback path: ``file_path`` is unknown AND ``file_source`` is provided
            AND ``self.cluster_attested`` is non-empty → compute the file's callee
            bag and assign it to the cluster whose attested set has the highest
            Jaccard similarity (ties broken by smallest cluster id). When the
            file's bag is empty, no cluster is assigned (era-10 behavior).
          - When neither path resolves, ``cluster_set`` is ``None`` and
            ``cluster_bonus`` does not fire (era-10 behavior preserved).

        The fallback assignment is computed on the fly and NOT cached on the
        scorer, so repeated calls with the same arguments are deterministic but
        do not mutate the static cluster maps.
        """
        if _has_root_error(hunk_content, self._language):
            return 0.0
        callees = extract_callees(hunk_content, self._language)
        weights: list[float] = []
        seen: set[str] = set()

        cluster_id = self.file_to_cluster.get(file_path)
        if cluster_id is None and file_source is not None and self.cluster_attested:
            cluster_id = self._nearest_cluster_for_source(file_source)
        cluster_set = self.cluster_attested.get(cluster_id) if cluster_id is not None else None

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
                weights.append(cluster_bonus)

        return min(sum(weights), cap)

    def _nearest_cluster_for_source(self, file_source: str) -> int | None:
        """Return the cluster id whose attested set is closest (Jaccard) to
        the callee bag of *file_source*.

        Returns None when the file's callee bag is empty (no signal to match).
        Ties on Jaccard are broken by smallest cluster id (deterministic).
        """
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
        return best_cid
