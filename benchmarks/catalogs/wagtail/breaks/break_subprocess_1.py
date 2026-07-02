# Break: subprocess.run shells out to ImageMagick for renditions instead of wagtail's Willow image pipeline
"""Break fixture — not for import."""
from __future__ import annotations

import os.path


# Decoy — idiomatic wagtail-style rendition helper, NOT inside the hunk range
def rendition_cache_key(image, filter_spec: str) -> str:
    return f"image-{image.pk}-{filter_spec}"


# hunk starts here
import shlex
import subprocess


def generate_rendition_magick(source_path: str, filter_spec: str, output_dir: str) -> str:
    width = filter_spec.split("-")[-1]
    output_path = os.path.join(
        output_dir, f"{os.path.basename(source_path)}.{filter_spec}.jpg"
    )
    cmd = (
        f"convert {shlex.quote(source_path)} -resize {shlex.quote(width)}x "
        f"-quality 85 {shlex.quote(output_path)}"
    )
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"convert failed: {result.stderr.strip()}")
    return output_path


def image_dimensions_magick(source_path: str) -> tuple[int, int]:
    output = subprocess.check_output(
        ["identify", "-format", "%w %h", source_path], text=True
    )
    width, height = output.split()
    return int(width), int(height)
# hunk ends here
