from __future__ import annotations

from collections import Counter
from dataclasses import dataclass, field

PAD_ID = 0
UNK_ID = 1
BOS_ID = 2
_SPECIAL = 3  # first non-special ID


@dataclass
class Vocab:
    token_to_id: dict[str, int] = field(default_factory=dict)
    id_to_token: list[str] = field(default_factory=list)

    @classmethod
    def build(
        cls,
        records: list[dict],  # type: ignore[type-arg]
        max_size: int = 8000,
        min_count: int = 2,
    ) -> Vocab:
        counts: Counter[str] = Counter()
        for r in records:
            for tok in r["context_before"]:
                counts[tok["text"]] += 1
            for tok in r["hunk_tokens"]:
                counts[tok["text"]] += 1
        vocab = cls()
        vocab.id_to_token = ["<pad>", "<unk>", "<bos>"]
        vocab.token_to_id = {"<pad>": PAD_ID, "<unk>": UNK_ID, "<bos>": BOS_ID}
        for token, count in counts.most_common(max_size - _SPECIAL):
            if count < min_count:
                break
            idx = len(vocab.id_to_token)
            vocab.id_to_token.append(token)
            vocab.token_to_id[token] = idx
        return vocab

    def encode(self, token_texts: list[str]) -> list[int]:
        return [self.token_to_id.get(t, UNK_ID) for t in token_texts]

    def state_dict(self) -> dict[str, object]:
        return {"token_to_id": self.token_to_id, "id_to_token": self.id_to_token}

    @classmethod
    def from_state_dict(cls, state: dict[str, object]) -> Vocab:
        vocab = cls()
        vocab.token_to_id = state["token_to_id"]  # type: ignore[assignment]
        vocab.id_to_token = state["id_to_token"]  # type: ignore[assignment]
        return vocab
