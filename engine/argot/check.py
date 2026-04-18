from __future__ import annotations

import argparse
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import joblib  # type: ignore[import-untyped]
import pygit2
import torch

from argot.git_walk import SUPPORTED_EXTENSIONS, _extension, walk_commits
from argot.jepa.encoder import TokenEncoder
from argot.jepa.model import JEPAArgot
from argot.jepa.predictor import ArgotPredictor
from argot.tokenize import language_for_path, tokenize_lines


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


def _score_patches(
    patches: Iterator[tuple[str, bytes, list[pygit2.DiffHunk]]],
    vectorizer: Any,
    model: JEPAArgot,
    label: str,
) -> tuple[list[tuple[float, str, int, str]], int]:
    """Score hunk patches; returns (results, total_hunk_count)."""
    context_lines = 50
    results: list[tuple[float, str, int, str]] = []
    hunk_count = 0

    with torch.no_grad():
        for file_path, post_blob, hunks in patches:
            lang = language_for_path(file_path)
            if lang is None:
                continue
            try:
                source_lines = post_blob.decode("utf-8", errors="replace").splitlines()
            except Exception:
                continue

            for hunk in hunks:
                hunk_count += 1
                hunk_start = hunk.new_start - 1
                hunk_end = hunk_start + hunk.new_lines
                if hunk_start < 0 or hunk_end > len(source_lines):
                    continue

                before_start = max(0, hunk_start - context_lines)
                ctx_tokens = tokenize_lines(source_lines, lang, before_start, hunk_start)
                hunk_tokens = tokenize_lines(source_lines, lang, hunk_start, hunk_end)

                ctx_text = " ".join(t.text for t in ctx_tokens)
                hunk_text = " ".join(t.text for t in hunk_tokens)

                ctx_vec = torch.tensor(
                    vectorizer.transform([ctx_text]).toarray(), dtype=torch.float32
                )
                hunk_vec = torch.tensor(
                    vectorizer.transform([hunk_text]).toarray(), dtype=torch.float32
                )

                score = model.surprise(ctx_vec, hunk_vec).item()
                results.append((score, file_path, hunk.new_start, label))

    return results, hunk_count


def main() -> None:
    parser = argparse.ArgumentParser(description="Check code surprise with argot JEPA model")
    parser.add_argument("repo_path")
    parser.add_argument("ref", nargs="?", default="")
    parser.add_argument("--model", default=".argot/model.pkl")
    parser.add_argument("--threshold", type=float, default=0.5)
    args = parser.parse_args()

    model_path = Path(args.model)
    if not model_path.exists():
        print(f"error: model not found at {model_path}", file=sys.stderr)
        sys.exit(2)

    bundle = joblib.load(model_path)
    vectorizer = bundle["vectorizer"]
    embed_dim: int = bundle["embed_dim"]
    input_dim: int = bundle["input_dim"]

    encoder = TokenEncoder(input_dim, embed_dim)
    encoder.load_state_dict(bundle["encoder_state"])
    predictor = ArgotPredictor(embed_dim=embed_dim)
    predictor.load_state_dict(bundle["predictor_state"])
    model = JEPAArgot(encoder, predictor)
    model.eval()

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

    results, hunk_count = _score_patches(patches, vectorizer, model, context_label)

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
                f"All {hunk_count} hunk(s) scored below threshold {args.threshold:.2f}"
                " — looks clean."
            )
        sys.exit(0)

    results.sort(key=lambda r: r[0], reverse=True)

    t = args.threshold
    print(f"{'SURPRISE':>9}  {'TAG':<10}  {'FILE':<48}  {'LINE':>5}  REF")
    for score, fp, line, ref in results:
        if score <= t:
            tag = "ok"
        elif score <= t + 0.3:
            tag = "unusual"
        elif score <= t + 0.6:
            tag = "suspicious"
        else:
            tag = "foreign"
        print(f"{score:>9.4f}  {tag:<10}  {fp:<48}  {line:>5}  {ref}")

    if any(s > args.threshold for s, *_ in results):
        sys.exit(1)


if __name__ == "__main__":
    main()
