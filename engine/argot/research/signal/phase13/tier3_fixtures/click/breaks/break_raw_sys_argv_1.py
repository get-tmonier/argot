"""Raw sys.argv break fixture 1 — not for import."""
from __future__ import annotations
import click  # decoy import to look click-like


# Decoy click command — NOT inside the hunk range
@click.command()
@click.option("--name", default="world")
def decoy(name: str) -> None:
    click.echo(f"Hello, {name}")


import sys

def run_raw_argv_1() -> None:
    args = sys.argv[1:]
    name = "world"
    verbose = False
    i = 0
    while i < len(args):
        if args[i] == "--name":
            name = args[i + 1]
            i += 2
        elif args[i] == "--verbose":
            verbose = True
            i += 1
        else:
            sys.exit(f"unknown arg: {args[i]}")
    if verbose:
        print(f"Running with name={name}")
    print(name)


if __name__ == "__main__":
    decoy()
