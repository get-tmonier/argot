from __future__ import annotations

import torch
from sentence_transformers import SentenceTransformer
from torch import nn

DEFAULT_MODEL = "nomic-ai/CodeRankEmbed"


def select_device() -> torch.device:
    """Pick the best available torch device (CUDA > MPS > CPU)."""
    if torch.cuda.is_available():
        return torch.device("cuda")
    if torch.backends.mps.is_available():
        return torch.device("mps")
    return torch.device("cpu")


class PretrainedEncoder(nn.Module):
    """Frozen pretrained code encoder (CodeRankEmbed by default).

    Encodes text into fixed embeddings via sentence-transformers. Weights are
    frozen (`requires_grad=False`). No fine-tuning; no gradient flow into the
    encoder during JEPA predictor training.

    `encode_texts(texts)` is the main entry point. `forward(x)` is a
    passthrough for compatibility with the JEPAArgot encoder slot when the
    training pipeline pre-encodes embeddings upfront.
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

        self._model = SentenceTransformer(
            model_name,
            device=str(self.torch_device),
            trust_remote_code=True,
        )
        # CodeRankEmbed defaults to 8192 tokens — vastly more than our hunks/contexts
        # need. Cap to bound memory (batch × seq_len × hidden) at a reasonable size.
        self._model.max_seq_length = max_seq_length

        for p in self._model.parameters():
            p.requires_grad = False
        self._model.eval()

        dim = self._model.get_sentence_embedding_dimension()
        if dim is None:
            raise RuntimeError(f"could not determine embedding dim for {model_name!r}")
        self.embed_dim: int = int(dim)

    def encode_texts(self, texts: list[str]) -> torch.Tensor:
        """Encode a list of code texts into a `(len(texts), embed_dim)` tensor.

        Tensor lives on `self.torch_device`. Empty strings are handled safely by
        sentence-transformers (they yield a zero-ish embedding rather than crashing).
        """
        if not texts:
            return torch.zeros(0, self.embed_dim, device=self.torch_device)
        safe_texts = [t if t.strip() else " " for t in texts]
        with torch.no_grad():
            emb = self._model.encode(
                safe_texts,
                batch_size=self.encode_batch_size,
                convert_to_tensor=True,
                show_progress_bar=False,
                normalize_embeddings=False,
            )
        assert isinstance(emb, torch.Tensor)
        out = emb.to(self.torch_device)
        # Release MPS cache between large encode batches; harmless on CUDA/CPU.
        if self.torch_device.type == "mps":
            torch.mps.empty_cache()
        return out

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        """Passthrough: the JEPAArgot wrapper expects an encoder callable,
        but we pre-encode texts before training and pass the embeddings directly.
        """
        return x
