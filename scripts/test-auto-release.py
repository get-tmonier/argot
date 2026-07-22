#!/usr/bin/env python3
"""Regression checks for the release commit and tag publication command."""

from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = ROOT / ".github/workflows/auto-release.yml"


def main() -> None:
    workflow = WORKFLOW.read_text()

    assert 'printf \'{\\n  "version": "%s"\\n}\\n\' "$NEW" > landing/src/data/release.json' in workflow
    assert 'git push --atomic origin HEAD:refs/heads/main "v$VERSION"' in workflow
    assert 'git push origin main "v$VERSION"' not in workflow


if __name__ == "__main__":
    main()
