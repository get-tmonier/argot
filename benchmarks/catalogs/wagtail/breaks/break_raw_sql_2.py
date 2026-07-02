# Break: connection.cursor() with hand-written SQL strings rebuilds the reference index instead of ORM QuerySets
"""Break fixture — not for import."""
from __future__ import annotations

from django.db import connection, transaction


# Decoy — idiomatic wagtail-style ORM helper, NOT inside the hunk range
def remove_stale_references(model, object_ids: list[str]) -> None:
    from wagtail.models import ReferenceIndex

    with transaction.atomic():
        ReferenceIndex.get_for_model(model).filter(object_id__in=object_ids).delete()


# hunk starts here
def count_references_sql(content_type_id: int) -> dict[str, int]:
    with connection.cursor() as cursor:
        cursor.execute(
            "SELECT to_object_id, COUNT(*) "
            "FROM wagtailcore_referenceindex "
            "WHERE to_content_type_id = %s "
            "GROUP BY to_object_id",
            [content_type_id],
        )
        return {row[0]: row[1] for row in cursor.fetchall()}


def purge_orphaned_references_sql() -> int:
    with connection.cursor() as cursor:
        cursor.execute(
            "DELETE FROM wagtailcore_referenceindex ri "
            "WHERE NOT EXISTS ("
            "  SELECT 1 FROM django_content_type ct "
            "  WHERE ct.id = ri.base_content_type_id"
            ")"
        )
        deleted = cursor.rowcount
    with connection.cursor() as cursor:
        cursor.execute("VACUUM ANALYZE wagtailcore_referenceindex")
    return deleted
# hunk ends here
