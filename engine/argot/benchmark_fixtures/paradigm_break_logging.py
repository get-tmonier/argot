from __future__ import annotations

import json
import sys
from pathlib import Path


def process_records(path: Path, limit: int | None = None) -> int:
    count = 0
    sys.stderr.write(f"  processing {path.name}\n")
    with path.open() as fh:
        for line in fh:
            if not line.strip():
                continue
            record = json.loads(line)
            sys.stdout.write(json.dumps(record) + "\n")
            count += 1
            if limit is not None and count >= limit:
                sys.stderr.write(f"  reached limit of {limit} records\n")
                return count
    sys.stderr.write(f"  done: {count} records\n")
    return count


def validate_output(path: Path) -> bool:
    if not path.exists():
        sys.stderr.write(f"  error: output not found: {path}\n")
        return False
    sys.stderr.write(f"  validated: {path.name}\n")
    return True

import logging
logger = logging.getLogger(__name__)


def process_records_logged(path: Path, limit: int | None = None) -> int:
    count = 0
    logger.info("processing %s", path.name)
    with path.open() as fh:
        for line in fh:
            if not line.strip():
                continue
            record = json.loads(line)
            sys.stdout.write(json.dumps(record) + "\n")
            count += 1
            if limit is not None and count >= limit:
                logger.info("reached limit of %d records", limit)
                return count
    logger.info("done: %d records", count)
    return count


def validate_output_logged(path: Path) -> bool:
    if not path.exists():
        logger.error("output not found: %s", path)
        return False
    logger.info("validated: %s", path.name)
    return True
