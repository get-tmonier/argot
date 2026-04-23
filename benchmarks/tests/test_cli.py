from argot_bench.cli import build_parser, main


def test_parser_defaults():
    parser = build_parser()
    args = parser.parse_args([])
    assert args.corpus is None
    assert args.quick is False
    assert args.fresh is False


def test_parser_corpus_filter():
    parser = build_parser()
    args = parser.parse_args(["--corpus=fastapi,hono"])
    assert args.corpus == ["fastapi", "hono"]


def test_list_corpora_subcommand():
    exit_code = main(["list-corpora"])
    assert exit_code == 0


def test_cli_accepts_typicality_filter_flag():
    parser = build_parser()
    ns = parser.parse_args(["--typicality-filter", "on", "--quick"])
    assert ns.typicality_filter == "on"

    ns_off = parser.parse_args([])
    assert ns_off.typicality_filter == "off"
