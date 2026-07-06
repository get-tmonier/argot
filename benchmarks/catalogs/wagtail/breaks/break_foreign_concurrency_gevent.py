# Break: gevent greenlet pool parallelises rendition generation
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.images.models import Filter


# Decoy — idiomatic wagtail synchronous rendition generation, NOT in the hunk
def render_one(image, spec: str):
    return image.get_rendition(Filter(spec=spec))


# hunk starts here
import gevent
from gevent.pool import Pool


def render_many(image, specs: list[str]) -> list:
    pool = Pool(size=8)
    greenlets = [pool.spawn(image.get_rendition, Filter(spec=s)) for s in specs]
    pool.join()
    return [g.value for g in greenlets]


def warm_renditions(images, spec: str) -> None:
    jobs = [gevent.spawn(img.get_rendition, Filter(spec=spec)) for img in images]
    gevent.joinall(jobs, timeout=30)
# hunk ends here
