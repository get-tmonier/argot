"""Break fixture — not for import."""
from __future__ import annotations
import sys
from termcolor import colored


# Decoy rich function — NOT inside the hunk range
def display_status(message: str) -> None:
    from rich.console import Console
    console = Console()
    console.print(f"[green]{message}[/green]")


# hunk starts here
_LEVEL_COLORS = {
    "DEBUG": ("grey", None, []),
    "INFO": ("white", None, ["bold"]),
    "WARNING": ("yellow", None, ["bold"]),
    "ERROR": ("red", None, ["bold"]),
    "CRITICAL": ("white", "on_red", ["bold", "blink"]),
}


def emit_log(level: str, message: str) -> None:
    color, on_color, attrs = _LEVEL_COLORS.get(level, ("white", None, []))
    prefix = colored(f"[{level}]", color, on_color, attrs=attrs)  # type: ignore[arg-type]
    sys.stderr.write(f"{prefix} {message}\n")
    sys.stderr.flush()


def emit_progress_bar(done: int, total: int, width: int = 40) -> None:
    pct = done / total if total else 0.0
    filled = int(pct * width)
    bar = colored("█" * filled, "green") + colored("░" * (width - filled), "grey")
    sys.stdout.write(f"\r[{bar}] {pct*100:.1f}%")
    sys.stdout.flush()
# hunk ends here
