#!/usr/bin/env python3
"""Turn a sweep's `redundant` fires into a self-contained judging pack.

The published false-alarm number for `redundant` is not the raw fire rate — a
real repo contains real duplication, so some fires are correct. The judged rate
comes from adjudicating each fire: is the new function genuinely a reinvention
of the one argot cites, or a false alarm?

That adjudication is only valid for the embedder whose fires were judged, so
the pack carries the sweep's model identity and the label file written from it
must carry the same string (`sem_consolidate.py` refuses a mismatch).

This script does the mechanical half: for every `redundant` fire it reads both
function bodies out of git at the fire's own commit, so a judge sees the two
pieces of code and nothing else it would have to go looking for.

  benchmarks/sem_extract_fires.py <corpus> [<corpus> ...] [--out DIR]

Reads  benchmarks/results/sem_fires/<corpus>.json
Writes <out>/<corpus>.json — {corpus, model, cases: [{id, new, matched, ...}]}
"""
import argparse, json, os, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FIRES = os.path.join(ROOT, "benchmarks", "results", "sem_fires")
DATA = os.path.join(ROOT, "benchmarks", "data")
# A function long enough to need eliding is long enough that its head and tail
# still decide the question; judges read code, not line counts.
MAX_LINES = 120


def show(repo, sha, path, start, end):
    """The lines of `path` at `sha`, 1-indexed and inclusive. `None` when the
    blob isn't there — a fire whose file was renamed away is not judgeable and
    must be reported as such, not silently dropped."""
    r = subprocess.run(["git", "-C", repo, "show", f"{sha}:{path}"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        return None
    lines = r.stdout.splitlines()
    if start is None:
        return None
    end = end or start
    body = lines[max(0, start - 1):end]
    if len(body) > MAX_LINES:
        keep = MAX_LINES // 2
        body = body[:keep] + [f"        … {len(body) - MAX_LINES} lines elided …"] + body[-keep:]
    return "\n".join(body)


def function_at(repo, sha, path, line):
    """The matched function's body. The index records its start line only, so
    take a fixed window — enough to judge, bounded so a judging pack stays
    readable."""
    return show(repo, sha, path, line, (line or 0) + 40)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("corpora", nargs="+")
    ap.add_argument("--out", default=os.path.join(ROOT, "benchmarks", "results", "sem_judge"))
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)

    for corpus in args.corpora:
        src = os.path.join(FIRES, f"{corpus}.json")
        if not os.path.isfile(src):
            print(f"{corpus}: no fires file at {src}", file=sys.stderr)
            continue
        d = json.load(open(src))
        repo = os.path.join(DATA, corpus, ".repo")
        cases, unreadable = [], 0
        for i, f in enumerate(x for x in d.get("fires", []) if x.get("reason") == "redundant"):
            sha = f["commit"]
            new = show(repo, sha, f["new_fn_path"], f.get("new_fn_line"), f.get("new_fn_line_end"))
            old = function_at(repo, sha, f.get("matched_path"), f.get("matched_line"))
            if new is None or old is None:
                unreadable += 1
            cases.append({
                "id": f"{corpus}-{i:03d}",
                "commit": sha,
                "similarity": f.get("similarity"),
                "evidence": f.get("evidence"),
                "new_path": f["new_fn_path"],
                "new_line": f.get("new_fn_line"),
                "new_code": new,
                "matched_symbol": f.get("matched_symbol"),
                "matched_path": f.get("matched_path"),
                "matched_line": f.get("matched_line"),
                "matched_code": old,
            })
        out = os.path.join(args.out, f"{corpus}.json")
        json.dump({"corpus": corpus, "language": d.get("language"),
                   "window": d.get("window"), "model": d.get("model"),
                   "raw_fires": d.get("redundant_fp"),
                   "unreadable": unreadable, "cases": cases},
                  open(out, "w"), indent=1)
        print(f"{corpus}: {len(cases)} cases → {out}"
              + (f"  ({unreadable} unreadable)" if unreadable else ""))


if __name__ == "__main__":
    main()
