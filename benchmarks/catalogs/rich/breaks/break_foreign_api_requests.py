# Break: requests.get pulls a palette from a remote service (foreign HTTP API)
"""Break fixture — not for import."""
from __future__ import annotations

from rich.color import Color


# Decoy — idiomatic rich color construction, NOT inside the hunk range
def parse_hex(value: str) -> Color:
    return Color.parse(value)


# hunk starts here
import requests


def load_remote_palette(url: str) -> list[str]:
    response = requests.get(url, timeout=5)
    response.raise_for_status()
    payload = requests.get(f"{url}/meta", timeout=5).json()
    return [entry["hex"] for entry in payload["colors"]]
# hunk ends here
