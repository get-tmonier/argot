"""
Paradigm break (framework_swap): Luigi class-based task pipeline substituting
Dagster's @asset / @op decorator pattern.

Dagster models data as @asset-decorated functions whose dependencies are inferred
from parameter names or declared via deps=[]; the asset graph is registered with
Definitions().  Luigi instead uses class-based Task objects that override requires(),
output(), and run(); upstream dependencies are expressed by returning other Task
instances from requires() and intermediate data is read via self.input().open().
Targets are luigi.LocalTarget or luigi.contrib.s3.S3Target objects returned from
output().  Key absent identifiers: luigi.Task, luigi.LocalTarget, luigi.Parameter,
self.input(), self.output(), luigi.build — none of which appear in the Dagster corpus.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import luigi


class FetchOrderData(luigi.Task):
    run_date = luigi.DateParameter()
    source_bucket = luigi.Parameter(default="s3://orders-raw")

    def output(self) -> luigi.LocalTarget:
        return luigi.LocalTarget(f"/tmp/orders_{self.run_date}.json")

    def run(self) -> None:
        rows: list[dict[str, Any]] = _download_orders(str(self.run_date), str(self.source_bucket))
        with self.output().open("w") as f:
            json.dump(rows, f)


class TransformOrders(luigi.Task):
    run_date = luigi.DateParameter()

    def requires(self) -> FetchOrderData:
        return FetchOrderData(run_date=self.run_date)

    def output(self) -> luigi.LocalTarget:
        return luigi.LocalTarget(f"/tmp/orders_clean_{self.run_date}.json")

    def run(self) -> None:
        with self.input().open("r") as f:
            raw: list[dict[str, Any]] = json.load(f)
        clean = [o for o in raw if o.get("status") != "cancelled"]
        with self.output().open("w") as f:
            json.dump(clean, f)


class LoadOrders(luigi.Task):
    run_date = luigi.DateParameter()

    def requires(self) -> TransformOrders:
        return TransformOrders(run_date=self.run_date)

    def output(self) -> luigi.LocalTarget:
        return luigi.LocalTarget(f"/tmp/orders_loaded_{self.run_date}.flag")

    def run(self) -> None:
        with self.input().open("r") as f:
            orders: list[dict[str, Any]] = json.load(f)
        _write_to_warehouse(orders)
        Path(self.output().path).touch()


def _download_orders(run_date: str, bucket: str) -> list[dict[str, Any]]:
    return []


def _write_to_warehouse(orders: list[dict[str, Any]]) -> None:
    pass


if __name__ == "__main__":
    luigi.build([LoadOrders(run_date="2024-01-15")], local_scheduler=True)
