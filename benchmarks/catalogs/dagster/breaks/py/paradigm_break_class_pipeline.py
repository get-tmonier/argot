"""
Paradigm break (asset_definition): class-based pipeline holding intermediate data
as instance attributes, substituting Dagster's @asset decorator pattern.

Dagster models data as independent @asset-decorated functions with pure return values;
the framework manages execution order and data passing automatically via the asset graph.
This file instead uses a Pipeline class that stores intermediate data as self.* attributes
and calls extraction, transformation, and loading steps as regular methods in a run()
orchestrator.  Key absent identifiers: @asset, AssetIn, Output, Definitions,
build_asset_context, MaterializeResult — none of which appear in the Dagster corpus.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from typing import Any

import sqlalchemy as sa

logger = logging.getLogger(__name__)


@dataclass
class InventoryPipeline:
    db_url: str
    warehouse_url: str
    raw_rows: list[dict[str, Any]] = field(default_factory=list)
    clean_rows: list[dict[str, Any]] = field(default_factory=list)
    _engine: sa.Engine | None = field(default=None, init=False, repr=False)
    _wh_engine: sa.Engine | None = field(default=None, init=False, repr=False)

    def connect(self) -> None:
        self._engine = sa.create_engine(self.db_url)
        self._wh_engine = sa.create_engine(self.warehouse_url)
        logger.info("connected to source and warehouse")

    def extract(self) -> None:
        assert self._engine is not None, "call connect() first"
        with self._engine.connect() as conn:
            result = conn.execute(sa.text("SELECT * FROM inventory"))
            self.raw_rows = [dict(row._mapping) for row in result]
        logger.info("extracted %d rows", len(self.raw_rows))

    def transform(self) -> None:
        self.clean_rows = [
            {**row, "quantity": max(0, int(row["quantity"]))}
            for row in self.raw_rows
            if row.get("sku") and row.get("quantity") is not None
        ]
        logger.info("kept %d/%d rows after transform", len(self.clean_rows), len(self.raw_rows))

    def load(self) -> None:
        assert self._wh_engine is not None, "call connect() first"
        with self._wh_engine.begin() as conn:
            conn.execute(sa.text("TRUNCATE TABLE inventory_clean"))
            if self.clean_rows:
                conn.execute(
                    sa.text("INSERT INTO inventory_clean VALUES (:sku, :quantity)"),
                    self.clean_rows,
                )
        logger.info("loaded %d rows", len(self.clean_rows))

    def run(self) -> None:
        self.connect()
        self.extract()
        self.transform()
        self.load()


if __name__ == "__main__":
    pipeline = InventoryPipeline(
        db_url="postgresql://localhost/inventory",
        warehouse_url="postgresql://localhost/warehouse",
    )
    pipeline.run()
