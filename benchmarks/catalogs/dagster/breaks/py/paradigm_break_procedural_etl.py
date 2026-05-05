"""
Paradigm break (asset_definition): procedural ETL functions that accumulate state
into a module-level dictionary, substituting Dagster's @asset decorator pattern.

Dagster models data assets as @asset-decorated functions whose return values are
persisted by an IOManager; upstream dependencies are declared as typed function
parameters resolved by the asset graph.  This file instead uses plain module-level
functions that mutate a shared _state dict, call each step explicitly inside a
run_pipeline() orchestrator, and log lineage metadata manually.  Key absent
identifiers: @asset, AssetIn, Output, IOManager, Definitions, MaterializeResult
— none of which appear in the Dagster corpus.
"""

from __future__ import annotations

import hashlib
import json
import logging
import urllib.request
from datetime import UTC, datetime
from typing import Any

logger = logging.getLogger(__name__)

_state: dict[str, Any] = {}


def extract_product_catalog(source_url: str) -> list[dict[str, Any]]:
    with urllib.request.urlopen(source_url) as resp:
        data: list[dict[str, Any]] = json.load(resp)
    logger.info("extracted %d products", len(data))
    _state["raw_catalog"] = data
    return data


def transform_product_catalog(raw: list[dict[str, Any]]) -> list[dict[str, Any]]:
    clean = [
        {
            "sku": p["sku"].strip().upper(),
            "name": p["name"].strip(),
            "price_cents": int(float(p["price"]) * 100),
        }
        for p in raw
        if p.get("sku") and p.get("name") and p.get("price")
    ]
    _state["clean_catalog"] = clean
    return clean


def load_product_catalog(clean: list[dict[str, Any]], table: str) -> dict[str, Any]:
    checksum = hashlib.md5(str(clean).encode()).hexdigest()
    metadata: dict[str, Any] = {
        "rows": len(clean),
        "table": table,
        "checksum": checksum,
        "loaded_at": datetime.now(UTC).isoformat(),
    }
    _state["load_metadata"] = metadata
    logger.info("loaded %d rows into %s (checksum=%s)", len(clean), table, checksum)
    return metadata


def run_product_catalog_pipeline(
    source_url: str, table: str = "product_catalog"
) -> dict[str, Any]:
    raw = extract_product_catalog(source_url)
    clean = transform_product_catalog(raw)
    return load_product_catalog(clean, table)


if __name__ == "__main__":
    run_product_catalog_pipeline("https://api.example.com/products")
