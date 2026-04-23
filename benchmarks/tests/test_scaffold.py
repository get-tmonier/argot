def test_argot_bench_imports():
    import argot_bench
    assert hasattr(argot_bench, "__version__")

def test_cli_main_callable():
    from argot_bench.cli import main
    assert callable(main)
