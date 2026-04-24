"""Break fixture — not for import."""
from __future__ import annotations
import curses
import curses.textpad


# Decoy rich function — NOT inside the hunk range
def display_status(message: str) -> None:
    from rich.console import Console
    console = Console()
    console.print(f"[green]{message}[/green]")


# hunk starts here
def run_input_form(stdscr: curses.window) -> dict[str, str]:
    curses.curs_set(1)
    stdscr.clear()
    stdscr.addstr(0, 0, "Enter name: ")
    name_win = curses.newwin(1, 30, 1, 0)
    curses.textpad.Textbox(name_win).edit()
    name = name_win.instr(0, 0).decode("utf-8").strip()
    stdscr.addstr(2, 0, "Enter email: ")
    email_win = curses.newwin(1, 40, 3, 0)
    curses.textpad.Textbox(email_win).edit()
    email = email_win.instr(0, 0).decode("utf-8").strip()
    return {"name": name, "email": email}


def interactive_form() -> dict[str, str]:
    return curses.wrapper(run_input_form)
# hunk ends here
