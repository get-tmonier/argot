"""
Paradigm break (framework_swap): Prefect @task / @flow pipeline substituting
Dagster's @op / @job / @asset decorator pattern.

Dagster registers computation as @op-decorated functions inside a @job, or as
@asset functions registered with Definitions().  Prefect uses @task-decorated
callables invoked inside a @flow function; dependencies form from ordinary Python
call order rather than explicit graph wiring.  Retry policy is expressed as a
@task(retries=3, retry_delay_seconds=...) parameter rather than a Dagster RetryPolicy
object.  Key absent identifiers: @flow, @task, retries=, retry_delay_seconds=,
task_runner, PrefectFuture — none of which appear in the Dagster corpus.
"""

from __future__ import annotations

from typing import Any

import pandas as pd
from prefect import flow, task
from prefect.tasks import exponential_backoff


@task(retries=3, retry_delay_seconds=exponential_backoff(backoff_factor=10))
def extract_customers(source_url: str) -> pd.DataFrame:
    df = pd.read_csv(source_url)
    return df[["id", "name", "signup_date", "tier"]]


@task(retries=2, retry_delay_seconds=30)
def transform_customers(raw: pd.DataFrame) -> pd.DataFrame:
    df = raw.copy()
    df["signup_date"] = pd.to_datetime(df["signup_date"])
    df["is_premium"] = df["tier"] == "premium"
    return df.dropna(subset=["id", "name"])


@task
def load_customers(clean: pd.DataFrame, target_table: str) -> dict[str, Any]:
    row_count = len(clean)
    clean.to_parquet(f"/tmp/{target_table}.parquet", index=False)
    return {"rows_written": row_count, "table": target_table}


@flow(name="customer-sync", log_prints=True)
def customer_sync_flow(source_url: str, target_table: str = "customers") -> None:
    raw = extract_customers(source_url)
    clean = transform_customers(raw)
    result = load_customers(clean, target_table)
    print(f"Loaded {result['rows_written']} rows into {result['table']}")


if __name__ == "__main__":
    customer_sync_flow(source_url="s3://my-bucket/customers.csv")
