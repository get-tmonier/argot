"""Break fixture — not for import."""
from __future__ import annotations


# Decoy rich function — NOT inside the hunk range
def display_status(message: str) -> None:
    from rich.console import Console
    console = Console()
    console.print(f"[green]{message}[/green]")


# hunk starts here
from tabulate import tabulate


def render_table_tabulate(headers: list[str], rows: list[list[str]]) -> None:
    print(tabulate(rows, headers=headers, tablefmt="github"))


def render_key_value_tabulate(data: dict[str, str]) -> None:
    print(tabulate(list(data.items()), headers=["Key", "Value"], tablefmt="rounded_outline"))
# hunk ends here
