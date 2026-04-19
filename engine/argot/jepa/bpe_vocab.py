from __future__ import annotations

from typing import Any

from tokenizers import Tokenizer  # type: ignore[import-untyped]
from tokenizers.models import BPE  # type: ignore[import-untyped]
from tokenizers.pre_tokenizers import Whitespace  # type: ignore[import-untyped]
from tokenizers.trainers import BpeTrainer  # type: ignore[import-untyped]

PAD_ID = 0
UNK_ID = 1
BOS_ID = 2


class BpeVocab:
    """BPE subword vocabulary trained on repo token texts."""

    def __init__(self, tokenizer: Tokenizer) -> None:
        self._tokenizer = tokenizer

    @classmethod
    def build(
        cls,
        records: list[dict[str, Any]],
        vocab_size: int = 8000,
    ) -> BpeVocab:
        tokenizer: Tokenizer = Tokenizer(BPE(unk_token="<unk>"))
        tokenizer.pre_tokenizer = Whitespace()
        trainer: BpeTrainer = BpeTrainer(
            vocab_size=vocab_size,
            special_tokens=["<pad>", "<unk>", "<bos>"],
            min_frequency=2,
        )
        texts = []
        for r in records:
            texts.append(" ".join(t["text"] for t in r["context_before"]))
            texts.append(" ".join(t["text"] for t in r["hunk_tokens"]))
        tokenizer.train_from_iterator(texts, trainer=trainer)
        return cls(tokenizer)

    def encode(self, token_texts: list[str]) -> list[int]:
        """Encode token texts into flattened BPE subword IDs."""
        text = " ".join(token_texts)
        return self._tokenizer.encode(text).ids  # type: ignore[no-any-return]

    @property
    def vocab_size(self) -> int:
        return self._tokenizer.get_vocab_size()  # type: ignore[no-any-return]

    def state_dict(self) -> dict[str, Any]:
        return {"tokenizer_json": self._tokenizer.to_str()}

    @classmethod
    def from_state_dict(cls, state: dict[str, Any]) -> BpeVocab:
        return cls(Tokenizer.from_str(state["tokenizer_json"]))
