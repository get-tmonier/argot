# Break: subprocess.run shells out to ImageMagick for thumbnails instead of in-process PIL
"""Break fixture — not for import."""
from __future__ import annotations

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def thumbnail_name(source_name: str, size: int) -> str:
    return f"{source_name}_thumbnail_{size}"


# hunk starts here
import shutil
import subprocess


def generate_thumbnail_via_convert(source_path: str, dest_path: str, size: int) -> str:
    convert = shutil.which("convert")
    if convert is None:
        raise RuntimeError("ImageMagick convert not found on PATH")
    result = subprocess.run(
        [convert, source_path, "-resize", f"{size}x{size}>", "-strip", dest_path],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"convert failed: {result.stderr.strip()}")
    return dest_path


def probe_image_dimensions(path: str) -> tuple[int, int]:
    out = subprocess.check_output(
        ["identify", "-format", "%w %h", path], text=True, timeout=10
    )
    width, height = out.split()
    return int(width), int(height)
# hunk ends here
