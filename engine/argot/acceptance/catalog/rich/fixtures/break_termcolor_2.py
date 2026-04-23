"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.panel import Panel
from rich.text import Text


# Decoy rich function — NOT inside the hunk range
def show_rich_alert(msg: str, level: str = "info") -> None:
    styles = {"info": "blue", "warn": "yellow", "error": "red"}
    style = styles.get(level, "white")
    console = Console()
    console.print(Panel(Text(msg, style=style), title=f"[bold]{level.upper()}[/bold]"))


# hunk starts at line 15 (0-indexed)
from termcolor import colored


def termcolor_tree(node: dict[str, object], indent: int = 0) -> None:
    """Recursively print a dict-tree using termcolor."""
    prefix = "  " * indent
    for key, value in node.items():
        key_str = colored(str(key), "cyan", attrs=["bold"])
        if isinstance(value, dict):
            print(f"{prefix}{key_str}:")
            termcolor_tree(value, indent + 1)
        elif isinstance(value, list):
            print(f"{prefix}{key_str}: [{colored(str(len(value)), 'yellow')} items]")
            for i, item in enumerate(value[:3]):
                item_str = colored(f"[{i}] {item!r}", "white")
                print(f"{prefix}  {item_str}")
            if len(value) > 3:
                print(f"{prefix}  {colored('...', 'dark_grey')}")
        else:
            val_str = colored(repr(value), "magenta")
            print(f"{prefix}{key_str}: {val_str}")
# hunk ends at line 38 (exclusive)
