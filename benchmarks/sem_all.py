#!/usr/bin/env python3
"""Unified semantic-layer benchmark driver — one command for every semantic sense.

Per corpus it does the minimum number of `argot fit`s and runs every semantic
measurement against them, writing ONE self-contained row per corpus:

  1. clean the clone (git reset -f, drop fit artifacts, scrub any planted dir);
  2. fit HEAD ONCE → the production index both RECALL senses share:
       F1 reinvention recall     (sem_bench.py  — plant renamed reimpls, real check)
       F2 placement recall + F2/F1 over-fire  (sem_analysis.py — index-only, numpy)
  3. clean-commit FALSE-POSITIVE (sem_fp.py — its own leak-free window fit + replay):
       F1 redundant FP + F2 misplaced FP, one structured record per fire.

Robust by design (the DX rework): every `argot fit` runs under a wall-clock timeout
with one retry; a corpus that still won't fit is recorded with an honest `error`
marker and the sweep moves on — one hung/huge corpus never stalls the run. numpy is
required (sem_analysis is vectorised; see benchmarks/requirements.txt).

The single output JSONL is the one source of truth downstream tooling reads
(consolidate.py → landing/src/data/semantic.json).

Usage:
  benchmarks/sem_all.py [corpus ...]      # named corpora (default: all with fixtures)
  benchmarks/sem_all.py --only recall|fp  # run just one half
  benchmarks/sem_all.py --window N        # FP holdout window (default 200)
  benchmarks/sem_all.py --fit-timeout S   # per-fit wall clock (default 2400s)
  benchmarks/sem_all.py --out PATH        # results JSONL (default results/sem_all.jsonl)
Env: ARGOT (binary), ARGOT_SEMANTIC_MODEL (gguf path).
"""
import argparse, json, os, subprocess, sys, time, shutil, glob

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "benchmarks", "data")
FIXT = os.path.join(ROOT, "benchmarks", "semantic-fixtures")

# corpus → source language (drives sem_bench's plant extension + the reported label)
LANG = {"fastapi": "python", "rich": "python", "faker": "python", "scrapy": "python",
        "saleor": "python", "wagtail": "python", "dagster": "python",
        "hono": "typescript", "ink": "typescript", "faker-js": "typescript",
        "excalidraw": "typescript", "outline": "typescript",
        "express": "javascript", "commander": "javascript", "eslint": "javascript",
        "gh-cli": "go", "hugo": "go", "redis": "c", "curl": "c",
        "junit5": "java", "guava": "java", "powershell": "csharp", "jellyfin": "csharp",
        "rubocop": "ruby", "homebrew": "ruby", "laravel": "php", "composer": "php",
        "ripgrep": "rust", "bat": "rust", "rocksdb": "cpp", "fmt": "cpp"}
LANG_BENCH = {"python": "py", "typescript": "ts", "javascript": "js", "go": "go",
              "rust": "rust", "c": "c", "cpp": "cpp", "java": "java",
              "csharp": "csharp", "ruby": "ruby", "php": "php"}
# corpora whose root .gitignore hides root-level planted files → plant inside src tree
PLANT_DIR = {"homebrew": "Library/Homebrew/_sembench"}


def plant_rel(corpus):
    return PLANT_DIR.get(corpus, "_sembench")


def git(repo, *a):
    return subprocess.run(["git", "-C", repo, *a], capture_output=True, text=True).stdout.strip()


def last_json(s):
    for l in reversed((s or "").strip().splitlines()):
        if l.strip().startswith("{"):
            try:
                return json.loads(l)
            except json.JSONDecodeError:
                pass
    return {}


def clean(repo, corpus):
    """Restore the clone to a pristine HEAD checkout and drop everything a prior
    fit/bench could have left behind (fit artifacts + any planted reimpl dir left
    by a killed sem_bench — those would otherwise get baked into the next voice)."""
    head = git(repo, "rev-parse", "HEAD")
    git(repo, "checkout", "-q", "-f", head)
    git(repo, "checkout", "-q", "--", ".gitignore")
    for art in ("argot.toml", ".argotignore"):
        try:
            os.remove(os.path.join(repo, art))
        except OSError:
            pass
    shutil.rmtree(os.path.join(repo, ".argot"), ignore_errors=True)
    shutil.rmtree(os.path.join(repo, plant_rel(corpus)), ignore_errors=True)
    return head


def fit(repo, env, timeout):
    """`argot fit` under a wall-clock timeout with one retry. Returns
    {ok, secs} on success (index verified present) or {ok:False, error} — never
    raises, so one huge/hung corpus can't stall the sweep."""
    idx = os.path.join(repo, ".argot", "semantic-index.json")
    err = "unknown"
    for attempt in (1, 2):
        shutil.rmtree(os.path.join(repo, ".argot"), ignore_errors=True)
        t = time.time()
        try:
            subprocess.run([env["ARGOT"], "fit", "--repo", repo],
                           capture_output=True, text=True, env=env, timeout=timeout)
        except subprocess.TimeoutExpired:
            err = f"fit exceeded {timeout}s (attempt {attempt})"
            continue
        if os.path.exists(idx):
            return {"ok": True, "secs": round(time.time() - t)}
        err = "fit produced no semantic-index.json (model offline?)"
    return {"ok": False, "error": err}


def run_step(argv, env, timeout):
    """Run a bench sub-script; return (last_json_dict, ok)."""
    try:
        r = subprocess.run(argv, capture_output=True, text=True, env=env, timeout=timeout)
    except subprocess.TimeoutExpired:
        return {}, False
    return last_json(r.stdout), True


def do_corpus(corpus, args, env, fires_dir):
    repo = os.path.join(DATA, corpus, ".repo")
    lang = LANG.get(corpus, "?")
    row = {"corpus": corpus, "language": lang, "errors": [], "secs": {}}
    if not os.path.isdir(repo):
        row["errors"].append("no clone at benchmarks/data/<corpus>/.repo")
        return row

    # ---- RECALL half: one HEAD fit shared by F1 recall + F2 recall/over-fire ----
    if args.only in (None, "recall"):
        head = clean(repo, corpus)
        row["head"] = head
        f = fit(repo, env, args.fit_timeout)
        if not f["ok"]:
            row["errors"].append(f"HEAD fit: {f['error']}")
        else:
            row["secs"]["fit_head"] = f["secs"]
            # F1 reinvention recall (plant + real check against the HEAD index)
            fixt = os.path.join(FIXT, corpus)
            bcmd = [sys.executable, os.path.join(ROOT, "benchmarks", "sem_bench.py"),
                    repo, fixt, "--lang", LANG_BENCH.get(lang, lang)]
            if corpus in PLANT_DIR:
                bcmd += ["--plant-dir", PLANT_DIR[corpus]]
            jb, ok = run_step(bcmd, env, args.step_timeout)
            row["f1"] = {"recall": jb.get("recall"), "fired": jb.get("fired"),
                         "planted": jb.get("planted")}
            if not ok or jb.get("planted") is None:
                row["errors"].append("F1 recall (sem_bench) failed/timed out")
            # F2 placement recall + over-fire, F1 over-fire (index-only)
            ja, ok = run_step([sys.executable, os.path.join(ROOT, "benchmarks", "sem_analysis.py"), repo],
                              env, args.step_timeout)
            row["f2"] = {"recall": ja.get("placement_recall"),
                         "overfire": ja.get("placement_over_fire"),
                         "n_areas": ja.get("n_areas"), "place_eval": ja.get("place_eval")}
            row["reinv_overfire"] = ja.get("reinvention_over_fire")
            row["functions"] = ja.get("functions")
            if not ok:
                row["errors"].append("F2 (sem_analysis) failed/timed out")
        clean(repo, corpus)

    # ---- FP half: sem_fp does its own leak-free window fit (separate tree) -------
    if args.only in (None, "fp"):
        fires_out = os.path.join(fires_dir, f"{corpus}.json")
        cmd = [sys.executable, os.path.join(ROOT, "benchmarks", "sem_fp.py"), repo,
               "--window", str(args.window), "--fires-out", fires_out, "--lang", lang]
        # sem_fp fits internally; give it fit_timeout + replay headroom
        jf, ok = run_step(cmd, env, args.fit_timeout + args.step_timeout)
        if not ok or jf.get("replay_commits") is None:
            row["errors"].append("FP (sem_fp) failed/timed out")
            row["fp"] = None
        else:
            row["fp"] = {"window": jf.get("window"), "fit_sha": jf.get("fit_sha"),
                         "replay_commits": jf.get("replay_commits"),
                         "hunks_scanned": jf.get("hunks_scanned"),
                         "redundant_fp": jf.get("redundant_fp"),
                         "misplaced_fp": jf.get("misplaced_fp"),
                         "redundant_per_commit": jf.get("redundant_per_commit"),
                         "misplaced_per_commit": jf.get("misplaced_per_commit")}
            row["fp_fires_file"] = fires_out
        clean(repo, corpus)
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("corpora", nargs="*", help="corpora to run (default: all with fixtures)")
    ap.add_argument("--only", choices=["recall", "fp"], default=None)
    ap.add_argument("--window", type=int, default=150)
    ap.add_argument("--fit-timeout", type=int, default=2400)
    ap.add_argument("--step-timeout", type=int, default=1800)
    ap.add_argument("--out", default=os.path.join(ROOT, "benchmarks", "results", "sem_all.jsonl"))
    args = ap.parse_args()

    env = dict(os.environ)
    env["ARGOT"] = os.path.join(ROOT, "target", "release", "argot")
    env.setdefault("ARGOT_SEMANTIC_MODEL",
                   os.path.expanduser("~/.cache/argot/models/jina-embeddings-v2-base-code-Q4_K_M.gguf"))

    corpora = args.corpora or sorted(d for d in os.listdir(FIXT)
                                     if os.path.isdir(os.path.join(FIXT, d)) and not d.startswith("."))
    os.makedirs(os.path.dirname(args.out) or ".", exist_ok=True)
    fires_dir = os.path.join(os.path.dirname(args.out), "sem_fires")
    os.makedirs(fires_dir, exist_ok=True)
    # fresh run: truncate the results file so it reflects exactly this sweep
    open(args.out, "w").close()

    print(f"sem_all: {len(corpora)} corpora, only={args.only or 'recall+fp'}, "
          f"window={args.window}, fit_timeout={args.fit_timeout}s → {args.out}", flush=True)
    t0 = time.time()
    for i, c in enumerate(corpora, 1):
        ts = time.time()
        row = do_corpus(c, args, env, fires_dir)
        with open(args.out, "a") as fh:
            fh.write(json.dumps(row) + "\n")
        f1 = (row.get("f1") or {}).get("recall")
        f2 = (row.get("f2") or {}).get("recall")
        fp = row.get("fp") or {}
        errs = f"  ERRORS: {row['errors']}" if row["errors"] else ""
        print(f"[{i}/{len(corpora)}] {c:12s} F1={f1} F2={f2} "
              f"redFP={fp.get('redundant_fp')} misFP={fp.get('misplaced_fp')} "
              f"({time.time()-ts:.0f}s){errs}", flush=True)
    print(f"\nsem_all DONE: {len(corpora)} corpora in {(time.time()-t0)/60:.1f} min → {args.out}", flush=True)


if __name__ == "__main__":
    main()
