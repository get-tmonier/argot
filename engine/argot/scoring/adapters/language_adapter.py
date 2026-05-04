"""LanguageAdapter protocol — language-specific seam for the production scorer."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Protocol, runtime_checkable


@dataclass(frozen=True)
class RepoModules:
    """Resolved internal module identifiers for a repository.

    ``exact`` holds full specifiers (e.g. ``"lodash"``, ``"@myorg/lib"``).
    ``prefixes`` holds stripped glob-alias prefixes (e.g. ``"@/"`` from
    ``"@/*": ["src/*"]`` in tsconfig ``compilerOptions.paths``).
    """

    exact: frozenset[str]
    prefixes: frozenset[str]


@runtime_checkable
class LanguageAdapter(Protocol):
    """Pluggable language-specific logic for import extraction, filtering, and prose masking.

    All methods are pure (no I/O) except ``resolve_repo_modules`` which reads the repo
    on disk.  Implementations must be safe to call on syntactically invalid fragments
    (e.g. mid-block hunk slices) — return empty sets / False gracefully.
    """

    file_extensions: frozenset[str]

    def extract_imports(self, source: str) -> set[str]:
        """Return top-level (non-relative) module specifiers imported in *source*.

        Relative paths (``./foo``, ``../bar``) are always repo-internal and must NOT
        be returned.  On parse error return an empty set — never raise.
        """
        ...

    def extract_imports_with_spans(self, source: str) -> list[tuple[str, int, int, int]]:
        """Return ``(specifier, line, col_start, col_end)`` for each import.

        Same content as :meth:`extract_imports` but with the source span of
        each top-level specifier carried alongside, so the import-evidence
        renderer can annotate flagged foreign specifiers with file lines
        (``msgspec (L7)``) and underline the offending bytes with eslint-
        style carets. ``line`` is 1-indexed; ``col_start``/``col_end`` are
        0-indexed byte offsets in their line, end exclusive. On parse
        error return ``[]`` — never raise.
        """
        ...

    def resolve_repo_modules(self, repo_root: Path) -> RepoModules:
        """Return the set of known internal module names for *repo_root*.

        For Python: returns empty exact/prefixes sets (internal modules are discovered
        via ``extract_imports`` over model_A files during ``ImportGraphScorer.fit``).
        For TypeScript: reads ``package.json`` (package name + workspace names) and
        ``tsconfig.json`` ``compilerOptions.paths`` (exact aliases and glob prefixes).
        """
        ...

    def is_data_dominant(self, source: str, threshold: float = 0.65) -> bool:
        """Return True if *source* is overwhelmingly composed of static data literals.

        Must trigger on locale-style files (arrays of strings / objects in export const
        form) regardless of file path — content-based only, no path heuristics.
        """
        ...

    def is_auto_generated(self, source: str) -> bool:
        """Return True if *source* contains an auto-generation marker in its header.

        Only inspects comment nodes (not string literals) to avoid false positives.
        """
        ...

    def enumerate_sampleable_ranges(self, source: str) -> list[tuple[int, int]]:
        """Return 1-indexed (start_line, end_line) spans for top-level sampleable units.

        A sampleable unit is a top-level function/class/arrow-const that a
        calibration corpus sampler should consider.  Implementations must be safe
        to call on partial/invalid source — return ``[]`` on parse error.

        The caller applies the MIN_BODY_LINES filter; this method returns all
        top-level units regardless of body size.
        """
        ...

    def prose_line_ranges(self, source: str) -> frozenset[int]:
        """Return 1-indexed line numbers that are pure prose (comments / docstrings).

        Used to blank prose before BPE scoring so natural-language tokens do not
        inflate the score.  On parse error return an empty frozenset.
        """
        ...

    def extract_callees(self, source: str) -> list[str]:
        """Return non-None dotted-callee strings for every call-expression in *source*.

        Delegates to ``argot.scoring.scorers.call_receiver.extract_callees`` for
        the language-specific parse.  Returns ``[]`` on parse error or empty source.
        None entries (complex chains) are filtered before returning.
        """
        ...

    @property
    def identifier_noise(self) -> frozenset[str]:
        """Tokens to drop from the repo-wide identifier vocabulary.

        Used by the calibration evidence builder to keep ``common here:``
        on the BPE evidence line semantically interesting. Without this
        filter, regex-extracted identifiers are dominated by language
        keywords (``import``, ``export``, ``return``, ``const`` …) and
        boilerplate (``self``, ``this``) that the user already knows;
        they crowd out the genuinely diagnostic identifiers (the repo's
        domain vocabulary).

        This is *language-level* knowledge, not framework-level — keywords
        are part of the language grammar, so naming them here doesn't
        violate the "no framework-specific literals" rule. It's the same
        category as ``prose_line_ranges`` (knows the language's comment
        syntax) or ``extract_imports`` (knows the language's import
        syntax).

        Implementations override; the default is empty so the protocol
        stays usable in test stubs without forcing every adapter to
        define a list.
        """
        return frozenset()
