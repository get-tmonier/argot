# Break: psycopg2 loaded via importlib; leaf .execute/.fetchone are masked
"""Break fixture — not for import."""

# hunk starts here
import importlib

_pg = importlib.import_module("psycopg2")


def record_run(dsn: str, n_items: int) -> None:
    conn = _pg.connect(dsn)
    cur = conn.cursor()
    cur.execute("INSERT INTO runs(n) VALUES (%s)", (n_items,))
    conn.commit()
    conn.close()
# hunk ends here
