"""Unit tests for ast_features.extract_features."""

from __future__ import annotations

from argot.research.signal.ast_features import extract_features


def test_raise_http_exception() -> None:
    src = "raise HTTPException(status_code=404)\n"
    feats = extract_features(src)
    # Raise node has exc field → Call node, whose func is Name "HTTPException"
    assert any("HTTPException" in v for v in feats.values())


def test_raise_value_error() -> None:
    src = "raise ValueError('bad')\n"
    feats = extract_features(src)
    assert any("ValueError" in v for v in feats.values())


def test_decorator_app_get() -> None:
    src = "@app.get('/items')\ndef f(): pass\n"
    feats = extract_features(src)
    assert any("app.get" in v for v in feats.values())


def test_base_class() -> None:
    src = "class Item(BaseModel): pass\n"
    feats = extract_features(src)
    assert any("BaseModel" in v for v in feats.values())


def test_call_attr() -> None:
    src = "asyncio.get_event_loop()\n"
    feats = extract_features(src)
    assert any("asyncio.get_event_loop" in v for v in feats.values())


def test_import_from() -> None:
    src = "from fastapi import HTTPException\n"
    feats = extract_features(src)
    # ImportFrom has names (alias), not directly an expr — won't appear as dotted name.
    # But the module string shows up as a Name node somewhere via ast.walk.
    # The key invariant: parsing doesn't raise and returns a dict.
    assert isinstance(feats, dict)


def test_syntax_error_returns_empty_dict() -> None:
    assert extract_features("def (") == {}


def test_empty_source_returns_empty_or_minimal() -> None:
    feats = extract_features("")
    assert isinstance(feats, dict)


def test_categories_are_ast_node_names() -> None:
    src = "@app.get('/x')\ndef f():\n    raise HTTPException(status_code=400)\n"
    feats = extract_features(src)
    import ast
    valid_node_names = {
        cls.__name__ for cls in ast.__dict__.values()
        if isinstance(cls, type) and issubclass(cls, ast.AST)
    }
    for cat in feats:
        assert cat in valid_node_names, f"Category {cat!r} is not an AST node class name"


def test_ambiguous_base_subscript_skipped() -> None:
    # x[0] as base — _dotted_name returns None, should not appear
    src = "class C(x[0]): pass\n"
    feats = extract_features(src)
    # No crash, and subscript value not emitted as a dotted name
    for values in feats.values():
        assert all("[" not in v for v in values)


def test_multiple_features_accumulated() -> None:
    src = (
        "from fastapi import HTTPException\n"
        "@app.get('/items')\n"
        "def list_items():\n"
        "    raise HTTPException(status_code=404)\n"
    )
    feats = extract_features(src)
    all_values = [v for values in feats.values() for v in values]
    assert len(all_values) >= 2


# ---------------------------------------------------------------------------
# parent_context tests
# ---------------------------------------------------------------------------


def test_parent_context_async_raise() -> None:
    """raise inside async def emits AsyncFunctionDef::Raise as a category key."""
    source = """
async def endpoint():
    raise HTTPException(status_code=404)
"""
    feats = extract_features(source, parent_context=True)
    assert "AsyncFunctionDef::Raise" in feats


def test_parent_context_async_vs_sync_keys_differ() -> None:
    """time.sleep() inside async def vs def produces different parent keys."""
    async_source = "async def f():\n    time.sleep(1)\n"
    sync_source = "def f():\n    time.sleep(1)\n"
    async_feats = extract_features(async_source, parent_context=True)
    sync_feats = extract_features(sync_source, parent_context=True)
    assert set(async_feats.keys()) != set(sync_feats.keys())


def test_parent_context_false_regression() -> None:
    """flag=False produces identical output to the default (no parent_context)."""
    source = "def f():\n    x = foo.bar\n"
    assert extract_features(source) == extract_features(source, parent_context=False)


# ---------------------------------------------------------------------------
# cooccurrence tests
# ---------------------------------------------------------------------------


def test_cooccurrence_pair_emitted() -> None:
    """Test D: co-occurrence pair emitted for two features in the same function."""
    source = """
def endpoint(x: SomeModel):
    result = db.query(Table)
    return result
"""
    feats = extract_features(source, cooccurrence=True)
    assert "cooccur" in feats
    assert len(feats["cooccur"]) > 0


def test_cooccurrence_scope_boundary() -> None:
    """Test E: features in different functions do NOT pair across scope boundaries."""
    source = """
def f1():
    x = foo.bar

def f2():
    y = baz.qux
"""
    feats = extract_features(source, cooccurrence=True)
    cooccur_vals = feats.get("cooccur", [])
    # No pair should contain features from both f1 (foo.bar) and f2 (baz.qux)
    for pair in cooccur_vals:
        parts = pair.split("|")
        assert not ("foo.bar" in " ".join(parts) and "baz.qux" in " ".join(parts))
