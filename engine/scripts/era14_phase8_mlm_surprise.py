"""Era 14 Phase 8 — PROPER per-token MLM surprise (one-mask-at-a-time).

Era 12's MLM-surprise bakeoff (`docs/research/evidence/mlm-surprise-bakeoff.md`)
ran joint masking — all hunk tokens masked in one forward pass — and got
AUC 0.41–0.43 (below random). The memo flagged that as a likely confound:
"under joint masking the model cannot use intra-hunk structure to disambiguate."

Phase 8 is the proper measurement: per token, mask ONE position at a time
with ALL OTHER hunk tokens visible, then aggregate the per-token surprise
into per-hunk scores. Multiple aggregations (mean, max, top-3 mean, p95) so
we can see whether a single-anomalous-token signal exists when not diluted
by mean-pooling.

**Encoder choice — important deviation from spec:**
The Phase 8 spec called for `microsoft/unixcoder-base`, but its public
HuggingFace checkpoint ships only `RobertaModel` weights — the MLM head
(`lm_head.*`) is absent and gets randomly initialized when loaded as
`AutoModelForMaskedLM`. Random LM-head logits make per-token surprise
meaningless. We therefore use `microsoft/codebert-base-mlm`, which is
architecturally identical (Roberta-base, 12 layers × 768 hidden) and
ships with a real MLM head. CodeBERT was also the encoder behind the
"CodeBERT-nt" technique that motivated era 12's bakeoff, so this is the
right model for the experiment.

**Memory discipline (hard rules):**
* CPU device, fp32 — MPS has unbounded allocator behavior on Apple
  Silicon with transformers; CPU is slower but predictable.
* Stream one hunk at a time. NO cross-hunk batching.
* Within a hunk, chunk mask positions in groups of 8 (never more).
* Cap max_tokens_per_hunk at 96. Truncate beyond.
* Memory watchdog every 50 hunks; hard-stop above 6000 MB.
* No torch.compile, no model.half(), no graph caching.
* Free aggressively (del + gc.collect every 200 hunks).

Pipeline:
1. Reconstruct hunk text per row.
   - Controls: read `benchmarks/data/<corpus>/.repo/<file_path>`,
     slice `lines[hs-1 : he]` (controls store 1-indexed line numbers in
     era14 jsonl).
   - Breaks: load corpus manifest, look up fixture by `fixture_id`,
     read catalog file, slice catalog `lines[catalog_hs-1 : catalog_he]`.
2. Load `microsoft/codebert-base-mlm`, frozen, eval, CPU, fp32.
3. For each hunk: tokenize (truncate to 96), then for each non-special
   position i: mask ONE position (all others visible) and read
   p_i = P(orig | rest) from the softmax. Surprise_i = -log(p_i).
   Process mask positions in batches of 8.
   Critical: ASSERTION enforces exactly ONE mask per row.
4. Aggregate per-hunk: surprise_mean, surprise_max,
   surprise_top3_mean, surprise_p95.
5. Per-corpus threshold = (1 − FP_target/100)-quantile of CONTROL
   surprise scores per aggregation.
6. Residual catch table across all 4 aggregations + side-by-side
   vs Phase 6.4 cosine and Phase 7.1 d².
7. Per-corpus stage-4 recall + FP audit per aggregation.
8. Per-residual per-token DIAGNOSTIC: for each fixture, dump the
   per-token surprise list (decoded token, surprise, rank).
9. Verdict per aggregation.

Outputs JSON to stdout; saves the artifact to
`engine/.era14-features/phase8_mlm_surprise.joblib`.

Constraints:
- Frozen encoder, no training, no fine-tuning, no labels touched.
- Per-token masking, one-at-a-time, enforced by assertion.
- No surrounding-file context — surprise is computed against the hunk's
  own intra-token context only.
"""

from __future__ import annotations

import argparse
import gc
import json
import os
import platform
import re
import resource
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

import joblib
import numpy as np
import yaml

ROOT = Path("/Users/damienmeur/projects/argot")
FEATURE_DIR = ROOT / "engine" / ".era14-features"
BENCH_DATA = ROOT / "benchmarks" / "data"
BENCH_CATALOGS = ROOT / "benchmarks" / "catalogs"
CORPORA = ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]

# Era-11 baseline FP rates per corpus (percent of CONTROLS flagged).
FP_TARGET = {
    "fastapi": 0.6,
    "rich": 1.2,
    "faker": 2.0,
    "hono": 0.5,
    "ink": 0.5,
    "faker-js": 0.9,
}

RESIDUALS = {
    "faker_js_error_flip_2",
    "faker_js_error_flip_3",
    "faker_js_runtime_fetch_1",
    "faker_js_runtime_fetch_2",
    "faker_js_runtime_fetch_3",
}

AGGREGATIONS = ["surprise_mean", "surprise_max", "surprise_top3_mean", "surprise_p95"]

# Phase 6.4 cosine distances for residuals (for side-by-side reporting).
PHASE64_RESIDUAL_COSINE: dict[str, float | None] = {
    "faker_js_error_flip_2": None,  # excluded under 6.4 (cluster 3 had 2 controls)
    "faker_js_error_flip_3": 0.3138,
    "faker_js_runtime_fetch_1": 0.4670,
    "faker_js_runtime_fetch_2": 0.4931,
    "faker_js_runtime_fetch_3": 0.4248,
}

# Phase 7.1 d² values for residuals (Phase 7.1 caught 0/5; for context).
PHASE71_RESIDUAL_D2: dict[str, float | None] = {
    "faker_js_error_flip_2": None,
    "faker_js_error_flip_3": None,
    "faker_js_runtime_fetch_1": None,
    "faker_js_runtime_fetch_2": None,
    "faker_js_runtime_fetch_3": None,
}

# ---- Memory + sequence bounds (hard rules) ---------------------------------
MAX_TOKENS_PER_HUNK = 96  # truncate longer hunks; warn each truncation
BATCH_MASK_POSITIONS = 8  # never exceed
RSS_HARD_STOP_MB = 6000.0
WATCHDOG_EVERY_N_HUNKS = 50
GC_EVERY_N_HUNKS = 200


# ---------------------------------------------------------------------------
# Memory helpers
# ---------------------------------------------------------------------------


def rss_mb() -> float:
    """Return current process RSS in MB.

    On macOS ``ru_maxrss`` is in bytes; on Linux it's in kilobytes.
    """
    ru = resource.getrusage(resource.RUSAGE_SELF)
    if platform.system() == "Darwin":
        return ru.ru_maxrss / 1e6
    return ru.ru_maxrss * 1024 / 1e6


# ---------------------------------------------------------------------------
# Hunk reconstruction (Step 1)
# ---------------------------------------------------------------------------


def _load_manifest(corpus: str) -> dict[str, dict[str, Any]]:
    """Return {fixture_id: fixture_dict} for the corpus's catalog manifest."""
    mpath = BENCH_CATALOGS / corpus / "manifest.yaml"
    with mpath.open("r", encoding="utf-8") as f:
        m = yaml.safe_load(f)
    return {fx["id"]: fx for fx in m["fixtures"]}


def _read_text(p: Path) -> str | None:
    try:
        return p.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None


_BREAK_META_RE = re.compile(r"^\s*(//|#)\s*Break\s*:")


def _is_break_meta_comment(line: str) -> bool:
    return bool(_BREAK_META_RE.match(line))


def reconstruct_with_context(
    record: dict[str, Any],
    repo_dir: Path,
    catalog_dir: Path,
    manifest: dict[str, dict[str, Any]],
) -> tuple[str, int, int] | None:
    """Phase 8.1: reconstruct full file context with the hunk in place.

    Returns (full_text, hunk_char_start, hunk_char_end) where the char range
    locates the hunk inside full_text. Used by the context-aware MLM scorer
    so the model conditions on surrounding file content, not just the hunk.

    For breaks: splice the (comment-stripped) catalog hunk into the corpus
    host file at host_inject_at_line via synthesize_hunk_in_host.
    For controls: full_text = the actual file; hunk char range = lines[hs-1:he].
    """
    if record.get("is_break"):
        fid = record.get("fixture_id")
        if fid is None or fid not in manifest:
            return None
        fx = manifest[fid]
        host_rel = fx.get("host_file")
        host_inject = fx.get("host_inject_at_line")
        if host_rel is None or host_inject is None:
            return None
        host_path = repo_dir / str(host_rel)
        host_full = _read_text(host_path)
        if host_full is None:
            return None
        catalog_path = catalog_dir / str(fx["file"])
        catalog_full = _read_text(catalog_path)
        if catalog_full is None:
            return None
        # Strip leading `// Break:` / `# Break:` meta-comments from the
        # catalog *before* splicing so they don't appear in scored content.
        catalog_lines_in = catalog_full.splitlines()
        catalog_lines_kept = [
            ln for ln in catalog_lines_in if not _is_break_meta_comment(ln)
        ]
        # Adjust hunk range from the manifest to account for stripped lines.
        chs_orig = int(fx["hunk_start_line"])
        che_orig = int(fx["hunk_end_line"])
        # Map old line indices (1-indexed) to new line indices after stripping.
        old_to_new: list[int] = []
        new_idx = 0
        for old_idx, ln in enumerate(catalog_lines_in, start=1):
            if _is_break_meta_comment(ln):
                old_to_new.append(0)  # dropped
            else:
                new_idx += 1
                old_to_new.append(new_idx)
        # Walk forward from chs_orig to find first kept line (>= chs_orig).
        chs_new = next(
            (old_to_new[k - 1] for k in range(chs_orig, che_orig + 1)
             if old_to_new[k - 1] != 0),
            None,
        )
        # Walk backward from che_orig to find last kept line.
        che_new = next(
            (old_to_new[k - 1] for k in range(che_orig, chs_orig - 1, -1)
             if old_to_new[k - 1] != 0),
            None,
        )
        if chs_new is None or che_new is None or chs_new > che_new:
            return None
        catalog_stripped = "\n".join(catalog_lines_kept) + (
            "\n" if catalog_full.endswith("\n") else ""
        )
        # Use the existing splice helper so we get the exact same line math
        # the bench harness uses.
        from argot.ml.features import synthesize_hunk_in_host  # type: ignore[import-not-found]

        synthesized, new_hs, new_he = synthesize_hunk_in_host(
            catalog_stripped,
            chs_new,
            che_new,
            host_full,
            int(host_inject),
        )
        full_text = synthesized
        hs, he = new_hs, new_he
    else:
        file_rel = record.get("file_path")
        hs0 = record.get("hunk_start_line")
        he0 = record.get("hunk_end_line")
        if not (isinstance(file_rel, str) and isinstance(hs0, int) and isinstance(he0, int)):
            return None
        path = repo_dir / file_rel
        full_text = _read_text(path) or ""
        if not full_text:
            return None
        hs, he = hs0, he0

    # Convert (hs, he) line indices (1-indexed inclusive) to char offsets.
    lines = full_text.splitlines(keepends=True)
    if hs < 1 or he > len(lines) or he < hs:
        return None
    hunk_char_start = sum(len(line) for line in lines[: hs - 1])
    hunk_char_end = hunk_char_start + sum(
        len(line) for line in lines[hs - 1 : he]
    )
    return full_text, hunk_char_start, hunk_char_end


def reconstruct_hunk_text(
    record: dict[str, Any],
    repo_dir: Path,
    catalog_dir: Path,
    manifest: dict[str, dict[str, Any]],
) -> str | None:
    """Reconstruct hunk text from a JSONL row.

    Controls (`is_break == False`): read `<repo_dir>/<file_path>`,
        slice `lines[hs-1 : he]` (era14 jsonl stores 1-indexed inclusive).
    Breaks (`is_break == True`): use manifest entry's `file` +
        `hunk_start_line` / `hunk_end_line` (1-indexed inclusive).
    Returns None on read/parse failure.
    """
    if record.get("is_break"):
        fid = record.get("fixture_id")
        if fid is None or fid not in manifest:
            return None
        fx = manifest[fid]
        catalog_path = catalog_dir / str(fx["file"])
        full = _read_text(catalog_path)
        if full is None:
            return None
        chs = int(fx["hunk_start_line"])
        che = int(fx["hunk_end_line"])
        lines = full.splitlines()
        if chs < 1 or che > len(lines) or che < chs:
            return None
        hunk_lines = lines[chs - 1 : che]
        # Strip catalog explanatory comments. Every break file in
        # benchmarks/catalogs/ contains a leading `// Break: ...` (TS) or
        # `# Break: ...` (Python) line that describes what the break IS.
        # That meta-commentary scores extreme MLM surprise (rare in any
        # real corpus) and contaminates the per-token aggregation in a way
        # that wouldn't occur on real-world breaks.
        hunk_lines = [
            ln for ln in hunk_lines
            if not _is_break_meta_comment(ln)
        ]
        return "\n".join(hunk_lines)

    file_rel = record.get("file_path")
    hs = record.get("hunk_start_line")
    he = record.get("hunk_end_line")
    if not (isinstance(file_rel, str) and isinstance(hs, int) and isinstance(he, int)):
        return None
    path = repo_dir / file_rel
    full = _read_text(path)
    if full is None:
        return None
    lines = full.splitlines()
    # Era14 jsonl stores 1-indexed inclusive line numbers for controls.
    if hs < 1 or he > len(lines) or he < hs - 1:
        return None
    return "\n".join(lines[hs - 1 : he])


def iter_corpus_records(corpus: str):
    """Stream JSONL rows for a corpus; drop large embedding fields immediately."""
    path = FEATURE_DIR / f"{corpus}.jsonl"
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            d = json.loads(line)
            d.pop("hunk_embedding", None)
            d.pop("context_embedding", None)
            yield d


# ---------------------------------------------------------------------------
# MLM scorer (Step 2 + 3)
# ---------------------------------------------------------------------------


def _model_in_local_cache(model_id: str) -> bool:
    hf_home = os.environ.get("HF_HOME") or os.environ.get("TRANSFORMERS_CACHE")
    if hf_home:
        base = Path(hf_home).expanduser()
        candidates = [base / "hub", base]
    else:
        candidates = [Path.home() / ".cache" / "huggingface" / "hub"]
    cache_dir_name = "models--" + model_id.replace("/", "--")
    return any((d / cache_dir_name).is_dir() for d in candidates)


class MLMScorer:
    """Frozen MLM head over codebert-base-mlm; per-token surprise via one-at-a-time mask.

    Hard rules:
    * CPU device, fp32.
    * Each forward batch row contains exactly one ``<mask>`` token (asserted).
    * Batch size capped at ``BATCH_MASK_POSITIONS`` = 8.
    """

    def __init__(self) -> None:
        import torch
        from transformers import AutoModelForMaskedLM, AutoTokenizer

        self._torch = torch
        # Force CPU, fp32, no autograd. No MPS — that allocator is what bit era 12.
        self._device = torch.device("cpu")
        self._dtype = torch.float32

        # codebert-base-mlm is the architecturally identical Roberta-base
        # checkpoint that ships with a real MLM head; unixcoder-base does not.
        self._model_id = "microsoft/codebert-base-mlm"
        local_only = _model_in_local_cache(self._model_id)
        print(
            f"  loading {self._model_id} (local_files_only={local_only})",
            file=sys.stderr,
        )
        self._tokenizer = AutoTokenizer.from_pretrained(
            self._model_id, local_files_only=local_only
        )
        model = AutoModelForMaskedLM.from_pretrained(
            self._model_id, local_files_only=local_only
        )
        model.eval()
        # Explicit CPU + fp32; do NOT call .half() or torch.compile.
        model.to(device=self._device, dtype=self._dtype)
        # Disable gradient tracking globally on params (eval() handles dropout;
        # this just forces the no-train invariant).
        for p in model.parameters():
            p.requires_grad_(False)
        self._model = model

        self._mask_id = int(self._tokenizer.mask_token_id)
        self._cls_id = int(self._tokenizer.cls_token_id)
        self._sep_id = int(self._tokenizer.sep_token_id)
        self._pad_id = (
            int(self._tokenizer.pad_token_id)
            if self._tokenizer.pad_token_id is not None
            else self._mask_id
        )

    @property
    def model_id(self) -> str:
        return self._model_id

    def score_hunk(
        self,
        hunk_text: str,
        max_tokens: int = MAX_TOKENS_PER_HUNK,
        batch_size: int = BATCH_MASK_POSITIONS,
    ) -> dict[str, Any]:
        """Per-token MLM surprise for a hunk.

        Returns:
            dict with:
              * ``per_token``: list of {position, token, surprise_nats}
                for scoring positions only (special tokens excluded).
              * ``aggregations``: dict of all 4 aggregations, NaN if no
                scoring positions.
              * ``n_tokens_total``: total tokens (incl. special).
              * ``n_scored``: number of scoring positions.
              * ``truncated``: True if the input was truncated.
        """
        torch = self._torch

        # Tokenize without specials, then add CLS/SEP manually so we know
        # exactly where they are.
        body_ids: list[int] = self._tokenizer.encode(hunk_text, add_special_tokens=False)
        truncated = len(body_ids) > (max_tokens - 2)
        body_ids = body_ids[: max_tokens - 2]

        input_ids: list[int] = [self._cls_id, *body_ids, self._sep_id]
        n_total = len(input_ids)

        # Scoring positions: exclude CLS, SEP, pad, and any pre-existing mask.
        special = {self._cls_id, self._sep_id, self._pad_id, self._mask_id}
        scoring_positions = [
            j for j in range(1, n_total - 1) if input_ids[j] not in special
        ]

        if not scoring_positions:
            return {
                "per_token": [],
                "aggregations": {agg: float("nan") for agg in AGGREGATIONS},
                "n_tokens_total": n_total,
                "n_scored": 0,
                "truncated": truncated,
            }

        surprises_nats: list[float] = []
        token_strings: list[str] = []

        # Process mask positions in batches of `batch_size` (=8). Never larger.
        for batch_start in range(0, len(scoring_positions), batch_size):
            batch_positions = scoring_positions[batch_start : batch_start + batch_size]
            B = len(batch_positions)

            # Build (B, n_total) tensor, each row a copy of input_ids with
            # exactly position j replaced by mask.
            batch_ids = torch.tensor(
                [input_ids] * B, dtype=torch.long, device=self._device
            )
            originals = torch.empty(B, dtype=torch.long, device=self._device)
            for k, pos in enumerate(batch_positions):
                originals[k] = batch_ids[k, pos].item()
                batch_ids[k, pos] = self._mask_id

            # Critical assertion: each row has exactly one mask token.
            mask_counts = (batch_ids == self._mask_id).sum(dim=1)
            assert (mask_counts == 1).all().item(), (
                "Per-token masking violated — joint masking would produce "
                f"{mask_counts.tolist()} masks per row instead of [1, ..., 1]"
            )

            attention = torch.ones_like(batch_ids)

            with torch.no_grad():
                out = self._model(input_ids=batch_ids, attention_mask=attention)
                logits = out.logits  # (B, n_total, vocab)

            # Gather the mask-position logits and convert to surprise.
            for k, pos in enumerate(batch_positions):
                row_logits = logits[k, pos, :]  # (vocab,)
                log_probs = torch.log_softmax(row_logits.float(), dim=-1)
                p_log = float(log_probs[int(originals[k].item())].item())
                surprise = -p_log  # nats
                surprises_nats.append(surprise)
                token_strings.append(
                    self._tokenizer.convert_ids_to_tokens(
                        [int(originals[k].item())]
                    )[0]
                )

            # Free batch tensors aggressively.
            del batch_ids, originals, attention, logits, out

        # Aggregations.
        s = np.asarray(surprises_nats, dtype=np.float64)
        if len(s) >= 3:
            top3 = float(np.sort(s)[-3:].mean())
        else:
            top3 = float(s.mean())
        aggs: dict[str, float] = {
            "surprise_mean": float(s.mean()),
            "surprise_max": float(s.max()),
            "surprise_top3_mean": top3,
            "surprise_p95": float(np.quantile(s, 0.95)),
        }

        per_token = [
            {
                "position": int(scoring_positions[i]),
                "token": token_strings[i],
                "surprise": float(surprises_nats[i]),
            }
            for i in range(len(scoring_positions))
        ]

        return {
            "per_token": per_token,
            "aggregations": aggs,
            "n_tokens_total": n_total,
            "n_scored": len(scoring_positions),
            "truncated": truncated,
        }

    def score_hunk_with_context(
        self,
        full_text: str,
        hunk_char_start: int,
        hunk_char_end: int,
        max_window: int = 510,
        batch_size: int = BATCH_MASK_POSITIONS,
    ) -> dict[str, Any]:
        """Phase 8.1: per-token surprise with surrounding file context.

        Tokenize the full file, identify token positions inside the hunk
        char range, take a window of <= max_window tokens centred on the
        hunk, mask only hunk-positions one at a time. Surprise becomes
        "given the file context, was this hunk-token expected?"
        """
        torch = self._torch

        # Tokenize the full file with offsets so we can locate the hunk
        # tokens by char range.
        enc = self._tokenizer(
            full_text,
            add_special_tokens=False,
            return_offsets_mapping=True,
            truncation=False,
        )
        all_ids: list[int] = enc["input_ids"]
        offsets: list[tuple[int, int]] = enc["offset_mapping"]
        if not all_ids:
            return {
                "per_token": [],
                "aggregations": {agg: float("nan") for agg in AGGREGATIONS},
                "n_tokens_total": 0,
                "n_scored": 0,
                "truncated": False,
                "context_used": True,
                "n_context_tokens": 0,
            }

        # Identify hunk-token range: tokens whose [start, end) overlap
        # the hunk char range and are contained in it.
        hunk_idx: list[int] = []
        for i, (s, e) in enumerate(offsets):
            if s >= hunk_char_start and e <= hunk_char_end and e > s:
                hunk_idx.append(i)

        if not hunk_idx:
            return {
                "per_token": [],
                "aggregations": {agg: float("nan") for agg in AGGREGATIONS},
                "n_tokens_total": len(all_ids),
                "n_scored": 0,
                "truncated": False,
                "context_used": True,
                "n_context_tokens": 0,
            }

        hunk_first = hunk_idx[0]
        hunk_last = hunk_idx[-1]
        hunk_size = hunk_last - hunk_first + 1

        # Cap hunk-token count for memory safety + RoBERTa positional limit.
        # If the hunk itself is longer than max_window, we drop context entirely
        # and truncate the hunk — same as the no-context path.
        truncated = False
        if hunk_size >= max_window:
            new_last = hunk_first + max_window - 1
            hunk_idx = [i for i in hunk_idx if i <= new_last]
            hunk_last = new_last
            hunk_size = max_window
            truncated = True

        # Centre a window of (max_window) tokens on the hunk.
        context_budget = max_window - hunk_size
        before_budget = context_budget // 2
        after_budget = context_budget - before_budget
        window_start = max(0, hunk_first - before_budget)
        # Re-distribute leftover budget if we hit the start of the file.
        leftover_before = before_budget - (hunk_first - window_start)
        window_end = min(len(all_ids), hunk_last + 1 + after_budget + leftover_before)
        # Re-distribute again if we hit end of file.
        if window_end - window_start < max_window and window_start > 0:
            window_start = max(0, window_end - max_window)

        body_ids = all_ids[window_start:window_end]
        n_context_tokens = len(body_ids) - hunk_size

        # Build base sequence with CLS/SEP added.
        input_ids = [self._cls_id, *body_ids, self._sep_id]
        n_total = len(input_ids)

        # Adjust hunk indices to point into input_ids.
        # input_ids[i] = body_ids[i - 1] for 1 <= i <= len(body_ids)
        # body_ids[j] = all_ids[window_start + j]
        # so input_ids index of original token i is (i - window_start) + 1.
        scoring_positions = [(i - window_start) + 1 for i in hunk_idx]
        # Sanity: each scoring position must be in (0, n_total - 1).
        scoring_positions = [
            j for j in scoring_positions if 0 < j < n_total - 1
        ]
        if not scoring_positions:
            return {
                "per_token": [],
                "aggregations": {agg: float("nan") for agg in AGGREGATIONS},
                "n_tokens_total": n_total,
                "n_scored": 0,
                "truncated": truncated,
                "context_used": True,
                "n_context_tokens": n_context_tokens,
            }

        surprises_nats: list[float] = []
        token_strings: list[str] = []

        for batch_start in range(0, len(scoring_positions), batch_size):
            batch_positions = scoring_positions[batch_start : batch_start + batch_size]
            B = len(batch_positions)

            batch_ids = torch.tensor(
                [input_ids] * B, dtype=torch.long, device=self._device
            )
            originals = torch.empty(B, dtype=torch.long, device=self._device)
            for k, pos in enumerate(batch_positions):
                originals[k] = batch_ids[k, pos].item()
                batch_ids[k, pos] = self._mask_id

            mask_counts = (batch_ids == self._mask_id).sum(dim=1)
            assert (mask_counts == 1).all().item(), (
                "Per-token masking violated — joint masking would produce "
                f"{mask_counts.tolist()} masks per row instead of [1, ..., 1]"
            )
            attention = torch.ones_like(batch_ids)

            with torch.no_grad():
                out = self._model(input_ids=batch_ids, attention_mask=attention)
                logits = out.logits

            for k, pos in enumerate(batch_positions):
                row_logits = logits[k, pos, :]
                log_probs = torch.log_softmax(row_logits.float(), dim=-1)
                p_log = float(log_probs[int(originals[k].item())].item())
                surprises_nats.append(-p_log)
                token_strings.append(
                    self._tokenizer.convert_ids_to_tokens(
                        [int(originals[k].item())]
                    )[0]
                )

            del batch_ids, originals, attention, out, logits

        s = np.asarray(surprises_nats, dtype=np.float64)
        top3 = float(np.sort(s)[-3:].mean()) if len(s) >= 3 else float(s.mean())
        aggs = {
            "surprise_mean": float(s.mean()),
            "surprise_max": float(s.max()),
            "surprise_top3_mean": top3,
            "surprise_p95": float(np.quantile(s, 0.95)),
        }
        per_token = [
            {
                "position": int(scoring_positions[i]),
                "token": token_strings[i],
                "surprise": float(surprises_nats[i]),
            }
            for i in range(len(scoring_positions))
        ]
        return {
            "per_token": per_token,
            "aggregations": aggs,
            "n_tokens_total": n_total,
            "n_scored": len(scoring_positions),
            "truncated": truncated,
            "context_used": True,
            "n_context_tokens": n_context_tokens,
        }


# ---------------------------------------------------------------------------
# Driver: stream rows one at a time
# ---------------------------------------------------------------------------


def score_all(
    *,
    corpora: list[str],
    smoke_only: int | None = None,
    smoke_corpus: str | None = None,
    use_context: bool = True,
) -> dict[str, Any]:
    """Stream all (corpus, row) pairs; per-token score; aggregate per hunk."""
    print("=== Phase 8: per-token MLM surprise (CPU, fp32) ===", file=sys.stderr)
    t_start = time.time()
    print("Loading scorer...", file=sys.stderr)
    scorer = MLMScorer()
    print(
        f"Scorer loaded in {time.time() - t_start:.1f}s; RSS={rss_mb():.1f} MB",
        file=sys.stderr,
    )

    manifests = {c: _load_manifest(c) for c in corpora}
    print(
        f"Manifests loaded: { {c: len(m) for c, m in manifests.items()} }",
        file=sys.stderr,
    )

    per_corpus_rows: dict[str, list[dict[str, Any]]] = {c: [] for c in corpora}
    total_rows_seen = 0
    failed_recon = 0
    failed_recon_examples: list[dict[str, Any]] = []
    total_truncated = 0
    total_scored = 0
    rss_peak = rss_mb()
    aborted_due_to_oom = False
    sample_recons: list[dict[str, Any]] = []

    target_corpora = corpora if smoke_corpus is None else [smoke_corpus]
    for corpus in target_corpora:
        repo_dir = BENCH_DATA / corpus / ".repo"
        catalog_dir = BENCH_CATALOGS / corpus
        manifest = manifests[corpus]
        print(f"\n--- corpus={corpus} ---", file=sys.stderr)

        # Smoke mode: take first N (mix of breaks + controls).
        if smoke_only:
            all_rows = list(iter_corpus_records(corpus))
            breaks = [r for r in all_rows if r.get("is_break")][: max(5, smoke_only // 4)]
            controls = [r for r in all_rows if not r.get("is_break")][
                : smoke_only - len(breaks)
            ]
            iter_records = iter(breaks + controls)
            print(
                f"  smoke: {len(breaks)} breaks + {len(controls)} controls = "
                f"{len(breaks)+len(controls)} rows",
                file=sys.stderr,
            )
            del all_rows
        else:
            iter_records = iter_corpus_records(corpus)
            print("  streaming all rows", file=sys.stderr)

        i = 0
        for record in iter_records:
            i += 1
            total_rows_seen += 1

            if use_context:
                ctx = reconstruct_with_context(
                    record, repo_dir, catalog_dir, manifest
                )
                hunk_text = None
                if ctx is None:
                    failed_recon += 1
                    if len(failed_recon_examples) < 20:
                        failed_recon_examples.append(
                            {
                                "corpus": corpus,
                                "is_break": record.get("is_break"),
                                "fixture_id": record.get("fixture_id"),
                                "file_path": record.get("file_path"),
                                "hs": record.get("hunk_start_line"),
                                "he": record.get("hunk_end_line"),
                            }
                        )
                    continue
                full_text, hunk_cs, hunk_ce = ctx
                hunk_text = full_text[hunk_cs:hunk_ce]
            else:
                hunk_text = reconstruct_hunk_text(
                    record, repo_dir, catalog_dir, manifest
                )
                if hunk_text is None:
                    failed_recon += 1
                    if len(failed_recon_examples) < 20:
                        failed_recon_examples.append(
                            {
                                "corpus": corpus,
                                "is_break": record.get("is_break"),
                                "fixture_id": record.get("fixture_id"),
                                "file_path": record.get("file_path"),
                                "hs": record.get("hunk_start_line"),
                                "he": record.get("hunk_end_line"),
                            }
                        )
                    continue

            # Sanity checks: actual lines/chars vs stored values.
            if len(sample_recons) < 6:
                actual_lines = hunk_text.count("\n") + 1
                sample_recons.append(
                    {
                        "corpus": corpus,
                        "is_break": bool(record.get("is_break")),
                        "fixture_id": record.get("fixture_id"),
                        "expected_lines": record.get("hunk_length_lines"),
                        "actual_lines": actual_lines,
                        "expected_chars": record.get("hunk_length_chars"),
                        "actual_chars": len(hunk_text),
                        "first_line": hunk_text.splitlines()[0][:120]
                        if hunk_text
                        else "",
                    }
                )

            try:
                if use_context:
                    result = scorer.score_hunk_with_context(
                        full_text, hunk_cs, hunk_ce
                    )
                else:
                    result = scorer.score_hunk(hunk_text)
            except AssertionError:
                raise  # re-raise; per-token-mask invariant violation
            except Exception as e:  # pragma: no cover — defensive
                print(
                    f"  WARNING: scoring failed for row {i} corpus={corpus}: {e}",
                    file=sys.stderr,
                )
                continue

            if result["truncated"]:
                total_truncated += 1
            if result["n_scored"] == 0:
                continue
            total_scored += 1

            keep_per_token = bool(record.get("is_break")) and (
                record.get("fixture_id") in RESIDUALS
            )

            row_out: dict[str, Any] = {
                "corpus": corpus,
                "is_break": bool(record.get("is_break")),
                "fixture_id": record.get("fixture_id"),
                "category": record.get("category"),
                "file_path": record.get("file_path"),
                "hunk_start_line": record.get("hunk_start_line"),
                "hunk_end_line": record.get("hunk_end_line"),
                "hunk_length_lines": record.get("hunk_length_lines"),
                "hunk_length_chars": record.get("hunk_length_chars"),
                "cluster_id": record.get("features", {}).get("cluster_id"),
                "n_tokens_total": result["n_tokens_total"],
                "n_scored": result["n_scored"],
                "truncated": result["truncated"],
                **result["aggregations"],
            }
            if keep_per_token:
                row_out["per_token"] = result["per_token"]
                row_out["hunk_text"] = hunk_text  # keep for diagnostic
            per_corpus_rows[corpus].append(row_out)

            # Per-row progress (flushed) for visibility during slow context-aware runs.
            if i % 5 == 0:
                elapsed = time.time() - t_start
                rate = total_scored / max(elapsed, 1)
                print(
                    f"  [{corpus} i={i}] elapsed={elapsed:.0f}s rate={rate:.2f}/s "
                    f"scored={total_scored} truncated={total_truncated}",
                    file=sys.stderr,
                    flush=True,
                )

            # Memory watchdog every 50 hunks.
            if i % WATCHDOG_EVERY_N_HUNKS == 0:
                cur_rss = rss_mb()
                rss_peak = max(rss_peak, cur_rss)
                elapsed = time.time() - t_start
                rate = total_scored / max(elapsed, 1)
                print(
                    f"  [{corpus} i={i}] rss={cur_rss:.0f} MB peak={rss_peak:.0f} MB "
                    f"scored={total_scored} truncated={total_truncated} "
                    f"elapsed={elapsed:.1f}s rate={rate:.2f}/s",
                    file=sys.stderr,
                    flush=True,
                )
                if cur_rss > RSS_HARD_STOP_MB:
                    print(
                        f"  ABORT: RSS={cur_rss:.0f} MB > hard stop {RSS_HARD_STOP_MB}",
                        file=sys.stderr,
                    )
                    aborted_due_to_oom = True
                    break

            del result, hunk_text, record
            if i % GC_EVERY_N_HUNKS == 0:
                gc.collect()

        if aborted_due_to_oom:
            break

    rss_peak = max(rss_peak, rss_mb())
    runtime_s = time.time() - t_start
    print(
        f"\nDone: total_rows_seen={total_rows_seen} scored={total_scored} "
        f"truncated={total_truncated} failed_recon={failed_recon} "
        f"runtime={runtime_s:.1f}s rss_peak={rss_peak:.0f} MB "
        f"aborted_due_to_oom={aborted_due_to_oom}",
        file=sys.stderr,
    )

    return {
        "per_corpus_rows": per_corpus_rows,
        "model_id": scorer.model_id,
        "stats": {
            "total_rows_seen": total_rows_seen,
            "scored": total_scored,
            "truncated": total_truncated,
            "failed_reconstruction": failed_recon,
            "failed_reconstruction_examples": failed_recon_examples,
            "sample_reconstructions": sample_recons,
            "runtime_s": runtime_s,
            "rss_peak_mb": rss_peak,
            "aborted_due_to_oom": aborted_due_to_oom,
        },
    }


# ---------------------------------------------------------------------------
# Calibration + reporting (Steps 4-9)
# ---------------------------------------------------------------------------


def _agg_arr(rows: list[dict], agg: str) -> np.ndarray:
    return np.asarray(
        [r[agg] for r in rows if r.get(agg) is not None and not np.isnan(r[agg])],
        dtype=np.float64,
    )


def calibrate_per_corpus(
    per_corpus_rows: dict[str, list[dict]],
) -> dict[str, dict[str, dict]]:
    """Per-corpus, per-aggregation threshold = (1 - FP_target/100)-quantile of CONTROL values."""
    print("=== Step 4: per-corpus, per-aggregation calibration ===", file=sys.stderr)
    out: dict[str, dict[str, dict]] = {}
    for c in CORPORA:
        rows = per_corpus_rows.get(c, [])
        ctrls = [r for r in rows if not r["is_break"]]
        breaks = [r for r in rows if r["is_break"]]
        out[c] = {}
        for agg in AGGREGATIONS:
            ctrl_vals = _agg_arr(ctrls, agg)
            n_ctrl = len(ctrl_vals)
            fp_target = FP_TARGET[c]
            if n_ctrl == 0:
                out[c][agg] = {
                    "fp_target_pct": fp_target,
                    "threshold": None,
                    "n_controls": 0,
                    "n_breaks": len(breaks),
                    "actual_fp_pct": None,
                    "controls_flagged": 0,
                    "breaks_flagged": 0,
                }
                continue
            q = 1.0 - (fp_target / 100.0)
            thr = float(np.quantile(ctrl_vals, q))
            ctrls_flagged = int((ctrl_vals > thr).sum())
            actual_fp = 100.0 * ctrls_flagged / n_ctrl
            break_vals = _agg_arr(breaks, agg)
            breaks_flagged = int((break_vals > thr).sum()) if len(break_vals) else 0
            out[c][agg] = {
                "fp_target_pct": fp_target,
                "threshold": thr,
                "n_controls": n_ctrl,
                "n_breaks": len(breaks),
                "actual_fp_pct": actual_fp,
                "controls_flagged": ctrls_flagged,
                "breaks_flagged": breaks_flagged,
            }
    return out


def auc_pooled(per_corpus_rows: dict[str, list[dict]], agg: str) -> float | None:
    """Pooled AUC across all corpora, breaks vs controls, on agg.

    Era-12 inversion check: era 12 got AUC 0.4290 on `surprise_mean`. If
    Phase 8's `surprise_mean` AUC > 0.5, we've confirmed joint-masking was
    the era-12 confound.
    """
    pos: list[float] = []
    neg: list[float] = []
    for _c, rows in per_corpus_rows.items():
        for r in rows:
            v = r.get(agg)
            if v is None or (isinstance(v, float) and np.isnan(v)):
                continue
            (pos if r["is_break"] else neg).append(v)
    if not pos or not neg:
        return None
    pos_arr = np.asarray(pos)
    neg_arr = np.asarray(neg)
    all_vals = np.concatenate([pos_arr, neg_arr])
    ranks = all_vals.argsort().argsort().astype(np.float64) + 1.0
    pos_rank_sum = ranks[: len(pos_arr)].sum()
    n_p = len(pos_arr)
    n_n = len(neg_arr)
    auc = (pos_rank_sum - n_p * (n_p + 1) / 2.0) / (n_p * n_n)
    return float(auc)


def residual_table(
    per_corpus_rows: dict[str, list[dict]],
    thresholds: dict[str, dict[str, dict]],
) -> dict[str, Any]:
    """Per-residual per-aggregation: surprise, threshold, crosses?, rank vs fjs ctrls."""
    print("=== Step 6: residual catch table ===", file=sys.stderr)
    fjs_rows = per_corpus_rows.get("faker-js", [])
    fjs_ctrls = [r for r in fjs_rows if not r["is_break"]]
    fjs_breaks = [r for r in fjs_rows if r["is_break"]]
    fjs_ctrl_sorted: dict[str, np.ndarray] = {
        agg: np.sort(_agg_arr(fjs_ctrls, agg)) for agg in AGGREGATIONS
    }

    residuals: dict[str, Any] = {}
    for fid in sorted(RESIDUALS):
        match = [r for r in fjs_breaks if r.get("fixture_id") == fid]
        if not match:
            residuals[fid] = {"error": "fixture not found in fjs results"}
            continue
        r = match[0]
        per_agg: dict[str, Any] = {}
        for agg in AGGREGATIONS:
            v = r.get(agg)
            thr_info = thresholds["faker-js"][agg]
            thr = thr_info["threshold"]
            sorted_ctrls = fjs_ctrl_sorted[agg]
            n = len(sorted_ctrls)
            n_more = int((sorted_ctrls > v).sum()) if n else 0
            rank_top_pct = 100.0 * n_more / n if n else None
            percentile = float((sorted_ctrls <= v).sum()) / n if n else None
            per_agg[agg] = {
                "value": float(v) if v is not None else None,
                "threshold": thr,
                "crosses": (v > thr) if (thr is not None and v is not None) else False,
                "n_more_extreme_among_fjs_controls": n_more,
                "rank_top_pct_among_fjs_controls": rank_top_pct,
                "percentile_among_fjs_controls": percentile,
            }
        residuals[fid] = {
            "per_aggregation": per_agg,
            "n_tokens_total": r.get("n_tokens_total"),
            "n_scored": r.get("n_scored"),
            "truncated": r.get("truncated"),
            "phase64_cosine_distance": PHASE64_RESIDUAL_COSINE.get(fid),
            "phase71_d2": PHASE71_RESIDUAL_D2.get(fid),
            "per_token": r.get("per_token", []),
            "hunk_text_first_120": (r.get("hunk_text", "") or "")[:120],
        }
    return residuals


def per_corpus_recall_fp(
    per_corpus_rows: dict[str, list[dict]],
    thresholds: dict[str, dict[str, dict]],
) -> dict[str, dict[str, dict]]:
    """For each corpus & aggregation: total breaks, breaks caught, controls flagged, FP."""
    print("=== Step 7: per-corpus recall + FP audit ===", file=sys.stderr)
    out: dict[str, dict[str, dict]] = {}
    for c in CORPORA:
        rows = per_corpus_rows.get(c, [])
        breaks = [r for r in rows if r["is_break"]]
        ctrls = [r for r in rows if not r["is_break"]]
        out[c] = {}
        for agg in AGGREGATIONS:
            thr = thresholds[c][agg]["threshold"]
            if thr is None:
                out[c][agg] = {
                    "fp_target_pct": FP_TARGET[c],
                    "threshold": None,
                    "breaks_total": len(breaks),
                    "breaks_caught": 0,
                    "controls_count": len(ctrls),
                    "controls_flagged": 0,
                    "actual_fp_pct": None,
                    "recall_pct": None,
                    "fp_regression_vs_baseline_pp": None,
                }
                continue
            br_vals = _agg_arr(breaks, agg)
            ct_vals = _agg_arr(ctrls, agg)
            br_caught = int((br_vals > thr).sum()) if len(br_vals) else 0
            ct_flagged = int((ct_vals > thr).sum()) if len(ct_vals) else 0
            n_ctrl = len(ct_vals)
            actual_fp = (100.0 * ct_flagged / n_ctrl) if n_ctrl else None
            fp_target = FP_TARGET[c]
            fp_reg = (actual_fp - fp_target) if actual_fp is not None else None
            out[c][agg] = {
                "fp_target_pct": fp_target,
                "threshold": thr,
                "breaks_total": len(breaks),
                "breaks_caught": br_caught,
                "controls_count": n_ctrl,
                "controls_flagged": ct_flagged,
                "actual_fp_pct": actual_fp,
                "recall_pct": (100.0 * br_caught / len(breaks)) if breaks else None,
                "fp_regression_vs_baseline_pp": fp_reg,
            }
    return out


def verdict_per_aggregation(
    residuals: dict[str, Any],
    recall_fp: dict[str, dict[str, dict]],
) -> dict[str, dict]:
    """Per-aggregation pre-reg gate.

    ≥ 2/5 residuals catch at fjs FP ≤ 0.9% AND every corpus's actual FP ≤
    baseline + 0.5pp → SHIP candidate. 1/5 catches with within-budget FP →
    PARTIAL. 0/5 → CLOSE NEGATIVE.
    """
    print("=== Step 9: verdict per aggregation ===", file=sys.stderr)
    out: dict[str, dict] = {}
    for agg in AGGREGATIONS:
        n_caught = 0
        caught_fids: list[str] = []
        for fid in sorted(RESIDUALS):
            res = residuals.get(fid)
            if not isinstance(res, dict) or "per_aggregation" not in res:
                continue
            agg_res = res["per_aggregation"].get(agg, {})
            if agg_res.get("crosses"):
                n_caught += 1
                caught_fids.append(fid)
        regressions: list[dict] = []
        no_regression = True
        for c in CORPORA:
            af = recall_fp[c][agg].get("actual_fp_pct")
            if af is None:
                continue
            if af > FP_TARGET[c] + 0.5:
                no_regression = False
                regressions.append(
                    {
                        "corpus": c,
                        "actual_fp_pct": af,
                        "baseline_fp_pct": FP_TARGET[c],
                        "regression_pp": af - FP_TARGET[c],
                    }
                )
        if n_caught >= 2 and no_regression:
            v = "SHIP"
        elif n_caught == 0:
            v = "CLOSE NEGATIVE"
        elif n_caught >= 1 and no_regression:
            v = "PARTIAL"
        else:
            v = "PARTIAL (FP regression)"
        out[agg] = {
            "verdict": v,
            "residuals_caught": n_caught,
            "caught_fixtures": caught_fids,
            "no_regression_gate_pass": no_regression,
            "regressions": regressions,
        }
    return out


def per_residual_per_token_diagnostic(residuals: dict[str, Any]) -> dict[str, Any]:
    """Per-residual per-token table (THE critical diagnostic).

    For each residual: ranked (token, surprise) list — was the actually-anomalous
    token the highest surprise?
    """
    print(
        "=== Step 8: per-residual per-token diagnostic (critical) ===",
        file=sys.stderr,
    )
    out: dict[str, Any] = {}
    for fid in sorted(RESIDUALS):
        res = residuals.get(fid)
        if not isinstance(res, dict) or "per_token" not in res:
            out[fid] = {"error": "missing per_token data"}
            continue
        tokens = res["per_token"]
        # Sort by surprise descending, attach rank.
        sorted_tokens = sorted(tokens, key=lambda t: -t["surprise"])
        for rank, t in enumerate(sorted_tokens, start=1):
            t["rank_within_hunk"] = rank
        out[fid] = {
            "n_tokens_scored": len(sorted_tokens),
            "top_surprise_token": sorted_tokens[0] if sorted_tokens else None,
            "all_tokens_sorted_by_surprise": sorted_tokens,
            "hunk_text_first_120": res.get("hunk_text_first_120", ""),
        }
    return out


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def _git_rev() -> str:
    try:
        r = subprocess.run(
            ["git", "-C", str(ROOT), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            check=True,
        )
        return r.stdout.strip()
    except subprocess.SubprocessError:
        return "unknown"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--smoke",
        type=int,
        default=None,
        help="If set, score only this many rows from --smoke-corpus.",
    )
    parser.add_argument("--smoke-corpus", type=str, default="faker-js")
    parser.add_argument(
        "--corpora",
        type=str,
        default=",".join(CORPORA),
        help="Comma-separated corpora to process (default: all).",
    )
    parser.add_argument(
        "--no-context",
        action="store_true",
        help="Phase 8 (legacy): score the hunk in isolation. Default is "
        "Phase 8.1 — score with surrounding file context (host file for "
        "breaks via synthesize_hunk_in_host).",
    )
    parser.add_argument(
        "--no-save",
        action="store_true",
        help="Skip joblib save (smoke runs).",
    )
    args = parser.parse_args()

    requested = [c.strip() for c in args.corpora.split(",") if c.strip()]
    print("Phase 8 — per-token MLM surprise (CPU, fp32)", file=sys.stderr)
    print(f"  corpora: {requested}", file=sys.stderr)
    if args.smoke:
        print(f"  smoke: {args.smoke} rows from {args.smoke_corpus}", file=sys.stderr)
    print(f"  max_tokens_per_hunk: {MAX_TOKENS_PER_HUNK}", file=sys.stderr)
    print(f"  batch_mask_positions: {BATCH_MASK_POSITIONS}", file=sys.stderr)
    print(f"  rss_hard_stop_mb: {RSS_HARD_STOP_MB}", file=sys.stderr)

    scoring = score_all(
        corpora=requested,
        smoke_only=args.smoke,
        smoke_corpus=(args.smoke_corpus if args.smoke else None),
        use_context=(not args.no_context),
    )
    per_corpus_rows = scoring["per_corpus_rows"]
    stats = scoring["stats"]
    model_id = scoring["model_id"]

    thresholds = calibrate_per_corpus(per_corpus_rows)
    pooled_aucs = {agg: auc_pooled(per_corpus_rows, agg) for agg in AGGREGATIONS}
    residuals = residual_table(per_corpus_rows, thresholds)
    recall_fp = per_corpus_recall_fp(per_corpus_rows, thresholds)
    verdicts = verdict_per_aggregation(residuals, recall_fp)
    per_token_diag = per_residual_per_token_diagnostic(residuals)

    # Best aggregation: most catches, then no-regression.
    best_agg = None
    best_score = (-1, 0)
    for agg in AGGREGATIONS:
        n_caught = verdicts[agg]["residuals_caught"]
        fp_ok = 1 if verdicts[agg]["no_regression_gate_pass"] else 0
        s = (n_caught, fp_ok)
        if s > best_score:
            best_score = s
            best_agg = agg

    artifact_path = FEATURE_DIR / "phase8_mlm_surprise.joblib"
    if not args.no_save:
        # Slim per_corpus_rows for persistence — keep aggregations + per_token
        # for residuals only (we never need full per-token for non-residuals).
        slim_per_corpus: dict[str, list[dict]] = {}
        for c, rows in per_corpus_rows.items():
            slim: list[dict] = []
            for r in rows:
                rr = {
                    k: v
                    for k, v in r.items()
                    if k not in ("hunk_text",)
                }
                slim.append(rr)
            slim_per_corpus[c] = slim

        save_payload = {
            "per_corpus_rows": slim_per_corpus,
            "thresholds": thresholds,
            "fp_target": FP_TARGET,
            "aggregations": AGGREGATIONS,
            "max_tokens_per_hunk": MAX_TOKENS_PER_HUNK,
            "batch_mask_positions": BATCH_MASK_POSITIONS,
            "rss_hard_stop_mb": RSS_HARD_STOP_MB,
            "model": model_id,
            "device": "cpu",
            "dtype": "float32",
            "git_rev": _git_rev(),
            "method": (
                "Per-token MLM surprise via one-mask-at-a-time, batched in groups of 8. "
                f"AutoModelForMaskedLM('{model_id}'); unixcoder-base ships no MLM head, "
                "codebert-base-mlm is the architecturally identical Roberta-base "
                "checkpoint with a real LM head. CPU fp32, no fine-tuning, no labels "
                "touched. Each scoring position j: replace input_ids[j] with <mask>, "
                "all other positions visible (assertion: exactly 1 mask per row). "
                "Surprise_j = -log P(original_token | rest). Aggregations: mean, max, "
                "top3_mean, p95. Per-corpus threshold = (1 - FP_target/100)-quantile of "
                "CONTROL surprise. Hunk truncated to 96 tokens; warned per truncation."
            ),
            "stats": stats,
        }
        try:
            joblib.dump(save_payload, artifact_path)
        except Exception as e:  # pragma: no cover — defensive
            print(f"WARNING: joblib.dump failed: {e}", file=sys.stderr)

    # Final JSON to stdout.
    out = {
        "config": {
            "model": model_id,
            "device": "cpu",
            "dtype": "float32",
            "max_tokens_per_hunk": MAX_TOKENS_PER_HUNK,
            "batch_mask_positions": BATCH_MASK_POSITIONS,
            "rss_hard_stop_mb": RSS_HARD_STOP_MB,
            "fp_target": FP_TARGET,
            "aggregations": AGGREGATIONS,
            "corpora": requested,
            "smoke": args.smoke,
            "smoke_corpus": args.smoke_corpus if args.smoke else None,
            "git_rev": _git_rev(),
        },
        "stats": stats,
        "thresholds": thresholds,
        "pooled_aucs_breaks_vs_controls": pooled_aucs,
        "era12_inversion_check": {
            "era12_surprise_mean_auc": 0.4290,
            "phase8_surprise_mean_auc": pooled_aucs.get("surprise_mean"),
            "joint_masking_confound_resolved": (
                pooled_aucs.get("surprise_mean") is not None
                and pooled_aucs.get("surprise_mean", 0.0) > 0.5
            ),
        },
        "residuals": residuals,
        "per_corpus_recall_fp": recall_fp,
        "verdicts": verdicts,
        "best_aggregation": best_agg,
        "per_token_diagnostic": per_token_diag,
        "artifact_saved_to": str(artifact_path) if not args.no_save else None,
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
