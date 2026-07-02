# Break: os.system + subprocess virus-scan and file shuffling on documents instead of Django storage APIs
"""Break fixture — not for import."""
from __future__ import annotations

import os


# Decoy — idiomatic wagtail-style document helper, NOT inside the hunk range
def document_filename(document) -> str:
    return os.path.basename(document.file.name)


# hunk starts here
import subprocess


def scan_document_for_viruses(document) -> bool:
    path = document.file.path
    exit_code = os.system(f"clamscan --no-summary {path}")
    if exit_code != 0:
        os.system(f"mv {path} /var/quarantine/")
        return False
    return True


def archive_documents(paths: list[str], archive_path: str) -> None:
    subprocess.run(
        ["tar", "czf", archive_path, *paths],
        check=True,
        capture_output=True,
    )
    for path in paths:
        subprocess.call(["rm", "-f", path])
# hunk ends here
