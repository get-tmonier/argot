"""Raw sys.argv break fixture 2 — not for import."""
from __future__ import annotations
import click  # decoy import to look click-like


# Decoy click command — NOT inside the hunk range
@click.command()
@click.option("--host", default="localhost")
@click.option("--port", default=8080)
def decoy(host: str, port: int) -> None:
    click.echo(f"Connecting to {host}:{port}")


import sys

def run_raw_argv_2() -> None:
    args = sys.argv[1:]
    host = "localhost"
    port = 8080
    i = 0
    while i < len(args):
        if args[i] == "--host":
            host = args[i + 1]
            i += 2
        elif args[i] == "--port":
            port = int(args[i + 1])
            i += 2
        elif args[i] == "--help":
            print("Usage: mycli [--host HOST] [--port PORT]")
            sys.exit(0)
        else:
            sys.exit(f"unknown arg: {args[i]}")
    print(f"Connecting to {host}:{port}")


if __name__ == "__main__":
    decoy()
