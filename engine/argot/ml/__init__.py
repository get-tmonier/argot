"""Machine-learning helpers — research surface.

This package houses research-only infrastructure for the engineered-
feature extractor and embedding utilities that the benchmark and the
research scripts under ``engine/scripts/`` consume.  None of it is
exercised by the production scoring path.

The production scorer (``SequentialImportBpeScorer``) is in
``argot.scoring``.

The module also exports :func:`features.synthesize_hunk_in_host`, used
by the bench's catalog-fixture routing to splice a catalog hunk into
its real host file before scoring.
"""

from __future__ import annotations
