# Break: pydantic BaseModel validates API payloads instead of DRF serializers
"""Break fixture — not for import."""
from __future__ import annotations

from rest_framework import serializers


# Decoy — idiomatic wagtail API DRF serializer, NOT inside the hunk range
class PageSummarySerializer(serializers.Serializer):
    id = serializers.IntegerField()
    title = serializers.CharField()


# hunk starts here
import pydantic
from pydantic import BaseModel, Field


class PageCreatePayload(BaseModel):
    title: str = Field(min_length=1, max_length=255)
    slug: str = Field(pattern=r"^[a-z0-9-]+$")
    live: bool = True
    seo_title: str | None = None

    @pydantic.field_validator("slug")
    @classmethod
    def _no_trailing_dash(cls, value: str) -> str:
        if value.endswith("-"):
            raise ValueError("slug must not end with a dash")
        return value


def parse_page_payload(raw: dict) -> PageCreatePayload:
    return PageCreatePayload.model_validate(raw)
# hunk ends here
