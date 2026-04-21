"""Argparse class-based break fixture 2 — not for import."""
from __future__ import annotations
import click  # decoy import to look click-like


# Decoy click command — NOT inside the hunk range
@click.command()
@click.option("--target", default="dist")
def decoy(target: str) -> None:
    click.echo(f"Building {target}")


import argparse

class BuildCommandParser(argparse.ArgumentParser):
    def __init__(self) -> None:
        super().__init__(description="Build tool argument parser")
        self.add_argument("--target", type=str, required=True)
        self.add_argument("--debug", action="store_true")
        self.add_argument("--config", type=str, default="config.yaml")

def run_argparse_2() -> None:
    parser = BuildCommandParser()
    args = parser.parse_args()
    if args.debug:
        print(f"Debug mode: building {args.target}")
    print(f"Config: {args.config}")


if __name__ == "__main__":
    decoy()
