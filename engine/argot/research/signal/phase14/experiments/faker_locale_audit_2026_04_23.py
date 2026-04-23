# engine/argot/research/signal/phase14/experiments/faker_locale_audit_2026-04-23.py
"""Locale audit: verify is_data_dominant flags ≥80% of faker-js src/locales/ .ts files.

Uses the cached faker-js repo if already cloned under .argot/research/repos/faker-js;
otherwise clones it to a temp dir.

Usage:
    uv run python engine/.../experiments/faker_locale_audit_2026-04-23.py
"""

from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

from argot.research.signal.phase14.adapters.typescript_adapter import TypeScriptAdapter

_ARGOT_PKG = Path(__file__).parent.parent.parent.parent.parent
_PROJECT_ROOT = _ARGOT_PKG.parent.parent
_REPOS_DIR = _PROJECT_ROOT / ".argot" / "research" / "repos"
_CACHED_REPO = _REPOS_DIR / "faker-js"
_FAKER_GH = "https://github.com/faker-js/faker.git"

_REQUIRED_RATE = 0.80
_SAMPLE_FALSE_LIMIT = 20


def _get_repo() -> Path:
    if _CACHED_REPO.exists():
        print(f"Using cached repo: {_CACHED_REPO}")
        return _CACHED_REPO
    tmp = tempfile.mkdtemp(prefix="faker-js-audit-")
    print(f"Cloning faker-js → {tmp} …")
    subprocess.run(
        ["git", "clone", "--depth=1", _FAKER_GH, tmp],
        check=True,
    )
    return Path(tmp)


def main() -> None:
    adapter = TypeScriptAdapter()
    repo = _get_repo()
    locale_root = repo / "src" / "locales"

    if not locale_root.exists():
        raise RuntimeError(f"src/locales not found under {repo}")

    ts_files = sorted(locale_root.rglob("*.ts"))
    total = len(ts_files)
    if total == 0:
        raise RuntimeError(f"No .ts files found under {locale_root}")

    true_files: list[Path] = []
    false_files: list[Path] = []

    for path in ts_files:
        try:
            src = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            false_files.append(path)
            continue
        if adapter.is_data_dominant(src):
            true_files.append(path)
        else:
            false_files.append(path)

    excluded = len(true_files)
    not_excluded = len(false_files)
    rate = excluded / total

    print()
    print("=" * 60)
    print("faker-js src/locales/ is_data_dominant audit")
    print("=" * 60)
    print(f"Total .ts files : {total}")
    print(f"Flagged True    : {excluded}  ({rate:.1%})")
    print(f"Flagged False   : {not_excluded}  ({1 - rate:.1%})")
    print(f"Required rate   : {_REQUIRED_RATE:.0%}")
    print()

    if rate >= _REQUIRED_RATE:
        print(f"CHECK 1 PASS — exclusion rate {rate:.1%} ≥ {_REQUIRED_RATE:.0%}")
    else:
        print(f"CHECK 1 FAIL — exclusion rate {rate:.1%} < {_REQUIRED_RATE:.0%}")
        sample = false_files[:_SAMPLE_FALSE_LIMIT]
        print(f"\nSample of non-excluded files (first {len(sample)}):")
        for p in sample:
            print(f"  {p.relative_to(repo)}")

    print()


if __name__ == "__main__":
    main()
