# Break: trio (aliased) drives an async crawl; only leaf .run collides with scrapy
"""Break fixture — not for import."""

# hunk starts here
import trio as _trio


def run_spider(main, *args) -> int:
    # trio.run drives the async crawl; .run collides with scrapy's own .run()
    _trio.run(main, *args)
    return 0
# hunk ends here
