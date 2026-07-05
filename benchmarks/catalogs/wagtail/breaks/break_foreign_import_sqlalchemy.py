# Break: SQLAlchemy engine + text() run raw audit-log queries, bypassing the Django ORM
"""Break fixture — not for import."""
from __future__ import annotations

from django.utils import timezone

from wagtail.models import ModelLogEntry


# Decoy — idiomatic wagtail ORM log lookup, NOT inside the hunk range
def recent_log_entries(content_type_id: int, limit: int = 20):
    return ModelLogEntry.objects.filter(content_type_id=content_type_id).order_by(
        "-timestamp"
    )[:limit]


# hunk starts here
import sqlalchemy
from sqlalchemy import create_engine, text

_engine = create_engine("postgresql://wagtail:wagtail@localhost/wagtail")


def audit_action_counts(since_days: int = 30) -> list[tuple[str, int]]:
    cutoff = timezone.now() - timezone.timedelta(days=since_days)
    stmt = text(
        "SELECT action, COUNT(*) AS n FROM wagtailcore_modellogentry "
        "WHERE timestamp >= :cutoff GROUP BY action ORDER BY n DESC"
    )
    with _engine.connect() as conn:
        rows = conn.execute(stmt, {"cutoff": cutoff}).fetchall()
    return [(row.action, row.n) for row in rows]


def purge_old_entries(before_days: int = 365) -> int:
    meta = sqlalchemy.MetaData()
    table = sqlalchemy.Table("wagtailcore_modellogentry", meta, autoload_with=_engine)
    stmt = sqlalchemy.delete(table).where(
        table.c.timestamp < timezone.now() - timezone.timedelta(days=before_days)
    )
    with _engine.begin() as conn:
        result = conn.execute(stmt)
    return result.rowcount
# hunk ends here
