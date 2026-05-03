"""Era-14 Phase 6.1 — tests for the optional UnixCoder embedder.

All tests in this module are skipped when ``torch`` is not installed (the
``embeddings`` extra is not enabled in baseline argot-engine installs, so CI
without the extra still passes).  Run with::

    uv pip install -e engine[embeddings]
    uv run pytest engine/argot/tests/test_ml_embeddings.py
"""

from __future__ import annotations

import pytest

# Skip the whole module if torch is unavailable.  Importing
# argot.ml.embeddings itself is cheap (no torch dependency), but constructing
# UnixCoderEmbedder() requires torch at runtime.
torch = pytest.importorskip("torch")  # noqa: F401 — pytest fixture-style skip

from argot.ml.embeddings import UnixCoderEmbedder  # noqa: E402 — after importorskip


@pytest.fixture(scope="module")
def embedder() -> UnixCoderEmbedder:
    """Module-scoped — loads the encoder once for all tests in the file."""
    return UnixCoderEmbedder()


def test_embedder_returns_768_dim_vector(embedder: UnixCoderEmbedder) -> None:
    """Smoke test: a tiny Python snippet returns a 768-float vector."""
    vec = embedder.embed("def foo(): pass")
    assert isinstance(vec, list)
    assert len(vec) == 768
    assert all(isinstance(x, float) for x in vec)


def test_embedder_deterministic(embedder: UnixCoderEmbedder) -> None:
    """Same input twice → exact same output (model in eval mode, no dropout)."""
    a = embedder.embed("def foo(): pass")
    b = embedder.embed("def foo(): pass")
    assert a == b


def test_embedder_truncates_long_input(embedder: UnixCoderEmbedder) -> None:
    """Inputs > 512 tokens still produce a 768-dim vector (no error)."""
    # Construct a string that comfortably exceeds 512 BPE tokens.
    long_src = "\n".join(f"x_{i} = {i} + {i + 1}" for i in range(2000))
    vec = embedder.embed(long_src)
    assert len(vec) == 768


def test_embedder_handles_empty_input(embedder: UnixCoderEmbedder) -> None:
    """Empty source still produces a valid 768-dim output (special tokens only)."""
    vec = embedder.embed("")
    assert len(vec) == 768


def test_embedder_context_window_returns_768_dim(embedder: UnixCoderEmbedder) -> None:
    """Context-window helper returns the same shape as plain embed()."""
    file_source = (
        "import os\n"
        "import sys\n"
        "\n"
        "def helper():\n"
        "    return 1\n"
        "\n"
        "def main():\n"
        "    return helper()\n"
    )
    vec = embedder.embed_context_window(file_source, hunk_start_line=4, hunk_end_line=5)
    assert len(vec) == 768


def test_embedder_context_window_short_file(embedder: UnixCoderEmbedder) -> None:
    """A file shorter than 512 tokens just embeds the whole thing."""
    file_source = "def f():\n    return 1\n"
    vec = embedder.embed_context_window(file_source, hunk_start_line=1, hunk_end_line=2)
    assert len(vec) == 768


def test_embedder_context_window_huge_hunk(embedder: UnixCoderEmbedder) -> None:
    """When the hunk alone exceeds 512 tokens, no surrounding context fits."""
    file_source = "\n".join(f"x_{i} = {i}" for i in range(1500))
    vec = embedder.embed_context_window(file_source, hunk_start_line=1, hunk_end_line=1500)
    assert len(vec) == 768


def test_embedder_hidden_size_constant(embedder: UnixCoderEmbedder) -> None:
    """Sanity: hidden_size property matches what every embed() call returns."""
    assert embedder.hidden_size == 768
