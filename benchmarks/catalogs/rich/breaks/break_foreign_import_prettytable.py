# Break: prettytable builds tabular output instead of rich.table.Table
"""Break fixture — not for import."""
from __future__ import annotations

from rich.table import Table


# Decoy — idiomatic rich Table construction, NOT inside the hunk range
def build_rich_table(rows: list[tuple[str, str]]) -> Table:
    table = Table(title="Summary")
    table.add_column("Key")
    table.add_column("Value")
    for key, value in rows:
        table.add_row(key, value)
    return table


# hunk starts here
from prettytable import PrettyTable


def render_summary(rows: list[tuple[str, str]]) -> str:
    grid = PrettyTable()
    grid.field_names = ["Key", "Value"]
    grid.align = "l"
    for key, value in rows:
        grid.add_row([key, value])
    return grid.get_string()
# hunk ends here
