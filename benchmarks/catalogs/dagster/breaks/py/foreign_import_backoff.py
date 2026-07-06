# Break: backoff.on_exception decorator instead of Dagster's RetryPolicy / Backoff.EXPONENTIAL.
"""Retry via the third-party `backoff` package rather than Dagster's RetryPolicy.

Dagster expresses op retries declaratively with RetryPolicy(max_retries=...,
backoff=Backoff.EXPONENTIAL, jitter=Jitter.PLUS_MINUS); this wraps the call in the
`backoff` package's @backoff.on_exception(backoff.expo, ...) decorator with runtime
retry. The `backoff` module, on_exception, expo, and full_jitter are absent from
the Dagster corpus (Dagster's own Backoff is an enum, not this package).
"""
import backoff


@backoff.on_exception(
    backoff.expo,
    ConnectionError,
    max_tries=5,
    max_time=300,
    jitter=backoff.full_jitter,
)
def load_partition_metadata(client, partition_key: str) -> dict:
    return client.read(partition_key)


@backoff.on_exception(backoff.fibo, TimeoutError, max_tries=3)
def commit_materialization(client, asset_key: str, payload: dict) -> None:
    client.write(asset_key, payload)
