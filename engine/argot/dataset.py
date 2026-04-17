# Wire format for the dataset JSONL. Every hunk emitted conforms to this schema.
from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

Language = Literal["typescript", "javascript", "python"]


@dataclass(frozen=True, slots=True)
class Token:
    text: str
    node_type: str  # tree-sitter node kind, e.g. "function_declaration"
    start_line: int
    end_line: int


@dataclass(frozen=True, slots=True)
class HunkRecord:
    commit_sha: str
    file_path: str
    language: Language
    hunk_start_line: int
    hunk_end_line: int
    context_before: list[Token]  # up to 50 lines before, tokenized
    hunk_tokens: list[Token]
    context_after: list[Token]  # up to 50 lines after
    parent_sha: str | None
    author_date_iso: str
