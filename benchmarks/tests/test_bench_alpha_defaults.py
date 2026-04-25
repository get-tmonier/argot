from __future__ import annotations

import dataclasses
import inspect


def test_bench_alpha_defaults_match_production() -> None:
    from argot.scoring.scorers.sequential_import_bpe import SequentialImportBpeScorer
    from argot_bench.run import RunConfig
    from argot_bench.score import build_scorer

    fields = {f.name: f for f in dataclasses.fields(RunConfig)}
    run_default = fields["call_receiver_alpha"].default

    sig = inspect.signature(build_scorer)
    score_default = sig.parameters["call_receiver_alpha"].default

    scorer_sig = inspect.signature(SequentialImportBpeScorer.__init__)
    scorer_default = scorer_sig.parameters["call_receiver_alpha"].default

    assert run_default == score_default == scorer_default == 2.0, (
        f"Bench defaults drifted from production: run={run_default}, "
        f"score={score_default}, scorer={scorer_default}"
    )
