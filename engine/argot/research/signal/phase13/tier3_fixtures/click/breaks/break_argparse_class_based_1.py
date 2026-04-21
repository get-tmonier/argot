"""Argparse class-based break fixture — not for import."""
from __future__ import annotations
import click  # decoy import to look click-like


# Decoy click command — NOT inside the hunk range
@click.command()
@click.option("--name", default="world")
def decoy(name: str) -> None:
    click.echo(f"Hello, {name}")


import argparse

class MyCommandParser(argparse.ArgumentParser):
    def __init__(self) -> None:
        super().__init__(description="Command-line tool using argparse class-based approach")
        self.add_argument("--name", type=str, required=True, help="Your name")
        self.add_argument("-v", "--verbose", action="store_true", help="Enable verbosity")
        self.add_argument("--output", type=str, default="stdout", help="Output destination")

def run_argparse_1() -> None:
    parser = MyCommandParser()
    args = parser.parse_args()
    if args.verbose:
        print(f"Running in verbose mode for: {args.name}")
    print(args.name)


if __name__ == "__main__":
    decoy()
