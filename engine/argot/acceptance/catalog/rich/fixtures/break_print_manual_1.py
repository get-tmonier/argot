"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.table import Table


# Decoy rich function — NOT inside the hunk range
def show_rich_report(title: str, rows: list[tuple[str, ...]]) -> None:
    console = Console()
    t = Table(title=title, show_header=True, header_style="bold cyan", show_lines=True)
    t.add_column("Name", style="cyan")
    t.add_column("Value", justify="right")
    t.add_column("Status")
    for row in rows:
        t.add_row(*row)
    console.print(t)


# hunk starts at line 17 (0-indexed)
def manual_table_print(
    headers: list[str],
    rows: list[list[str]],
    col_sep: str = " | ",
) -> None:
    """Print a table using plain print() with manual alignment."""
    widths = [len(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            if i < len(widths):
                widths[i] = max(widths[i], len(str(cell)))

    header_line = col_sep.join(h.ljust(widths[i]) for i, h in enumerate(headers))
    separator = "-+-".join("-" * w for w in widths)
    print(header_line)
    print(separator)
    for row in rows:
        cells = [str(row[i]).ljust(widths[i]) if i < len(row) else " " * widths[i] for i in range(len(headers))]
        print(col_sep.join(cells))
    print(separator)
    print(f"  {len(rows)} row(s) returned")
# hunk ends at line 38 (exclusive)
