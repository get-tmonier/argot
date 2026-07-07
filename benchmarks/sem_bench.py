#!/usr/bin/env python3
"""Semantic-layer bench runner (definitive run, CLI-driven = production path).

Reinvention (F1): plant each spec-only reimpl (renamed → a new name, in a new
file so it's cross-file) into the corpus, run the real `argot check`, and count
`redundant` fires → recall. Faithful: exercises the shipped binary end to end.

Usage: sem_bench.py <corpus_repo> <reimpls_dir> [--lang py|ts]
Env: ARGOT (binary), ARGOT_SEMANTIC_MODEL (gguf path).
"""
import json, os, re, subprocess, sys, shutil, tempfile

ARGOT = os.environ.get("ARGOT", "/Users/damienmeur/projects/argot/target/release/argot")
CORPUS = sys.argv[1]
REIMPLS = sys.argv[2]
LANG = "py"
for a in sys.argv[3:]:
    if a.startswith("--lang"):
        LANG = a.split("=")[-1] if "=" in a else "py"

BENCH_DIR = os.path.join(CORPUS, "_sembench")

# argot:recommended excluded top-dirs — a fixture whose ORIGINAL target lives
# here can never validly fire (the index no longer contains it), so it's not a
# valid reinvention fixture. Skip it rather than count it as a miss.
_EXCLUDED_DIRS = {"test", "tests", "testdata", "testing", "__tests__", "doc", "docs",
                  "example", "examples", "migration", "migrations", "benchmark",
                  "benchmarks", "fixtures", "scripts", "build", "dist"}


def target_excluded(raw):
    """True if the fixture's `# ID: <path>:<line>` target sits in an excluded dir."""
    m = re.search(r"#\s*ID:\s*(\S+?):", raw)
    if not m:
        return False
    return any(part in _EXCLUDED_DIRS for part in m.group(1).split("/"))


def rename_fn(code, new_name):
    """Rename the (first) top-level def to new_name; return (code, orig_name)."""
    m = re.search(r"^\s*def\s+(\w+)\s*\(", code, re.M) if LANG == "py" else \
        re.search(r"(?:function\s+(\w+)|const\s+(\w+)\s*=)", code)
    if not m:
        return code, None
    orig = next(g for g in m.groups() if g)
    return code.replace(orig, new_name, 1), orig


def main():
    ext = ".py" if LANG == "py" else ".ts"
    shutil.rmtree(BENCH_DIR, ignore_errors=True)
    os.makedirs(BENCH_DIR, exist_ok=True)

    planted = []  # (fixture_id, orig_symbol)
    for i, fn in enumerate(sorted(os.listdir(REIMPLS))):
        if not fn.endswith(ext):
            continue
        raw = open(os.path.join(REIMPLS, fn)).read()
        if target_excluded(raw):  # fixture targets excluded (non-canonical) code
            continue
        # strip the "# ID: path:line" header, keep the body
        body = "\n".join(l for l in raw.splitlines() if not l.strip().startswith("# ID:"))
        orig_from_name = fn.split("__")[0]
        code, _ = rename_fn(body, f"reinvented_{i}")
        open(os.path.join(BENCH_DIR, f"reinv_{i}{ext}"), "w").write(code.strip() + "\n")
        planted.append((f"reinv_{i}", orig_from_name))

    n = len(planted)
    # one check over all planted new files (untracked → new functions)
    out = subprocess.run([ARGOT, "check", "--repo", CORPUS, "--format", "json"],
                         capture_output=True, text=True)
    try:
        doc = json.loads(out.stdout)
    except json.JSONDecodeError:
        print("check failed:", out.stderr[:400], file=sys.stderr)
        shutil.rmtree(BENCH_DIR, ignore_errors=True)
        sys.exit(1)

    fired = {}  # fixture_id -> evidence line
    for h in doc.get("hits", []):
        if h.get("reason") != "redundant":
            continue
        path = h.get("path", "")
        m = re.search(r"reinv_(\d+)", path)
        if m:
            fired[f"reinv_{m.group(1)}"] = " ".join(h.get("evidence", []))

    recall = len(fired) / n if n else 0.0
    print(f"\n===== REINVENTION (F1) — {os.path.basename(CORPUS.rstrip('/'))} =====")
    print(f"planted reimpls: {n}   fired redundant: {len(fired)}   recall: {recall:.0%}")
    for fid, orig in planted:
        mark = "🔴 fires" if fid in fired else "·  quiet"
        ev = f"  {fired[fid]}" if fid in fired else ""
        print(f"  {mark}  {fid} (reimpl of {orig}){ev}")

    shutil.rmtree(BENCH_DIR, ignore_errors=True)
    print(json.dumps({"corpus": os.path.basename(CORPUS.rstrip('/')),
                      "channel": "reinvention", "planted": n,
                      "fired": len(fired), "recall": round(recall, 3)}))


if __name__ == "__main__":
    main()
