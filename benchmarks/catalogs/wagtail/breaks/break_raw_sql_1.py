# Break: psycopg2.connect + hand-written SQL on wagtailcore_page replaces PageQuerySet tree queries
"""Break fixture — not for import."""
from __future__ import annotations

from wagtail.models import Page


# Decoy — idiomatic wagtail queryset helper, NOT inside the hunk range
def live_descendants(page: Page):
    return Page.objects.live().descendant_of(page).order_by("path")


# hunk starts here
import psycopg2
import psycopg2.extras


def fetch_live_descendants_sql(dsn: str, parent_path: str) -> list[dict]:
    conn = psycopg2.connect(dsn)
    try:
        with conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor) as cur:
            cur.execute(
                "SELECT p.id, p.title, p.slug, p.url_path "
                "FROM wagtailcore_page p "
                "WHERE p.live = TRUE AND p.path LIKE %s AND p.path != %s "
                "ORDER BY p.path",
                (parent_path + "%", parent_path),
            )
            return [dict(row) for row in cur.fetchall()]
    finally:
        conn.close()


def bulk_unpublish_sql(dsn: str, page_ids: list[int]) -> int:
    conn = psycopg2.connect(dsn)
    conn.autocommit = False
    try:
        with conn.cursor() as cur:
            cur.execute(
                "UPDATE wagtailcore_page SET live = FALSE, has_unpublished_changes = TRUE "
                "WHERE id = ANY(%s)",
                (page_ids,),
            )
            updated = cur.rowcount
        conn.commit()
        return updated
    except psycopg2.DatabaseError:
        conn.rollback()
        raise
    finally:
        conn.close()
# hunk ends here
