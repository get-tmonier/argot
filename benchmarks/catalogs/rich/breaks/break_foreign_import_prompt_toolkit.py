# Break: prompt_toolkit.shortcuts dialogs replace rich.prompt / rich.status
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic rich status message, NOT inside the hunk range
def show_status(message: str) -> None:
    from rich.console import Console

    Console().print(f"[cyan]{message}[/cyan]")


# hunk starts here
from prompt_toolkit.shortcuts import radiolist_dialog, message_dialog


def choose_theme(themes: list[str]) -> str | None:
    result = radiolist_dialog(
        title="Theme",
        text="Pick a theme",
        values=[(name, name) for name in themes],
    ).run()
    return result


def notify(message: str) -> None:
    message_dialog(title="Status", text=message).run()
# hunk ends here
