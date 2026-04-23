"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.table import Table


# Decoy rich function — NOT inside the hunk range
def show_rich_banner() -> None:
    console = Console()
    console.print("[bold green]Welcome to the dashboard[/bold green]")
    table = Table(title="Status")
    table.add_column("Service", style="cyan")
    table.add_column("State", style="magenta")
    table.add_row("API", "[green]OK[/green]")
    console.print(table)


# hunk starts at line 18 (0-indexed)
RESET = "\033[0m"
RED = "\033[31m"
GREEN = "\033[32m"
YELLOW = "\033[33m"
BLUE = "\033[34m"
BOLD = "\033[1m"
DIM = "\033[2m"


def _ansi(code: str, text: str) -> str:
    return f"{code}{text}{RESET}"


def print_status_report(services: list[tuple[str, bool]]) -> None:
    header = _ansi(BOLD + BLUE, "Service Status Report")
    print(header)
    print(_ansi(DIM, "-" * 40))
    for name, ok in services:
        color = GREEN if ok else RED
        state = _ansi(color, "OK  ") if ok else _ansi(color, "FAIL")
        label = _ansi(BOLD, name.ljust(20))
        print(f"  {label} {state}")
    print(_ansi(DIM, "-" * 40))
    total = len(services)
    passed = sum(1 for _, ok in services if ok)
    summary = _ansi(YELLOW, f"{passed}/{total} services healthy")
    print(summary)
# hunk ends at line 43 (exclusive)
