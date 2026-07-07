#!/usr/bin/env python3
"""Clean-commit (temporal-holdout) false-positive bench for the semantic channel.

Mirrors argot-bench's holdout discipline (#92), focused on F1/F2:
  1. fit the index at an OLD commit (`window` first-parent steps behind HEAD);
  2. replay every non-merge commit STRICTLY AFTER the fit point through the real
     `argot check --commit` path (leak-free: replayed code is not in the fit tree);
  3. count `redundant` / `misplaced` fires. These are FALSE POSITIVES — a real
     developer commit is real new work, not a reinvention of existing code — so
     the fire rate is an honest ceiling on the semantic layer's false-alarm rate.

This is the RIGHT FP metric (LOO self-fire conflates genuine internal duplication
with false alarms). Usage: sem_fp.py <corpus_repo> [--window N]
Env: ARGOT (semantic binary), ARGOT_SEMANTIC_MODEL (gguf path).
"""
import json, os, subprocess, sys

ARGOT = os.environ.get("ARGOT", "/Users/damienmeur/projects/argot/target/release/argot")
REPO = sys.argv[1].rstrip("/")
WINDOW = 200
for a in sys.argv[2:]:
    if a.startswith("--window"):
        WINDOW = int(a.split("=")[-1] if "=" in a else sys.argv[sys.argv.index(a) + 1])


def git(*args):
    return subprocess.run(["git", "-C", REPO, *args], capture_output=True, text=True).stdout.strip()


def main():
    head = git("rev-parse", "HEAD")
    # fit point = WINDOW first-parent steps back; replay set = non-merge commits in fit..head
    fit_point = git("rev-parse", f"HEAD~{WINDOW}")
    if not fit_point:
        print("not enough history", file=sys.stderr); sys.exit(1)
    replay = git("rev-list", "--no-merges", "--reverse", f"{fit_point}..{head}").split()
    if not replay:
        print("no replay commits", file=sys.stderr); sys.exit(1)

    # fit at the OLD commit (detached), build the semantic index on that tree.
    # .argot is gitignored, so a clean checkout is unaffected by prior fits.
    git("checkout", "-q", "-f", fit_point)
    subprocess.run([ARGOT, "fit", "--repo", REPO], capture_output=True, text=True)

    sem_index = os.path.exists(os.path.join(REPO, ".argot", "semantic-index.json"))
    redundant = misplaced = commits_with_fp = scanned = total_hits = hunks_scanned = 0
    fires = []
    for c in replay:
        out = subprocess.run([ARGOT, "check", "--repo", REPO, "--commit", c, "--format", "json"],
                             capture_output=True, text=True)
        try:
            doc = json.loads(out.stdout)
        except json.JSONDecodeError:
            continue
        scanned += 1
        hunks_scanned += doc.get("hunks_scanned", 0)
        hit_red = hit_mis = 0
        total_hits += len(doc.get("hits", []))
        for h in doc.get("hits", []):
            r = h.get("reason")
            if r == "redundant":
                hit_red += 1
                fires.append((c[:9], "redundant", h.get("path", ""), " ".join(h.get("evidence", []))[:80]))
            elif r == "misplaced":
                hit_mis += 1
                fires.append((c[:9], "misplaced", h.get("path", ""), ""))
        redundant += hit_red; misplaced += hit_mis
        if hit_red or hit_mis:
            commits_with_fp += 1

    # restore
    # restore robustly: `argot fit` dirties .gitignore + writes argot.toml, so a
    # plain checkout can fail — force, then discard those fit artifacts.
    git("checkout", "-q", "-f", head)
    git("checkout", "-q", "--", ".gitignore")
    try:
        os.remove(os.path.join(REPO, "argot.toml"))
    except OSError:
        pass

    # corpus name: parent dir when the clone lives in <corpus>/.repo
    name = os.path.basename(os.path.dirname(REPO)) if REPO.endswith("/.repo") else os.path.basename(REPO)
    print(f"\n== {name} — clean-commit FP (fit {fit_point[:9]}, replay {scanned} commits) ==")
    print(f"  semantic index present: {sem_index}   hunks scanned: {hunks_scanned}   total hits: {total_hits}")
    print(f"  redundant false fires: {redundant}   misplaced: {misplaced}")
    print(f"  commits with >=1 semantic FP: {commits_with_fp}/{scanned} = "
          f"{commits_with_fp/scanned:.1%}" if scanned else "  no commits")
    print(f"  redundant fires per replayed commit: {redundant/scanned:.3f}" if scanned else "")
    for c, r, p, ev in fires[:15]:
        print(f"    {c} [{r}] {p}  {ev}")
    print(json.dumps({"corpus": name, "replay_commits": scanned,
                      "redundant_fp": redundant, "misplaced_fp": misplaced,
                      "commit_fp_rate": round(commits_with_fp/scanned, 4) if scanned else None,
                      "redundant_per_commit": round(redundant/scanned, 4) if scanned else None}))


if __name__ == "__main__":
    main()
