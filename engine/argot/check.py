from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Iterator
from pathlib import Path

import pygit2

from argot.git_walk import SUPPORTED_EXTENSIONS, _extension, walk_commits
from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer

_FILE_COL_WIDTH = 55


def _trunc(fp: str, width: int = _FILE_COL_WIDTH) -> str:
    """Truncate a file path from the left to fit within width chars."""
    return fp if len(fp) <= width else "..." + fp[-(width - 3) :]


def _resolve_shas(repo: pygit2.Repository, ref: str) -> set[str]:
    """Parse a git range (A..B or bare ref) into a set of commit SHAs."""
    if ".." in ref:
        start_ref, end_ref = ref.split("..", 1)
    else:
        start_ref = ref + "^"
        end_ref = ref

    end_oid = repo.revparse_single(end_ref).id
    try:
        start_oid = repo.revparse_single(start_ref).id
    except (pygit2.GitError, KeyError):
        start_oid = None

    shas: set[str] = set()
    for commit in repo.walk(end_oid, pygit2.enums.SortMode.TOPOLOGICAL):
        if start_oid is not None and commit.id == start_oid:
            break
        shas.add(str(commit.id))
    return shas


def _workdir_patches(
    repo_path: str,
) -> Iterator[tuple[str, bytes, list[pygit2.DiffHunk]]]:
    """Yield (file_path, content, hunks) for uncommitted changes vs HEAD."""
    repo = pygit2.Repository(repo_path)
    head_oid = repo.revparse_single("HEAD").id
    diff = repo.diff(a=head_oid)
    diff.find_similar()
    workdir = Path(repo.workdir)
    for patch in diff:
        if patch is None:
            continue
        file_path = patch.delta.new_file.path
        if _extension(file_path) not in SUPPORTED_EXTENSIONS:
            continue
        hunks = list(patch.hunks)
        if not hunks:
            continue
        full_path = workdir / file_path
        if not full_path.exists():
            continue
        yield file_path, full_path.read_bytes(), hunks


def _load_phase14_scorer(argot_dir: Path) -> SequentialImportBpeScorer:
    """Load Phase 14 scorer from .argot/ artifacts."""
    model_a_txt = argot_dir / "model_a.txt"
    model_b_json = argot_dir / "model_b.json"
    config_json = argot_dir / "scorer-config.json"

    for p, msg in [
        (model_a_txt, "run argot-train first"),
        (model_b_json, "run argot-train first"),
        (config_json, "run argot-calibrate first"),
    ]:
        if not p.exists():
            print(f"error: {p} not found — {msg}", file=sys.stderr)
            sys.exit(2)

    model_a_files = [Path(line) for line in model_a_txt.read_text().splitlines() if line.strip()]
    config: dict[str, object] = json.loads(config_json.read_text())
    threshold = float(config["threshold"])  # type: ignore[arg-type]
    call_receiver_alpha = float(config.get("call_receiver_alpha", 1.0))  # type: ignore[arg-type]
    call_receiver_cap = int(config.get("call_receiver_cap", 5))  # type: ignore[call-overload]

    return SequentialImportBpeScorer(
        model_a_files=model_a_files,
        bpe_model_b_path=model_b_json,
        bpe_threshold=threshold,
        call_receiver_alpha=call_receiver_alpha,
        call_receiver_cap=call_receiver_cap,
    )


def _score_patches_phase14(
    patches: Iterator[tuple[str, bytes, list[pygit2.DiffHunk]]],
    scorer: SequentialImportBpeScorer,
    label: str,
) -> tuple[list[tuple[float, str, int, str]], int]:
    """Score hunk patches with Phase 14 scorer."""
    results: list[tuple[float, str, int, str]] = []
    hunk_count = 0

    for file_path, post_blob, hunks in patches:
        try:
            file_source = post_blob.decode("utf-8", errors="replace")
        except Exception:
            continue
        file_lines = file_source.splitlines()

        for hunk in hunks:
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
            score = float(scored["bpe_score"])
            results.append((score, file_path, hunk.new_start, label))

    return results, hunk_count


def main() -> None:
    parser = argparse.ArgumentParser(description="Check code with argot scorer")
    parser.add_argument("repo_path")
    parser.add_argument("ref", nargs="?", default="")
    parser.add_argument("--threshold", type=float, default=None)
    parser.add_argument(
        "--argot-dir",
        default=".argot",
        help="Directory containing argot artifacts",
    )
    args = parser.parse_args()

    argot_dir = Path(args.argot_dir)
    scorer = _load_phase14_scorer(argot_dir)
    threshold = args.threshold if args.threshold is not None else scorer.bpe_threshold

    if args.ref == "":
        patches: Iterator[tuple[str, bytes, list[pygit2.DiffHunk]]] = _workdir_patches(
            args.repo_path
        )
        context_label = "workdir"
        commit_info = "working tree"
    else:
        repo = pygit2.Repository(args.repo_path)
        shas = _resolve_shas(repo, args.ref)
        if not shas:
            print("No commits found in range", file=sys.stderr)
            sys.exit(0)

        def _committed_patches() -> Iterator[tuple[str, bytes, list[pygit2.DiffHunk]]]:
            for _commit, file_path, post_blob, hunks in walk_commits(args.repo_path, shas):
                yield file_path, post_blob, hunks

        patches = _committed_patches()
        context_label = args.ref
        commit_info = f"{len(shas)} commit(s)"

    results, hunk_count = _score_patches_phase14(patches, scorer, context_label)

    if not results:
        if hunk_count == 0:
            exts = " ".join(sorted(SUPPORTED_EXTENSIONS))
            print(
                f"No changes to supported files found ({commit_info} scanned).\n"
                f"Supported extensions: {exts}"
            )
            if args.ref != "":
                print("Try a wider range, e.g.: argot check HEAD~20..HEAD")
        else:
            print(
                f"All {hunk_count} hunk(s) scored below threshold {threshold:.2f}" " — looks clean."
            )
        sys.exit(0)

    results.sort(key=lambda r: r[0], reverse=True)

    t = threshold
    col_w = _FILE_COL_WIDTH
    print(f"{'SURPRISE':>9}  {'TAG':<10}  {'FILE':<{col_w}}  {'LINE':>5}  REF")
    for score, fp, line, ref in results:
        if score <= t:
            tag = "ok"
        elif score <= t + 0.3:
            tag = "unusual"
        elif score <= t + 0.6:
            tag = "suspicious"
        else:
            tag = "foreign"
        print(f"{score:>9.4f}  {tag:<10}  {_trunc(fp):<{col_w}}  {line:>5}  {ref}")

    if any(s > threshold for s, *_ in results):
        sys.exit(1)


if __name__ == "__main__":
    main()
