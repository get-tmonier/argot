# engine/argot/research/signal/phase14/fixtures/stage2_only/async_adoption.py
"""async def using asyncio.gather / asyncio.to_thread / asyncio.Semaphore — stdlib only.

Pattern: structured async concurrency primitives used together in a utility module,
rather than the FastAPI endpoint-level async def pattern with Depends/router.
Imports: asyncio (stdlib, in FastAPI corpus).
"""
from __future__ import annotations

import asyncio


async def gather_all(coros: list[object]) -> list[object]:
    return list(await asyncio.gather(*coros))  # type: ignore[arg-type]


async def run_in_thread(fn: object, *args: object) -> object:
    return await asyncio.to_thread(fn, *args)  # type: ignore[arg-type]


async def with_timeout(coro: object, timeout: float) -> object:
    try:
        return await asyncio.wait_for(coro, timeout=timeout)  # type: ignore[arg-type]
    except asyncio.TimeoutError:
        return None


async def bounded_gather(items: list[str], limit: int = 4) -> list[str]:
    sem = asyncio.Semaphore(limit)

    async def process(item: str) -> str:
        async with sem:
            await asyncio.sleep(0)
            return item.upper()

    return list(await asyncio.gather(*[process(i) for i in items]))
