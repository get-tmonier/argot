# Break: SQLAlchemy engine persists scraped items, bypassing scrapy pipelines
"""Break fixture — not for import."""

# hunk starts here
import sqlalchemy
from sqlalchemy import create_engine, text

_engine = create_engine("postgresql://localhost/scrapy")


def persist_item(url: str, size: int) -> None:
    with _engine.begin() as conn:
        conn.execute(
            text("INSERT INTO scraped(url, size) VALUES (:u, :s)"),
            {"u": url, "s": sqlalchemy.bindparam("s", size)},
        )
# hunk ends here
