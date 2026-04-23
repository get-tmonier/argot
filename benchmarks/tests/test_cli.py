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


def test_cli_seeds_flag():
    parser = build_parser()
    ns = parser.parse_args(["--seeds", "1"])
    assert ns.seeds == 1

    ns_default = parser.parse_args([])
    assert ns_default.seeds is None


def test_cli_sample_controls_flag():
    parser = build_parser()
    ns = parser.parse_args(["--sample-controls", "500"])
    assert ns.sample_controls == 500

    ns_default = parser.parse_args([])
    assert ns_default.sample_controls is None
