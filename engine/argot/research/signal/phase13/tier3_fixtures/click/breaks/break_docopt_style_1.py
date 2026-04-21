"""Docopt-style break fixture — not for import."""
from __future__ import annotations
import click  # decoy import to look click-like


# Decoy click command — NOT inside the hunk range
@click.command()
@click.argument("name", default="world")
def decoy(name: str) -> None:
    click.echo(f"Hello, {name}")


USAGE = """
Usage:
  mycli [--verbose] <name>
  mycli --help

Options:
  --verbose    Enable verbose output
  --help       Show this help

"""

def run_docopt_1() -> None:
    # docopt-style manual parsing from USAGE docstring
    args = {}
    import sys as _sys
    raw = _sys.argv[1:]
    args["--verbose"] = "--verbose" in raw
    args["--help"] = "--help" in raw
    positional = [a for a in raw if not a.startswith("--")]
    args["<name>"] = positional[0] if positional else "world"
    if args["--help"]:
        print(USAGE)
        _sys.exit(0)
    if args["--verbose"]:
        print(f"Hello, {args['<name>']}")


if __name__ == "__main__":
    decoy()
