"""Era-14 Phase 6.1 — UnixCoder semantic embeddings for hunks + file context.

Phase 5 (the engineered-feature ML stage) hit a structural wall: the residual
faker-js fixtures have ``n_unattested_callees == 0`` by definition, so any
classifier whose dominant input is callee-based engineered features cannot
catch them.  Phase 6.1 adds **frozen** UnixCoder semantic embeddings as a
parallel input channel — the encoder is shared with the existing tokenizer
(``microsoft/unixcoder-base``) so we only pay the model-loading cost once per
subprocess.

Public surface
--------------
* :class:`UnixCoderEmbedder` — wraps :class:`transformers.AutoModel` for
  ``microsoft/unixcoder-base`` in eval mode under ``torch.no_grad()``.

Design constraints
------------------
* **Optional dependency.** ``torch`` is *not* a baseline argot-engine dep;
  it is declared under the ``embeddings`` extra in ``engine/pyproject.toml``.
  Importing this module is cheap; calling :class:`UnixCoderEmbedder()` raises
  a helpful ``ImportError`` if torch is missing.
* **Frozen weights.** The encoder is in ``eval()`` mode and every forward
  pass is wrapped in ``torch.no_grad()`` — no gradient buffers, no fine-tuning.
* **Per-subprocess single load.** Construct one
  :class:`UnixCoderEmbedder` per corpus extraction (the existing
  subprocess-per-corpus pattern bounds RAM); ~500 MB resident encoder.
* **Single-sample forward passes.** No batching in this phase — one hunk at a
  time keeps the API small; batching is a future optimisation.

Context-window strategy
-----------------------
UnixCoder's max sequence length is 512 tokens.  We expose two separate
helpers:

* :meth:`UnixCoderEmbedder.embed` — embed an arbitrary source string
  (truncated to the first 512 tokens) and return the [CLS] vector as a
  ``list[float]`` of length 768.
* :meth:`UnixCoderEmbedder.embed_context_window` — embed a window of ~512
  tokens **centred on the hunk** within its file: roughly
  ``floor((512 - hunk_tokens) / 2)`` tokens of the file before the hunk +
  the hunk itself + the rest after, all packed to ``max_length=512``.  When
  the file is shorter than 512 tokens, the whole file is embedded.  When the
  hunk alone is longer than 512 tokens, the first 512 hunk tokens are used
  (no surrounding context fits).
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:  # pragma: no cover — only for type checking
    from collections.abc import Sequence

_MODEL_NAME = "microsoft/unixcoder-base"
_MAX_LEN = 512
# UnixCoder-base hidden size — used only to document the return shape.
_HIDDEN_SIZE = 768


def _require_torch() -> Any:
    """Import torch lazily and raise a friendly error if the extra is missing.

    Torch is declared as an *optional* dependency under the ``embeddings``
    extra in ``engine/pyproject.toml``; baseline ``argot-engine`` installs
    omit it.  The mypy override for ``module = "torch"`` keeps type-checking
    green whether or not the extra is installed (see pyproject.toml).
    """
    try:
        import torch
    except ImportError as e:  # pragma: no cover — exercised in CI without extra
        raise ImportError(
            "argot.ml.embeddings requires PyTorch.  Install the optional "
            "'embeddings' extra:  uv pip install -e engine[embeddings]  "
            "(or  pip install argot-engine[embeddings]  if installed from PyPI)."
        ) from e
    return torch


class UnixCoderEmbedder:
    """Frozen UnixCoder encoder producing 768-dim [CLS] embeddings.

    A single instance is intended to be constructed once per subprocess and
    reused for every hunk + file-context call.  Holds ~500 MB of encoder
    weights in CPU memory.

    Examples:
        >>> emb = UnixCoderEmbedder()                      # doctest: +SKIP
        >>> v = emb.embed("def foo(): return 1")           # doctest: +SKIP
        >>> assert len(v) == 768                           # doctest: +SKIP

    Notes:
        * The model is loaded with ``model.eval()`` and every forward call is
          wrapped in ``torch.no_grad()`` — no autograd buffers.
        * Inputs are tokenized with truncation to 512 tokens.  Longer inputs
          silently drop the tail.  Empty inputs still produce a valid vector
          (the tokenizer emits the special ``[CLS]`` / ``</s>`` markers).
    """

    def __init__(self) -> None:
        torch = _require_torch()
        from transformers import AutoModel, AutoTokenizer

        self._torch = torch
        self._tokenizer = AutoTokenizer.from_pretrained(_MODEL_NAME)
        model = AutoModel.from_pretrained(_MODEL_NAME)
        model.eval()
        self._model = model
        # CPU-only: explicit device pin to avoid surprise GPU scheduling on
        # machines that happen to have a CUDA build of torch.
        self._device = torch.device("cpu")
        self._model.to(self._device)

    @property
    def hidden_size(self) -> int:
        """Return the embedder's output dimensionality (768 for UnixCoder-base)."""
        return _HIDDEN_SIZE

    def embed(self, source: str) -> list[float]:
        """Return the 768-dim [CLS] embedding for *source*.

        The input is tokenized with ``truncation=True, max_length=512``; any
        tokens beyond the limit are silently dropped.  Empty *source* still
        runs through the encoder (the tokenizer emits special markers).

        Returns:
            list[float] of length 768.
        """
        torch = self._torch
        encoded = self._tokenizer(
            source,
            truncation=True,
            max_length=_MAX_LEN,
            return_tensors="pt",
        )
        input_ids = encoded["input_ids"].to(self._device)
        attention_mask = encoded["attention_mask"].to(self._device)
        with torch.no_grad():
            outputs = self._model(input_ids=input_ids, attention_mask=attention_mask)
        # last_hidden_state shape: (batch=1, seq_len, hidden=768).  The first
        # token is [CLS] (or its UnixCoder equivalent <s>).  We take row 0.
        cls = outputs.last_hidden_state[0, 0, :]
        return [float(x) for x in cls.detach().cpu().tolist()]

    def embed_context_window(
        self,
        file_source: str,
        *,
        hunk_start_line: int,
        hunk_end_line: int,
    ) -> list[float]:
        """Return the [CLS] embedding for a 512-token window centred on the hunk.

        Strategy:
          1. Tokenize the **file prefix** (lines before *hunk_start_line*),
             the **hunk** itself, and the **file suffix** (lines after
             *hunk_end_line*) separately, without special tokens.
          2. Reserve the hunk tokens (truncated to ``_MAX_LEN`` if needed).
          3. Split the remaining budget evenly between prefix tail and suffix
             head, taking the *last* N prefix tokens (closest to the hunk)
             and the *first* M suffix tokens.
          4. Concatenate ``[<s>] + prefix_tail + hunk + suffix_head + [</s>]``,
             clamp to ``_MAX_LEN`` total, and forward through the encoder.

        Edge cases:
          * Whole-file shorter than 512 tokens → the entire file is embedded.
          * Hunk alone >= 512 tokens → no surrounding context fits; the first
            512 hunk tokens are used.
          * *hunk_start_line* < 1 or *hunk_end_line* < *hunk_start_line* are
            tolerated (treated as "no prefix" / "empty hunk") — invalid lines
            do not raise here because the caller has already validated them
            during feature extraction.

        Args:
            file_source: Full source of the file (or synthesised host content
                for fixtures with ``host_file``).
            hunk_start_line: 1-indexed first line of the hunk.
            hunk_end_line: 1-indexed inclusive last line of the hunk.

        Returns:
            list[float] of length 768.
        """
        lines = file_source.splitlines(keepends=True)
        n_lines = len(lines)
        # Clamp to valid 1-indexed slice range.  splitlines drops trailing \n
        # but keepends=True preserves it; either way "".join restores source.
        start = max(1, hunk_start_line)
        end = min(n_lines, max(hunk_end_line, start - 1))
        prefix_text = "".join(lines[: start - 1])
        hunk_text = "".join(lines[start - 1 : end])
        suffix_text = "".join(lines[end:])

        # Encode each chunk without specials so we can pack a single window.
        prefix_ids = self._tokenizer.encode(prefix_text, add_special_tokens=False)
        hunk_ids = self._tokenizer.encode(hunk_text, add_special_tokens=False)
        suffix_ids = self._tokenizer.encode(suffix_text, add_special_tokens=False)

        # Reserve room for the two specials ([CLS], [SEP] / <s>, </s>).
        budget = _MAX_LEN - 2

        # Hunk gets first claim on the budget — we always want the hunk in
        # the window if it fits at all.
        hunk_ids = hunk_ids[:budget]
        remaining = budget - len(hunk_ids)
        half = remaining // 2
        # Prefix: take the LAST `half` tokens (closest to the hunk).
        prefix_take = prefix_ids[-half:] if half > 0 else []
        # Suffix: take the FIRST (remaining - len(prefix_take)) tokens so we
        # use the full budget even when the prefix was shorter than `half`.
        suffix_quota = remaining - len(prefix_take)
        suffix_take = suffix_ids[:suffix_quota] if suffix_quota > 0 else []
        # If the suffix was also short, redistribute leftover quota to the
        # prefix (use more of it).
        if len(suffix_take) < suffix_quota:
            extra = suffix_quota - len(suffix_take)
            extra_prefix = prefix_ids[-(half + extra) : -half] if half > 0 else prefix_ids[-extra:]
            prefix_take = list(extra_prefix) + list(prefix_take)

        ids = self._build_window(prefix_take, hunk_ids, suffix_take)
        return self._forward_ids(ids)

    # -- internals ---------------------------------------------------------

    def _build_window(
        self,
        prefix_ids: Sequence[int],
        hunk_ids: Sequence[int],
        suffix_ids: Sequence[int],
    ) -> list[int]:
        """Concatenate prefix+hunk+suffix and add ``[CLS]`` / ``[SEP]`` markers.

        UnixCoder uses the RoBERTa special tokens ``<s>`` (cls_token_id) and
        ``</s>`` (sep_token_id).  We wrap the body manually because the fast
        tokenizer in some transformers versions does not expose
        ``build_inputs_with_special_tokens`` as an instance attribute.  The
        body is pre-clamped to ``_MAX_LEN - 2`` so the two specials always
        fit, then the final result is re-clamped defensively.
        """
        body: list[int] = list(prefix_ids) + list(hunk_ids) + list(suffix_ids)
        # Clamp body to budget so the special tokens always fit.
        body = body[: _MAX_LEN - 2]
        cls_id = self._tokenizer.cls_token_id
        sep_id = self._tokenizer.sep_token_id
        wrapped: list[int] = [int(cls_id), *body, int(sep_id)]
        return wrapped[:_MAX_LEN]

    def _forward_ids(self, input_ids: list[int]) -> list[float]:
        """Forward a pre-built id list and return the [CLS] vector."""
        torch = self._torch
        ids_tensor = torch.tensor([input_ids], dtype=torch.long, device=self._device)
        attn = torch.ones_like(ids_tensor)
        with torch.no_grad():
            outputs = self._model(input_ids=ids_tensor, attention_mask=attn)
        cls = outputs.last_hidden_state[0, 0, :]
        return [float(x) for x in cls.detach().cpu().tolist()]
