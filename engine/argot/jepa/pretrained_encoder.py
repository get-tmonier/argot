from __future__ import annotations

import torch
import torch.nn.functional as F  # noqa: N812
from sentence_transformers import SentenceTransformer
from torch import nn
from transformers import AutoModel, AutoTokenizer

DEFAULT_MODEL = "nomic-ai/CodeRankEmbed"

# Models that require HuggingFace direct inference (mean-pool last hidden state)
# rather than the SentenceTransformer interface.
_HF_DIRECT_MODELS: frozenset[str] = frozenset({"microsoft/unixcoder-base"})


def select_device() -> torch.device:
    """Pick the best available torch device (CUDA > MPS > CPU)."""
    if torch.cuda.is_available():
        return torch.device("cuda")
    if torch.backends.mps.is_available():
        return torch.device("mps")
    return torch.device("cpu")


class PretrainedEncoder(nn.Module):
    """Frozen pretrained code encoder (CodeRankEmbed by default).

    Supports two backends:
    - SentenceTransformer (default): wraps any ST-compatible model.
    - HuggingFace direct: mean-pools last hidden state; needed for models like
      microsoft/unixcoder-base that are not native SentenceTransformer models.

    `encode_texts(texts, normalize_embeddings=False)` is the main entry point.
    `forward(x)` is a passthrough for compatibility with the JEPAArgot encoder slot
    when the training pipeline pre-encodes embeddings upfront.
    """

    def __init__(
        self,
        model_name: str = DEFAULT_MODEL,
        device: torch.device | str | None = None,
        encode_batch_size: int = 64,
        max_seq_length: int = 512,
    ) -> None:
        super().__init__()
        self.model_name = model_name
        self.torch_device = torch.device(device) if device is not None else select_device()
        self.encode_batch_size = encode_batch_size
        self._use_hf_direct = model_name in _HF_DIRECT_MODELS

        if self._use_hf_direct:
            self._hf_tokenizer = AutoTokenizer.from_pretrained(model_name)  # type: ignore[no-untyped-call]
            self._hf_model = AutoModel.from_pretrained(model_name)
            self._hf_model.to(self.torch_device)
            self._hf_model.eval()
            for p in self._hf_model.parameters():
                p.requires_grad = False
            self._st_model: SentenceTransformer | None = None
            self.embed_dim: int = int(self._hf_model.config.hidden_size)
        else:
            self._hf_tokenizer = None
            self._hf_model = None
            self._st_model = SentenceTransformer(
                model_name,
                device=str(self.torch_device),
                trust_remote_code=True,
            )
            # CodeRankEmbed defaults to 8192 tokens — vastly more than our hunks/contexts
            # need. Cap to bound memory (batch × seq_len × hidden) at a reasonable size.
            self._st_model.max_seq_length = max_seq_length
            for p in self._st_model.parameters():
                p.requires_grad = False
            self._st_model.eval()
            dim = self._st_model.get_sentence_embedding_dimension()
            if dim is None:
                raise RuntimeError(f"could not determine embedding dim for {model_name!r}")
            self.embed_dim = int(dim)

    def encode_texts(self, texts: list[str], *, normalize_embeddings: bool = False) -> torch.Tensor:
        """Encode a list of code texts into a `(len(texts), embed_dim)` tensor.

        Tensor lives on `self.torch_device`. Empty strings are handled safely.
        Pass `normalize_embeddings=True` for cosine-trained models (e.g. unixcoder-base).
        """
        if not texts:
            return torch.zeros(0, self.embed_dim, device=self.torch_device)
        safe_texts = [t if t.strip() else " " for t in texts]
        if self._use_hf_direct:
            return self._encode_hf_direct(safe_texts, normalize_embeddings=normalize_embeddings)
        return self._encode_st(safe_texts, normalize_embeddings=normalize_embeddings)

    def _encode_st(self, texts: list[str], *, normalize_embeddings: bool) -> torch.Tensor:
        assert self._st_model is not None
        with torch.no_grad():
            emb = self._st_model.encode(
                texts,
                batch_size=self.encode_batch_size,
                convert_to_tensor=True,
                show_progress_bar=False,
                normalize_embeddings=normalize_embeddings,
            )
        assert isinstance(emb, torch.Tensor)
        out = emb.to(self.torch_device)
        # Release MPS cache between large encode batches; harmless on CUDA/CPU.
        if self.torch_device.type == "mps":
            torch.mps.empty_cache()
        return out

    def _encode_hf_direct(self, texts: list[str], *, normalize_embeddings: bool) -> torch.Tensor:
        """Mean-pool last hidden state from a raw HuggingFace model."""
        assert self._hf_model is not None
        assert self._hf_tokenizer is not None
        all_embeddings: list[torch.Tensor] = []
        with torch.no_grad():
            for i in range(0, len(texts), self.encode_batch_size):
                batch = texts[i : i + self.encode_batch_size]
                encoded = self._hf_tokenizer(
                    batch,
                    padding=True,
                    truncation=True,
                    max_length=512,
                    return_tensors="pt",
                )
                encoded = {k: v.to(self.torch_device) for k, v in encoded.items()}
                output = self._hf_model(**encoded)
                last_hidden = output.last_hidden_state
                mask = encoded["attention_mask"].unsqueeze(-1).float()
                pooled = (last_hidden * mask).sum(dim=1) / mask.sum(dim=1).clamp(min=1e-9)
                if normalize_embeddings:
                    pooled = F.normalize(pooled, dim=-1)
                all_embeddings.append(pooled)
        out = torch.cat(all_embeddings, dim=0)
        if self.torch_device.type == "mps":
            torch.mps.empty_cache()
        return out

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Passthrough: the JEPAArgot wrapper expects an encoder callable,
        but we pre-encode texts before training and pass the embeddings directly.
        """
        return x
