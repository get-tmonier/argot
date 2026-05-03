"""Machine-learning stage helpers (era-14).

This package houses infrastructure for training and serving the era-14 ML
classifier — a 4th scoring stage that fires only when stages 1-3 of
SequentialImportBpeScorer return ``flagged=False``.

Phase 1 ships the feature extractor (``features.py``) plus its CLI
(``cli.py`` → ``argot-extract-features``).  No models are trained from this
package directly; downstream phases will consume the JSONL produced here.
"""

from __future__ import annotations
