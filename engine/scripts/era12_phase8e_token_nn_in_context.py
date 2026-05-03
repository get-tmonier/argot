"""Era 12 Phase 8e — per-token NN distance in file context (single forward pass).

Production-viable variant of Phase 8.1's "per-token with context" insight.

For each hunk:
  1. Tokenize full file with offsets; locate hunk char range → hunk token range.
  2. Build a window of <=512 tokens centred on the hunk.
  3. ONE forward pass through the encoder (eval, no_grad).
  4. Last hidden states give per-token embeddings (768-d).
  5. For each HUNK token: cosine distance to its nearest CONTEXT token.
  6. Aggregate per hunk: max, top-3 mean, mean.

Compared to per-token MLM (~30 forward passes per hunk):
  - 1 forward pass per hunk → ~30× cheaper at inference, production-viable.
  - Mechanism: a hunk token whose semantic embedding is far from anything in
    its surrounding 460 context tokens is unusual for that file. `Math.random`
    in a faker-js provider should sit in "JS global library call" space while
    its context tokens are `faker.X`, `core.locale`, etc.
"""

from __future__ import annotations

import gc
import json
import sys
import time
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

MAX_WINDOW = 510  # leave room for CLS/SEP


# ---------- hunk reconstruction (same logic as Phase 8.1) ----------

import re

_BREAK_META_RE = re.compile(r"^\s*(//|#)\s*Break\s*:")


def _is_break_meta(ln: str) -> bool:
    return bool(_BREAK_META_RE.match(ln))


def _read(p: Path) -> str | None:
    try:
        return p.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None


def _load_manifests() -> dict[str, dict[str, dict[str, Any]]]:
    out: dict[str, dict[str, dict[str, Any]]] = {}
    for c in CORPORA:
        with open(BENCH_CATALOGS / c / "manifest.yaml") as f:
            m = yaml.safe_load(f)
        out[c] = {fx["id"]: fx for fx in m["fixtures"]}
    return out


def reconstruct(record: dict[str, Any], repo_dir: Path, catalog_dir: Path,
                manifest: dict[str, dict[str, Any]]) -> tuple[str, int, int] | None:
    """Return (full_text, hunk_char_start, hunk_char_end) or None."""
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
        # Strip break-meta comments before splicing.
        cat_lines = catalog_full.splitlines()
        cat_kept = [ln for ln in cat_lines if not _is_break_meta(ln)]
        chs0 = int(fx["hunk_start_line"])
        che0 = int(fx["hunk_end_line"])
        # Map old indices to new (1-indexed).
        old_to_new: list[int] = []
        new_idx = 0
        for ln in cat_lines:
            if _is_break_meta(ln):
                old_to_new.append(0)
            else:
                new_idx += 1
                old_to_new.append(new_idx)
        chs_new = next((old_to_new[k - 1] for k in range(chs0, che0 + 1)
                        if old_to_new[k - 1] != 0), None)
        che_new = next((old_to_new[k - 1] for k in range(che0, chs0 - 1, -1)
                        if old_to_new[k - 1] != 0), None)
        if chs_new is None or che_new is None:
            return None
        cat_stripped = "\n".join(cat_kept) + ("\n" if catalog_full.endswith("\n") else "")
        from argot.ml.features import synthesize_hunk_in_host  # type: ignore[import-not-found]
        full_text, hs, he = synthesize_hunk_in_host(
            cat_stripped, chs_new, che_new, host_full, int(host_inject)
        )
    else:
        rel = record.get("file_path")
        hs = record.get("hunk_start_line")
        he = record.get("hunk_end_line")
        if not (isinstance(rel, str) and isinstance(hs, int) and isinstance(he, int)):
            return None
        full_text = _read(repo_dir / rel) or ""
        if not full_text:
            return None
    lines = full_text.splitlines(keepends=True)
    if hs < 1 or he > len(lines) or he < hs:
        return None
    cs = sum(len(line) for line in lines[: hs - 1])
    ce = cs + sum(len(line) for line in lines[hs - 1 : he])
    return full_text, cs, ce


# ---------- scorer ----------


class TokenNNScorer:
    def __init__(self) -> None:
        import torch
        from transformers import AutoModel, AutoTokenizer  # type: ignore[import-not-found]

        self._torch = torch
        model_id = "microsoft/codebert-base-mlm"  # encoder is identical to base
        self._tokenizer = AutoTokenizer.from_pretrained(model_id, local_files_only=True)
        self._model = AutoModel.from_pretrained(model_id, local_files_only=True).eval()
        for p in self._model.parameters():
            p.requires_grad_(False)
        self._device = torch.device("cpu")
        self._cls = self._tokenizer.cls_token_id
        self._sep = self._tokenizer.sep_token_id

    def score(self, full_text: str, hunk_cs: int, hunk_ce: int) -> dict[str, Any]:
        torch = self._torch
        enc = self._tokenizer(full_text, add_special_tokens=False,
                              return_offsets_mapping=True, truncation=False)
        all_ids: list[int] = enc["input_ids"]
        offsets: list[tuple[int, int]] = enc["offset_mapping"]
        if not all_ids:
            return {"ok": False, "reason": "empty tokenization"}

        hunk_idx = [i for i, (s, e) in enumerate(offsets)
                    if s >= hunk_cs and e <= hunk_ce and e > s]
        if not hunk_idx:
            return {"ok": False, "reason": "no hunk tokens"}

        first, last = hunk_idx[0], hunk_idx[-1]
        hunk_size = last - first + 1
        truncated = False
        if hunk_size >= MAX_WINDOW:
            new_last = first + MAX_WINDOW - 1
            hunk_idx = [i for i in hunk_idx if i <= new_last]
            last = new_last
            hunk_size = MAX_WINDOW
            truncated = True

        ctx_budget = MAX_WINDOW - hunk_size
        before = ctx_budget // 2
        after = ctx_budget - before
        wstart = max(0, first - before)
        leftover = before - (first - wstart)
        wend = min(len(all_ids), last + 1 + after + leftover)
        if wend - wstart < MAX_WINDOW and wstart > 0:
            wstart = max(0, wend - MAX_WINDOW)

        body = all_ids[wstart:wend]
        n_context_tokens = len(body) - hunk_size
        input_ids = [self._cls, *body, self._sep]
        # Hunk positions in the windowed sequence: shift by 1 for CLS.
        hunk_pos_in_seq = [(i - wstart) + 1 for i in hunk_idx
                           if 0 <= (i - wstart) < len(body)]
        if not hunk_pos_in_seq:
            return {"ok": False, "reason": "hunk fell outside window"}
        # Context positions = everything except hunk positions and CLS/SEP.
        all_pos = set(range(1, len(body) + 1))
        ctx_pos_in_seq = sorted(all_pos - set(hunk_pos_in_seq))
        if not ctx_pos_in_seq:
            return {"ok": False, "reason": "no context tokens (window all hunk)"}

        ids_t = torch.tensor([input_ids], dtype=torch.long, device=self._device)
        with torch.no_grad():
            out = self._model(input_ids=ids_t)
            hs = out.last_hidden_state[0]  # (seq_len, 768)

        # L2-normalise so cosine sim = dot product.
        hs_n = hs / (hs.norm(dim=-1, keepdim=True) + 1e-12)
        hunk_emb = hs_n[hunk_pos_in_seq]    # (n_hunk, 768)
        ctx_emb = hs_n[ctx_pos_in_seq]      # (n_ctx, 768)
        # cosine SIMILARITY (n_hunk, n_ctx) → distance per hunk-token = 1 - max sim.
        sim = hunk_emb @ ctx_emb.T          # (n_hunk, n_ctx)
        nn_sim, _ = sim.max(dim=-1)         # nearest context-token similarity per hunk-token
        nn_dist = (1.0 - nn_sim).cpu().numpy()  # (n_hunk,)

        del ids_t, out, hs, hs_n, hunk_emb, ctx_emb, sim, nn_sim

        if len(nn_dist) == 0:
            return {"ok": False, "reason": "no hunk distances computed"}

        per_token = []
        for k, pos in enumerate(hunk_pos_in_seq):
            tok = self._tokenizer.convert_ids_to_tokens([input_ids[pos]])[0]
            per_token.append({
                "position": int(pos),
                "token": tok,
                "nn_dist": float(nn_dist[k]),
            })
        aggs = {
            "nn_dist_max": float(nn_dist.max()),
            "nn_dist_mean": float(nn_dist.mean()),
            "nn_dist_top3_mean": float(np.sort(nn_dist)[-3:].mean()
                                       if len(nn_dist) >= 3 else nn_dist.mean()),
            "nn_dist_p95": float(np.quantile(nn_dist, 0.95)),
        }
        return {
            "ok": True,
            "aggregations": aggs,
            "per_token": per_token,
            "n_hunk_tokens": len(hunk_pos_in_seq),
            "n_context_tokens": len(ctx_pos_in_seq),
            "truncated": truncated,
        }


# ---------- driver ----------


def iter_corpus(c: str):
    with (FEATURE_DIR / f"{c}.jsonl").open() as f:
        for line in f:
            yield json.loads(line)


def main() -> None:
    print("=== Phase 8e: per-token NN distance with file context ===", file=sys.stderr)
    t0 = time.time()
    scorer = TokenNNScorer()
    print(f"  scorer loaded in {time.time() - t0:.1f}s", file=sys.stderr, flush=True)

    manifests = _load_manifests()

    rows_out: list[dict[str, Any]] = []
    failed = 0

    for corpus in CORPORA:
        repo_dir = BENCH_DATA / corpus / ".repo"
        catalog_dir = BENCH_CATALOGS / corpus
        man = manifests[corpus]
        print(f"\n--- corpus={corpus} ---", file=sys.stderr, flush=True)
        i = 0
        for record in iter_corpus(corpus):
            i += 1
            ctx = reconstruct(record, repo_dir, catalog_dir, man)
            if ctx is None:
                failed += 1
                continue
            full_text, cs, ce = ctx
            try:
                r = scorer.score(full_text, cs, ce)
            except Exception as e:  # pragma: no cover
                print(f"  [{corpus} i={i}] ERROR {e}", file=sys.stderr, flush=True)
                failed += 1
                continue
            if not r.get("ok"):
                failed += 1
                continue
            rec_out: dict[str, Any] = {
                "corpus": corpus,
                "is_break": bool(record.get("is_break")),
                "fixture_id": record.get("fixture_id"),
                "category": record.get("category"),
                "file_path": record.get("file_path"),
                "cluster_id": record.get("features", {}).get("cluster_id"),
                "n_hunk_tokens": r["n_hunk_tokens"],
                "n_context_tokens": r["n_context_tokens"],
                "truncated": r["truncated"],
                **r["aggregations"],
            }
            if record.get("fixture_id") in RESIDUALS:
                rec_out["per_token"] = r["per_token"]
            rows_out.append(rec_out)
            if i % 25 == 0:
                el = time.time() - t0
                print(f"  [{corpus} i={i}] elapsed={el:.0f}s rate={len(rows_out)/max(el,1):.2f}/s",
                      file=sys.stderr, flush=True)
            if i % 200 == 0:
                gc.collect()

    print(f"\nDone: scored={len(rows_out)} failed={failed} runtime={time.time()-t0:.0f}s",
          file=sys.stderr, flush=True)

    # Build NumPy arrays for analysis.
    is_break = np.array([r["is_break"] for r in rows_out])
    corpus = np.array([r["corpus"] for r in rows_out])
    fid = np.array([r.get("fixture_id") or "" for r in rows_out])
    aggs_avail = ["nn_dist_max", "nn_dist_mean", "nn_dist_top3_mean", "nn_dist_p95"]

    from sklearn.metrics import roc_auc_score  # type: ignore[import-not-found]

    pooled_aucs: dict[str, float] = {}
    per_corpus_aucs: dict[str, dict[str, float | None]] = {a: {} for a in aggs_avail}
    thresholds: dict[str, dict[str, dict[str, Any]]] = {a: {} for a in aggs_avail}
    per_corpus_recall_fp: dict[str, dict[str, dict[str, Any]]] = {a: {} for a in aggs_avail}
    residuals: dict[str, dict[str, Any]] = {fx: {"per_aggregation": {}} for fx in RESIDUALS}

    for agg in aggs_avail:
        scores = np.array([r[agg] for r in rows_out])
        try:
            pooled_aucs[agg] = float(roc_auc_score(is_break.astype(int), scores))
        except ValueError:
            pooled_aucs[agg] = float("nan")
        for c in CORPORA:
            m = corpus == c
            y = is_break[m].astype(int)
            s = scores[m]
            if y.sum() in (0, len(y)):
                per_corpus_aucs[agg][c] = None
                continue
            per_corpus_aucs[agg][c] = float(roc_auc_score(y, s))
        for c in CORPORA:
            ctrl = (corpus == c) & (~is_break)
            n = int(ctrl.sum())
            if n == 0:
                continue
            q = 1 - FP_TARGET[c] / 100
            thr = float(np.quantile(scores[ctrl], q))
            cf = int((scores[ctrl] > thr).sum())
            thresholds[agg][c] = {
                "threshold": thr, "n_controls": n,
                "controls_flagged": cf, "actual_fp_pct": 100 * cf / n,
                "fp_target_pct": FP_TARGET[c],
            }
            br = (corpus == c) & is_break
            nb = int(br.sum())
            bc = int((scores[br] > thr).sum())
            per_corpus_recall_fp[agg][c] = {
                "threshold": thr, "fp_target_pct": FP_TARGET[c],
                "breaks_total": nb, "breaks_caught": bc,
                "stage4_recall_pct": 100 * bc / nb if nb else 0.0,
                "actual_fp_pct": 100 * cf / n,
                "fp_regression_pp": 100 * cf / n - FP_TARGET[c],
            }
        # Residual ranks.
        fjs_thr = thresholds[agg]["faker-js"]["threshold"]
        fjs_ctrl = scores[(corpus == "faker-js") & (~is_break)]
        fjs_sorted = np.sort(fjs_ctrl)
        n_fjs = len(fjs_sorted)
        for fx in RESIDUALS:
            idx = np.where(fid == fx)[0]
            if len(idx) == 0:
                residuals[fx]["per_aggregation"][agg] = {"missing": True}
                continue
            i = int(idx[0])
            s = float(scores[i])
            n_more = int((fjs_sorted > s).sum())
            residuals[fx]["per_aggregation"][agg] = {
                "score": s, "threshold": fjs_thr, "crosses": s > fjs_thr,
                "rank_top_pct": 100 * n_more / n_fjs,
            }

    verdict = {}
    for agg in aggs_avail:
        n_caught = sum(
            1 for fx in RESIDUALS
            if residuals[fx]["per_aggregation"].get(agg, {}).get("crosses") is True
        )
        no_reg = all(
            v["actual_fp_pct"] <= FP_TARGET[c] + 0.5
            for c, v in per_corpus_recall_fp[agg].items()
        )
        verdict[agg] = {
            "residuals_caught": n_caught,
            "ship_gate_pass": n_caught >= 2 and no_reg,
            "no_regression_gate_pass": no_reg,
        }

    # Per-residual diagnostic — keep top-5 per-token NN distances for each residual.
    per_token_diag = {}
    for r in rows_out:
        if r.get("fixture_id") in RESIDUALS and "per_token" in r:
            pt_sorted = sorted(r["per_token"], key=lambda x: -x["nn_dist"])[:10]
            per_token_diag[r["fixture_id"]] = pt_sorted

    out = {
        "method": "per-token NN distance to context tokens, single forward pass on [hunk + 460 ctx tokens]",
        "model": "microsoft/codebert-base-mlm (last_hidden_state)",
        "n_scored": len(rows_out),
        "n_failed": failed,
        "pooled_aucs_breaks_vs_controls": pooled_aucs,
        "per_corpus_auc": per_corpus_aucs,
        "thresholds": thresholds,
        "per_corpus_recall_fp": per_corpus_recall_fp,
        "residuals": residuals,
        "per_token_diag": per_token_diag,
        "verdict": verdict,
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
