# Break: gevent greenlets drive concurrent screen updates (foreign concurrency lib)
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic synchronous screen update, NOT inside the hunk range
def refresh(lines: list[str]) -> str:
    return "\n".join(lines)


# hunk starts here
import gevent
from gevent.pool import Pool


def render_regions(regions: list[str]) -> list[str]:
    pool = Pool(size=8)
    jobs = [pool.spawn(_render_region, region) for region in regions]
    gevent.joinall(jobs)
    return [job.value for job in jobs]


def _render_region(region: str) -> str:
    gevent.sleep(0)
    return region.upper()
# hunk ends here
