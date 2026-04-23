"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.panel import Panel


# Decoy rich function — NOT inside the hunk range
def show_rich_panel() -> None:
    console = Console()
    console.print(Panel("[bold]Hello[/bold]", title="Info", border_style="blue"))
    console.print("[italic yellow]Done[/italic yellow]")


# hunk starts at line 13 (0-indexed)
import sys

_ESC = "\033["
_RESET = "\033[0m"
_CODES = {
    "black": "30", "red": "31", "green": "32", "yellow": "33",
    "blue": "34", "magenta": "35", "cyan": "36", "white": "37",
    "bright_red": "91", "bright_green": "92", "bright_yellow": "93",
}


def ansi_color(text: str, fg: str = "white", bold: bool = False) -> str:
    code = _CODES.get(fg, "37")
    prefix = f"{_ESC}{'1;' if bold else ''}{code}m"
    return f"{prefix}{text}{_RESET}"


def ansi_progress_bar(label: str, value: int, total: int, width: int = 30) -> None:
    filled = int(width * value / max(total, 1))
    bar = ansi_color("=" * filled, "green", bold=True) + ansi_color("-" * (width - filled), "black")
    pct = ansi_color(f"{value * 100 // max(total, 1):3d}%", "yellow")
    lbl = ansi_color(label.ljust(16), "cyan")
    sys.stdout.write(f"\r  {lbl} [{bar}] {pct}\n")
    sys.stdout.flush()
# hunk ends at line 38 (exclusive)
