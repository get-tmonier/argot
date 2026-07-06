# Break: marshmallow Schema validates API payloads instead of DRF serializers
"""Break fixture — not for import."""
from __future__ import annotations

from rest_framework import serializers

from wagtail.models import Page


# Decoy — idiomatic wagtail DRF serializer, NOT inside the hunk range
class PageSummarySerializer(serializers.ModelSerializer):
    class Meta:
        model = Page
        fields = ["id", "title", "slug"]


# hunk starts here
from marshmallow import Schema, fields, validate


class PagePayloadSchema(Schema):
    title = fields.Str(required=True, validate=validate.Length(min=1, max=255))
    slug = fields.Str(required=True)
    live = fields.Bool(load_default=False)
    seo_title = fields.Str(load_default="")


def validate_page_payload(raw: dict) -> dict:
    schema = PagePayloadSchema()
    return schema.load(raw)


def dump_page(page: Page) -> dict:
    return PagePayloadSchema().dump(
        {"title": page.title, "slug": page.slug, "live": page.live}
    )
# hunk ends here
