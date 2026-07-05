# Break: pyarrow (imported in the hunk) builds an Arrow/IPC table for a stock export, replacing petl/CSV
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style stock helper, NOT inside the hunk range
def available_quantity(stock) -> int:
    return max(stock.quantity - stock.quantity_allocated, 0)


# hunk starts here
import pyarrow


def stock_levels_to_arrow(rows: list[dict]) -> bytes:
    table = pyarrow.Table.from_pylist(rows)
    sink = pyarrow.BufferOutputStream()
    with pyarrow.ipc.new_stream(sink, table.schema) as writer:
        writer.write_table(table)
    return sink.getvalue().to_pybytes()
# hunk ends here
