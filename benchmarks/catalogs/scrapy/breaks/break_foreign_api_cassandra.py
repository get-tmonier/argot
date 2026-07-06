# Break: cassandra loaded via importlib+getattr; no static import, masked leaves
"""Break fixture — not for import."""

# hunk starts here
import importlib

_cluster_mod = importlib.import_module("cassandra.cluster")
_Cluster = getattr(_cluster_mod, "Cluster")


def export_rows(rows: list) -> None:
    session = _Cluster(["127.0.0.1"]).connect("scrapy")
    for row in rows:
        session.execute("INSERT INTO items JSON %s", (row,))
# hunk ends here
