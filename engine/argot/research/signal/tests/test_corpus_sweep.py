from __future__ import annotations

import random


def test_subsample_respects_seed_and_size() -> None:
    corpus = [{"id": i} for i in range(100)]
    s1 = random.Random(0).sample(corpus, 20)
    s2 = random.Random(0).sample(corpus, 20)
    s3 = random.Random(1).sample(corpus, 20)
    assert len(s1) == 20
    assert s1 == s2
    assert s1 != s3


def test_subsample_clamps_to_corpus_size() -> None:
    corpus = [{"id": i} for i in range(10)]
    s = random.Random(0).sample(corpus, min(500, len(corpus)))
    assert len(s) == 10
