# Break: gevent greenlet Pool fan-out replacing Dagster's StepDelegatingExecutor step handlers.
"""Greenlet-based concurrent step dispatch via gevent instead of Dagster's executor.

Dagster's StepDelegatingExecutor hands each ready step to a StepHandler and polls
for completion; this maps the same steps onto gevent greenlets through
gevent.pool.Pool and pool.spawn, joining with pool.join(). gevent, its Pool, and
spawn/sleep primitives never appear in the Dagster corpus.
"""
import gevent
from gevent.pool import Pool


def dispatch_step(step_key: str) -> dict:
    gevent.sleep(0.05)
    return {"step": step_key, "status": "success"}


def run_active_steps(step_keys: list[str], max_concurrent: int = 8) -> list[dict]:
    pool = Pool(max_concurrent)
    greenlets = [pool.spawn(dispatch_step, key) for key in step_keys]
    pool.join()
    return [greenlet.value for greenlet in greenlets]
