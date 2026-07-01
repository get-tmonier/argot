# Rust Port — Porting Notes (living doc)

Behaviour-preserving port of argot (TS/Bun CLI + Python engine) → one Cargo
workspace. Parity-gated against `main`. This doc is the durable record of
what was learned during recon and the parity-critical decisions. Keep it
current — it is the map for the port.

## Workspace layout

- `Cargo.toml` (root) — workspace, resolver 2, edition 2021 (rustc 1.83).
- `crates/argot-core` — engine: git walk, tokenize, scorers, calibration,
  check. Language- and corpus-agnostic (no framework literals — CLAUDE.md).
- `crates/argot-cli` — `argot` binary (clap). Replaces the TS/Bun shell AND
  the four Python entry points. No subprocess.

Python/TS stay in-tree and shippable until Rust reaches parity, then cut over.

## Parity-critical dependency choices

| Concern | Python | Rust | Why |
|---|---|---|---|
| Git diff/hunks | pygit2 1.19.2 (libgit2 1.9.x) | **git2 0.20** (libgit2) | Same C lib ⇒ exact diff + `find_similar` parity. gix (pure-Rust) risks hunk divergence. |
| Parse trees | tree_sitter 0.23.2 | tree-sitter 0.23 | ABI must match for identical parse trees. |
| Grammars | py 0.23.6 / ts 0.23.2 / js 0.23.1 | tree-sitter-{python,typescript,javascript} 0.23.x | Grammar version drives tokenization; must match. |
| BPE | transformers tokenizer over `generic_tokens_bpe.json` | hand-rolled, port `tokenize.py`/BPE exactly | Parity-critical vocab. |
| KMeans | sklearn (call_receiver, 1 use) | port with same init/seed/iteration | Deterministic clustering. |
| RNG (calibration) | `np.random.default_rng(seed)` = **PCG64** + `Generator.choice(replace=False)` | must reproduce numpy PCG64 + choice bit-for-bit | Hardest parity item. See below. |
| roc_auc (eval) | sklearn `roc_auc_score` | port directly (Mann-Whitney w/ tie ranks) | |
| percentile | numpy `np.percentile` (linear interp) | port linear-interpolation percentile | |
| Terminal fmt | pygments | port caret/evidence formatter (no syntect needed for parity) | |
| model/config | JSON (scorer-config.json v2) | serde | No pkl compat needed (pre-prod). |

## Byte-parity gotchas (extract path)

- **JSON separators**: Python `json.dumps(x)` uses `", "` and `": "`
  separators (spaces). serde_json compact uses no spaces. Need a custom
  Python-style formatter for `dataset.jsonl` and `scorer-config.json`
  (the latter uses `indent=2`).
- **`str.splitlines()`**: CPython splits on a wide Unicode boundary set
  (`\n \r \r\n \v \f \x1c \x1d \x1e \x85    `), NOT just `\n`.
  Ported in `argot-core::text::splitlines`. Line indexing depends on it.
- **UTF-8 decode**: Python `bytes.decode("utf-8", errors="replace")` ↔
  Rust `String::from_utf8_lossy` (both substitute U+FFFD per maximal
  subpart). Low risk; verify on fixtures.
- **`tokenize_lines`**: slices `source_lines[start:end]`, `"\n".join(...)`,
  `.encode()` (utf-8), re-parses the slice, then offsets line numbers by
  `+start_line`. Leaf tokens only (`child_count == 0 and node.text`).
- **author_date_iso**: `str(commit.author.time)` — the unix timestamp int,
  stringified. Not an ISO string despite the name.
- **Extract walk**: topological sort, skip commits whose `len(parents) != 1`
  (merges + root). Supported exts `.ts .tsx .js .jsx .py`. `diff.find_similar()`
  before reading hunks. `hunk_start = new_start - 1`, `hunk_end = start +
  new_lines`; skip if out of source bounds. CONTEXT_LINES = 50.

## Pipeline artifacts

- `extract` → `.argot/dataset.jsonl` — one `HunkRecord` per line. Schema in
  `argot-core::dataset` (field order fixed for parity).
- `train` (`argot-train`) — trivial: collect source file paths (rglob,
  exclude dirs/tests) → `.argot/repo-corpus.txt` (`"\n".join`, **no trailing
  newline**); copy pre-baked `scoring/bpe/generic_tokens_bpe.json` →
  `.argot/generic-baseline.json`. Note: justfile `train` recipe is stale
  (`--out model.pkl`); real entry is `train.py:main` with `--repo`.
- `calibrate` (`argot-calibrate`) → `.argot/scorer-config.json` (v2).
- `check` (`argot-check`) → stdout report, exit 0 (no hits) / 1 (hits) / 2 (error).

## scorer-config.json v2 schema

`{ "version": 2, "languages": { "<lang>": {...} } }`. Per-language keys:
`threshold` (float), `call_receiver_alpha` 2.0, `call_receiver_cap` 5,
`call_receiver_root_bonus` 2.0, `call_receiver_n_clusters` 8,
`call_receiver_cluster_seed` 0, `call_receiver_cluster_bonus` 5.0,
`call_receiver_cluster_rare_threshold` (int), `call_receiver_cluster_size_min`
(int), `import_modules` (sorted str[]), `import_module_prefixes` (sorted
str[]), `calibration` {n_cal, seed, n_seeds, repo_sha, timestamp_utc},
`evidence_corpus` {imports[], identifiers{}, callees_by_cluster{}, totals{}}.
Written with `json.dumps(config, indent=2)`. `timestamp_utc`/`repo_sha` are
non-deterministic metadata (fine).

## PRODUCTION SCORING SURFACE (the real scope)

Production `check` + `calibrate` use exactly ONE composite scorer:
**`SequentialImportBpeScorer`** (`scorers/sequential_import_bpe.py`, 737 LOC),
which composes:
- `ImportGraphScorer` (import_graph.py, 138) — foreign-import stage.
- `CallReceiverScorer` (call_receiver.py, 665) — **sklearn KMeans** clustering.
- BPE token-surprise scoring (internal).
- `TypicalityModel` (filters/typicality.py, 280) — atypical short-circuit.
- data_dominant (163) / autogenerated (190) filters — short-circuit.
- Adapters: python_adapter (128), typescript (658), parsers/python_ts (165),
  language_adapter (protocol).

score_hunk reasons: `import | bpe | call_receiver | none | atypical |
atypical_file | auto_generated`. Multi-reason resolution: highest
`score/threshold` ratio wins; tie precedence `call_receiver > import > bpe`.
Severity: `>= t+1.5 foreign`, `>= t+0.5 suspicious`, else `unusual`.

### OFF the default production path (opt-in research/bench only)

- The 5 "shape primitive" scorers — `namespace_jsd`, `call_scope_fraction`,
  `typical_call_density`, `except_return_raise_ratio`, `fall_through_guards`
  — plus `shape_primitive*` registry. Wired ONLY via the bench/scorer-config
  `--enable-shape-primitives` flag; production scorer-config has no
  shape-primitive config, so they never run in `check`/`calibrate` defaults.
- `ml/` (embeddings, features, cli — UnixCoder/torch) — research CLI
  `argot-extract-features` only. Not production scoring.

## SCOPE DECISIONS (user, 2026-07-01)

1. **Shape-primitive scorers: PORT ALL 5** for 1:1 test parity, including the
   `shape_primitive` registry and the bench `--enable-shape-primitives`
   capability. Their `test_*.py` DO get Rust equivalents.
2. **Research/embeddings `ml/` path: DROP ENTIRELY.** No Rust port of
   `engine/argot/ml/`. `argot-extract-features` is removed from the CLI
   surface (not stubbed). `test_ml_*.py` do NOT get Rust equivalents — this
   removal is documented in `docs/research/evidence/`, not a parity failure.
   torch/transformers/UnixCoder have no place in the single static binary.

## Benchmark harness (Phase 7 oracle)

- `benchmarks/src/argot_bench` (~2.6k LOC Python). Per-corpus AUC =
  `roc_auc_score(y_true, y_score)` with catalog "breaks" as positives (1),
  real-PR "control" hunks as negatives (0). Corpora: fastapi, rich, faker,
  hono, ink, faker-js, dagster (see `benchmarks/targets.yaml`). Corpora are
  cloned under `benchmarks/data/`.
- Harness uses `argot-extract` as a subprocess (`benchmarks/.../extract.py`)
  but imports the **scorers directly as a Python library** for scoring
  (`benchmarks/.../score.py`, the single designed adapter seam). To bench
  Rust: expose a batch `score` subcommand on the Rust binary and repoint
  `score.py` at it (only that file changes — by its own docstring).

## Calibration determinism (hardest parity item)

- `sample_hunks(seed)`: `rng = np.random.default_rng(seed)`;
  `idx = rng.choice(len(candidates), size=n, replace=False)`; return
  `[candidates[i] for i in sorted(idx)]`. Candidates from
  `sorted(source_dir.rglob("*<ext>"))`, exclusions, `is_data_dominant` /
  typicality skips, `enumerate_sampleable_ranges`, `MIN_BODY_LINES = 5`.
- Threshold: per seed `np.percentile(scores, 100.0)` (== max) by default;
  final threshold = `statistics.median` over `n_seeds = 7` seeds
  (base_seed..base_seed+6).
- **PCG64 + choice**: must port numpy's PCG64 (SeedSequence init) and
  `Generator.choice(replace=False)` algorithm exactly, or parity on
  calibration thresholds fails. This is the top parity risk; isolate + unit
  test against numpy-generated vectors captured from Python.

## Progress log

- **Scaffold (Phase 2): DONE.** Workspace `argot-core` + `argot-cli` builds
  clean. Toolchain bumped to stable Rust ≥1.85 (git2→url→ICU4X needs
  edition2024); `rust-toolchain.toml` pins `stable`.
- **Extract vertical (Phase 3, part): DONE + PARITY-VERIFIED.** Modules:
  `text::splitlines`, `json` (Python formatter), `tokenize` (tree-sitter),
  `git_walk` (git2), `extract`, `dataset`. Byte-identical to Python
  `argot-extract` on: synthetic fixture (committed golden) AND real corpora
  fastapi (500), faker (400, non-ASCII), faker-js (400, TS/JS), hono (400,
  TS). CLI `argot extract` mirrors `argot-extract`.
  - Parity fix: pygit2 opens with `git_repository_open_ext(path,0,NULL)` which
    **searches parent dirs**; git2's `Repository::open` does not. Use
    `git_walk::open_repo` (open_ext, empty flags) everywhere.
  - Corpus layout: real repo is `benchmarks/data/<corpus>/.repo` (has `.git`);
    the top `<corpus>/` dir has no `.git` (resolves up to argot itself).

- **BPE tokenizer (Phase 3, part): DONE + PARITY-VERIFIED.** `bpe::BpeTokenizer`
  loads the embedded `data/unixcoder_tokenizer.json` (exported from the Python
  `transformers` fast backend; 3.4MB) via the Rust `tokenizers` crate (0.22,
  onig feature — same lib the Python 0.22.2 wraps). `encode(add_special=false)`
  is bit-identical on a 14.5k-token golden set (real Py/TS files, unicode,
  whitespace). Vocab size 51416. This is THE parity-critical scoring token
  stream — no torch, no HF hub round-trip.

- **stats + train + BPE scorer (Phase 3, part): DONE + PARITY-VERIFIED.**
  - `stats`: `percentile` (numpy linear interp) + `compute_auc` (sklearn
    roc_auc via average-rank Mann–Whitney, tie-safe) — golden-tested.
  - `train`: `collect_source_files` (train.py filters) + emits embedded
    `data/generic_tokens_bpe.json`. repo-corpus.txt order is sorted (Python's
    rglob order is FS-nondeterministic; downstream is order-independent →
    justified divergence).
  - `scoring::bpe_scorer::BpeScorer`: `token_surprise` / `bpe_score` /
    `is_meaningful_token`. Parity vs Python `_bpe_score` golden: total_repo &
    total_generic exact, surprise bit-level, per-hunk score < 1e-9.
  - `text::read_text_lossy` / `universal_newlines` for Python `read_text`
    CRLF→LF parity when reading repo-corpus files.

- **Python adapter + filters + import_graph (Phase 3): DONE + PARITY** (agent,
  9-sample golden). `scoring/adapters/python.rs`, `scoring/filters/*`,
  `scoring/import_graph.rs`. Shared `LanguageAdapter` trait + `Language` enum in
  `scoring/adapters/mod.rs` (impl for PythonAdapter).
- **call_receiver (Phase 4): DONE (deterministic parts PARITY).**
  `scoring/call_receiver.rs`: extract_callees (py+ts), MinHash (embedded seed-0
  params `minhash_params_seed0.rs`), weighted_contribution / _for_file, Jaccard
  nearest-cluster, hand-rolled k-means++ (SplitMix64, n_init=10 — AUC-fallback,
  NOT sklearn-parity). Golden-tested: callees, minhash sig, weighted_contribution
  (n_clusters=1). Cluster-affected scores gated on AUC later. md-5 dep added.
- **typicality: DONE + PARITY** (`scoring/typicality.rs`, golden-tested; also
  fixed a borrow-drop bug in call_receiver.rs:119).
- **TS adapter: DONE + PARITY** (`scoring/adapters/typescript.rs`, 938 LOC,
  impl LanguageAdapter, golden-tested incl. resolve_repo_modules).
- **sequential (Phase 4): DONE + PARITY.** `scoring/sequential.rs`
  `SequentialImportBpeScorer::from_config` + `score_hunk` (typicality→import→
  BPE→call_receiver→multi-reason). `sequential_parity` test matches the Python
  no-CR golden (bpe/import/atypical/multi-reason). `text::splitlines_keepends`
  for prose blanking. Evidence deferred (returns None).

- **calibration (Phase 5): DONE (deterministic sampler, documented divergence).**
  `scoring/calibration.rs`: candidate collection (is_excluded_path, MIN_BODY_LINES,
  sorted rglob, data-dominant skip), SplitMix64 sampler, multi-seed threshold
  (cluster_bonus folded via weighted_contribution_for_file), evidence corpus
  (imports/identifiers-regex/callees_by_cluster/totals), scorer-config.json v2
  emission. `run_calibrate(repo, repo_corpus, generic_baseline, out, opts)`.
  Smoke test: train→calibrate on the check fixture → valid v2 config (threshold,
  import_modules incl "math", evidence). Added CallReceiver getter
  `cluster_callee_counts_for_evidence`.
- **check (Phase 5): IN FLIGHT** (background agent, 3 goldens: clean/render/workdir).

- **check (Phase 5): DONE + BYTE-PARITY.** `check.rs` (run_check → CheckOutcome).
  Patch collection all modes (committed via walk_commits; workdir/staged/
  untracked via git2 diff_index_to_workdir/diff_tree_to_index/statuses).
  Byte-identical on 3 goldens (clean/render/workdir) + exit codes 0/1/2. Subtle:
  file sort seeds file_max at 0.0 (defaultdict) → all-negative scores tie → stable
  first-appearance order. hit.score = stages.bpe_score, file_path=None (no KMeans).
  Evidence lines + ANSI color path DEFERRED (goldens are NO_COLOR/plain).
- **CLI (Phase 6, part): DONE for pipeline.** `argot extract|train|calibrate|fit|
  check` all wired. **FULL PIPELINE RUNS END-TO-END via the single binary,
  matching Python**: train (6 files), calibrate (threshold 0.4856 — IDENTICAL to
  Python: small repos sample all candidates so the RNG divergence vanishes), check
  (byte-identical render). Remaining CLI: `status`/`list`/`update` (repo registry
  infra), user-facing `argot check` branded header + no-subcommand help banner.

**STATUS: engine essentially COMPLETE.** Extract, train, tokenize, BPE, stats,
git_walk, all scorers (composite + subs), adapters (py+ts), filters, typicality,
calibration — all ported, most parity-verified. Remaining: check (agent), CLI
wiring (train/calibrate/check subcommands + user-facing names), evidence
formatters (check evidence lines — deferred), 5 shape-primitive scorers
(off-default, test parity), test port, bench wiring + AUC, dogfood, clippy -D.

### Remaining port surface (dependency order)
adapters (python_adapter, typescript, parsers/python_ts) → import_graph →
filters (data_dominant, autogenerated) → typicality → call_receiver (KMeans,
hardest) → sequential (score_hunk integration) → calibration (PCG64) → check →
evidence → CLI → 5 shape-primitive scorers → test port → bench wiring + AUC.

## CallReceiverScorer + KMeans — THE hard parity core

`call_receiver.py` composes deterministic tree-walking with THREE RNG/sklearn
hotspots. Deterministic parts (port directly, golden-test):
- `extract_callees(src, lang)`: tree-walk (stack DFS, `reversed(children)`), per
  call/new node → dotted callee via `_extract_python_callee` /
  `_extract_typescript_callee` (walk member chain, `<call>` sentinel for
  call-rooted chains, None for subscript/paren). `_has_root_error` = any direct
  root child is `ERROR`.
- `weighted_contribution` (no cluster): per distinct callee not in attested →
  `alpha+root_bonus` if root in attested_roots else `alpha`; `min(sum, cap)`.
- `nearest_cluster_for_source`: Jaccard of file callee-bag vs each cluster's
  attested set, max (ties → smallest cid); None if bag empty.
- MinHash signature: md5(callee)[:8] little-endian u64 % PRIME(2^31-1); per perm
  i: `min((a[i]*h+b[i]) % PRIME)`; empty bag → all-zeros. 128 perms.

RNG/sklearn hotspots (the risk):
1. **MinHash params** `_generate_minhash_params(seed)` = numpy
   `default_rng(seed).integers(1,PRIME,128)` (a) + `integers(0,PRIME,128)` (b),
   **PCG64**. Mitigation: `cluster_seed` is ALWAYS 0 in production → params are
   FIXED. **Precompute in Python and embed as Rust constants.** No PCG64 needed
   for MinHash. (Captured to fixtures.)
2. **KMeans labels** `_cluster_by_signatures`: sigs = array/PRIME normalized;
   `KMeans(k=min(n_clusters,n), random_state=seed, n_init=10).fit_predict`.
   sklearn k-means++ uses numpy **MT19937** (`RandomState`) + Lloyd + BLAS —
   bit-exact cross-impl reproduction is ~infeasible. Labels feed
   cluster_attested → cluster_bonus (5.0) firing → scores/threshold. THIS is the
   top open risk. Strategy: (a) try faithful k-means++ w/ matched RNG, verify
   labels empirically on the real corpora; (b) if labels can't match exactly,
   fall back to the AUC gate (≥ main) + documented epsilon, since well-separated
   MinHash sigs may converge to the same partition regardless. Decide after
   measuring.
3. **Calibration sampling** `sample_hunks`: `default_rng(seed).choice(n_cand,
   size, replace=False)` then `sorted(idx)`, **PCG64**. Needed for threshold
   regeneration (calibrate). At CHECK time the threshold is LOADED from
   scorer-config.json — so check parity does NOT need calibration RNG; only
   `calibrate`/dogfood/bench-threshold regeneration does. Reproducing numpy
   PCG64 + SeedSequence + `choice(replace=False)` is a bounded but real effort.

Production call_receiver config (from scorer-config.json): alpha 2.0, cap 5,
root_bonus 2.0, n_clusters 8, cluster_seed 0, cluster_bonus 5.0,
cluster_rare_threshold 0, cluster_size_min 0. rare-threshold 0 ⇒ rare branch
never fires in prod; shape_primitives empty in prod.

### KMeans reproducibility — MEASURED (2026-07-01)
Empirical test on 150 real fastapi files' MinHash sigs: sklearn KMeans
partitions are seed-SENSITIVE (seed 0 vs 1/2/7/42 → all different partitions;
n_init=1 vs 10 → different). ⇒ There is no "stable global optimum any KMeans
finds"; matching sklearn's seed-0 partition requires bit-exact k-means++
(numpy MT19937) + Lloyd + BLAS float order — realistically infeasible
cross-implementation.

Two facts that make this survivable:
- **Scoring is partition-label-invariant.** cluster_bonus fires on callees
  absent from the file's OWN cluster's attested set; `nearest_cluster_for_source`
  compares against ALL clusters. So only the PARTITION (grouping) matters, not
  cluster-id numbering. (Still need the same grouping, which we can't guarantee.)
- check re-clusters every run at seed 0 ⇒ each side is internally
  deterministic.

**DECISION (pending final AUC measurement):** exact per-hunk parity on
cluster-affected hunks is NOT achievable (KMeans). Plan:
1. Implement a deterministic hand-rolled k-means++ + Lloyd (fixed seed, n_init)
   over normalized sigs — best-effort structural match to sklearn.
2. Isolate & PROVE the cluster-scoring MATH is exact by injecting Python's
   cluster assignments into the Rust scorer for a parity fixture (everything
   except the KMeans partition is bit-exact).
3. Binding gate for cluster-affected scores = AUC ≥ main on every corpus
   (measured at the end). Document the divergence + bound the affected-hunk
   fraction. This is the "justified divergence" the goal allows.
Alternative if AUC regresses: persist Python-fitted clusters into the artifact
(one-time), but that reintroduces a Python dependency — avoid unless forced.

### KEY SCOPING (verified): KMeans only affects the BENCH, not `check`
- `check.py` calls `score_hunk(hunk, file_source=, hunk_start_line=,
  hunk_end_line=)` — **NO `file_path`**. So call-receiver contribution uses
  `weighted_contribution` (non-cluster, attested-based) → fully deterministic &
  reproducible. **`argot check` scoring does NOT depend on KMeans.** The
  golden-fixture check-decisions gate is achievable byte/epsilon-exact.
- The bench `score.py` DOES pass `file_path` → `weighted_contribution_for_file`
  (cluster path) → KMeans matters ONLY for bench AUC. That's exactly where the
  AUC-gate fallback applies. So: check = exact parity; bench = AUC ≥ main.

**USER DECISION (2026-07-01): AUC-gate fallback.** Hand-rolled deterministic
k-means++ (fixed seed, n_init) in pure Rust; prove cluster-scoring math exact
by injecting Python clusters into a fixture; gate cluster-affected scores on
AUC ≥ main (every corpus) + document divergence & affected-hunk bound. Keep the
single-binary, no-Python goal.

## Calibration RNG — DOCUMENTED DIVERGENCE (2026-07-01)

`sample_hunks` uses `np.random.default_rng(seed).choice(n, size, replace=False)`.
MEASURED: numpy 2.4.4's `choice(replace=False, p=None)` is NOT
`permutation[:size]` and NOT simple front/back Fisher-Yates — it switches
strategy by size/pop ratio (version-specific). Reproducing it bit-exactly is
disproportionate.

Why it's safe to diverge (justified, like KMeans):
- **AUC (the bench gate) is 100% threshold-independent** — it's
  `roc_auc_score` over raw break/control scores; the calibration threshold only
  affects the flagged/recall/FP decision, never AUC. So calibration RNG does
  NOT affect the "AUC ≥ main" gate at all.
- `check` loads the threshold from scorer-config.json (does not re-calibrate).
- `dogfood` only asserts a scorer-config is emitted, not its value.
So calibration-sampling RNG affects ONLY the exact calibrated-threshold byte
gate. **DECISION: deterministic Rust sampler (SplitMix64-based), reproducible
Rust-side; NOT numpy-byte-identical.** No numpy PCG64 port needed. The
scorer-config.json is schema-identical & check-consumable; its threshold float
is a documented divergence. (numpy_rng goldens captured but unused — kept for
reference.)

## CLI surface (Phase 6 target — from cli/src)

Two layers:
- **Engine entry points** (bench + dogfood call these): `argot-extract`,
  `argot-train`, `argot-calibrate`, `argot-check`. No branded header, plain
  engine output. Rust: `argot extract|train|calibrate|check` subcommands.
- **User-facing `argot`** (TS binary, cli/src): subcommands
  `extract` · `fit` (=train+calibrate one-shot) · `check` · `status` · `list`
  · `update`. No-subcommand → help banner (COMMANDS list). `argot check` prints
  `{brandedArgot} · {ctx.name} ({ctx.gitRoot})` header BEFORE engine output,
  exit 1 on violations. check flags: `<ref>` (default ""), `--staged`,
  `--unstaged`, `--commit SHA`, `--only GLOB` (repeat), `--exclude GLOB`
  (repeat), `--verbose`, `--min-severity {unusual|suspicious|foreign}` (default
  unusual), `--threshold N` (--threshold must be numeric → else exit 2).
  Mutual-exclusion errors → exit 2 (see check.command.ts).
- `status`/`list` use RepoContext (a repo registry in a settings file);
  `update` checks for a newer CLI version. These are non-scoring infra to port
  in Phase 6 (fs-repo-context.adapter.ts, update-notify.ts).
- **RepoContext** (`fs-repo-context.adapter.ts`): gitRoot = `git rev-parse
  --show-toplevel` (fallback cwd). Registry at `~/.argot/settings.json`
  `{repos: {<gitRoot>: {name: basename, registeredAt, lastUsedAt}}}` (upsert on
  resolve, JSON indent 2). Paths: argotDir=`<root>/.argot`, dataset.jsonl,
  repo-corpus.txt, generic-baseline.json. name = registry name or basename.
- **`fit`**: header `{branded} · {name} ({gitRoot})`; `Step 1/2: training voice
  model …`; train; `Step 2/2: calibrating threshold …`; calibrate (nCal 500,
  seed 0, out=argotDir/scorer-config.json); `Done. Scorer config: {path}`.
- **`status`**: `Repo:     {name} ({gitRoot})`; `Dataset:  {N records · SIZE ·
  last extracted AGE}` or `—`; `Model:    trained AGE · SIZE` or `not trained`;
  `Calibrated: threshold {t:.2f} · last calibrated AGE` or `not calibrated — run
  \`argot fit\``. (Note: status reads config.threshold at TOP level, but v2 puts
  it under languages.<lang>.threshold — a latent TS bug; replicate behavior:
  shows `?`/`not calibrated` for v2. Low priority.) formatBytes: <1024 B, <1MB
  KB(.1), else MB(.1). formatAge: <1h "just now", <24h "{h}h ago", else "{d}d ago".
- **`list`**: repos from registry sorted by name.localeCompare; per repo path,
  name, isCurrent, dataset/model stat.
- CLI reconciliation: `argot check` user-facing takes `[ref]` (repo=cwd/gitRoot);
  engine `argot-check` took `repo_path [ref]`. The Rust `check` subcommand (agent)
  is engine-style (repo_path positional) for bench/dogfood; the user `argot
  check` layers context resolution. Reconcile in Phase 6; update justfile dogfood
  to drive the Rust binary (cd into path or pass repo_path).
- `just dogfood` currently calls `uv run argot-{train,calibrate,check}`; after
  cutover it calls the Rust binary's subcommands. dogfood only asserts exit +
  both .py/.ts rows in dataset.jsonl + scorer-config emitted (not exact output),
  so the branded header etc. don't break it.

## DoD item 2 (bench AUC ≥ main) — RESOLVED BY CONSTRUCTION (2026-07-01)

The bench AUC = `auc_catalog(break_scores, ctrl_scores)` where BOTH score lists
are `r["bpe_score"]` = **`stages.bpe_score`** (raw BPE surprise), and the
control exclusion set is `{atypical, atypical_file, excluded_path,
auto_generated}` (typicality/path/auto-gen — NONE depend on call_receiver).
⇒ The AUC depends ONLY on the BPE path + typicality + data-dominant + excluded-
path, ALL of which are bit-identical or parity-verified. KMeans, cluster_bonus,
calibration threshold have ZERO effect on AUC (they move flagged/recall/FP only).
⇒ **Rust AUC == Python AUC EXACTLY on every corpus.** Evidence + proof:
`docs/research/evidence/rust-port-auc-parity.md`. The entire KMeans/calibration
divergence discussion above is MOOT for the AUC gate.

## Verified end-to-end (2026-07-01)

- **Mixed-language dogfood PASSES via the single binary**: extract (both .py +
  .ts rows) → train → calibrate → scorer-config.json with BOTH `python` and
  `typescript` blocks. DoD item 4 dogfood semantics satisfied.
- **`just dogfood-rust` / `just verify-rust` / `just build-rust`** added
  (Rust equivalents of dogfood/verify).
- **pygit2 robustness gap**: Python `argot-extract` CRASHES on dagster
  (`GitError: illegal byte sequence` in diff iteration); Rust/git2 handles it
  and extracts 4000 mixed rows. Benign divergence (Rust more robust). Byte-parity
  comparison isn't possible where Python crashes; py+ts extraction parity already
  proven separately (fastapi/faker/rich + hono/faker-js), and extract is
  per-file/per-language so mixed = union.

## FINAL STATE (2026-07-01, this session)

**Complete + verified:**
- Single binary `argot` (crates/argot-core + argot-cli): extract, train,
  calibrate, fit, check, status, list, update, no-subcommand help banner.
- **67 tests green, `cargo clippy --workspace --all-targets -D warnings` clean,
  `cargo fmt` clean.**
- Parity: extract byte-identical (fastapi/faker/faker-js/hono); BPE encode
  bit-identical (14.5k tokens); bpe_score/import/multi-reason parity; check
  byte-identical (3 modes); typicality/adapters/call-receiver-callees/minhash/
  shape-primitives (50 cases) golden-tested.
- **AUC gate PROVEN met exactly** (bpe_score-only dependency) + baseline captured
  (fastapi 0.9946). Evidence: docs/research/evidence/rust-port-auc-parity.md.
- Mixed-language dogfood passes end-to-end.
- ml/ research path DROPPED (documented). 5 shape primitives PORTED.

**Genuine remaining work (breadth/polish, not the hard core):**
1. **Evidence terminal rendering** — the ↳ evidence lines, eslint carets, and
   ANSI color path in `check` output are DEFERRED. `check` byte-parity holds on
   the NO_COLOR / no-evidence path (the goldens); a hit WITH evidence/colors is
   not yet byte-identical. Needs porting scoring/evidence/{types,formatters,
   collectors,layout,bpe_reconstruction} (~900 LOC) + wiring into check + the
   `_collect_evidence` path in sequential + colored render. Largest remaining gap.
2. **Literal 1:1 test port** — Rust has behaviour-equivalent parity tests for
   every core module, but not a file-for-file port of all ~30 test_*.py + TS
   tests. (Interpretation of "100% ported tests".)
3. **Cutover** — flip justfile/CI to the Rust binary and retire the Python
   engine + TS CLI once evidence rendering lands. Python kept shippable meanwhile
   (guardrail).
4. **Full 7-corpus bench evidence** — fastapi baseline captured; others follow by
   the AUC proof but could be run for completeness.

## EVIDENCE + BENCH WIRING (2026-07-01, session 2)

- **Evidence layer: DONE + BYTE-PARITY.** `scoring/evidence/` (types, layout,
  formatters, bpe_reconstruction, bpe/imports/call_receiver collectors) +
  `sequential` collect_evidence + `check` renders `↳` lines / carets /
  common-here. Two goldens byte-match Python: BPE evidence (fit@HEAD~1) + import
  evidence (fit@HEAD, carets). **79 tests green, clippy -D clean.** Last
  check-parity gap CLOSED.
- **Bench wiring: DONE.** Rust `argot score` hidden subcommand (batch stdin JSONL
  → per-hunk import_score/bpe_score/flagged/reason). `benchmarks/.../score.py`
  `_RustBenchScorer` coprocess gated by `ARGOT_BENCH_RUST=1` (the designed
  adapter seam — only score.py changes). Same harness runs either engine over
  the exact same corpus.
- **Speed: Rust is 4.6× (extract, work-bound) to 30–40× (fit/check,
  invocation-overhead-bound) faster.** `argot check` 2.3s → 0.07s.
- Bench AUC (Python baseline + Rust) running across all corpora for the
  side-by-side numbers (proof already establishes AUC == exactly).

## CURRENT STATE SUMMARY
Single Rust binary `argot` (crates/argot-core + argot-cli) is a complete pipeline
replacement: `extract|train|calibrate|fit|check`. 59 tests green, clippy -D clean,
fmt clean. Parity: extract byte-identical (4 corpora), BPE bit-identical, scoring
(bpe_score/import/multi-reason) parity, check byte-identical (3 modes), calibration
schema-valid + threshold exact on small repos. **AUC gate proven met exactly.**
Remaining: shape primitives (agent), status/list/update + help banner + user-check
header, 1:1 test-port breadth, cutover (keep Python engine until flip).

## Parity harness plan

Golden fixtures captured from Python `main` per corpus: `dataset.jsonl`,
per-hunk scores, calibrated `scorer-config.json`, `check` decisions. Rust must
reproduce: exact bytes for int/string, tight documented float epsilon
otherwise. Store under `docs/rust-port/golden/` (or `.scratch/`), keep the
capture script.

## FULL-BENCH METRIC PARITY (2026-07-01, session 3)

Goal escalated from "AUC ≥ main" to *all* bench metrics identical-or-better
(AUC, recall, fp_rate) on every corpus. AUC was already provably exact
(bpe_score bit-identical). Closing recall/fp needed three fixes; each was
diagnosed against the clean Python baseline (`benchmarks/results/20260505T023341Z`
— NOTE the recent multi-corpus runs `20260701T193206Z`/`194209Z` are
concurrency-**corrupted**: nctrl dropped, fp garbage — do not use as baseline).

1. **Calibration sampler → numpy-exact** (`scoring/numpy_sampler.rs`). The
   threshold is `max(cal_scores)` over `sorted(np.random.default_rng(seed)
   .choice(n_candidates, n_cal, replace=False))`. The old SplitMix64 sampler
   drew a different pool → wrong threshold on the two smallest corpora (hono,
   ink). Reproduced numpy bit-for-bit: SeedSequence→PCG64 (XSL-RR 128/64),
   buffered `next_uint32`, Lemire-bounded ints, and `choice`'s **Floyd** branch
   (+ the `pop>10000 && n>pop/50` tail-shuffle branch). Verified against numpy
   2.4.4 reference vectors (unit tests). Fix made **thresholds identical on all
   6 corpora**.
2. **Bench cluster_rare auto-select** (`benchmarks/.../score.py`). Production
   ships `cluster_rare_threshold=0`, but the *bench* uses `2` with a per-corpus
   auto-select probe (KEEP if fire-rate < 5%, else DISABLE). My Rust coprocess
   hardcoded 0 → faker-js recall 0.76 vs 0.94 (it's the one corpus that KEEPs
   the rule). Extracted the probe into `_probe_keep_cluster_rare_rule` (shared
   by both paths) and pass the decision to `argot score --cluster-rare-threshold`.
   Decisions: **only faker-js KEEPs (2)**; all others DISABLE (0). The Rust
   cluster_rare rule + KMeans clustering already matched Python exactly (same
   flagged fixture set). Fix made **recall identical on all 6**.
3. **`argot score` repo-module resolution** (`--repo-root`). Python inference
   builds the scorer with `repo_root=repo_dir`, so `ImportGraphScorer.fit`
   attests the repo's own package name (package.json `name`) + workspace
   packages as internal. My `score` command left `import_module_prefixes=[]`, so
   self-package imports (`import … from 'ink'`) were flagged foreign →
   spurious `import`-stage FPs on ink/faker-js. Added `--repo-root`; the command
   now calls `adapter.resolve_repo_modules(root)` and folds `.exact`/`.prefixes`
   in, matching Python. Fixes the last fp gap.

Result (vs clean baseline `20260505T023341Z`), full 6-corpus bench with the
final binary — **identical-or-better on every metric, every corpus**:

| corpus   | AUC (R=P) | recall R/P | fp R/P | thr (R=P) |
|----------|-----------|------------|--------|-----------|
| fastapi  | 0.9946 | 0.9375/0.9375 | 0.0073/0.0073 | 5.2585 |
| rich     | 0.9964 | 1.0000/1.0000 | 0.0101/0.0101 | 4.6469 |
| faker    | 0.9537 | 0.9375/0.9375 | 0.0207/0.0211 | 5.3845 |
| faker-js | 0.9477 | 0.9412/0.9412 | 0.0194/0.0194 | 4.8607 |
| hono     | 0.8326 | 0.8824/0.8824 | 0.0051/0.0052 | 4.2707 |
| ink      | 0.9905 | 0.9412/0.9412 | 0.0039/0.0039 | 4.9932 |

AUC, threshold, recall exact on all 6; fp identical or *better* (faker, hono).

### Speed (Rust vs Python engine)

Production commands (the shipped path): `extract` **5.2×** (2.95s vs 15.2s),
`calibrate` **~3.5×** (2.2s vs 7.8s), `check` **~23×** (0.015s vs 0.345s).

Bench harness wall-clock is NOT representative: the Rust bench path scores each
control hunk over a per-hunk stdin/stdout IPC round-trip with the `argot score`
coprocess, so control-heavy corpora (faker-js, 256k controls) are IPC-bound.
Shipped `check` scores a PR's hunks in-process (the 0.015s above). If bench
speed ever matters, batch the coprocess protocol (send all hunks, read all
results) instead of one round-trip per hunk.
