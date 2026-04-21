"""Argparse class-based break fixture 3 — not for import."""
from __future__ import annotations
import click  # decoy import to look click-like


# Decoy click command — NOT inside the hunk range
@click.command()
@click.option("--env", default="dev")
def decoy(env: str) -> None:
    click.echo(f"Deploying to {env}")


import argparse

class DeployCommandParser(argparse.ArgumentParser):
    def __init__(self) -> None:
        super().__init__(description="Deployment CLI parser")
        self.add_argument("--env", choices=["dev", "staging", "prod"], required=True)
        self.add_argument("--dry-run", action="store_true")
        self.add_argument("--replicas", type=int, default=1)

def run_argparse_3() -> None:
    parser = DeployCommandParser()
    args = parser.parse_args()
    if args.dry_run:
        print(f"Dry run: would deploy {args.replicas} replicas to {args.env}")
    else:
        print(f"Deploying to {args.env}")


if __name__ == "__main__":
    decoy()
