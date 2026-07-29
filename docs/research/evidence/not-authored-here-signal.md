# `not-authored-here` — finding the code a repo stores but never writes

**Date:** 2026-07-29
**What:** a third ground for `argot init --suggest`, built on git history rather
than file contents: directories the repository **stores** but does not
**author** — vendored libraries, forked upstream copies, machine-translated
bindings, generated trees.
**Why:** `--suggest` could only see what a *content* filter can see — an
auto-generation marker, or a file dominated by data literals. Imported source is
ordinary hand-written code carrying no marker at all, so the largest and most
common category of "code that shouldn't shape the voice" was invisible to it, in
every language.

## The report that started it

`argot init --suggest` on MSEide/MSEgui (Object Pascal, 9 835 commits, 921
supported files) returned **zero candidates**. The tree it was silent about:

| directory | files | lines | what it is |
| --- | --- | --- | --- |
| `lib/common/mzeoslib` | 148 | 179 049 | ZeosLib, an upstream DB layer |
| `lib/common/fpccompatibility` | 108 | 74 787 | forked Free Pascal FCL units |

Roughly a third of the corpus, teaching the voice a codebase nobody in the
project writes. The first reading was "the heuristics don't know Object Pascal
conventions". That reading was wrong, and usefully so: the heuristics don't know
*any* language's conventions for this, because no such convention exists. What
does exist, identically in every language, is the shape of the history.

## The signal

Imported code has a history no authored code has: it arrives in one drop and is
then left alone, while the repo's own code is edited again and again. Measured
on MSEgui:

| directory | commits per file | share arriving in one commit |
| --- | --- | --- |
| `lib/common/mzeoslib` | 0.01 | 1.00 |
| `lib/common/kernel` (authored) | 22.1 | 0.08 |
| `lib/common/widgets` (authored) | 71.3 | 0.07 |

Three orders of magnitude, from `git log` alone.

Three ratios, each normalised against the repository's own norms so a
twenty-year repo and a six-month one are judged the same way
(`crates/argot-rules-voice/src/ignore_suggest.rs`):

- **churn** — the directory's mean commits-per-file ÷ the repo's, `≤ 0.15`;
- **bulk arrival** — `≥ 85 %` of its files introduced by one single commit,
  **or** **untouched** — `≥ 80 %` of its files never edited since;
- floors — `≥ 8` files and `≥ 2 000` lines, because ignoring a directory is a
  permanent line in a committed config and the categories this catches are never
  small.

Below `3.0` mean commits per file the repository has too little history to
compare against — in a young repo nothing has been edited twice yet — and the
signal abstains entirely rather than reporting the whole tree.

`argot_engine::git_walk::path_edit_history` supplies the facts: one pass over
the reachable non-merge history, **following renames**, so a directory that was
merely *moved* is not mistaken for one dropped in yesterday. Diffs fan out
across cores; only `(status, old path, new path)` is kept, and rename detection
runs only on the commits that have something to pair up.

## Result — 41 corpora, 12 languages

Ran `argot init --suggest --format json` over every bench corpus with any
bench-authored `argot.toml` moved aside, so each scan saw the repository as a
new user would. **Silent on 28 of 41.** The 33 `not-authored-here` candidates it
did report, adjudicated by hand:

| repo | directory | files | lines | churn | verdict |
| --- | --- | --- | --- | --- | --- |
| mormot2 | `ex/ThirdPartyDemos/dmvc-ai` | 261 | 25 876 | 0.017 | third-party demos |
| mormot2 | `res/static/libquickjs` | 21 | 75 104 | 0.020 | vendored QuickJS |
| mormot2 | `res/static/libzlib` | 21 | 19 383 | 0.016 | vendored zlib |
| mormot2 | `res/static/liblizard` | 29 | 9 682 | 0.020 | vendored Lizard |
| mormot2 | `res/static/libdeflate` | 39 | 12 063 | 0.035 | vendored libdeflate |
| mormot2 | `ex/ThirdPartyDemos/tbo` | 22 | 3 776 | 0.027 | third-party demos |
| mseide-msegui | `lib/common/mzeoslib` | 148 | 179 049 | 0.035 | vendored ZeosLib |
| castle-engine | `src/vampyre_imaginglib` | 162 | 120 908 | 0.058 | vendored Vampyre Imaging |
| castle-engine | `tools/build-tool/data/android/services/sound` | 254 | 70 217 | 0.075 | bundled Android SDK services |
| castle-engine | `src/window/gtk/gtk3` | 14 | 69 564 | 0.071 | GTK3 header translations |
| castle-engine | `src/scene/transform_manipulate_data` | 8 | 20 575 | 0.037 | generated mesh data |
| castle-engine | `tools/build-tool/data/ios/services/ogg_vorbis` | 27 | 9 937 | 0.046 | vendored ogg/vorbis |
| castle-engine | `tools/castle-editor/components` | 53 | 16 061 | 0.147 | vendored Lazarus components |
| homebrew | `…/vendor/bundle/ruby/4.0.0/gems/concurrent-ruby-1.3.7` | 118 | 14 338 | 0.105 | vendored gem |
| homebrew | `…/gems/elftools-1.3.1` | 24 | 2 218 | 0.106 | vendored gem |
| homebrew | `…/gems/ruby-macho-5.0.0` | 11 | 3 780 | 0.124 | vendored gem |
| redis | `deps/lua` | 67 | 17 407 | 0.063 | vendored Lua |
| redis | `deps/tre` | 24 | 7 801 | 0.034 | vendored TRE |
| rocksdb | `utilities/transactions/lock/range` | 46 | 10 132 | 0.099 | vendored TokuDB range locking |
| rocksdb | `c_api_gen` | 27 | 7 774 | 0.028 | generated C API |
| rocksdb | `tools/c_api_gen` | 10 | 13 808 | 0.028 | generated C API |
| rich | `rich/_unicode_data` | 23 | 11 869 | 0.148 | generated Unicode tables |
| dagster | `dagster/_vendored` | 20 | 7 463 | 0.063 | vendored |
| dagster | `…/dagster_rest_resources/__generated__` | 49 | 5 184 | 0.070 | generated |
| dagster | `…/dagster_cloud_cli/core` | 26 | 8 251 | 0.146 | arguable — a separate product's CLI |
| powershell | `…/ManagementList/Common` | 44 | 4 583 | 0.132 | arguable — imported WPF control suite |
| dagster | `js_modules/ui-core/src/workspace` | 55 | 8 275 | 0.142 | **false alarm** — authored React |

**29 clear true positives, 2 arguable, 1 false alarm** — 88 % clear precision,
97 % counting the arguables. The single false alarm is a component-per-file
React tree where each file genuinely is written once and left; at churn `0.142`
it sits just inside the bar. Tightening the bar to `0.12` would remove it and
also remove `castle-editor/components`, `rich/_unicode_data` and both arguables:
two true positives lost to kill one false alarm, so the bar stays at `0.15`.

### What it deliberately does not report

`lib/common/fpccompatibility` — the other big MSEgui tree — comes out at churn
`0.16`, bulk `0.36`, untouched `0.13`. MSEgui *maintains* its forked FCL units:
mean 4.7 commits per file against a repo mean of 29. The history says
"maintained", and the signal says nothing. Whether upstream's idiom should shape
the voice is a judgement about idiom, not about authorship, and it stays with
the reader — which is what `--suggest` is for.

## Side effect: `vendor/` became visible

`SKIP_TRAVERSAL_DIRS` in the suggest walk pruned `vendor/`, `third_party/` and
`third-party/`. The corpus walk (`argot-engine::corpus`) does not prune them, so
a committed `vendor/` tree **was shaping the voice while being structurally
unreportable**. Those three entries are gone from the prune list; the remaining
entries are build output and caches, which are gitignored and not corpus.
Homebrew's three vendored gems are the first thing this surfaced.

## Cost

The history pass is paid by `--suggest` and by every `fit` (which persists its
verdict to `.argot/health.json`). Measured by running the same binary with and
without it:

| corpus | commits | scan without history | with history | the pass |
| --- | --- | --- | --- | --- |
| mseide-msegui | 9 835 | 11,4 s | 13,0 s | +1,6 s |
| castle-engine | 26 013 | 19,4 s | 36,6 s | +17 s |
| dagster | 27 168 | 8,2 s | 30,3 s | +22 s |
| rocksdb | 13 967 | 36,1 s | 32,8 s | within noise |

Seconds at ten thousand commits, tens of seconds on the largest monorepos — on a
command whose tree walk and parse already cost that much, run once at setup and
in the background on refit. No commit cap: capping the walk would quietly turn
"argot could not see far enough back" into "this directory was never edited",
which is exactly the mistake the abstain-below-`MIN_REPO_EDITS_PER_FILE` guard
exists to prevent.
