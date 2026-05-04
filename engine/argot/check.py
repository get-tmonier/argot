from __future__ import annotations

import argparse
import dataclasses
import fnmatch
import itertools
import json
import os
import sys
from collections import defaultdict
from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

import pygit2
from pygments import highlight
from pygments.formatters import TerminalFormatter
from pygments.lexers import TextLexer, get_lexer_for_filename
from pygments.util import ClassNotFound

from argot.git_walk import SUPPORTED_EXTENSIONS, _extension, _resolve_shas, walk_commits
from argot.scoring.adapters.registry import get_adapter
from argot.scoring.calibration.random_hunk_sampler import (
    DEFAULT_EXCLUDE_DIRS,
    is_excluded_path,
)
from argot.scoring.evidence.formatters import format_evidence
from argot.scoring.evidence.types import Evidence, EvidenceCorpus
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

# ANSI color codes for terminal output.
_RED = "\x1b[91m"
_YELLOW = "\x1b[33m"
_CYAN = "\x1b[2;36m"
_DIM = "\x1b[2m"
_BOLD = "\x1b[1m"
_RESET = "\x1b[0m"
# Brand color — truecolor (24-bit) midpoint of argot's ochre→rust logo gradient
# #E67E45 → #A0411C (see docs/argot-mark.svg). Used to make `argot` visually
# anchor the output without painting the whole rainbow.
_BRAND = "\x1b[1;38;2;195;95;48m"

# Default number of hunk-body lines shown under each above-threshold hit.
_DEFAULT_HUNK_LINES = 6
# Severity tier ordering, weakest first. Used by the --min-severity filter and
# anywhere we need to compare two tiers.
_SEVERITY_ORDER = ("unusual", "suspicious", "foreign")

# Source labels for the four patch origins. Commit-mode sources use a short SHA
# (7 chars) so identical-looking hits at the same line in different commits stay
# visually distinguishable.
_SOURCE_WORKDIR = "workdir"
_SOURCE_STAGED = "staged"
_SOURCE_UNTRACKED = "untracked"


class _HunkLike(Protocol):
    """Structural interface required from diff hunk objects."""

    new_start: int
    new_lines: int


@dataclass
class _SyntheticHunk:
    """Full-file hunk synthesized for untracked files (no real diff available)."""

    new_start: int
    new_lines: int


@dataclass(frozen=True)
class _PatchBatch:
    """One file's diff in a single source (workdir/staged/untracked/commit).

    `source` is "workdir" / "staged" / "untracked" for working-tree origins, or
    a 7-char commit SHA when the batch came from a committed change. It is
    propagated all the way to the rendered output so users can tell two
    same-line same-score hits from different commits apart.
    """

    file_path: str
    content: bytes
    hunks: Sequence[_HunkLike]
    source: str


@dataclass(frozen=True)
class _Hit:
    """One above-threshold hunk plus everything needed to explain it.

    `reason` is the scorer's verdict ("bpe" / "call_receiver" / "import" /
    "none") — what triggered the score. `hunk_content` is the post-image text
    of the hunk, used to render a few lines of context under each hit.
    `line` and `line_end` bound the hunk in the post-image file. `evidence`
    is the per-reason payload built by the scorer's collector (``None`` when
    the scorer was constructed without an :class:`EvidenceCorpus`).
    """

    score: float
    file_path: str
    line: int
    line_end: int
    source: str
    reason: str
    hunk_content: str
    evidence: Evidence | None = None


# User-facing translations of the scorer's internal `reason` codes. The raw
# code is kept in parentheses on the headline so power users can still trace
# which scorer fired without forcing every reader to learn the vocabulary.
_REASON_LABEL = {
    "bpe": "rare token sequence",
    "call_receiver": "unfamiliar callee",
    "import": "foreign import",
}


def _supports_color() -> bool:
    """Return True when the terminal likely supports ANSI color."""
    return os.environ.get("NO_COLOR") is None and sys.stdout.isatty()


def _modified_patches(repo_path: str) -> Iterator[_PatchBatch]:
    """Yield batches for unstaged changes vs the index (source="workdir").

    Calls repo.diff() with no arguments, which compares the working directory
    against the current index — only unstaged hunks, not staged-only changes.
    """
    repo = pygit2.Repository(repo_path)
    repo.index.read()
    try:
        diff = repo.diff()  # workdir vs index
    except pygit2.GitError:
        return
    diff.find_similar()
    workdir = Path(repo.workdir)
    for patch in diff:
        if patch is None:
            continue
        file_path = patch.delta.new_file.path
        if _extension(file_path) not in SUPPORTED_EXTENSIONS:
            continue
        hunks: Sequence[_HunkLike] = list(patch.hunks)
        if not hunks:
            continue
        full_path = workdir / file_path
        if not full_path.exists():
            continue
        yield _PatchBatch(file_path, full_path.read_bytes(), hunks, _SOURCE_WORKDIR)


def _staged_patches(repo_path: str) -> Iterator[_PatchBatch]:
    """Yield batches for staged changes vs HEAD (source="staged").

    Calls repo.diff(a='HEAD', cached=True), which compares the index against
    the HEAD commit tree — only staged hunks, not unstaged working-directory
    changes. Content is read from the index blob, not the working directory.
    """
    repo = pygit2.Repository(repo_path)
    repo.index.read()
    try:
        # repo.diff(a='HEAD', cached=True) → Case 3: HEAD tree diff_to_index
        # This is equivalent to `git diff --cached` (staged changes vs HEAD).
        diff = repo.diff(a="HEAD", cached=True)
    except (pygit2.GitError, KeyError):
        # No HEAD (empty repo) or HEAD ref not found — no staged changes to show.
        return
    diff.find_similar()
    for patch in diff:
        if patch is None:
            continue
        file_path = patch.delta.new_file.path
        if _extension(file_path) not in SUPPORTED_EXTENSIONS:
            continue
        hunks: Sequence[_HunkLike] = list(patch.hunks)
        if not hunks:
            continue
        # Read content from the index blob, not the working directory.
        try:
            entry = repo.index[file_path]
            blob = repo.get(entry.id)
            if not isinstance(blob, pygit2.Blob):
                continue
            content = blob.data
        except KeyError:
            continue
        yield _PatchBatch(file_path, content, hunks, _SOURCE_STAGED)


def _untracked_patches(repo_path: str) -> Iterator[_PatchBatch]:
    """Yield batches for untracked supported files (source="untracked").

    Walks repo.status() for files with the WT_NEW flag. Synthesizes a single
    hunk spanning the entire file (new_start=1, new_lines=line_count).
    Files with unsupported extensions are skipped.
    """
    repo = pygit2.Repository(repo_path)
    workdir = Path(repo.workdir)
    wt_new = pygit2.enums.FileStatus.WT_NEW
    for file_path, flags in repo.status().items():
        if not (flags & wt_new):
            continue
        if _extension(file_path) not in SUPPORTED_EXTENSIONS:
            continue
        full_path = workdir / file_path
        if not full_path.exists():
            continue
        content = full_path.read_bytes()
        line_count = len(content.splitlines())
        if line_count == 0:
            continue
        yield _PatchBatch(
            file_path,
            content,
            [_SyntheticHunk(new_start=1, new_lines=line_count)],
            _SOURCE_UNTRACKED,
        )


def _chain_workdir_patches(repo_path: str) -> Iterator[_PatchBatch]:
    """Chain modified, staged, and untracked patches for the full workdir view."""
    return itertools.chain(
        _modified_patches(repo_path),
        _staged_patches(repo_path),
        _untracked_patches(repo_path),
    )


def _committed_patches(repo_path: str, shas: set[str]) -> Iterator[_PatchBatch]:
    """Yield batches for committed changes (source=short commit SHA, 7 chars).

    The per-batch SHA disambiguates same-line same-score hits across commits in
    a multi-commit range — without it the user sees apparent duplicates.
    """
    for commit, file_path, post_blob, hunks in walk_commits(repo_path, shas):
        yield _PatchBatch(file_path, post_blob, hunks, str(commit.id)[:7])


def _apply_filters(file_paths: list[str], only: list[str], exclude: list[str]) -> list[str]:
    """Return file_paths that pass only/exclude glob filters.

    --exclude overrides --only: a path matching both is excluded.
    If only is non-empty, a path must match at least one glob to pass.
    Uses fnmatch for shell-style glob matching.
    """
    result = []
    for fp in file_paths:
        if any(fnmatch.fnmatch(fp, pat) for pat in exclude):
            continue
        if only and not any(fnmatch.fnmatch(fp, pat) for pat in only):
            continue
        result.append(fp)
    return result


def _is_out_of_scope(file_path: str, content: bytes, repo_root: Path) -> bool:
    """Mirror the calibration corpus's file-level exclusions at check time.

    Calibration drops two file classes the scorers can't speak about meaningfully:

    1. Directory / filename exclusions via :func:`is_excluded_path` —
       ``test/``, ``docs/``, ``migrations/``, ``*.spec.*``, ``*.test.*``, etc.
    2. Data-dominant files via ``adapter.is_data_dominant`` — modules whose
       body is ≥80% top-level array / object literals (locale tables,
       fixture data, generated lookup arrays).

    Without the same gates at check time the BPE / call-receiver scorers fire
    on tokens they were never trained to recognise — test-register words
    (``expect``, ``describe``, ``actual``) and string-literal payloads in
    locale data files. Structural false positives. Symmetric scope: argot
    lints what it learned from.
    """
    if is_excluded_path(repo_root / file_path, repo_root, DEFAULT_EXCLUDE_DIRS):
        return True
    ext = _extension(file_path)
    if ext not in SUPPORTED_EXTENSIONS:
        return True
    source = content.decode("utf-8", errors="replace")
    return get_adapter(ext).is_data_dominant(source)


def _filter_patches(
    patches: Iterator[_PatchBatch],
    only: list[str],
    exclude: list[str],
    repo_root: Path,
) -> Iterator[_PatchBatch]:
    """Apply only/exclude file-path glob filters and scope filter to patches."""
    for batch in patches:
        if _is_out_of_scope(batch.file_path, batch.content, repo_root):
            continue
        if _apply_filters([batch.file_path], only, exclude):
            yield batch


def _load_phase14_scorer(argot_dir: Path) -> SequentialImportBpeScorer:
    """Load Phase 14 scorer from .argot/ artifacts."""
    repo_corpus_txt = argot_dir / "repo-corpus.txt"
    generic_baseline_json = argot_dir / "generic-baseline.json"
    config_json = argot_dir / "scorer-config.json"

    for p, msg in [
        (repo_corpus_txt, "run `argot fit` first"),
        (generic_baseline_json, "run `argot fit` first"),
        (config_json, "run `argot fit` first"),
    ]:
        if not p.exists():
            print(f"error: {p} not found — {msg}", file=sys.stderr)
            sys.exit(2)

    repo_corpus_files = [
        Path(line) for line in repo_corpus_txt.read_text().splitlines() if line.strip()
    ]
    config: dict[str, object] = json.loads(config_json.read_text())
    threshold = float(config["threshold"])  # type: ignore[arg-type]
    # Configs written before era 9 don't have these fields; use current shipping defaults.
    call_receiver_alpha = float(config.get("call_receiver_alpha", 2.0))  # type: ignore[arg-type]
    call_receiver_cap = int(config.get("call_receiver_cap", 5))  # type: ignore[call-overload]
    call_receiver_root_bonus = float(config.get("call_receiver_root_bonus", 2.0))  # type: ignore[arg-type]
    call_receiver_n_clusters = int(config.get("call_receiver_n_clusters", 8))  # type: ignore[call-overload]
    call_receiver_cluster_seed = int(config.get("call_receiver_cluster_seed", 0))  # type: ignore[call-overload]
    call_receiver_cluster_bonus = float(config.get("call_receiver_cluster_bonus", 5.0))  # type: ignore[arg-type]
    call_receiver_cluster_rare_threshold = int(  # type: ignore[call-overload]
        config.get("call_receiver_cluster_rare_threshold", 0)
    )

    # The evidence_corpus block is required from era-evidence-layer onward.
    # No back-compat for old configs (PRD: pre-prod, regenerate). Surfacing a
    # specific error keeps the failure mode obvious for users on old artefacts.
    raw_corpus = config.get("evidence_corpus")
    if not isinstance(raw_corpus, dict):
        print(
            f"error: {config_json} is missing the 'evidence_corpus' block — "
            "this calibration was produced by an older argot. Re-run "
            "`argot calibrate` to regenerate it.",
            file=sys.stderr,
        )
        sys.exit(2)
    evidence_corpus = EvidenceCorpus.from_json_dict(raw_corpus)

    return SequentialImportBpeScorer(
        repo_corpus_files=repo_corpus_files,
        bpe_generic_baseline_path=generic_baseline_json,
        bpe_threshold=threshold,
        call_receiver_alpha=call_receiver_alpha,
        call_receiver_cap=call_receiver_cap,
        call_receiver_root_bonus=call_receiver_root_bonus,
        call_receiver_n_clusters=call_receiver_n_clusters,
        call_receiver_cluster_seed=call_receiver_cluster_seed,
        call_receiver_cluster_bonus=call_receiver_cluster_bonus,
        call_receiver_cluster_rare_threshold=call_receiver_cluster_rare_threshold,
        evidence_corpus=evidence_corpus,
    )


def _score_patches_phase14(
    patches: Iterator[_PatchBatch],
    scorer: SequentialImportBpeScorer,
) -> tuple[list[_Hit], int]:
    """Score hunk patches with the production scorer.

    Returns (hits, hunk_count). The scorer's reason and the hunk's post-image
    text are captured on each hit so the renderer can show *why* a hunk scored
    high (which scorer fired) and *what* the hunk looks like (a few lines of
    context under each headline).
    """
    hits: list[_Hit] = []
    hunk_count = 0

    for batch in patches:
        try:
            file_source = batch.content.decode("utf-8", errors="replace")
        except Exception:
            continue
        file_lines = file_source.splitlines()

        for hunk in batch.hunks:
            hunk_count += 1
            hunk_start = hunk.new_start - 1
            hunk_end = hunk_start + hunk.new_lines
            if hunk_start < 0 or hunk_end > len(file_lines):
                continue

            hunk_content = "\n".join(file_lines[hunk_start:hunk_end])
            scored = scorer.score_hunk(
                hunk_content,
                file_source=file_source,
                hunk_start_line=hunk_start + 1,
                hunk_end_line=hunk_end,
            )
            # Headline ``score`` is the BPE-stage score regardless of which
            # reason won — the severity tier in :func:`_severity` is tuned to
            # the BPE log-likelihood scale, and switching to the winner's
            # native score (e.g. an import_score of 1) would put every import
            # hit in the lowest severity tier. The winning reason's name and
            # score still surface via ``scored.reason`` and the per-reason
            # evidence payload below.
            hits.append(
                _Hit(
                    score=scored.stages.bpe_score,
                    file_path=batch.file_path,
                    line=hunk.new_start,
                    line_end=hunk.new_start + hunk.new_lines - 1,
                    source=batch.source,
                    reason=scored.reason,
                    hunk_content=hunk_content,
                    evidence=scored.evidence,
                )
            )

    return hits, hunk_count


def _severity(score: float, threshold: float) -> str:
    """Classify score into a severity tier relative to the calibrated threshold.

    unusual    t ≤ score < t+0.5  (borderline — review but don't trust the call)
    suspicious t+0.5 ≤ score < t+1.5  (likely worth a look)
    foreign    score ≥ t+1.5  (high-confidence anomaly)
    """
    if score >= threshold + 1.5:
        return "foreign"
    if score >= threshold + 0.5:
        return "suspicious"
    return "unusual"


def _highlight_lines(content: str, file_path: str, use_color: bool) -> list[str]:
    """Syntax-highlight the hunk body using pygments when colors are enabled.

    Returns one rendered line per *source* line — guaranteed equal length to
    ``content.splitlines()``. Pygments' TerminalFormatter collapses leading
    empty lines, which would otherwise misalign the line-number gutter and
    eventually IndexError the renderer; we pad the front with empty strings
    to restore the count. Falls back to the raw content (no escapes) when
    colors are off or pygments has no lexer for the file's language.
    """
    raw = content.splitlines()
    if not use_color:
        return raw
    try:
        lexer = get_lexer_for_filename(file_path, stripall=False)
    except ClassNotFound:
        lexer = TextLexer()
    formatted = highlight(content, lexer, TerminalFormatter())
    # TerminalFormatter trails a newline; rstrip to avoid an empty tail.
    out = formatted.rstrip("\n").splitlines()
    # Pad leading lines pygments swallowed; defensively trim if it ever runs
    # long. Empty source lines need no color, so "" is the correct filler.
    if len(out) < len(raw):
        out = [""] * (len(raw) - len(out)) + out
    elif len(out) > len(raw):
        out = out[-len(raw) :]
    return out


def _render_hunk_body(
    content: str,
    file_path: str,
    start_line: int,
    max_lines: int | None,
    use_color: bool,
) -> tuple[list[str], int]:
    """Format the hunk body as a numbered, syntax-highlighted code block.

    Each rendered row looks like ``  689 │ <code>`` so the user can locate the
    hunk in their editor without counting. When ``max_lines`` is None, no cap
    is applied (verbose mode); when it's 0, the body is suppressed entirely;
    otherwise lines beyond ``max_lines`` are elided with a "(+N more lines)"
    footer. When use_color is False, a plain ASCII pipe is used instead of the
    box-drawing gutter so the output stays parseable in NO_COLOR / non-tty
    contexts.

    Returns (lines, overflow) where ``overflow`` is how many lines were
    elided. Callers use this to decide whether to print the "pass --verbose"
    hint at the end of the run.
    """
    if max_lines is not None and max_lines <= 0:
        return [], len(content.splitlines())
    raw_lines = content.splitlines()
    if not raw_lines:
        return [], 0
    highlighted = _highlight_lines(content, file_path, use_color)
    shown_count = len(raw_lines) if max_lines is None else min(max_lines, len(raw_lines))
    overflow = len(raw_lines) - shown_count
    gutter = f"{_DIM}│{_RESET}" if use_color else "|"
    width = len(str(start_line + shown_count - 1))
    out: list[str] = []
    for i in range(shown_count):
        ln = start_line + i
        ln_str = f"{_DIM}{ln:>{width}}{_RESET}" if use_color else f"{ln:>{width}}"
        out.append(f"  {ln_str} {gutter} {highlighted[i]}")
    if overflow > 0:
        plural = "s" if overflow != 1 else ""
        marker = f"(+{overflow} more line{plural})"
        if use_color:
            marker = f"{_DIM}{marker}{_RESET}"
        out.append(f"  {' ' * width}   {marker}")
    return out, overflow


def _dump_evidence_debug(hits: list[_Hit]) -> None:
    """Emit one JSON line to stderr per hit with the raw evidence payload.

    Maintainer-only output: paste-friendly for bug reports and bench
    validation. Each line is a self-contained JSON object — the consumer
    can ``jq`` or ``json.loads`` line-by-line without splitting on the
    rendered stdout output. Hits without evidence (no calibration corpus
    loaded, or short-circuited) emit ``null`` evidence so the line count
    stays predictable.
    """
    for hit in hits:
        record: dict[str, object] = {
            "file": hit.file_path,
            "line": hit.line,
            "line_end": hit.line_end,
            "source": hit.source,
            "score": hit.score,
            "reason": hit.reason,
            "evidence": dataclasses.asdict(hit.evidence) if hit.evidence is not None else None,
        }
        print(json.dumps(record), file=sys.stderr)


def _render_results(
    hits: list[_Hit],
    threshold: float,
    use_color: bool,
    hunk_lines: int | None = _DEFAULT_HUNK_LINES,
) -> bool:
    """Print grouped check results to stdout.

    Groups hits by file, sorted by max-score-in-file desc. Within each file
    hits are sorted by line number asc. Each hit shows source (workdir / staged
    / untracked / short SHA) and the scorer reason on its headline, followed by
    up to `hunk_lines` lines of the post-image content (None = no cap, verbose
    mode; 0 = suppress body). All entries are assumed to be at or above
    threshold.

    Returns True if any hunk body was truncated — the caller uses this to emit
    the "pass --verbose" hint after the per-file output.
    """
    tier_counts: dict[str, int] = {"foreign": 0, "suspicious": 0, "unusual": 0}
    for hit in hits:
        tier_counts[_severity(hit.score, threshold)] += 1

    total = len(hits)
    severity_color = {"foreign": _RED, "suspicious": _YELLOW, "unusual": _CYAN}
    tier_parts: list[str] = []
    for t in ("foreign", "suspicious", "unusual"):
        if tier_counts[t] == 0:
            continue
        if use_color:
            tier_parts.append(f"{tier_counts[t]} {severity_color[t]}{t}{_RESET}")
        else:
            tier_parts.append(f"{tier_counts[t]} {t}")
    brand = f"{_BRAND}argot{_RESET}" if use_color else "argot"
    banner = f"{brand} check · {total} hunk{'s' if total != 1 else ''} above threshold"
    if tier_parts:
        banner += f" ({' · '.join(tier_parts)})"
    print(banner)
    print("note: argot is a probabilistic style linter — verify before action.")
    print()

    # Group by file; sort files by max score desc, then within file by line asc.
    file_max: dict[str, float] = defaultdict(float)
    file_hits: dict[str, list[_Hit]] = defaultdict(list)
    for hit in hits:
        file_max[hit.file_path] = max(file_max[hit.file_path], hit.score)
        file_hits[hit.file_path].append(hit)

    sorted_files = sorted(file_max, key=lambda fp: file_max[fp], reverse=True)

    severity_glyph_color = {"foreign": "●", "suspicious": "◐", "unusual": "○"}
    severity_glyph_ascii = {"foreign": "!", "suspicious": "?", "unusual": "."}

    any_truncated = False
    for i, fp in enumerate(sorted_files):
        header = f"{_BOLD}{fp}{_RESET}" if use_color else fp
        print(header)

        for hit in sorted(file_hits[fp], key=lambda h: h.line):
            sev = _severity(hit.score, threshold)
            line_str = (
                f"L{hit.line}" if hit.line == hit.line_end else f"L{hit.line}-L{hit.line_end}"
            )
            friendly = _REASON_LABEL.get(hit.reason, hit.reason)
            reason_str = f"{friendly} ({hit.reason})" if friendly != hit.reason else hit.reason
            meta = f"· {hit.source} · {reason_str}"
            if use_color:
                glyph = severity_glyph_color[sev]
                color = severity_color[sev]
                print(
                    f"  {color}{glyph}  {line_str:<13} {hit.score:>6.2f}  {sev}{_RESET}"
                    f"  {_DIM}{meta}{_RESET}"
                )
            else:
                glyph = severity_glyph_ascii[sev]
                print(f"  {glyph}  {line_str:<13} {hit.score:>6.2f}  {sev}  {meta}")

            # Per-reason evidence (names + ``common here:`` orientation) sits
            # between the headline and the hunk body. Layout decisions —
            # indentation, truncation, dim wrapping — all live in
            # :mod:`scoring.evidence.formatters`; this function is only the
            # dispatcher and printer.
            if hit.evidence is not None:
                for evidence_line in format_evidence(hit.evidence, use_color=use_color):
                    print(evidence_line)

            body_lines, overflow = _render_hunk_body(
                hit.hunk_content, hit.file_path, hit.line, hunk_lines, use_color
            )
            for body_line in body_lines:
                print(body_line)
            if overflow > 0:
                any_truncated = True

        if i < len(sorted_files) - 1:
            print()  # blank line between files

    return any_truncated


def main() -> None:
    """Entry point for argot-check."""
    parser = argparse.ArgumentParser(description="Check code with argot scorer")
    parser.add_argument("repo_path")
    parser.add_argument("ref", nargs="?", default="")
    parser.add_argument("--staged", action="store_true", help="Check staged changes only")
    parser.add_argument("--unstaged", action="store_true", help="Check unstaged changes only")
    parser.add_argument("--commit", default=None, metavar="SHA", help="Check a single commit")
    parser.add_argument(
        "--only", action="append", default=[], metavar="GLOB", help="Restrict to matching files"
    )
    parser.add_argument(
        "--exclude", action="append", default=[], metavar="GLOB", help="Drop matching files"
    )
    # Undocumented escape hatch: bench/dev users may override the calibrated threshold.
    parser.add_argument("--threshold", type=float, default=None)
    parser.add_argument(
        "--argot-dir", default=".argot", help="Directory containing argot artifacts"
    )
    parser.add_argument(
        "--hunk-lines",
        type=int,
        default=_DEFAULT_HUNK_LINES,
        metavar="N",
        help=(
            f"Hunk-body lines shown under each above-threshold hit "
            f"(default {_DEFAULT_HUNK_LINES}, 0 to suppress)"
        ),
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Show full hunk contents (no truncation; overrides --hunk-lines)",
    )
    parser.add_argument(
        "--min-severity",
        choices=_SEVERITY_ORDER,
        default="unusual",
        help="Only show hits at or above this severity (default unusual = all)",
    )
    # Maintainer-only debug switch: dumps each hit's evidence dataclass as a
    # JSON line on stderr so bench validation and bug reports have a
    # mechanically grep-able record. Hidden from --help (argparse.SUPPRESS)
    # because end users have no use for it. The ``ARGOT_DEBUG_EVIDENCE=1``
    # env var works the same way for CI / hooks that can't pass flags.
    parser.add_argument(
        "--debug-evidence",
        action="store_true",
        default=False,
        help=argparse.SUPPRESS,
    )
    args = parser.parse_args()
    if not args.debug_evidence and os.environ.get("ARGOT_DEBUG_EVIDENCE") == "1":
        args.debug_evidence = True

    # Mutual exclusion validation — fail fast with a clear message.
    if args.staged and args.unstaged:
        print("error: --staged and --unstaged are mutually exclusive", file=sys.stderr)
        sys.exit(2)
    if args.commit and args.ref:
        print("error: --commit and ref positional are mutually exclusive", file=sys.stderr)
        sys.exit(2)
    if args.commit and (args.staged or args.unstaged):
        print("error: --commit is mutually exclusive with --staged/--unstaged", file=sys.stderr)
        sys.exit(2)
    if args.ref and (args.staged or args.unstaged):
        print(
            "error: ref positional is mutually exclusive with --staged/--unstaged",
            file=sys.stderr,
        )
        sys.exit(2)

    argot_dir = Path(args.argot_dir)
    scorer = _load_phase14_scorer(argot_dir)
    threshold = args.threshold if args.threshold is not None else scorer.bpe_threshold

    # Build patch source and scan description based on mode.
    patches: Iterator[_PatchBatch]
    scan_label: str

    if args.commit:
        repo = pygit2.Repository(args.repo_path)
        shas = _resolve_shas(repo, args.commit)
        if not shas:
            print(f"No commits found for {args.commit!r}", file=sys.stderr)
            sys.exit(2)
        patches = _committed_patches(args.repo_path, shas)
        scan_label = f"1 commit ({args.commit[:8]})"
    elif args.ref:
        repo = pygit2.Repository(args.repo_path)
        if ".." in args.ref:
            # Explicit range: commits only, no workdir changes.
            shas = _resolve_shas(repo, args.ref)
            if not shas:
                print(f"No commits found in range {args.ref!r}", file=sys.stderr)
                sys.exit(0)
            patches = _committed_patches(args.repo_path, shas)
            scan_label = f"{len(shas)} commit(s) ({args.ref})"
        else:
            # Bare ref: range from ref to current state, including uncommitted (matches `git diff
            # <ref>`). Commits in <ref>..HEAD plus full workdir.
            shas = _resolve_shas(repo, f"{args.ref}..HEAD")
            workdir = _chain_workdir_patches(args.repo_path)
            if shas:
                patches = itertools.chain(_committed_patches(args.repo_path, shas), workdir)
                scan_label = f"workdir + {len(shas)} commit(s) since {args.ref}"
            else:
                patches = workdir
                scan_label = f"workdir (no commits since {args.ref})"
    elif args.staged:
        patches = _staged_patches(args.repo_path)
        scan_label = "staged changes"
    elif args.unstaged:
        patches = _modified_patches(args.repo_path)
        scan_label = "unstaged changes"
    else:
        # Default: all workdir changes (modified + staged + untracked).
        patches = _chain_workdir_patches(args.repo_path)
        scan_label = "workdir"

    filtered = _filter_patches(patches, args.only, args.exclude, Path(args.repo_path))
    hits, hunk_count = _score_patches_phase14(filtered, scorer)

    above_threshold = [h for h in hits if h.score >= threshold]

    # --min-severity drops weaker tiers from both the rendered output AND the
    # banner counts so the summary reflects what the user actually sees. With
    # the default (unusual) this is a no-op.
    min_idx = _SEVERITY_ORDER.index(args.min_severity)
    visible = [
        h
        for h in above_threshold
        if _SEVERITY_ORDER.index(_severity(h.score, threshold)) >= min_idx
    ]

    if not visible:
        exts = " ".join(sorted(SUPPORTED_EXTENSIONS))
        if hunk_count == 0:
            print(
                f"No changes to supported files found ({scan_label} scanned).\n"
                f"Supported extensions: {exts}"
            )
        elif above_threshold:
            # Hits exist but were filtered out by --min-severity.
            print(
                f"All {len(above_threshold)} hit(s) below severity '{args.min_severity}' "
                f"— pass a lower --min-severity to see them."
            )
        else:
            print(f"All {hunk_count} hunk(s) scored below threshold {threshold:.2f} — looks clean.")
        sys.exit(0)

    use_color = _supports_color()
    hunk_lines = None if args.verbose else args.hunk_lines
    if args.debug_evidence:
        _dump_evidence_debug(visible)
    any_truncated = _render_results(visible, threshold, use_color, hunk_lines)

    if any_truncated and not args.verbose:
        # Surface the escape hatch right where the user noticed it was missing.
        hint = "tip: pass --verbose (-v) to expand truncated hunks."
        print()
        print(f"{_DIM}{hint}{_RESET}" if use_color else hint)

    sys.exit(1)


if __name__ == "__main__":
    main()
