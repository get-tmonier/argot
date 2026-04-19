# Phase 7.0 — Honest Eval Rebuild — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the scoring-research eval so the primary metric is `synthetic_auc_mean` (4 deterministic token-level mutations) measured independently within same-language sub-corpora, replacing the language-confounded TS+Py cross-repo metric.

**Architecture:** Branch off `research/bpe-tokenisation` (has `EncoderKind` dispatch). Add `engine/argot/mutations.py` (pure functions `(record, seed) -> record`). Extend `_load_records` to preserve `language`; extend `_benchmark_one` to compute per-mutation AUCs and a language-scoped `cross_auc_same_lang`. Extract six new repos, build two same-language sub-corpora per bucket, gate everything behind a new `just research honest-benchmark` recipe. Existing encoders (tfidf, char_ngrams, token_embed, bpe) are untouched — they are re-run on the new eval in Phase 7.1.

**Tech Stack:** Python 3.13 / uv; `argot.corpus`, `argot.validate`, `argot.train`; `pytest`; `just`; tree-sitter extractor; `pygit2` for clone.

---

## File Structure

| path                                             | status   | responsibility                                                   |
|:-------------------------------------------------|:---------|:-----------------------------------------------------------------|
| `engine/argot/mutations.py`                      | **new**  | 4 mutation fns + dispatcher `apply_mutation(name, record, seed)` |
| `engine/argot/tests/test_mutations.py`           | **new**  | Unit tests per mutation (determinism, shape, language dispatch)  |
| `engine/argot/corpus.py`                         | modified | `_load_records` keeps `language`; `_benchmark_one` adds per-mutation AUCs and `cross_auc_same_lang`; new `--lang` CLI flag |
| `engine/argot/tests/test_corpus_benchmark.py`    | modified | Assert new row keys (`synthetic_auc_mean`, `synthetic_auc_<id>`, `cross_auc_same_lang`) |
| `engine/argot/validate.py`                       | unchanged | Already returns `language` via `stratify_scores`; no change      |
| `justfile`                                       | modified | Add `research-honest-benchmark` recipe                           |
| `docs/research/scoring/15-honest-corpus.md`      | **new**  | Corpus pins (URLs, SHAs, counts), swap-out log, sample mutated records |
| `docs/research/scoring/ROADMAP.md`               | modified | Mark 7.0 complete at the end                                     |
| `scripts/research/extract-honest-corpus.sh`      | **new**  | Idempotent clone-and-extract for the 6 same-language repos       |
| `.argot/research/datasets-v2/<repo>.jsonl`       | **new**  | Per-repo extracts (gitignored)                                   |
| `.argot/research/buckets-v2/<bucket>-<lang>.jsonl`| **new**  | Language-scoped concatenated buckets (gitignored)                |

**Note on directory layout:** the Phase 7 design doc says `engine/argot/benchmark/mutations.py`. A `benchmark.py` *file* already exists at `engine/argot/benchmark.py` (scoring-time quality check), so creating a `benchmark/` package would collide. This plan puts the module at `engine/argot/mutations.py` (top-level, consistent with `extract.py` / `validate.py` / `corpus.py`).

---

### Task 0: Create worktree on parent branch

**Files:**
- Create: `.worktrees/phase-7-honest-eval/` (git worktree)

**Context:** `research/bpe-tokenisation` already has `EncoderKind` dispatch (`tfidf`, `word_ngrams`, `token_embed`, `bpe`, `transformer`) wired through `train.py`, `validate.py`, and `corpus.py`. That is the ancestor we need. `research/combined-optimizations` lacks the encoder switches.

- [ ] **Step 1: Create the worktree**

```bash
git fetch origin
git worktree add .worktrees/phase-7-honest-eval -b research/phase-7-honest-eval origin/research/bpe-tokenisation
cd .worktrees/phase-7-honest-eval
```

Expected: new worktree, branch `research/phase-7-honest-eval` tracking nothing yet. All subsequent work in this worktree.

- [ ] **Step 2: Verify baseline state**

```bash
just verify
```

Expected: PASS (baseline is green on `research/bpe-tokenisation`).

- [ ] **Step 3: Commit the branch marker**

```bash
git commit --allow-empty -m "chore(research): start Phase 7.0 — honest eval rebuild"
```

---

### Task 1: Create `mutations.py` scaffold with dispatcher

**Files:**
- Create: `engine/argot/mutations.py`
- Create: `engine/argot/tests/test_mutations.py`

**Contract for every mutation fn:**

```python
MutationFn = Callable[[dict[str, Any], int], dict[str, Any]]
# Input: a record with keys {"_repo", "author_date_iso", "language", "context_before", "hunk_tokens"}
# Output: a NEW record with the same keys. Only "hunk_tokens" may differ. "context_before" is untouched.
# Determinism: same (record, seed) always yields the same output.
# Shape: len(out["hunk_tokens"]) may differ from input (debug_inject grows it).
```

- [ ] **Step 1: Write the failing dispatcher test**

```python
# engine/argot/tests/test_mutations.py
from __future__ import annotations

import pytest

from argot.mutations import MUTATIONS, apply_mutation


def _make_record(lang: str = "python", hunk_texts: list[str] | None = None) -> dict[str, object]:
    return {
        "_repo": "demo",
        "author_date_iso": "1700000000",
        "language": lang,
        "context_before": [{"text": "x"}],
        "hunk_tokens": [{"text": t} for t in (hunk_texts or ["def", "foo", "(", ")", ":"])],
    }


def test_dispatcher_lists_all_four_mutations() -> None:
    assert set(MUTATIONS) == {"case_swap", "debug_inject", "error_flip", "quote_flip"}


def test_dispatcher_rejects_unknown_name() -> None:
    with pytest.raises(KeyError):
        apply_mutation("not_a_mutation", _make_record(), seed=0)


def test_dispatcher_preserves_context_before() -> None:
    rec = _make_record()
    out = apply_mutation("quote_flip", rec, seed=0)
    assert out["context_before"] == rec["context_before"]


def test_dispatcher_returns_new_record() -> None:
    rec = _make_record()
    out = apply_mutation("quote_flip", rec, seed=0)
    assert out is not rec
    assert out["hunk_tokens"] is not rec["hunk_tokens"]
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run pytest engine/argot/tests/test_mutations.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'argot.mutations'`.

- [ ] **Step 3: Write minimal dispatcher**

```python
# engine/argot/mutations.py
from __future__ import annotations

from collections.abc import Callable
from typing import Any

MutationFn = Callable[[dict[str, Any], int], dict[str, Any]]

MUTATIONS: dict[str, MutationFn] = {}


def _register(name: str) -> Callable[[MutationFn], MutationFn]:
    def deco(fn: MutationFn) -> MutationFn:
        MUTATIONS[name] = fn
        return fn

    return deco


def apply_mutation(name: str, record: dict[str, Any], seed: int) -> dict[str, Any]:
    if name not in MUTATIONS:
        raise KeyError(f"unknown mutation: {name!r}")
    return MUTATIONS[name](record, seed)


def _clone_with_hunk(record: dict[str, Any], new_hunk: list[dict[str, Any]]) -> dict[str, Any]:
    return {**record, "hunk_tokens": new_hunk}


# Stubs so the dispatcher is populated. Real implementations in later tasks.
@_register("case_swap")
def _case_swap(record: dict[str, Any], seed: int) -> dict[str, Any]:
    raise NotImplementedError


@_register("debug_inject")
def _debug_inject(record: dict[str, Any], seed: int) -> dict[str, Any]:
    raise NotImplementedError


@_register("error_flip")
def _error_flip(record: dict[str, Any], seed: int) -> dict[str, Any]:
    raise NotImplementedError


@_register("quote_flip")
def _quote_flip(record: dict[str, Any], seed: int) -> dict[str, Any]:
    raise NotImplementedError
```

- [ ] **Step 4: Run tests — dispatcher tests pass, stubs remain `NotImplementedError`**

Run: `uv run pytest engine/argot/tests/test_mutations.py::test_dispatcher_lists_all_four_mutations engine/argot/tests/test_mutations.py::test_dispatcher_rejects_unknown_name -v`
Expected: both PASS.

The `test_dispatcher_preserves_context_before` and `test_dispatcher_returns_new_record` tests will still fail because `quote_flip` is `NotImplementedError` — that's fine; they pass in Task 2.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/mutations.py engine/argot/tests/test_mutations.py
git commit -m "feat(research): mutations dispatcher scaffold (Phase 7.0)"
```

---

### Task 2: Implement `quote_flip` (simplest, language-agnostic)

**Files:**
- Modify: `engine/argot/mutations.py` (replace `_quote_flip` body)
- Modify: `engine/argot/tests/test_mutations.py` (add quote_flip tests)

**Spec:** A token's `text` field is a string-literal token if it matches the regex `^(['"])(.*)\1$` (single or double quoted, balanced). Swap the quote character; leave content untouched. Non-string tokens pass through. No randomness — `seed` is accepted for interface uniformity but unused.

- [ ] **Step 1: Write the failing tests**

Append to `engine/argot/tests/test_mutations.py`:

```python
def test_quote_flip_swaps_double_to_single() -> None:
    rec = _make_record(hunk_texts=['"hello"', "x"])
    out = apply_mutation("quote_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["'hello'", "x"]


def test_quote_flip_swaps_single_to_double() -> None:
    rec = _make_record(hunk_texts=["'hello'", "'world'"])
    out = apply_mutation("quote_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ['"hello"', '"world"']


def test_quote_flip_leaves_non_strings_unchanged() -> None:
    rec = _make_record(hunk_texts=["def", "foo", "(", ")", ":"])
    out = apply_mutation("quote_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["def", "foo", "(", ")", ":"]


def test_quote_flip_ignores_unbalanced_quotes() -> None:
    # e.g. '"hello' or 'hello"' — don't touch
    rec = _make_record(hunk_texts=['"hello', "hello'"])
    out = apply_mutation("quote_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ['"hello', "hello'"]


def test_quote_flip_is_deterministic() -> None:
    rec = _make_record(hunk_texts=['"a"', "'b'"])
    assert apply_mutation("quote_flip", rec, seed=0) == apply_mutation("quote_flip", rec, seed=42)
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run pytest engine/argot/tests/test_mutations.py -v -k quote_flip`
Expected: all 5 FAIL with `NotImplementedError`.

- [ ] **Step 3: Implement `_quote_flip`**

Replace the `_quote_flip` stub in `engine/argot/mutations.py`:

```python
import re

_STRING_RE = re.compile(r"^(['\"])(.*)\1$", re.DOTALL)


@_register("quote_flip")
def _quote_flip(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed  # deterministic transform
    new_hunk: list[dict[str, Any]] = []
    for tok in record["hunk_tokens"]:
        text = tok["text"]
        m = _STRING_RE.match(text)
        if m is None:
            new_hunk.append(tok)
            continue
        old_quote, body = m.group(1), m.group(2)
        new_quote = "'" if old_quote == '"' else '"'
        new_hunk.append({**tok, "text": f"{new_quote}{body}{new_quote}"})
    return _clone_with_hunk(record, new_hunk)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run pytest engine/argot/tests/test_mutations.py -v -k quote_flip`
Expected: all 5 PASS. Also the `test_dispatcher_preserves_context_before` and `test_dispatcher_returns_new_record` tests from Task 1 now PASS.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/mutations.py engine/argot/tests/test_mutations.py
git commit -m "feat(research): quote_flip mutation (Phase 7.0)"
```

---

### Task 3: Implement `case_swap` (identifier-kind transform)

**Files:**
- Modify: `engine/argot/mutations.py`
- Modify: `engine/argot/tests/test_mutations.py`

**Spec:** For every token whose `text` is an identifier, flip its case convention:

- `snake_case` (contains `_`, no uppercase letters) → `camelCase`
- `camelCase` (starts lowercase, contains uppercase) → `snake_case`
- `PascalCase` (starts uppercase, contains uppercase elsewhere OR is a single uppercase-prefixed word) → `snake_case`
- Single-word all-lowercase (`foo`) → no change (ambiguous; would be a no-op)
- Single-word all-uppercase (`CONST`) → no change (convention preserved)
- Non-identifier tokens (anything with non-alphanumeric chars other than `_`) → no change

Identifier detection: `re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", text)`. `seed` is accepted but unused.

- [ ] **Step 1: Write the failing tests**

Append to `engine/argot/tests/test_mutations.py`:

```python
def test_case_swap_snake_to_camel() -> None:
    rec = _make_record(hunk_texts=["get_user_id", "foo_bar"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["getUserId", "fooBar"]


def test_case_swap_camel_to_snake() -> None:
    rec = _make_record(hunk_texts=["getUserId", "fooBar"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["get_user_id", "foo_bar"]


def test_case_swap_pascal_to_snake() -> None:
    rec = _make_record(hunk_texts=["HttpClient", "UserRecord"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["http_client", "user_record"]


def test_case_swap_preserves_single_lowercase_word() -> None:
    rec = _make_record(hunk_texts=["foo", "bar", "baz"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["foo", "bar", "baz"]


def test_case_swap_preserves_screaming_snake() -> None:
    rec = _make_record(hunk_texts=["MAX_SIZE", "DEBUG"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["MAX_SIZE", "DEBUG"]


def test_case_swap_skips_non_identifier_tokens() -> None:
    rec = _make_record(hunk_texts=["(", ")", ":", "'", '"hello"', "123"])
    out = apply_mutation("case_swap", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["(", ")", ":", "'", '"hello"', "123"]
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run pytest engine/argot/tests/test_mutations.py -v -k case_swap`
Expected: all 6 FAIL with `NotImplementedError`.

- [ ] **Step 3: Implement `_case_swap`**

Replace the `_case_swap` stub in `engine/argot/mutations.py`:

```python
_IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_CAMEL_SPLIT_RE = re.compile(r"(?<!^)(?=[A-Z])")


def _swap_case(ident: str) -> str:
    if "_" in ident and ident.upper() == ident:
        return ident  # SCREAMING_SNAKE — leave alone
    if "_" in ident:
        # snake_case → camelCase
        parts = ident.split("_")
        if not parts[0]:
            return ident  # leading underscore — leave alone
        return parts[0] + "".join(p.capitalize() for p in parts[1:] if p)
    # camelCase or PascalCase → snake_case
    parts = _CAMEL_SPLIT_RE.split(ident)
    if len(parts) == 1:
        return ident  # single lowercase or single uppercase word
    return "_".join(p.lower() for p in parts if p)


@_register("case_swap")
def _case_swap(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    new_hunk: list[dict[str, Any]] = []
    for tok in record["hunk_tokens"]:
        text = tok["text"]
        if _IDENT_RE.fullmatch(text) is None:
            new_hunk.append(tok)
            continue
        new_hunk.append({**tok, "text": _swap_case(text)})
    return _clone_with_hunk(record, new_hunk)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run pytest engine/argot/tests/test_mutations.py -v -k case_swap`
Expected: all 6 PASS.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/mutations.py engine/argot/tests/test_mutations.py
git commit -m "feat(research): case_swap mutation (Phase 7.0)"
```

---

### Task 4: Implement `debug_inject` (language-aware)

**Files:**
- Modify: `engine/argot/mutations.py`
- Modify: `engine/argot/tests/test_mutations.py`

**Spec:** Insert a short debug-call token subsequence at a seeded-random position in the hunk. `record["language"]` dispatches the template:

- `python`: `["print", "(", '"DEBUG"', ")"]`
- `typescript` / `javascript`: `["console", ".", "log", "(", '"DEBUG"', ")"]`
- Any other language: fall back to the Python template (test ensures this is stable).

Each injected token has `{"text": "<t>"}` only (no node_type; the harness only reads `text` via `_load_records`). The insertion point is `random.Random(seed).randint(0, len(hunk))`. The original hunk must not be empty — if it is, return the record unchanged.

- [ ] **Step 1: Write the failing tests**

Append to `engine/argot/tests/test_mutations.py`:

```python
def test_debug_inject_python_adds_print_call() -> None:
    rec = _make_record(lang="python", hunk_texts=["def", "foo", "(", ")"])
    out = apply_mutation("debug_inject", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "print" in texts
    assert '"DEBUG"' in texts
    assert len(out["hunk_tokens"]) == 4 + 4  # original + 4-token print call


def test_debug_inject_typescript_adds_console_log() -> None:
    rec = _make_record(lang="typescript", hunk_texts=["const", "x", "=", "1"])
    out = apply_mutation("debug_inject", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "console" in texts
    assert "log" in texts
    assert "." in texts
    assert len(out["hunk_tokens"]) == 4 + 6


def test_debug_inject_javascript_uses_same_template_as_typescript() -> None:
    rec = _make_record(lang="javascript", hunk_texts=["a"])
    out = apply_mutation("debug_inject", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "console" in texts


def test_debug_inject_unknown_language_falls_back_to_python() -> None:
    rec = _make_record(lang="rust", hunk_texts=["let", "x"])
    out = apply_mutation("debug_inject", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "print" in texts


def test_debug_inject_is_deterministic_per_seed() -> None:
    rec = _make_record(lang="python", hunk_texts=["a", "b", "c", "d"])
    a = apply_mutation("debug_inject", rec, seed=7)
    b = apply_mutation("debug_inject", rec, seed=7)
    assert a == b


def test_debug_inject_differs_across_seeds() -> None:
    # With a long enough hunk, seeds 0 and 1 should almost surely pick different positions.
    rec = _make_record(lang="python", hunk_texts=[str(i) for i in range(50)])
    a = apply_mutation("debug_inject", rec, seed=0)
    b = apply_mutation("debug_inject", rec, seed=1)
    assert a != b


def test_debug_inject_empty_hunk_passes_through() -> None:
    rec = _make_record(lang="python", hunk_texts=[])
    out = apply_mutation("debug_inject", rec, seed=0)
    assert out["hunk_tokens"] == []
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run pytest engine/argot/tests/test_mutations.py -v -k debug_inject`
Expected: all 7 FAIL.

- [ ] **Step 3: Implement `_debug_inject`**

Replace the `_debug_inject` stub in `engine/argot/mutations.py`:

```python
import random

_DEBUG_TEMPLATES: dict[str, list[str]] = {
    "python": ["print", "(", '"DEBUG"', ")"],
    "typescript": ["console", ".", "log", "(", '"DEBUG"', ")"],
    "javascript": ["console", ".", "log", "(", '"DEBUG"', ")"],
}


def _debug_template_for(language: str | None) -> list[str]:
    if language in _DEBUG_TEMPLATES:
        return _DEBUG_TEMPLATES[language]
    return _DEBUG_TEMPLATES["python"]


@_register("debug_inject")
def _debug_inject(record: dict[str, Any], seed: int) -> dict[str, Any]:
    hunk = record["hunk_tokens"]
    if not hunk:
        return _clone_with_hunk(record, list(hunk))
    rng = random.Random(seed)
    pos = rng.randint(0, len(hunk))
    injection = [{"text": t} for t in _debug_template_for(record.get("language"))]
    new_hunk = list(hunk[:pos]) + injection + list(hunk[pos:])
    return _clone_with_hunk(record, new_hunk)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run pytest engine/argot/tests/test_mutations.py -v -k debug_inject`
Expected: all 7 PASS.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/mutations.py engine/argot/tests/test_mutations.py
git commit -m "feat(research): debug_inject mutation (Phase 7.0)"
```

---

### Task 5: Implement `error_flip` (language-aware surface rewrite)

**Files:**
- Modify: `engine/argot/mutations.py`
- Modify: `engine/argot/tests/test_mutations.py`

**Spec:** For every token in the hunk, rewrite error-handling keywords to their inverted counterparts. Language-aware because TS and Py use different keywords. Non-error tokens pass through.

Python mapping:
- `"except"` → `"finally"`
- `"raise"` → `"return"`

TypeScript / JavaScript mapping:
- `"catch"` → `"finally"`
- `"throw"` → `"return"`
- `"throws"` → `"returns"` (rare but present in type signatures)

Unknown language: fall back to the Python mapping. `seed` is accepted but unused (deterministic token-level rewrite).

**Rationale:** the design doc described this as "where a `try` token appears, rewrite adjacent error-handling tokens". That's brittle — a single keyword may appear outside a `try` block (e.g., a Result monad's `throw` helper in `effect`). Doing a flat per-token surface rewrite is simpler, deterministic, and on miss it's still a style violation (returning where the repo raises is an idiom flip). We document this simplification here.

- [ ] **Step 1: Write the failing tests**

Append to `engine/argot/tests/test_mutations.py`:

```python
def test_error_flip_python_except_and_raise() -> None:
    rec = _make_record(
        lang="python",
        hunk_texts=["try", ":", "pass", "except", "Exception", ":", "raise"],
    )
    out = apply_mutation("error_flip", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "except" not in texts
    assert "raise" not in texts
    assert texts.count("finally") == 1
    assert texts.count("return") == 1


def test_error_flip_typescript_catch_and_throw() -> None:
    rec = _make_record(
        lang="typescript",
        hunk_texts=["try", "{", "}", "catch", "(", "e", ")", "{", "throw", "e", "}"],
    )
    out = apply_mutation("error_flip", rec, seed=0)
    texts = [t["text"] for t in out["hunk_tokens"]]
    assert "catch" not in texts
    assert "throw" not in texts
    assert "finally" in texts
    assert "return" in texts


def test_error_flip_leaves_unrelated_tokens_alone() -> None:
    rec = _make_record(lang="python", hunk_texts=["def", "foo", "(", ")", ":", "pass"])
    out = apply_mutation("error_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["def", "foo", "(", ")", ":", "pass"]


def test_error_flip_unknown_language_uses_python_mapping() -> None:
    rec = _make_record(lang="rust", hunk_texts=["raise", "except"])
    out = apply_mutation("error_flip", rec, seed=0)
    assert [t["text"] for t in out["hunk_tokens"]] == ["return", "finally"]


def test_error_flip_is_deterministic() -> None:
    rec = _make_record(lang="python", hunk_texts=["raise", "x"])
    assert apply_mutation("error_flip", rec, seed=1) == apply_mutation("error_flip", rec, seed=99)
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `uv run pytest engine/argot/tests/test_mutations.py -v -k error_flip`
Expected: all 5 FAIL.

- [ ] **Step 3: Implement `_error_flip`**

Replace the `_error_flip` stub in `engine/argot/mutations.py`:

```python
_ERROR_MAPS: dict[str, dict[str, str]] = {
    "python": {"except": "finally", "raise": "return"},
    "typescript": {"catch": "finally", "throw": "return", "throws": "returns"},
    "javascript": {"catch": "finally", "throw": "return", "throws": "returns"},
}


def _error_map_for(language: str | None) -> dict[str, str]:
    if language in _ERROR_MAPS:
        return _ERROR_MAPS[language]
    return _ERROR_MAPS["python"]


@_register("error_flip")
def _error_flip(record: dict[str, Any], seed: int) -> dict[str, Any]:
    del seed
    mapping = _error_map_for(record.get("language"))
    new_hunk = [
        {**tok, "text": mapping[tok["text"]]} if tok["text"] in mapping else tok
        for tok in record["hunk_tokens"]
    ]
    return _clone_with_hunk(record, new_hunk)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run pytest engine/argot/tests/test_mutations.py -v`
Expected: all ~25 mutation tests PASS.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/mutations.py engine/argot/tests/test_mutations.py
git commit -m "feat(research): error_flip mutation (Phase 7.0)"
```

---

### Task 6: Preserve `language` in `_load_records`

**Files:**
- Modify: `engine/argot/corpus.py:52-84` (the `_load_records` function)
- Modify: `engine/argot/tests/test_corpus_benchmark.py` (update `test_load_records_strips_to_required_fields`)

**Rationale:** `debug_inject` and `error_flip` dispatch on `record["language"]`. Currently `_load_records` drops `language`. Add it back. Keep the slim — don't restore the other heavy fields (`hunk_start_line`, tokens' `node_type`, etc.).

- [ ] **Step 1: Update the failing test**

In `engine/argot/tests/test_corpus_benchmark.py`, replace `test_load_records_strips_to_required_fields`:

```python
def test_load_records_strips_to_required_fields(tmp_path: Path) -> None:
    dataset = tmp_path / "data.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=5)

    records = _load_records(dataset)

    assert len(records) == 10
    expected_keys = {"_repo", "author_date_iso", "language", "context_before", "hunk_tokens"}
    for r in records:
        assert set(r.keys()) == expected_keys
        assert r["language"] == "python"  # _make_tagged_dataset hard-codes python
        for token in r["context_before"]:
            assert set(token.keys()) == {"text"}
        for token in r["hunk_tokens"]:
            assert set(token.keys()) == {"text"}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `uv run pytest engine/argot/tests/test_corpus_benchmark.py::test_load_records_strips_to_required_fields -v`
Expected: FAIL — set mismatch (`language` missing).

- [ ] **Step 3: Update `_load_records`**

In `engine/argot/corpus.py`, change the `slim` dict inside the stream loop:

```python
slim = {
    "_repo": r["_repo"],
    "author_date_iso": r["author_date_iso"],
    "language": r["language"],
    "context_before": [{"text": t["text"]} for t in r["context_before"]],
    "hunk_tokens": [{"text": t["text"]} for t in r["hunk_tokens"]],
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run pytest engine/argot/tests/test_corpus_benchmark.py -v`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add engine/argot/corpus.py engine/argot/tests/test_corpus_benchmark.py
git commit -m "refactor(research): preserve language in _load_records (Phase 7.0)"
```

---

### Task 7: Add per-mutation AUCs to `_benchmark_one`

**Files:**
- Modify: `engine/argot/corpus.py:124-166` (the `_benchmark_one` function)
- Modify: `engine/argot/tests/test_corpus_benchmark.py` (extend `test_benchmark_writes_one_row_per_size_seed`)

**Spec:** For each held-out record, produce a mutated copy per mutation. Score both; AUC = AUC between `good` and `mutated` lists per mutation. Add keys:

- `synthetic_auc_case_swap`, `synthetic_auc_debug_inject`, `synthetic_auc_error_flip`, `synthetic_auc_quote_flip`
- `synthetic_auc_mean` = arithmetic mean of the four

Also add a `cross_auc_same_lang` field that is a float when the sub-corpus is single-language (all records share the same `language`), and `None` otherwise. Keep the existing `shuffled_auc`, `cross_auc`, `injected_auc` untouched so Phase 2–6 comparability is preserved.

- [ ] **Step 1: Update the failing test**

Replace `test_benchmark_writes_one_row_per_size_seed` in `engine/argot/tests/test_corpus_benchmark.py`:

```python
def test_benchmark_writes_one_row_per_size_seed(tmp_path: Path) -> None:
    dataset = tmp_path / "combined.jsonl"
    out = tmp_path / "results.jsonl"
    _make_tagged_dataset(dataset, n_per_repo=40)

    run_benchmark(
        dataset=dataset,
        sizes=[40, 60],
        seeds=2,
        output=out,
        epochs=1,
        batch_size=16,
    )

    rows = [json.loads(line) for line in out.read_text().strip().splitlines()]
    assert len(rows) == 4

    mutation_names = {"case_swap", "debug_inject", "error_flip", "quote_flip"}
    for row in rows:
        assert "shuffled_auc" in row
        assert "cross_auc" in row
        assert "injected_auc" in row
        assert "synthetic_auc_mean" in row
        for name in mutation_names:
            assert f"synthetic_auc_{name}" in row
        assert "cross_auc_same_lang" in row
        # _make_tagged_dataset is all python → same-lang should compute, not be None
        assert row["cross_auc_same_lang"] is not None
```

- [ ] **Step 2: Run tests to verify failure**

Run: `uv run pytest engine/argot/tests/test_corpus_benchmark.py::test_benchmark_writes_one_row_per_size_seed -v`
Expected: FAIL — missing `synthetic_auc_*` keys.

- [ ] **Step 3: Extend `_benchmark_one`**

In `engine/argot/corpus.py`, replace the `_benchmark_one` function body (keep the signature). Add `from argot.mutations import MUTATIONS, apply_mutation` at the top of the file.

```python
from argot.mutations import MUTATIONS, apply_mutation


def _benchmark_one(
    *,
    records: list[dict[str, Any]],
    size: int,
    seed: int,
    epochs: int,
    batch_size: int,
    encoder: EncoderKind = "tfidf",
) -> dict[str, Any]:
    sample = stratified_downsample(records, target_size=size, seed=seed)

    repo_groups: dict[str, list[dict[str, Any]]] = {}
    for r in sample:
        repo_groups.setdefault(r["_repo"], []).append(r)

    foreign_name = min(repo_groups, key=lambda n: len(repo_groups[n]))
    foreign = repo_groups[foreign_name]
    home = [r for r in sample if r["_repo"] != foreign_name]

    train_records, held_out = split_by_time(home, ratio=0.8)
    bundle = train_model(train_records, epochs=epochs, batch_size=batch_size, encoder=encoder)

    good = score_records(bundle, held_out)
    shuffled = score_records(bundle, shuffle_negatives(held_out, seed=seed))
    cross = score_records(bundle, foreign)
    injected = score_records(bundle, inject_foreign(held_out, foreign, seed=seed))

    per_mutation_auc: dict[str, float] = {}
    for name in MUTATIONS:
        mutated = [apply_mutation(name, r, seed=seed) for r in held_out]
        per_mutation_auc[name] = compute_auc(good, score_records(bundle, mutated))

    synthetic_mean = float(np.mean(list(per_mutation_auc.values())))

    langs = {r.get("language") for r in sample}
    if len(langs) == 1 and len(repo_groups) >= 2:
        cross_auc_same_lang: float | None = compute_auc(good, cross)
    else:
        cross_auc_same_lang = None

    good_arr = np.array(good) if good else np.array([0.0])
    row: dict[str, Any] = {
        "size": size,
        "seed": seed,
        "encoder": encoder,
        "n_repos": len(repo_groups),
        "n_train": len(train_records),
        "n_held_out": len(held_out),
        "n_foreign": len(foreign),
        "shuffled_auc": compute_auc(good, shuffled),
        "cross_auc": compute_auc(good, cross),
        "injected_auc": compute_auc(good, injected),
        "cross_auc_same_lang": cross_auc_same_lang,
        "synthetic_auc_mean": synthetic_mean,
        "good_median": float(np.median(good_arr)),
        "good_p95": float(np.percentile(good_arr, 95)),
        "trained_at": datetime.now(UTC).isoformat(),
    }
    for name, auc in per_mutation_auc.items():
        row[f"synthetic_auc_{name}"] = auc
    return row
```

Also extend the progress-print line in `run_benchmark` to include the new metric:

```python
print(
    f"size={size:>6d} seed={seed}  "
    f"shuffled={row['shuffled_auc']:.3f}  "
    f"cross={row['cross_auc']:.3f}  "
    f"injected={row['injected_auc']:.3f}  "
    f"synth_mean={row['synthetic_auc_mean']:.3f}"
)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run pytest engine/argot/tests/test_corpus_benchmark.py -v`
Expected: all PASS.

- [ ] **Step 5: Run full verify to catch type/lint regressions**

Run: `just verify`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add engine/argot/corpus.py engine/argot/tests/test_corpus_benchmark.py
git commit -m "feat(research): per-mutation AUCs + synthetic_auc_mean in benchmark (Phase 7.0)"
```

---

### Task 8: Pin the honest corpus

**Files:**
- Create: `docs/research/scoring/15-honest-corpus.md`

**Spec:** Document the chosen pairs, URLs, commit SHAs (use current `HEAD` at clone time — plan author fills in during Task 11), expected record counts (hypotheses from Phase 7 design), and an empty **Swap-out log** section to be filled during execution.

- [ ] **Step 1: Write the corpus doc**

Create `docs/research/scoring/15-honest-corpus.md`:

```markdown
# Phase 7.0 — Honest Corpus

**Branch**: `research/phase-7-honest-eval`
**Status**: in progress — SHAs pinned at clone time; counts filled in post-extract

## Design

Same-language sub-corpora per bucket. Each bucket has a Python pair and a
TypeScript pair benchmarked **independently** (no TS+Py mixing).

Selection criteria:
- **Same domain** where possible (controls for domain-specific vocabulary).
- **Strong style contrast** on ≥ 3 axes from the Phase 7 style-axis reference
  (`DESIGN-phase-7.md` §Style-axis reference).

## Repos

| bucket | lang | repo A (URL, SHA)                            | repo B (URL, SHA)                            |
|:-------|:-----|:---------------------------------------------|:---------------------------------------------|
| small  | py   | https://github.com/encode/httpx @ <SHA>      | https://github.com/psf/requests @ <SHA>      |
| small  | ts   | https://github.com/sindresorhus/ky @ <SHA>   | https://github.com/chalk/chalk @ <SHA>       |
| medium | py   | https://github.com/tiangolo/fastapi @ <SHA>  | https://github.com/pallets/flask @ <SHA>     |
| medium | ts   | https://github.com/vitejs/vite @ <SHA>       | https://github.com/rollup/rollup @ <SHA>     |
| large  | py   | https://github.com/pydantic/pydantic @ <SHA> | https://github.com/django/django @ <SHA>     |
| large  | ts   | https://github.com/Effect-TS/effect @ <SHA>  | https://github.com/angular/angular @ <SHA>   |

**Pin SHAs to whatever `HEAD` is at clone time** (documented below per repo).
Do not hunt for a "nice" commit; reproducibility is what matters.

## Extracted record counts (post-extract)

Filled in after Task 11.

| repo          | records |
|:--------------|--------:|
| httpx         |       ? |
| requests      |       ? |
| ky            |       ? |
| chalk         |       ? |
| fastapi       |       ? |
| flask         |       ? |
| vite          |       ? |
| rollup        |       ? |
| pydantic      |       ? |
| django        |       ? |
| effect        |       ? |
| angular       |       ? |

## Bucket composition (post-downsample)

Each bucket concatenates its two same-language repos and downsamples to the
target count. Minimum share per sub-corpus after downsample: ≥ 40%.

| bucket | lang | target | actual (home + foreign, share) |
|:-------|:-----|-------:|:-------------------------------|
| small  | py   |   3000 | ?                              |
| small  | ts   |   3000 | ?                              |
| medium | py   |   7000 | ?                              |
| medium | ts   |   7000 | ?                              |
| large  | py   |  20000 | ?                              |
| large  | ts   |  20000 | ?                              |

If targets are unreachable (e.g. both chalk + ky land below 3000), rename the
bucket to its actual size rather than force-fit.

## Mutations

See `DESIGN-phase-7.md` §Feasible mutation set. Four mutations:
`case_swap`, `debug_inject`, `error_flip`, `quote_flip`.

Sample mutated records (filled in Task 14 after the sanity run).

## Harness usage

```bash
just research-honest-benchmark
# → runs the six (bucket × lang) combinations with the chosen encoder
#   writes to .argot/research/results-honest.jsonl
```

## Swap-out log

(empty — fill in if any candidate gets dropped during extraction)
```

- [ ] **Step 2: Commit the scaffold**

```bash
git add docs/research/scoring/15-honest-corpus.md
git commit -m "docs(research): 15-honest-corpus.md scaffold (Phase 7.0)"
```

---

### Task 9: Idempotent clone-and-extract script

**Files:**
- Create: `scripts/research/extract-honest-corpus.sh`

**Spec:** Shell script that, for each of the 12 repos (6 new + 6 already-extracted), clones if missing, checks out the default branch's current HEAD, runs `just extract` with `--repo-name`, and writes to `.argot/research/datasets-v2/<name>.jsonl`. Idempotent: skip if target file exists. Log the SHA it used per repo.

- [ ] **Step 1: Write the script**

```bash
# scripts/research/extract-honest-corpus.sh
#!/usr/bin/env bash
set -euo pipefail

WORK="${ARGOT_WORK:-$HOME/argot-research}"
OUT=".argot/research/datasets-v2"
LOG=".argot/research/datasets-v2/SHAS.md"

mkdir -p "$WORK" "$OUT"

declare -A REPOS=(
  [httpx]=https://github.com/encode/httpx
  [requests]=https://github.com/psf/requests
  [ky]=https://github.com/sindresorhus/ky
  [chalk]=https://github.com/chalk/chalk
  [fastapi]=https://github.com/tiangolo/fastapi
  [flask]=https://github.com/pallets/flask
  [vite]=https://github.com/vitejs/vite
  [rollup]=https://github.com/rollup/rollup
  [pydantic]=https://github.com/pydantic/pydantic
  [django]=https://github.com/django/django
  [effect]=https://github.com/Effect-TS/effect
  [angular]=https://github.com/angular/angular
)

echo "# Pinned SHAs" > "$LOG"
echo "" >> "$LOG"
echo "| repo | sha | default branch |" >> "$LOG"
echo "|:-----|:----|:---------------|" >> "$LOG"

for name in "${!REPOS[@]}"; do
  url="${REPOS[$name]}"
  out_jsonl="$OUT/$name.jsonl"
  repo_dir="$WORK/$name"

  if [[ ! -d "$repo_dir" ]]; then
    echo "==> cloning $name"
    git clone --depth 5000 "$url" "$repo_dir"
  fi

  branch=$(git -C "$repo_dir" symbolic-ref --short HEAD || echo "HEAD")
  sha=$(git -C "$repo_dir" rev-parse HEAD)
  echo "| $name | \`$sha\` | $branch |" >> "$LOG"

  if [[ -s "$out_jsonl" ]]; then
    echo "==> $name already extracted at $out_jsonl — skipping"
    continue
  fi

  echo "==> extracting $name → $out_jsonl"
  uv run --package argot-engine python -m argot.extract "$repo_dir" \
    --out "$out_jsonl" \
    --repo-name "$name"
done

echo ""
echo "SHA log written to $LOG"
```

- [ ] **Step 2: Make executable**

```bash
chmod +x scripts/research/extract-honest-corpus.sh
```

- [ ] **Step 3: Commit**

```bash
git add scripts/research/extract-honest-corpus.sh
git commit -m "chore(research): clone-and-extract script for honest corpus (Phase 7.0)"
```

---

### Task 10: Add `research-honest-benchmark` justfile recipe

**Files:**
- Modify: `justfile`

- [ ] **Step 1: Add the recipe**

Append to `justfile` under `# --- research ---`:

```
research-extract-honest:
    scripts/research/extract-honest-corpus.sh

research-concat-honest:
    uv run --package argot-engine python -m argot.corpus concat \
        .argot/research/datasets-v2/httpx.jsonl .argot/research/datasets-v2/requests.jsonl \
        -o .argot/research/buckets-v2/small-py.jsonl
    uv run --package argot-engine python -m argot.corpus concat \
        .argot/research/datasets-v2/ky.jsonl .argot/research/datasets-v2/chalk.jsonl \
        -o .argot/research/buckets-v2/small-ts.jsonl
    uv run --package argot-engine python -m argot.corpus concat \
        .argot/research/datasets-v2/fastapi.jsonl .argot/research/datasets-v2/flask.jsonl \
        -o .argot/research/buckets-v2/medium-py.jsonl
    uv run --package argot-engine python -m argot.corpus concat \
        .argot/research/datasets-v2/vite.jsonl .argot/research/datasets-v2/rollup.jsonl \
        -o .argot/research/buckets-v2/medium-ts.jsonl
    uv run --package argot-engine python -m argot.corpus concat \
        .argot/research/datasets-v2/pydantic.jsonl .argot/research/datasets-v2/django.jsonl \
        -o .argot/research/buckets-v2/large-py.jsonl
    uv run --package argot-engine python -m argot.corpus concat \
        .argot/research/datasets-v2/effect.jsonl .argot/research/datasets-v2/angular.jsonl \
        -o .argot/research/buckets-v2/large-ts.jsonl

research-honest-benchmark encoder="tfidf" seeds="3" out=".argot/research/results-honest.jsonl":
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset .argot/research/buckets-v2/small-py.jsonl --sizes 3000 --seeds {{seeds}} \
        --encoder {{encoder}} --out {{out}}
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset .argot/research/buckets-v2/small-ts.jsonl --sizes 3000 --seeds {{seeds}} \
        --encoder {{encoder}} --out {{out}}
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset .argot/research/buckets-v2/medium-py.jsonl --sizes 7000 --seeds {{seeds}} \
        --encoder {{encoder}} --out {{out}}
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset .argot/research/buckets-v2/medium-ts.jsonl --sizes 7000 --seeds {{seeds}} \
        --encoder {{encoder}} --out {{out}}
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset .argot/research/buckets-v2/large-py.jsonl --sizes 20000 --seeds {{seeds}} \
        --encoder {{encoder}} --out {{out}}
    uv run --package argot-engine python -m argot.corpus benchmark \
        --dataset .argot/research/buckets-v2/large-ts.jsonl --sizes 20000 --seeds {{seeds}} \
        --encoder {{encoder}} --out {{out}}
```

- [ ] **Step 2: Verify the recipes are listed**

```bash
just --list | grep honest
```

Expected: three lines matching `research-extract-honest`, `research-concat-honest`, `research-honest-benchmark`.

- [ ] **Step 3: Commit**

```bash
git add justfile
git commit -m "chore(research): just recipes for honest corpus + benchmark (Phase 7.0)"
```

---

### Task 11: Extract the honest corpus

**Files:**
- Generate: `.argot/research/datasets-v2/*.jsonl` (12 files)
- Generate: `.argot/research/datasets-v2/SHAS.md`

- [ ] **Step 1: Run the extraction**

```bash
just research-extract-honest
```

Expected: 12 JSONL files written. Approximate runtime: 20–60 minutes total for cold clones of Angular + Django.

- [ ] **Step 2: Count records per repo**

```bash
for f in .argot/research/datasets-v2/*.jsonl; do
  wc -l "$f"
done | tee /tmp/honest-counts.txt
```

- [ ] **Step 3: Update `15-honest-corpus.md`**

Fill in the "Extracted record counts" table and the "Repos" table's SHA column with values from `.argot/research/datasets-v2/SHAS.md` and `/tmp/honest-counts.txt`.

- [ ] **Step 4: Verify every extract has `language` populated**

```bash
for f in .argot/research/datasets-v2/*.jsonl; do
  head -1 "$f" | python -c "import json,sys; r=json.loads(sys.stdin.read()); assert 'language' in r and r['language'] in ('python','typescript','javascript'), (f, r.get('language')); print('ok:', sys.argv[1] if len(sys.argv)>1 else '', r['language'])" "$f"
done
```

Expected: every line prints `ok: <lang>`.

- [ ] **Step 5: Decision gate — post-extract hypothesis check**

Per the Phase 7 design (§Record-count targets are hypotheses):

- For each bucket×lang, is the combined record count within ±25% of the target (3k / 7k / 20k)?
- Is each sub-corpus ≥ 40% of its bucket target?

If a candidate fails the 40% rule, log the swap in `15-honest-corpus.md` §Swap-out log and either pick an alternative (note: angular alternative is `typescript-eslint`) or rename the bucket to its actual size.

**STOP — do not continue past this step without the user's confirmation on the counts.**

- [ ] **Step 6: Commit corpus pins**

```bash
git add docs/research/scoring/15-honest-corpus.md .argot/research/datasets-v2/SHAS.md
git commit -m "docs(research): pin honest corpus SHAs + counts (Phase 7.0)"
```

---

### Task 12: Build same-language buckets

**Files:**
- Generate: `.argot/research/buckets-v2/{small,medium,large}-{py,ts}.jsonl` (6 files)

- [ ] **Step 1: Concat**

```bash
just research-concat-honest
```

Expected: six bucket files written; printed summaries show per-repo counts per bucket.

- [ ] **Step 2: Spot-check one bucket**

```bash
head -1 .argot/research/buckets-v2/small-py.jsonl | python -c "import json,sys; print(json.loads(sys.stdin.read()).keys())"
```

Expected: includes `_repo`, `language`, `context_before`, `hunk_tokens` (and others). `language` must be `python`.

---

### Task 13: Sanity-check end-to-end on small-py

**Files:**
- Generate: `.argot/research/results-honest-sanity.jsonl`

**Purpose:** Prove the new harness produces the new metrics end-to-end on a single bucket before burning compute on the full matrix. Use TF-IDF because it's fast.

- [ ] **Step 1: Run the smallest slice**

```bash
uv run --package argot-engine python -m argot.corpus benchmark \
  --dataset .argot/research/buckets-v2/small-py.jsonl \
  --sizes 3000 --seeds 1 \
  --encoder tfidf \
  --out .argot/research/results-honest-sanity.jsonl
```

- [ ] **Step 2: Validate output shape**

```bash
cat .argot/research/results-honest-sanity.jsonl | python -c "
import json, sys
row = json.loads(sys.stdin.read().strip().splitlines()[-1])
required = {
    'shuffled_auc', 'cross_auc', 'injected_auc',
    'synthetic_auc_mean',
    'synthetic_auc_case_swap', 'synthetic_auc_debug_inject',
    'synthetic_auc_error_flip', 'synthetic_auc_quote_flip',
    'cross_auc_same_lang',
}
missing = required - set(row.keys())
assert not missing, f'missing keys: {missing}'
assert row['cross_auc_same_lang'] is not None, 'small-py is single-lang — should be computed'
for name in ('case_swap','debug_inject','error_flip','quote_flip'):
    auc = row[f'synthetic_auc_{name}']
    assert 0.0 <= auc <= 1.0, (name, auc)
print('sanity ok:', {k: row[k] for k in sorted(required)})
"
```

Expected: `sanity ok: {...}` with all AUCs in `[0,1]`.

- [ ] **Step 3: Capture sample mutated records**

For the `15-honest-corpus.md` doc, dump 2 sample mutated records per mutation:

```bash
uv run --package argot-engine python -c "
import json
from argot.mutations import apply_mutation, MUTATIONS
with open('.argot/research/buckets-v2/small-py.jsonl') as f:
    recs = [json.loads(l) for l in f][:2]
for name in MUTATIONS:
    for i, r in enumerate(recs):
        r_slim = {'language': r['language'], 'context_before': [{'text': t['text']} for t in r['context_before']], 'hunk_tokens': [{'text': t['text']} for t in r['hunk_tokens']]}
        r_slim['_repo']='demo'; r_slim['author_date_iso']='0'
        out = apply_mutation(name, r_slim, seed=0)
        before = ' '.join(t['text'] for t in r_slim['hunk_tokens'][:30])
        after = ' '.join(t['text'] for t in out['hunk_tokens'][:30])
        print(f'### {name} sample {i}')
        print(f'- before: \`{before}\`')
        print(f'- after:  \`{after}\`')
        print()
" | tee /tmp/mutation-samples.md
```

- [ ] **Step 4: Append samples to `15-honest-corpus.md`**

Paste the contents of `/tmp/mutation-samples.md` into the `## Mutations` section of `docs/research/scoring/15-honest-corpus.md`.

- [ ] **Step 5: Decision gate — mutation sanity**

Per the design (§Style-axis reference), each mutation should eventually produce AUC ≥ 0.55 when re-baselined against a reasonable encoder. At this sanity step we only check that AUCs *moved off 0.5*: if any mutation's AUC is in `[0.48, 0.52]` for TF-IDF, the mutation is either broken or invisible to bag-of-words. Likely suspects: `quote_flip` may be invisible to TF-IDF if the tokenizer already normalises quotes — document and continue anyway (Phase 7.1 with char_ngrams / BPE is the real test).

**STOP — show the sanity row to the user, wait for acknowledgement.**

- [ ] **Step 6: Commit**

```bash
git add docs/research/scoring/15-honest-corpus.md
git commit -m "docs(research): sample mutated records + sanity check (Phase 7.0)"
```

---

### Task 14: Full baseline-encoder matrix (optional — belongs to Phase 7.1)

**This task is deliberately empty.** The full matrix run across 4 encoders × 6 (bucket, lang) × 3 seeds is Phase 7.1 (`16-rebaseline.md`). Phase 7.0 concludes at the sanity step.

---

### Task 15: Update ROADMAP and close Phase 7.0

**Files:**
- Modify: `docs/research/scoring/ROADMAP.md`

- [ ] **Step 1: Mark 7.0 complete**

In `docs/research/scoring/ROADMAP.md`, change the Phase 7 checkbox:

```markdown
- [x] 7.0 eval rebuild — same-language pairs + synthetic mutations
      (`15-honest-corpus.md`). Post-extract: re-verify that candidate pairs
      (httpx+requests, fastapi+flask, pydantic+django, ky+chalk, vite+rollup,
      effect+angular) actually cluster at expected bucket sizes; swap/rename
      buckets if they don't.
```

Append a session-log entry under `## Session log`:

```markdown
- **YYYY-MM-DD**: Phase 7.0 complete. Honest corpus pinned (12 repos, SHAs
  recorded). `mutations.py` with 4 mutations (case_swap, debug_inject,
  error_flip, quote_flip) landed with unit-test coverage. `_benchmark_one`
  now emits `synthetic_auc_mean` + per-mutation AUCs + `cross_auc_same_lang`.
  Sanity run on small-py confirmed end-to-end; mutation samples recorded in
  `15-honest-corpus.md`. Next: Phase 7.1 — re-baseline 4 existing encoders
  on the new eval.
```

(Substitute today's date.)

Update the header's `Last touched`:

```markdown
**Last touched**: YYYY-MM-DD
```

- [ ] **Step 2: Verify**

```bash
just verify
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add docs/research/scoring/ROADMAP.md
git commit -m "docs(research): Phase 7.0 complete — honest eval harness + corpus pinned"
```

---

## Self-review checklist

Run these checks before handing off:

- [ ] Every task that changes code has a failing test *before* the implementation.
- [ ] `engine/argot/mutations.py` is imported by `corpus.py` — not `validate.py` — because the mutation step happens at benchmark time, not at score time.
- [ ] `cross_auc_same_lang` is `None` whenever the bucket contains more than one language. That can't happen in the new buckets (they're single-language by construction), but the field is defined generically so the harness remains correct if someone runs it on an old TS+Py bucket.
- [ ] The Phase 7 design-doc reference to `engine/argot/benchmark/mutations.py` is overridden by this plan's `engine/argot/mutations.py` — the rationale is in the File Structure section.
- [ ] `just verify` passes (mypy strict, ruff, oxfmt, knip, bun tests) after every code-changing task — not just the final one.
- [ ] The only `research/combined-optimizations` change during Phase 7.0 is zero — Phase 7 branches off `research/bpe-tokenisation`, the ship path is untouched.
