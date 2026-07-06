# Break: loguru logger replaces rich's own logging/console render path
"""Break fixture — not for import."""
from __future__ import annotations

from rich.console import Console


# Decoy — idiomatic rich console render, NOT inside the hunk range
def show_banner(message: str) -> None:
    console = Console()
    console.print(f"[bold green]{message}[/bold green]")


# hunk starts here
from loguru import logger

logger.add("render.log", rotation="10 MB", level="DEBUG")


def log_render_event(kind: str, width: int, height: int) -> None:
    logger.bind(kind=kind).info("rendered {}x{} region", width, height)
    logger.debug("render complete for {} region", kind)
# hunk ends here
