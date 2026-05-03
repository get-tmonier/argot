"""Era 12 Phase 9 — import-source-aware rule-based anomaly scorer.

Diagnosed mechanism (Phase 8 series): the faker-js residuals are
**lexical / import-graph anomalies**, not semantic embedding anomalies.

  - runtime_fetch_*: use ``fetch`` — a JS global never imported by faker-js
    providers. Project policy: no network calls.
  - error_flip_*: introduce ``throw new Error`` in functions that normally
    return fallbacks.
  - foreign_rng_* (catalog non-residual): ``Math.random`` — JS global
    forbidden in faker-js (deterministic RNG via ``core.faker.*`` only).
  - http_sink_*, threading_*: similar global-API anomalies.

Architecture: language-agnostic scoring on top of the existing
``LanguageAdapter`` abstraction (TypeScript via tree-sitter, Python via
tree-sitter, future languages by registering a new adapter). No
``if language == ...`` branches in the scoring logic — only the curated
GLOBAL/IMPORT lookup tables vary per ecosystem (and they unify naturally
because callee strings like ``"fetch"``, ``"requests.get"`` are unique
across languages).

Score per hunk = count of dotted-callees that match a curated anomalous-
globals predicate AND are not covered by the file's import sources.
Per-corpus calibration at era-11 baseline FP. Residual catch + cross-
corpus FP audit.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any

import numpy as np
import yaml

ROOT = Path("/Users/damienmeur/projects/argot")
FEATURE_DIR = ROOT / "engine" / ".era12-features"
BENCH_DATA = ROOT / "benchmarks" / "data"
BENCH_CATALOGS = ROOT / "benchmarks" / "catalogs"
CORPORA = ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]

FP_TARGET = {
    "fastapi": 0.6, "rich": 1.2, "faker": 2.0,
    "hono": 0.5, "ink": 0.5, "faker-js": 0.9,
}

RESIDUALS = {
    "faker_js_error_flip_2", "faker_js_error_flip_3",
    "faker_js_runtime_fetch_1", "faker_js_runtime_fetch_2",
    "faker_js_runtime_fetch_3",
}

# ---------------------------------------------------------------------------
# Curated anomaly tables — language-agnostic by virtue of unique dotted-callee
# strings. The adapter parses callees; we just check membership / patterns.
# ---------------------------------------------------------------------------

# Network APIs across ecosystems. Each entry is a dotted-callee string OR a
# regex pattern (matched against the dotted callee).
_NETWORK_CALL_LITERALS: frozenset[str] = frozenset({
    # JS / TS
    "fetch", "XMLHttpRequest", "WebSocket", "EventSource",
    # Python
    "requests.get", "requests.post", "requests.put", "requests.patch",
    "requests.delete", "requests.head", "requests.options", "requests.request",
    "requests.Session",
    "httpx.get", "httpx.post", "httpx.put", "httpx.patch", "httpx.delete",
    "httpx.AsyncClient", "httpx.Client",
    "aiohttp.ClientSession", "aiohttp.request",
    "urllib.request.urlopen", "urllib.urlopen", "urlopen",
    "urllib3.PoolManager", "urllib3.request",
    "tornado.httpclient.HTTPClient", "tornado.httpclient.AsyncHTTPClient",
})

# Patterns: any dotted callee that begins with these → network-flavoured.
_NETWORK_CALL_PREFIX: tuple[str, ...] = (
    "axios.", "got.", "ky.", "request.", "superagent.",
)

# Non-deterministic globals — RNG, time. Faker is supposed to use seeded RNG
# from `core.faker.*` and avoid wall-clock dependence.
_NONDET_CALL_LITERALS: frozenset[str] = frozenset({
    "Math.random",
    "Date.now",
    "performance.now",
    # Python equivalents
    "random.random", "random.randint", "random.choice", "random.sample",
    "random.uniform", "random.seed", "random.shuffle",
    "time.time", "time.monotonic",
})

# Crypto — `crypto.randomBytes` etc. are flagged unless the file imports
# `crypto` / `node:crypto` / Python's `secrets` etc.
_CRYPTO_CALL_PREFIX: tuple[str, ...] = (
    "crypto.", "secrets.",
)

# File / process / env globals — mostly out of scope for this phase but
# helpful as background.
_ENV_CALL_PREFIX: tuple[str, ...] = (
    "process.env.", "os.environ.", "os.getenv",
)

# If the file imports any of these sources, network-flavoured callees are
# considered legitimate. Cross-language; the adapter normalises to module
# specifier strings (e.g. ``"axios"``, ``"node-fetch"``, ``"httpx"``,
# ``"requests"``).
_NETWORK_IMPORT_SOURCES: frozenset[str] = frozenset({
    "axios", "node-fetch", "got", "ky", "request", "superagent",
    "undici", "isomorphic-fetch", "cross-fetch", "whatwg-fetch",
    "https", "http", "node:http", "node:https", "node:net",
    "hono",
    "requests", "urllib", "urllib.request", "urllib3",
    "httpx", "aiohttp", "tornado", "treq", "pycurl",
})

_CRYPTO_IMPORT_SOURCES: frozenset[str] = frozenset({
    "crypto", "node:crypto", "secrets", "cryptography",
    "Crypto",  # PyCryptodome
})

# `throw`/`raise` patterns — these we count separately as a control-flow
# feature (Python doesn't expose them as callees, so keep regex here but
# keep it small; for TS, `throw new X(...)` is a rare construct in faker
# providers and is captured similarly).
_THROW_NEW_RE = re.compile(r"\bthrow\s+new\s+\w+\s*\(")
_RAISE_RE = re.compile(r"\braise\s+\w+\s*\(")


# ---------------------------------------------------------------------------
# Hunk + file reconstruction (same as Phase 8.1, copied here for self-
# containment of the research script).
# ---------------------------------------------------------------------------

_BREAK_META_RE = re.compile(r"^\s*(//|#)\s*Break\s*:")


def _is_break_meta(ln: str) -> bool:
    return bool(_BREAK_META_RE.match(ln))


def _read(p: Path) -> str | None:
    try:
        return p.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None


def _load_manifest(corpus: str) -> dict[str, dict[str, Any]]:
    with (BENCH_CATALOGS / corpus / "manifest.yaml").open() as f:
        m = yaml.safe_load(f)
    return {fx["id"]: fx for fx in m["fixtures"]}


def reconstruct(record: dict[str, Any], repo_dir: Path, catalog_dir: Path,
                manifest: dict[str, dict[str, Any]]
                ) -> tuple[str, int, int, str] | None:
    """Return (full_text, hunk_char_start, hunk_char_end, file_extension)."""
    if record.get("is_break"):
        fid = record.get("fixture_id")
        if fid is None or fid not in manifest:
            return None
        fx = manifest[fid]
        host_rel = fx.get("host_file")
        host_inject = fx.get("host_inject_at_line")
        if host_rel is None or host_inject is None:
            return None
        host_full = _read(repo_dir / str(host_rel))
        if host_full is None:
            return None
        catalog_full = _read(catalog_dir / str(fx["file"]))
        if catalog_full is None:
            return None
        cat_lines = catalog_full.splitlines()
        cat_kept = [ln for ln in cat_lines if not _is_break_meta(ln)]
        chs0 = int(fx["hunk_start_line"])
        che0 = int(fx["hunk_end_line"])
        old_to_new: list[int] = []
        new_idx = 0
        for ln in cat_lines:
            if _is_break_meta(ln):
                old_to_new.append(0)
            else:
                new_idx += 1
                old_to_new.append(new_idx)
        chs_new = next(
            (old_to_new[k - 1] for k in range(chs0, che0 + 1)
             if old_to_new[k - 1] != 0), None
        )
        che_new = next(
            (old_to_new[k - 1] for k in range(che0, chs0 - 1, -1)
             if old_to_new[k - 1] != 0), None
        )
        if chs_new is None or che_new is None:
            return None
        cat_stripped = "\n".join(cat_kept) + ("\n" if catalog_full.endswith("\n") else "")
        from argot.ml.features import synthesize_hunk_in_host  # type: ignore[import-not-found]

        full_text, hs, he = synthesize_hunk_in_host(
            cat_stripped, chs_new, che_new, host_full, int(host_inject)
        )
        # Use the host file's extension (real corpus file) as language hint.
        host_path_str = str(host_rel)
    else:
        rel = record.get("file_path")
        hs = record.get("hunk_start_line")
        he = record.get("hunk_end_line")
        if not (isinstance(rel, str) and isinstance(hs, int) and isinstance(he, int)):
            return None
        full_text = _read(repo_dir / rel) or ""
        if not full_text:
            return None
        host_path_str = rel

    lines = full_text.splitlines(keepends=True)
    if hs < 1 or he > len(lines) or he < hs:
        return None
    cs = sum(len(line) for line in lines[: hs - 1])
    ce = cs + sum(len(line) for line in lines[hs - 1 : he])
    ext = host_path_str[host_path_str.rfind("."):] if "." in host_path_str else ".ts"
    return full_text, cs, ce, ext


# ---------------------------------------------------------------------------
# Scoring (language-agnostic — uses adapter for parsing)
# ---------------------------------------------------------------------------


def score_hunk(
    hunk_text: str,
    file_text: str,
    adapter: Any,
) -> dict[str, Any]:
    """Compute Phase 9 features.

    Uses ``adapter.extract_callees`` and ``adapter.extract_imports`` for
    language-aware parsing. The scoring predicates are language-agnostic.
    """
    callees = adapter.extract_callees(hunk_text)
    file_import_sources = adapter.extract_imports(file_text)

    has_network_import = bool(file_import_sources & _NETWORK_IMPORT_SOURCES)
    has_crypto_import = bool(file_import_sources & _CRYPTO_IMPORT_SOURCES)

    n_unimported_globals = 0
    n_fetch_like = 0
    n_nondet = 0
    n_crypto = 0
    n_env = 0
    flagged: list[str] = []

    for callee in callees:
        if callee in _NETWORK_CALL_LITERALS:
            if not has_network_import:
                n_fetch_like += 1
                n_unimported_globals += 1
                flagged.append(f"network:{callee}")
            continue
        if any(callee.startswith(pfx) for pfx in _NETWORK_CALL_PREFIX):
            if not has_network_import:
                n_fetch_like += 1
                n_unimported_globals += 1
                flagged.append(f"network:{callee}")
            continue
        if callee in _NONDET_CALL_LITERALS:
            n_nondet += 1
            n_unimported_globals += 1
            flagged.append(f"nondet:{callee}")
            continue
        if any(callee.startswith(pfx) for pfx in _CRYPTO_CALL_PREFIX):
            if not has_crypto_import:
                n_crypto += 1
                n_unimported_globals += 1
                flagged.append(f"crypto:{callee}")
            continue
        if any(callee.startswith(pfx) for pfx in _ENV_CALL_PREFIX):
            n_env += 1
            n_unimported_globals += 1
            flagged.append(f"env:{callee}")

    n_throw_new = len(_THROW_NEW_RE.findall(hunk_text)) + len(
        _RAISE_RE.findall(hunk_text)
    )

    return {
        "n_unimported_globals": float(n_unimported_globals),
        "n_fetch_like": float(n_fetch_like),
        "n_nondet": float(n_nondet),
        "n_crypto": float(n_crypto),
        "n_env": float(n_env),
        "n_throw_new": float(n_throw_new),
        "_callees": callees,
        "_flagged": flagged,
    }


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def iter_corpus(c: str):
    with (FEATURE_DIR / f"{c}.jsonl").open() as f:
        for line in f:
            yield json.loads(line)


def main() -> None:
    print("=== Phase 9: import-source-aware rules (all corpora) ===", file=sys.stderr)

    from argot.scoring.adapters.registry import get_adapter  # type: ignore[import-not-found]

    rows_out: list[dict[str, Any]] = []
    failed: dict[str, int] = {c: 0 for c in CORPORA}

    # Cache: (file extension -> adapter) and (file_path -> imports).
    imports_cache: dict[str, frozenset[str]] = {}

    for corpus in CORPORA:
        repo_dir = BENCH_DATA / corpus / ".repo"
        catalog_dir = BENCH_CATALOGS / corpus
        manifest = _load_manifest(corpus)
        n = 0
        for record in iter_corpus(corpus):
            n += 1
            ctx = reconstruct(record, repo_dir, catalog_dir, manifest)
            if ctx is None:
                failed[corpus] += 1
                continue
            full_text, cs, ce, ext = ctx
            hunk_text = full_text[cs:ce]
            adapter = get_adapter(ext)

            cache_key = (
                f"break::{record.get('fixture_id')}"
                if record.get("is_break") else f"ctrl::{corpus}::{record.get('file_path')}"
            )
            if cache_key in imports_cache:
                # Re-extract for breaks: the synthesized text differs from cached.
                # Only safe to cache controls; for breaks always re-parse.
                if record.get("is_break"):
                    file_imports = adapter.extract_imports(full_text)
                else:
                    file_imports = imports_cache[cache_key]
            else:
                file_imports = adapter.extract_imports(full_text)
                if not record.get("is_break"):
                    imports_cache[cache_key] = file_imports
            # Adapter-call wrapper: call score_hunk with a tiny shim that
            # presents pre-computed imports.

            class _ShimAdapter:
                def __init__(self, base: Any, imports: frozenset[str]) -> None:
                    self._base = base
                    self._imports = imports

                def extract_callees(self, source: str) -> list[str]:
                    return self._base.extract_callees(source)

                def extract_imports(self, source: str) -> frozenset[str]:
                    return self._imports

            shim = _ShimAdapter(adapter, file_imports)
            feats = score_hunk(hunk_text, full_text, shim)
            callees = feats.pop("_callees")
            flagged = feats.pop("_flagged")

            rec_out: dict[str, Any] = {
                "corpus": corpus,
                "is_break": bool(record.get("is_break")),
                "fixture_id": record.get("fixture_id"),
                "category": record.get("category"),
                "file_path": record.get("file_path"),
                "cluster_id": record.get("features", {}).get("cluster_id"),
                "n_callees": len(callees),
                **feats,
            }
            if record.get("fixture_id") in RESIDUALS:
                rec_out["flagged_tokens"] = flagged
                rec_out["callees"] = callees
                rec_out["hunk_first_line"] = (hunk_text.splitlines() or [""])[0][:160]
                rec_out["import_sources"] = sorted(file_imports)
            rows_out.append(rec_out)

        print(f"  {corpus}: scored={n - failed[corpus]} failed={failed[corpus]}",
              file=sys.stderr, flush=True)

    is_break = np.array([r["is_break"] for r in rows_out])
    corpus = np.array([r["corpus"] for r in rows_out])
    fid = np.array([r.get("fixture_id") or "" for r in rows_out])

    feature_names = ["n_unimported_globals", "n_fetch_like", "n_nondet",
                     "n_crypto", "n_env", "n_throw_new"]
    feature_arrays: dict[str, np.ndarray] = {
        f: np.array([r[f] for r in rows_out]) for f in feature_names
    }

    # Per-corpus + pooled AUC, threshold, residual catch, FP audit.
    from sklearn.metrics import roc_auc_score  # type: ignore[import-not-found]

    out: dict[str, Any] = {
        "method": "import-source-aware rule features (adapter-based; language-agnostic scoring)",
        "n_rows_scored": len(rows_out),
        "failed_per_corpus": failed,
        "feature_aucs": {},
        "per_corpus": {},
        "residuals": {},
    }

    # Pooled AUCs.
    for f, arr in feature_arrays.items():
        try:
            out["feature_aucs"][f] = float(roc_auc_score(is_break.astype(int), arr))
        except ValueError:
            out["feature_aucs"][f] = None

    # Per-corpus, per-feature: threshold, FP, recall.
    for c in CORPORA:
        cmask = corpus == c
        out["per_corpus"][c] = {}
        for f in feature_names:
            arr = feature_arrays[f]
            ctrl = cmask & (~is_break)
            ctrl_scores = arr[ctrl]
            if len(ctrl_scores) == 0:
                continue
            q = 1 - FP_TARGET[c] / 100
            thr = float(np.quantile(ctrl_scores, q))
            cf = int((ctrl_scores > thr).sum())
            actual_fp = 100 * cf / len(ctrl_scores)
            br = cmask & is_break
            nb = int(br.sum())
            bc = int((arr[br] > thr).sum())
            out["per_corpus"][c][f] = {
                "threshold": thr,
                "fp_target_pct": FP_TARGET[c],
                "actual_fp_pct": actual_fp,
                "fp_regression_pp": actual_fp - FP_TARGET[c],
                "n_controls": int(len(ctrl_scores)),
                "controls_flagged": cf,
                "breaks_total": nb,
                "breaks_caught": bc,
                "stage4_recall_pct": 100 * bc / nb if nb else 0.0,
            }

    # Residual catch (faker-js residuals; per-feature).
    for fx in sorted(RESIDUALS):
        idx = np.where(fid == fx)[0]
        if len(idx) == 0:
            out["residuals"][fx] = {"missing": True}
            continue
        i = int(idx[0])
        rec = rows_out[i]
        d: dict[str, Any] = {
            "values": {f: float(feature_arrays[f][i]) for f in feature_names},
            "callees": rec.get("callees"),
            "flagged_tokens": rec.get("flagged_tokens"),
            "hunk_first_line": rec.get("hunk_first_line"),
            "import_sources": rec.get("import_sources", []),
        }
        # For each feature: would this residual cross its faker-js threshold?
        d["catch_per_feature"] = {}
        for f in feature_names:
            thr = out["per_corpus"]["faker-js"][f]["threshold"]
            d["catch_per_feature"][f] = {
                "threshold": thr,
                "value": float(feature_arrays[f][i]),
                "crosses": float(feature_arrays[f][i]) > thr,
            }
        out["residuals"][fx] = d

    # Cross-corpus FP-regression report — focus on n_fetch_like and
    # n_unimported_globals, which are the candidates for shipping.
    out["ship_summary"] = {}
    for f in ["n_fetch_like", "n_unimported_globals"]:
        rows = []
        for c in CORPORA:
            v = out["per_corpus"][c].get(f, {})
            rows.append({
                "corpus": c,
                "thr": v.get("threshold"),
                "actual_fp_pct": v.get("actual_fp_pct"),
                "fp_target_pct": v.get("fp_target_pct"),
                "fp_regression_pp": v.get("fp_regression_pp"),
                "breaks_caught": v.get("breaks_caught"),
                "breaks_total": v.get("breaks_total"),
                "stage4_recall_pct": v.get("stage4_recall_pct"),
            })
        out["ship_summary"][f] = rows

    # Residual catch counts for each feature.
    out["residual_catch_per_feature"] = {}
    for f in feature_names:
        n_caught = sum(
            1 for fx in RESIDUALS
            if not out["residuals"].get(fx, {}).get("missing")
            and out["residuals"][fx]["catch_per_feature"][f]["crosses"]
        )
        out["residual_catch_per_feature"][f] = n_caught

    # Per-corpus list of CAUGHT break fixture-ids for the candidate features
    # (so we can cross-check against era 11's existing catches).
    out["caught_break_ids"] = {}
    for f in ["n_fetch_like", "n_unimported_globals"]:
        out["caught_break_ids"][f] = {}
        for c in CORPORA:
            thr = out["per_corpus"][c][f]["threshold"]
            ids = []
            for r in rows_out:
                if r["corpus"] != c or not r["is_break"]:
                    continue
                if r[f] > thr:
                    ids.append({
                        "fixture_id": r["fixture_id"],
                        "category": r["category"],
                        "value": r[f],
                    })
            out["caught_break_ids"][f][c] = ids

    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
