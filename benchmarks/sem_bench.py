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
# Where to plant the reimpl files (relative to the repo root). Default is a
# `_sembench/` dir at the repo root; override with --plant-dir for repos whose
# root .gitignore ignores everything except a source subtree (e.g. homebrew's
# `/*` + `!/Library`), which would otherwise hide root-level planted files from
# `argot check` (0 hunks scanned). The planted dir must sit inside the tracked
# source tree but is still cross-file from every fixture's target.
PLANT_REL = "_sembench"
_args = sys.argv[3:]
for i, a in enumerate(_args):
    if a.startswith("--lang="):
        LANG = a.split("=", 1)[1]
    elif a == "--lang" and i + 1 < len(_args):
        LANG = _args[i + 1]
    elif a.startswith("--plant-dir="):
        PLANT_REL = a.split("=", 1)[1]
    elif a == "--plant-dir" and i + 1 < len(_args):
        PLANT_REL = _args[i + 1]

BENCH_DIR = os.path.join(CORPUS, PLANT_REL)

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


# Per-language file extension + a best-effort "first definition name" regex.
# The regex only powers the same-name neutralisation edge case (below); fixtures
# are authored with a synonym name, so it rarely fires. Extension is what matters:
# argot detects language by extension, so the planted file must carry the right one.
LANG_SPEC = {
    "py":     (".py",   r"^\s*def\s+(\w+)\s*\("),
    "ts":     (".ts",   r"(?:function\s+(\w+)|const\s+(\w+)\s*=)"),
    "js":     (".js",   r"(?:function\s+(\w+)|(?:const|let|var)\s+(\w+)\s*=|(?:exports|module\.exports|\w+(?:\.\w+)+)\.(\w+)\s*=\s*function)"),
    "go":     (".go",   r"func\s+(?:\([^)]*\)\s*)?(\w+)\s*\("),
    "rust":   (".rs",   r"fn\s+(\w+)\s*[<(]"),
    "c":      (".c",    r"^\s*(?:[\w\*\s]+?)\b(\w+)\s*\([^;]*\)\s*\{"),
    "cpp":    (".cc",   r"^\s*(?:[\w\*\s:<>,]+?)\b(\w+)\s*\([^;]*\)\s*\{"),
    "java":   (".java", r"(?:public|private|protected|static|final|\s)+[\w\<\>\[\]]+\s+(\w+)\s*\("),
    "csharp": (".cs",   r"(?:public|private|protected|internal|static|virtual|override|\s)+[\w\<\>\[\]]+\s+(\w+)\s*\("),
    "php":    (".php",  r"function\s+(\w+)\s*\("),
    "ruby":   (".rb",   r"^\s*def\s+(?:self\.)?(\w+)"),
}


def rename_fn(code, new_name):
    """Rename the (first) top-level def to new_name; return (code, orig_name)."""
    pattern = LANG_SPEC.get(LANG, LANG_SPEC["ts"])[1]
    m = re.search(pattern, code, re.M)
    if not m:
        return code, None
    orig = next(g for g in m.groups() if g)
    return code.replace(orig, new_name, 1), orig


def main():
    ext = LANG_SPEC.get(LANG, LANG_SPEC["ts"])[0]
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
        # Keep the fixture's own function name — a real agent reinvention has a
        # MEANINGFUL synonym (the agents renamed to one), and that name carries
        # domain vocabulary that is legitimate reinvention signal. Renaming to
        # "reinvented_N" (the old behaviour) wiped it and made the test unrealistic.
        # Only neutralise the name when the agent left it identical to the original,
        # which would otherwise hit the same-name move gate.
        _, cur_name = rename_fn(body, "_probe_")
        if cur_name and cur_name.lower() == orig_from_name.lower():
            code, _ = rename_fn(body, f"{orig_from_name}_alt")
        else:
            code = body
        open(os.path.join(BENCH_DIR, f"reinv_{i}{ext}"), "w").write(code.strip() + "\n")
        planted.append((f"reinv_{i}", orig_from_name))

    n = len(planted)
    # one check over all planted new files (untracked → new functions)
    out = subprocess.run([ARGOT, "check", "--repo", CORPUS, "--format", "json"],
                         capture_output=True, text=True)
    # A silently-degraded semantic pass (model failed to load under GPU
    # pressure) would record 0 fires as "0% recall" — that is a measurement
    # failure, not a result. Fail loudly instead.
    if "[argot] semantic model" in (out.stderr or "") or "semantic embedding failed" in (out.stderr or ""):
        print("SEMANTIC DEGRADED during check:", out.stderr[-300:], file=sys.stderr)
        shutil.rmtree(BENCH_DIR, ignore_errors=True)
        sys.exit(1)
    try:
        doc = json.loads(out.stdout)
    except json.JSONDecodeError:
        print("check failed:", out.stderr[:400], file=sys.stderr)
        shutil.rmtree(BENCH_DIR, ignore_errors=True)
        sys.exit(1)

    fired = {}  # fixture_id -> evidence line
    for h in doc.get("hits", []):
        if h.get("rule") != "redundant":
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
