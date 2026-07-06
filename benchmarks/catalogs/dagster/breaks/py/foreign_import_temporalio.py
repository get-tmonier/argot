# Break: Temporal Python SDK (@workflow.defn / @activity.defn) replacing Dagster's @op / @job orchestration.
"""Temporal workflow definition standing in for a Dagster @job.

Dagster wires computation as @op-decorated functions inside a @job and executes
through the Dagster instance; Temporal expresses the same durable pipeline as a
@workflow.defn class whose steps are @activity.defn coroutines dispatched via
workflow.execute_activity and connected through temporalio.client.Client.connect().
None of temporalio, @workflow.defn, @activity.defn, or execute_activity appear in
the Dagster corpus.
"""
import asyncio
from datetime import timedelta

from temporalio import activity, workflow
from temporalio.client import Client


@activity.defn
async def extract_partition(partition_key: str) -> list[dict]:
    return [{"partition": partition_key, "row": i} for i in range(100)]


@workflow.defn
class MaterializeAssetWorkflow:
    @workflow.run
    async def run(self, partition_key: str) -> int:
        rows = await workflow.execute_activity(
            extract_partition,
            partition_key,
            start_to_close_timeout=timedelta(minutes=5),
        )
        return len(rows)


async def launch_materialization(partition_key: str) -> int:
    client = await Client.connect("localhost:7233")
    handle = await client.start_workflow(
        MaterializeAssetWorkflow.run,
        partition_key,
        id=f"materialize-{partition_key}",
        task_queue="dagster-assets",
    )
    return await handle.result()


if __name__ == "__main__":
    asyncio.run(launch_materialization("2024-01-01"))
