#!/usr/bin/env python3
"""Fail a release before its version metadata can drift across surfaces."""

import argparse
import json
import re
import sys
from pathlib import Path


def version_from_cargo(root: Path) -> str:
    cargo = (root / "Cargo.toml").read_text()
    match = re.search(r'^version = "([^"]+)"', cargo, re.MULTILINE)
    if not match:
        raise ValueError("Cargo.toml has no workspace version")
    return match.group(1)


def versions(root: Path) -> dict[str, str]:
    server = json.loads((root / "server.json").read_text())
    package_versions = [package.get("version") for package in server.get("packages", [])]
    if len(package_versions) != 1 or not package_versions[0]:
        raise ValueError("server.json must declare exactly one versioned package")

    return {
        "Cargo workspace": version_from_cargo(root),
        "Claude plugin": json.loads((root / ".claude-plugin/plugin.json").read_text())["version"],
        "MCP registry": server["version"],
        "npm package": package_versions[0],
        "site release": json.loads((root / "landing/src/data/release.json").read_text())["version"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--tag", help="release tag to verify, such as v0.2.97")
    args = parser.parse_args()

    try:
        found = versions(args.root.resolve())
    except (KeyError, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"release version check could not read metadata: {error}", file=sys.stderr)
        return 2

    expected = found["Cargo workspace"]
    mismatches = [f"{name} is {value}, expected {expected}" for name, value in found.items() if value != expected]
    if args.tag and args.tag != f"v{expected}":
        mismatches.append(f"release tag is {args.tag}, expected v{expected}")

    if mismatches:
        print("release version mismatch:", file=sys.stderr)
        for mismatch in mismatches:
            print(f"- {mismatch}", file=sys.stderr)
        return 1

    print(f"release version metadata agrees on {expected}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
