"""Optparse deprecated break fixture 2 — not for import."""
from __future__ import annotations
import click  # decoy import to look click-like


# Decoy click command — NOT inside the hunk range
@click.command()
@click.option("--host", default="localhost")
@click.option("--port", default=8080)
def decoy(host: str, port: int) -> None:
    click.echo(f"Connecting to {host}:{port}")


from optparse import OptionParser, make_option

def run_optparse_2() -> None:
    parser = OptionParser(
        usage="%prog [options] <target>",
        option_list=[
            make_option("--host", dest="host", default="localhost"),
            make_option("--port", dest="port", type="int", default=8080),
            make_option("--debug", action="store_true", dest="debug", default=False),
        ],
    )
    (options, remaining_args) = parser.parse_args()
    if options.debug:
        print(f"Debug: connecting to {options.host}:{options.port}")


if __name__ == "__main__":
    decoy()
