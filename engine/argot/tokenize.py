from __future__ import annotations

from functools import cache
from pathlib import Path
from typing import TYPE_CHECKING

import tree_sitter_javascript
import tree_sitter_python
import tree_sitter_typescript
from tree_sitter import Language, Node, Parser

from argot.dataset import Language as Lang
from argot.dataset import Token

if TYPE_CHECKING:
    pass

EXTENSION_TO_LANGUAGE: dict[str, Lang] = {
    ".ts": "typescript",
    ".tsx": "typescript",
    ".js": "javascript",
    ".jsx": "javascript",
    ".py": "python",
}


@cache
def _get_parser(lang: Lang) -> Parser:
    if lang == "typescript":
        ts_lang = Language(tree_sitter_typescript.language_typescript())
    elif lang == "javascript":
        ts_lang = Language(tree_sitter_javascript.language())
    else:
        ts_lang = Language(tree_sitter_python.language())
    parser = Parser(ts_lang)
    return parser


def language_for_path(file_path: str) -> Lang | None:
    ext = Path(file_path).suffix.lower()
    return EXTENSION_TO_LANGUAGE.get(ext)


def _collect_tokens(node: Node, tokens: list[Token]) -> None:
    if node.child_count == 0 and node.text:
        tokens.append(
            Token(
                text=node.text.decode("utf-8", errors="replace"),
                node_type=node.type,
                start_line=node.start_point[0],
                end_line=node.end_point[0],
            )
        )
    for child in node.children:
        _collect_tokens(child, tokens)


def tokenize(source: bytes, lang: Lang) -> list[Token]:
    """Parse source bytes and return a flat list of leaf tokens."""
    parser = _get_parser(lang)
    tree = parser.parse(source)
    tokens: list[Token] = []
    _collect_tokens(tree.root_node, tokens)
    return tokens


def tokenize_lines(
    source_lines: list[str], lang: Lang, start_line: int, end_line: int
) -> list[Token]:
    """Tokenize a slice of source lines (0-indexed, end_line exclusive)."""
    slice_source = "\n".join(source_lines[start_line:end_line]).encode()
    raw_tokens = tokenize(slice_source, lang)
    # Adjust line numbers to be absolute
    return [
        Token(
            text=t.text,
            node_type=t.node_type,
            start_line=t.start_line + start_line,
            end_line=t.end_line + start_line,
        )
        for t in raw_tokens
    ]
