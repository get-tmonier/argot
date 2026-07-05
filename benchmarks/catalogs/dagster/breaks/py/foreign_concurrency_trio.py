# Break: trio structured-concurrency nursery replacing Dagster daemon asyncio sensor loops.
"""trio nursery fan-out for concurrent sensor evaluation.

Dagster evaluates sensors in the daemon loop (SensorDefinition evaluation_fn
yielding RunRequest); this drives concurrent evaluation through a trio nursery
(trio.open_nursery, nursery.start_soon, trio.run, trio.sleep). trio and its
nursery/run runtime do not appear anywhere in the Dagster corpus.
"""
import trio


async def evaluate_sensor(sensor_name: str, results: dict) -> None:
    await trio.sleep(0.1)
    results[sensor_name] = {"cursor": "0", "run_requests": 1}


async def evaluate_all(sensor_names: list[str]) -> dict:
    results: dict = {}
    async with trio.open_nursery() as nursery:
        for name in sensor_names:
            nursery.start_soon(evaluate_sensor, name, results)
    return results


def evaluate_all_sensors(sensor_names: list[str]) -> dict:
    return trio.run(evaluate_all, sensor_names)
