"""
Paradigm break (validation): Pydantic v2 BaseModel classes for pipeline event
validation, substituting Dagster's ConfigSchema / EventMetadata pattern.

Dagster expresses config and metadata schemas via dagster.Config (a Pydantic-based
superclass) or dagster.Field / dagster.Shape for legacy config; events carry typed
metadata via MetadataValue objects stored in the event log.  This file declares
standalone Pydantic v2 BaseModel classes and validates pipeline events independently
of Dagster's type system, using model_validate() / model_dump() and Pydantic field
validators.  Key absent identifiers: dagster.Config, dagster.Field, MetadataValue,
AssetMaterialization.metadata, DagsterType — none of which appear in the Dagster corpus.
"""

from __future__ import annotations

import logging
from datetime import UTC, datetime
from enum import Enum
from typing import Annotated, Any

from pydantic import BaseModel, Field, field_validator, model_validator

logger = logging.getLogger(__name__)


class AssetEventType(str, Enum):
    MATERIALIZATION = "materialization"
    OBSERVATION = "observation"
    FAILURE = "failure"


class AssetEventMetadata(BaseModel):
    rows_affected: int | None = None
    bytes_written: int | None = None
    source_hash: str | None = Field(default=None, pattern=r"^[a-f0-9]{64}$")
    custom: dict[str, Any] = Field(default_factory=dict)


class AssetEvent(BaseModel):
    asset_key: Annotated[str, Field(min_length=1, max_length=256)]
    event_type: AssetEventType
    partition_key: str | None = None
    run_id: Annotated[str, Field(pattern=r"^[a-f0-9\-]{36}$")]
    occurred_at: datetime = Field(default_factory=lambda: datetime.now(UTC))
    metadata: AssetEventMetadata = Field(default_factory=AssetEventMetadata)

    @field_validator("asset_key")
    @classmethod
    def asset_key_no_spaces(cls, v: str) -> str:
        if " " in v:
            raise ValueError("asset_key must not contain spaces")
        return v.lower()

    @model_validator(mode="after")
    def failure_requires_no_rows(self) -> AssetEvent:
        if self.event_type == AssetEventType.FAILURE and self.metadata.rows_affected is not None:
            raise ValueError("failure events must not report rows_affected")
        return self


def record_event(raw: dict[str, Any]) -> AssetEvent:
    event = AssetEvent.model_validate(raw)
    logger.info("recorded %s event for %s", event.event_type, event.asset_key)
    return event


def dump_events(events: list[AssetEvent]) -> list[dict[str, Any]]:
    return [e.model_dump(mode="json") for e in events]
