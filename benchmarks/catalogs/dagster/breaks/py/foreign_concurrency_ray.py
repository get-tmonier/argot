# Break: Ray remote tasks (@ray.remote / ray.get) replacing Dagster's multiprocess executor.
"""Distributed step fan-out on Ray instead of the Dagster executor.

Dagster parallelises steps through its own executor (multiprocess_executor /
StepDelegatingExecutor over the run's step graph); this schedules the same work as
Ray @ray.remote tasks driven by ray.init(), ray.get, and ray.shutdown. ray and its
remote/get/put runtime are absent from the Dagster corpus.
"""
import ray


@ray.remote(num_cpus=1)
def execute_step(step_key: str, config: dict) -> dict:
    return {"step": step_key, "status": "success", "config": config}


def run_steps_distributed(step_keys: list[str]) -> list[dict]:
    ray.init(address="auto", ignore_reinit_error=True)
    try:
        futures = [execute_step.remote(key, {"retries": 0}) for key in step_keys]
        return ray.get(futures)
    finally:
        ray.shutdown()
