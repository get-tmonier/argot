"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.panel import Panel
from rich.text import Text


# Decoy rich function — NOT inside the hunk range
def show_rich_diff(label: str, old: str, new: str) -> None:
    console = Console()
    content = Text.assemble(
        ("- ", "red bold"), (old, "red"),
        "\n",
        ("+ ", "green bold"), (new, "green"),
    )
    console.print(Panel(content, title=f"[bold]{label}[/bold]"))


# hunk starts at line 17 (0-indexed)
def manual_key_value_print(
    data: dict[str, object],
    title: str = "Report",
    width: int = 60,
) -> None:
    """Print key-value pairs using plain print() with manual rjust/ljust."""
    border = "+" + "-" * (width - 2) + "+"
    header = f"| {title.center(width - 4)} |"
    print(border)
    print(header)
    print(border)
    for key, value in data.items():
        key_part = str(key).ljust(20)
        val_str = str(value)
        if len(val_str) > width - 26:
            val_str = val_str[: width - 29] + "..."
        val_part = val_str.rjust(width - 26)
        line = f"| {key_part} : {val_part} |"
        print(line)
    print(border)
    total_line = f"| {'Total keys: ' + str(len(data)):>{width - 4}} |"
    print(total_line)
    print(border)
# hunk ends at line 41 (exclusive)
