# engine/argot/research/signal/phase14/fixtures/stage2_only/fstring_adoption.py
"""F-strings with nested format specs (e.g. {val!r:>10}) throughout — stdlib only.

FastAPI corpus uses simple f-strings or .format(); nested spec expressions are rare.
Imports: datetime (stdlib, in FastAPI corpus).
"""
from __future__ import annotations

import datetime


def format_table_row(label: str, value: float, width: int = 20) -> str:
    return f"| {label:<{width}} | {value:>10.4f} |"


def format_duration(seconds: float) -> str:
    h, rem = divmod(int(seconds), 3600)
    m, s = divmod(rem, 60)
    ms = int((seconds - int(seconds)) * 1000)
    return f"{h:02d}:{m:02d}:{s:02d}.{ms:03d}"


def build_report_header(title: str, ts: datetime.datetime, version: str = "1.0") -> str:
    bar = "=" * 60
    return (
        f"{bar}\n"
        f"  {title!r:^56}\n"
        f"  Generated: {ts:%Y-%m-%dT%H:%M:%S}  v{version!s:>8}\n"
        f"{bar}"
    )


def format_bytes(n: int) -> str:
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024:
            return f"{n:.1f}\u00a0{unit}"
        n //= 1024
    return f"{n:.1f}\u00a0TB"
