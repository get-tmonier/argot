"""
Paradigm break (validation): manual jsonschema validation of pipeline run configs
substituting Dagster's ConfigSchema / @config_mapping pattern.

Dagster validates run configuration through its ConfigSchema system: config is
declared with dagster.Field, dagster.Shape, and dagster.Array; invalid configs are
caught before execution with a typed DagsterInvalidConfigError.  This file instead
builds JSON Schema dicts and validates them with jsonschema.validate(), raising
ValueError on failure — a pattern common in general-purpose Python services but absent
from Dagster's corpus.  Key absent identifiers: dagster.Field, dagster.Shape,
ConfigSchema, @config_mapping, DagsterInvalidConfigError — none of which appear
in the Dagster corpus.
"""

from __future__ import annotations

import logging
from typing import Any

import jsonschema
from jsonschema import ValidationError

logger = logging.getLogger(__name__)

_MATERIALIZATION_SCHEMA: dict[str, Any] = {
    "type": "object",
    "required": ["asset_key", "partition_key", "run_id"],
    "properties": {
        "asset_key": {"type": "string", "minLength": 1},
        "partition_key": {"type": "string"},
        "run_id": {"type": "string", "pattern": "^[a-f0-9-]{36}$"},
        "metadata": {
            "type": "object",
            "additionalProperties": {"type": ["string", "number", "boolean"]},
        },
        "tags": {
            "type": "array",
            "items": {"type": "string"},
        },
    },
    "additionalProperties": False,
}


def validate_materialization_config(config: dict[str, Any]) -> dict[str, Any]:
    try:
        jsonschema.validate(config, _MATERIALIZATION_SCHEMA)
    except ValidationError as exc:
        raise ValueError(f"invalid materialization config: {exc.message}") from exc
    return config


def validate_batch(configs: list[dict[str, Any]]) -> list[dict[str, Any]]:
    errors: list[str] = []
    valid: list[dict[str, Any]] = []
    for i, cfg in enumerate(configs):
        try:
            validate_materialization_config(cfg)
            valid.append(cfg)
        except ValueError as exc:
            errors.append(f"[{i}] {exc}")
    if errors:
        logger.warning("skipped %d invalid configs:\n%s", len(errors), "\n".join(errors))
    return valid


def build_materialization_config(
    asset_key: str,
    partition_key: str,
    run_id: str,
    metadata: dict[str, Any] | None = None,
    tags: list[str] | None = None,
) -> dict[str, Any]:
    raw: dict[str, Any] = {
        "asset_key": asset_key,
        "partition_key": partition_key,
        "run_id": run_id,
    }
    if metadata is not None:
        raw["metadata"] = metadata
    if tags is not None:
        raw["tags"] = tags
    return validate_materialization_config(raw)
