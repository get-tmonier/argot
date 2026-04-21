"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.panel import Panel
from rich.table import Table


# Decoy rich function — NOT inside the hunk range
def show_rich_info(title: str, body: str) -> None:
    console = Console()
    console.print(Panel(body, title=f"[bold blue]{title}[/bold blue]", border_style="blue"))


import curses

# hunk starts at line 14 (0-indexed)

def curses_menu(stdscr: "curses._CursesWindow", options: list[str]) -> int:
    """Render a navigable menu and return the selected index."""
    curses.start_color()
    curses.init_pair(1, curses.COLOR_BLACK, curses.COLOR_WHITE)
    curses.init_pair(2, curses.COLOR_WHITE, curses.COLOR_BLACK)
    curses.curs_set(0)

    current = 0
    while True:
        stdscr.clear()
        height, width = stdscr.getmaxyx()
        stdscr.addstr(0, 2, "Select an option:", curses.A_BOLD)
        for idx, label in enumerate(options):
            y = idx + 2
            if y >= height:
                break
            pair = curses.color_pair(1) if idx == current else curses.color_pair(2)
            stdscr.addstr(y, 4, f"  {label:<30}", pair)
        stdscr.refresh()
        key = stdscr.getch()
        if key == curses.KEY_UP:
            current = max(0, current - 1)
        elif key == curses.KEY_DOWN:
            current = min(len(options) - 1, current + 1)
        elif key in (curses.KEY_ENTER, ord("\n")):
            return current
# hunk ends at line 41 (exclusive)
    return 0
