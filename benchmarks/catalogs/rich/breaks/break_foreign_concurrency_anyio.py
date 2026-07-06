# Break: anyio task group runs cell measurement concurrently (foreign async runtime)
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic synchronous cell width, NOT inside the hunk range
def total_width(widths: list[int]) -> int:
    return sum(widths)


# hunk starts here
import anyio


async def measure_all(texts: list[str]) -> list[int]:
    results: list[int] = []
    async with anyio.create_task_group() as tg:
        for text in texts:
            tg.start_soon(_measure_one, text, results)
    return results


async def _measure_one(text: str, sink: list[int]) -> None:
    await anyio.sleep(0)
    sink.append(len(text))
# hunk ends here
