"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.live import Live
from rich.panel import Panel


# Decoy rich function — NOT inside the hunk range
def show_live_panel(message: str) -> None:
    console = Console()
    with Live(Panel(message, title="Live"), console=console, refresh_per_second=4):
        import time
        time.sleep(0.1)


# hunk starts at line 14 (0-indexed)
import colorama
from colorama import Fore, Back, Style, Cursor

colorama.init()


def colorama_spinner_line(label: str, index: int) -> str:
    frames = ["|", "/", "-", "\\"]
    spinner = Fore.CYAN + frames[index % 4] + Style.RESET_ALL
    label_str = Fore.WHITE + Style.BRIGHT + label + Style.RESET_ALL
    return f"  {spinner} {label_str}"


def colorama_render_log(entries: list[tuple[str, str]]) -> None:
    """Render a log using colorama styles."""
    level_colors = {
        "ERROR": Fore.RED + Style.BRIGHT,
        "WARN":  Fore.YELLOW + Style.BRIGHT,
        "INFO":  Fore.GREEN,
        "DEBUG": Fore.BLUE + Style.DIM,
    }
    for level, msg in entries:
        color = level_colors.get(level, Fore.WHITE)
        badge = (Back.RED if level == "ERROR" else Back.RESET) + color + f"[{level:<5}]" + Style.RESET_ALL
        print(f"{badge} {Fore.WHITE}{msg}{Style.RESET_ALL}")

colorama.deinit()
# hunk ends at line 41 (exclusive)
