# Break: os.system shell-outs with string-formatted commands for file handling
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def export_file_name(prefix: str, extension: str) -> str:
    return f"{prefix}_export.{extension}"


# hunk starts here
import os


def compress_export_file(path: str) -> str:
    exit_code = os.system(f"gzip -f {path}")
    if exit_code != 0:
        raise RuntimeError(f"gzip failed with code {exit_code}")
    return f"{path}.gz"


def archive_media_directory(media_root: str, out_name: str) -> None:
    os.system(f"tar -czf /tmp/{out_name}.tar.gz {media_root}")
    os.system(f"chmod 644 /tmp/{out_name}.tar.gz")


def purge_old_exports(directory: str, days: int = 7) -> None:
    os.system(f"find {directory} -name '*.gz' -mtime +{days} -delete")
# hunk ends here
