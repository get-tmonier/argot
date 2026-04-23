"""Memory regression test for compute_features.

Parses 5000 hunks in a tight loop and asserts process RSS stays under 500 MB.
Guards against the per-call TsParser instantiation pattern that caused ~30 GB
growth on full bench runs (~150k+ hunks across 6 corpora).
"""

from __future__ import annotations

import resource

_NORMAL_HUNK = """\
def parse(request, registry):
    handlers = registry.lookup(request.path)
    if not handlers:
        raise KeyError(request.path)
    for h in handlers:
        if h.matches(request):
            return h.handle(request)
    return None
"""

_DATA_HUNK = "EMOJI = {\n" + "".join(f'    "emoji_{i}": "U+{i:05X}",\n' for i in range(80)) + "}\n"

_RSS_LIMIT_MB = 500


def _rss_mb() -> float:
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / (1024 * 1024)


def test_compute_features_does_not_leak_memory_over_5k_calls() -> None:
    from argot_bench.typicality import compute_features

    # Warm up — first call may allocate caches.
    compute_features(_NORMAL_HUNK, "python")
    compute_features(_DATA_HUNK, "python")

    rss_before = _rss_mb()

    for i in range(5000):
        src = _NORMAL_HUNK if i % 2 == 0 else _DATA_HUNK
        compute_features(src, "python")

    rss_after = _rss_mb()
    growth_mb = rss_after - rss_before

    assert growth_mb < _RSS_LIMIT_MB, (
        f"RSS grew by {growth_mb:.1f} MB over 5000 compute_features calls "
        f"(limit {_RSS_LIMIT_MB} MB). Per-call TsParser allocation likely reintroduced."
    )
