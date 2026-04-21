"""Optparse deprecated break fixture 1 — not for import."""
from __future__ import annotations
import click  # decoy import to look click-like


# Decoy click command — NOT inside the hunk range
@click.command()
@click.option("--file", default="input.txt")
def decoy(file: str) -> None:
    click.echo(f"Processing {file}")


from optparse import OptionParser, make_option

def run_optparse_1() -> None:
    parser = OptionParser(
        usage="%prog [options]",
        option_list=[
            make_option("-f", "--file", dest="filename", help="Input file"),
            make_option("-v", "--verbose", action="store_true", dest="verbose", default=False),
            make_option("-o", "--output", dest="output", default="out.txt"),
        ],
    )
    (options, args) = parser.parse_args()
    if options.verbose:
        print(f"Processing {options.filename}")
    print(f"Output: {options.output}")


if __name__ == "__main__":
    decoy()
