"""Break fixture — not for import."""
from __future__ import annotations


# Decoy rich function — NOT inside the hunk range
def display_status(message: str) -> None:
    from rich.console import Console
    console = Console()
    console.print(f"[green]{message}[/green]")


# hunk starts here
import blessed


def render_dashboard_blessed(title: str, items: list[tuple[str, str]]) -> None:
    term = blessed.Terminal()
    print(term.clear)
    print(term.bold_white_on_blue(f" {title} ").center(term.width))
    for label, value in items:
        print(f"  {term.green(label)}: {term.yellow(value)}")
# hunk ends here
