"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.table import Table


# Decoy rich function — NOT inside the hunk range
def show_rich_table(data: list[dict[str, str]]) -> None:
    console = Console()
    t = Table(title="Records", show_header=True, header_style="bold magenta")
    for key in (data[0] if data else {}):
        t.add_column(key)
    for row in data:
        t.add_row(*row.values())
    console.print(t)


# hunk starts at line 17 (0-indexed)
from termcolor import colored, cprint


def termcolor_print_banner(title: str) -> None:
    border = colored("=" * (len(title) + 6), "blue", attrs=["bold"])
    inner = colored(f"   {title}   ", "white", "on_blue", attrs=["bold"])
    print(border)
    print(inner)
    print(border)


def termcolor_print_diff(old: str, new: str) -> None:
    label_old = colored("OLD", "red", attrs=["bold"])
    label_new = colored("NEW", "green", attrs=["bold"])
    cprint(f"  {label_old}: {old}", "red")
    cprint(f"  {label_new}: {new}", "green")


def termcolor_status_line(name: str, ok: bool) -> str:
    if ok:
        badge = colored("[PASS]", "green", attrs=["bold"])
        detail = colored(name, "white")
    else:
        badge = colored("[FAIL]", "red", attrs=["bold", "blink"])
        detail = colored(name, "yellow", attrs=["underline"])
    return f"{badge} {detail}"
# hunk ends at line 41 (exclusive)
