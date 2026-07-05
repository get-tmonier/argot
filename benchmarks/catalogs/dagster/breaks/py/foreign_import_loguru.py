# Break: loguru logger.add() sink config instead of Dagster's context.log / get_dagster_logger().
"""Structured logging via loguru inside op execution.

Dagster ops log through context.log (a DagsterLogManager) or
dagster.get_dagster_logger(); loguru configures a process-global logger with
logger.add(sink, ...), logger.remove(), and logger.bind(...) context. loguru,
logger.add, and logger.bind never appear in the Dagster corpus.
"""
import sys

from loguru import logger

logger.remove()
logger.add(sys.stderr, level="INFO", backtrace=True, diagnose=True)
logger.add("dagster_run_{time}.log", rotation="100 MB", retention="7 days")


def log_asset_materialization(asset_key: str, num_rows: int) -> None:
    bound = logger.bind(asset_key=asset_key)
    bound.info("Materialized {asset} with {n} rows", asset=asset_key, n=num_rows)
    bound.success("Asset {asset} committed to storage", asset=asset_key)
