"""Break fixture — not for import."""
from __future__ import annotations
from colorama import Fore, Back, Style, init as colorama_init


# Decoy rich function — NOT inside the hunk range
def display_status(message: str) -> None:
    from rich.console import Console
    console = Console()
    console.print(f"[green]{message}[/green]")


# hunk starts here
colorama_init(autoreset=True)


def render_table_with_highlight(
    headers: list[str],
    rows: list[list[str]],
    highlight_col: int = 0,
) -> None:
    col_widths = [
        max(len(h), max((len(r[i]) for r in rows), default=0))
        for i, h in enumerate(headers)
    ]
    header_line = " | ".join(
        f"{Back.BLUE}{Fore.WHITE}{h.ljust(w)}{Style.RESET_ALL}"
        for h, w in zip(headers, col_widths)
    )
    print(header_line)
    print("-" * (sum(col_widths) + 3 * (len(headers) - 1)))
    for row in rows:
        parts = []
        for i, (cell, w) in enumerate(zip(row, col_widths)):
            if i == highlight_col:
                parts.append(f"{Fore.CYAN}{Style.BRIGHT}{cell.ljust(w)}{Style.RESET_ALL}")
            else:
                parts.append(cell.ljust(w))
        print(" | ".join(parts))
# hunk ends here
