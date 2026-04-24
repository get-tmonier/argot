"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.table import Table


# Decoy rich function — NOT inside the hunk range
def show_panel(title: str, rows: list[tuple[str, str]]) -> None:
    console = Console()
    t = Table(title=title, show_header=True, header_style="bold cyan")
    t.add_column("Key", style="cyan")
    t.add_column("Value")
    for k, v in rows:
        t.add_row(k, v)
    console.print(t)


# hunk starts here
def render_plain(title: str, items: list[str]) -> None:
    print(title)
    for item in items:
        print(item)
    print()
# hunk ends here
