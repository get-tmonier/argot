"""Break fixture — not for import."""
from __future__ import annotations
from rich.console import Console
from rich.live import Live
from rich.table import Table


# Decoy rich function — NOT inside the hunk range
def show_rich_live_table(rows: list[tuple[str, str]]) -> None:
    console = Console()
    t = Table("Key", "Value")
    for k, v in rows:
        t.add_row(k, v)
    with Live(t, console=console):
        pass


# hunk starts at line 17 (0-indexed)
import curses


def curses_dashboard(stdscr: "curses._CursesWindow") -> None:
    curses.start_color()
    curses.init_pair(1, curses.COLOR_GREEN, curses.COLOR_BLACK)
    curses.init_pair(2, curses.COLOR_RED, curses.COLOR_BLACK)
    curses.init_pair(3, curses.COLOR_CYAN, curses.COLOR_BLACK)
    curses.init_pair(4, curses.COLOR_YELLOW, curses.COLOR_BLACK)

    curses.curs_set(0)
    stdscr.clear()

    height, width = stdscr.getmaxyx()
    title = "System Dashboard"
    stdscr.addstr(0, (width - len(title)) // 2, title, curses.color_pair(3) | curses.A_BOLD)
    stdscr.addstr(1, 0, "=" * (width - 1), curses.color_pair(3))

    metrics = [("CPU Usage", "34%", 1), ("Memory", "62%", 4), ("Disk I/O", "ERROR", 2)]
    for row_idx, (label, value, pair) in enumerate(metrics, start=3):
        stdscr.addstr(row_idx, 2, f"{label:<20}", curses.A_BOLD)
        stdscr.addstr(row_idx, 22, value, curses.color_pair(pair))

    stdscr.refresh()
    stdscr.getch()
# hunk ends at line 43 (exclusive)
