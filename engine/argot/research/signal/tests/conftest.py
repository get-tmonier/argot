from __future__ import annotations


def make_corpus(n: int = 5) -> list[dict[str, object]]:
    return [{"hunk_tokens": [{"text": f"token{i}"}], "timestamp": i} for i in range(n)]


def make_fixtures(n: int = 2) -> list[dict[str, object]]:
    return [{"hunk_tokens": [{"text": f"fix{i}"}]} for i in range(n)]
