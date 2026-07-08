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
with false alarms).

Every fire is emitted as a fully structured record — enough for a downstream judge
to read BOTH functions read-only and label genuine-catch vs false-alarm:
  redundant: {commit, new_fn_path, new_fn_line[..end], matched_symbol,
              matched_path, matched_line, similarity}
  misplaced: {commit, new_fn_path, new_fn_line[..end], neighbor_area,
              actual_area, peer_symbol, peer_path, peer_line}
The reinventing (new) fn is read at `git show <commit>:<new_fn_path>` lines
new_fn_line..new_fn_line_end; the matched existing fn at
`git show <fit_sha>:<matched_path>` at matched_line. Written to --fires-out.

Usage: sem_fp.py <corpus_repo> [--window N] [--fires-out path.json] [--lang X]
Env: ARGOT (semantic binary), ARGOT_SEMANTIC_MODEL (gguf path).
"""
import json, os, re, subprocess, sys

ARGOT = os.environ.get("ARGOT", "/Users/damienmeur/projects/argot/target/release/argot")
REPO = sys.argv[1].rstrip("/")
WINDOW = 200
FIRES_OUT = None
LANG = None
_rest = sys.argv[2:]
for i, a in enumerate(_rest):
    if a.startswith("--window"):
        WINDOW = int(a.split("=")[-1] if "=" in a else _rest[i + 1])
    elif a.startswith("--fires-out"):
        FIRES_OUT = a.split("=", 1)[1] if "=" in a else _rest[i + 1]
    elif a.startswith("--lang"):
        LANG = a.split("=", 1)[1] if "=" in a else _rest[i + 1]

# "duplicates {sym} ({path}:{line}) — similarity {sim}"  (F1 F4 evidence)
_RE_RED = re.compile(r"duplicates\s+(\S+)\s+\((.+?):(\d+)\)\s+—\s+similarity\s+([\d.]+)")
# "looks like {area} code filed under {filed}"  (F2 F4 evidence, line 0)
_RE_MIS = re.compile(r"looks like\s+(.+?)\s+code filed under\s+(.+)$")
# "nearest peer: {sym} ({path}:{line})"          (F2 F4 evidence, line 1)
_RE_PEER = re.compile(r"nearest peer:\s+(\S+)\s+\((.+?):(\d+)\)")


def git(*args):
    return subprocess.run(["git", "-C", REPO, *args], capture_output=True, text=True).stdout.strip()


def parse_fire(commit, h):
    """Build a structured fire record from one JSON hit + its F4 evidence."""
    reason = h.get("reason")
    ev = h.get("evidence", []) or []
    rec = {
        "commit": commit,
        "reason": reason,
        "new_fn_path": h.get("path", ""),
        "new_fn_line": h.get("line_start"),
        "new_fn_line_end": h.get("line_end"),
        "evidence": " ".join(ev)[:200],
    }
    if reason == "redundant":
        rec["similarity"] = round(h.get("score", 0.0), 4)
        m = _RE_RED.search(" ".join(ev))
        if m:
            rec["matched_symbol"] = m.group(1)
            rec["matched_path"] = m.group(2)
            rec["matched_line"] = int(m.group(3))
    elif reason == "misplaced":
        joined = " ".join(ev)
        m = _RE_MIS.search(ev[0]) if ev else None
        if m:
            rec["neighbor_area"] = m.group(1).strip()
            rec["actual_area"] = m.group(2).strip()
        p = _RE_PEER.search(joined)
        if p:
            rec["peer_symbol"] = p.group(1)
            rec["peer_path"] = p.group(2)
            rec["peer_line"] = int(p.group(3))
    return rec


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
                fires.append(parse_fire(c, h))
            elif r == "misplaced":
                hit_mis += 1
                fires.append(parse_fire(c, h))
        redundant += hit_red; misplaced += hit_mis
        if hit_red or hit_mis:
            commits_with_fp += 1

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
    summary = {"corpus": name, "language": LANG, "window": WINDOW, "fit_sha": fit_point,
               "replay_commits": scanned, "hunks_scanned": hunks_scanned, "total_hits": total_hits,
               "redundant_fp": redundant, "misplaced_fp": misplaced,
               "commit_fp_rate": round(commits_with_fp / scanned, 4) if scanned else None,
               "redundant_per_commit": round(redundant / scanned, 4) if scanned else None,
               "misplaced_per_commit": round(misplaced / scanned, 4) if scanned else None}
    print(f"\n== {name} — clean-commit FP (fit {fit_point[:9]}, window {WINDOW}, replay {scanned} commits) ==")
    print(f"  semantic index present: {sem_index}   hunks scanned: {hunks_scanned}   total hits: {total_hits}")
    print(f"  redundant false fires: {redundant}   misplaced: {misplaced}")
    print(f"  commits with >=1 semantic FP: {commits_with_fp}/{scanned} = "
          f"{commits_with_fp/scanned:.1%}" if scanned else "  no commits")
    print(f"  redundant per replayed commit: {redundant/scanned:.3f}"
          f"   misplaced per commit: {misplaced/scanned:.3f}" if scanned else "")
    for f in fires[:15]:
        print(f"    {f['commit'][:9]} [{f['reason']}] {f['new_fn_path']}:{f.get('new_fn_line')}  {f['evidence'][:70]}")

    if FIRES_OUT:
        os.makedirs(os.path.dirname(FIRES_OUT) or ".", exist_ok=True)
        with open(FIRES_OUT, "w") as fh:
            json.dump({**summary, "fires": fires}, fh, indent=2)
        print(f"  → {len(fires)} fires written to {FIRES_OUT}")
    print(json.dumps(summary))


if __name__ == "__main__":
    main()
