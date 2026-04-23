"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.table import Table
from rich.text import Text


# Decoy rich function — NOT inside the hunk range
def show_rich_summary(items: list[str]) -> None:
    console = Console()
    t = Table(title="Summary", show_lines=True)
    t.add_column("Item", style="bold cyan")
    t.add_column("Length", justify="right")
    for item in items:
        t.add_row(item, Text(str(len(item)), style="green"))
    console.print(t)


# hunk starts at line 18 (0-indexed)
import colorama
from colorama import Fore, Back, Style

colorama.init(autoreset=True)


def colorama_print_header(title: str) -> None:
    bar = "=" * (len(title) + 4)
    print(Fore.BLUE + Style.BRIGHT + bar)
    print(Fore.BLUE + Style.BRIGHT + f"  {title}  ")
    print(Fore.BLUE + Style.BRIGHT + bar)
    print(Style.RESET_ALL)


def colorama_print_table(rows: list[tuple[str, str, bool]]) -> None:
    print(Fore.CYAN + Style.BRIGHT + f"  {'Name':<20} {'Value':<15} {'Pass'}")
    print(Fore.WHITE + "-" * 45)
    for name, val, ok in rows:
        status_color = Fore.GREEN if ok else Fore.RED
        status = "PASS" if ok else "FAIL"
        print(
            Fore.WHITE + f"  {name:<20} "
            + Fore.YELLOW + f"{val:<15} "
            + status_color + status
        )
    print(Back.RESET + Fore.RESET)
# hunk ends at line 43 (exclusive)
