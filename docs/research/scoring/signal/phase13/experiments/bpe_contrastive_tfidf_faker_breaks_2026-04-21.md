# Phase 13 — BPE Contrastive TF-IDF: Faker Break Scoring (2026-04-21)

**Goal:** Score the 5 paradigm-break fixtures against the faker corpus and determine separation from the 159-hunk normal distribution.

## 1. Break Scores

| fixture | category | max_score | mean_score | top-3 tokens |
|---|---|---|---|---|
| break_mimesis_alt_1 | mimesis_alt | 4.2043 | -0.1646 | Ġhere, username, username |
| break_threading_provider_1 | threading_provider | 6.9589 | 2.0334 | thread, Ġthreading, Ġthreading |
| break_sqlalchemy_sink_1 | sqlalchemy_sink | 6.9568 | 1.5625 | metadata, Record, Record |
| break_numpy_random_1 | numpy_random | 4.5933 | 0.6909 | FIRST, FIRST, FIRST |
| break_requests_source_1 | requests_source | 7.3802 | 1.8033 | Ġresponse, Ġresponse, Ġresponse |

## 2. Overlap with Normal Distribution

Percentile rank = % of 159 normal scores strictly below this break's max_score.

| break | max_score | higher than X% of normal | percentile rank |
|---|---|---|---|
| break_mimesis_alt_1 | 4.2043 | 80.5% | p80.5 |
| break_threading_provider_1 | 6.9589 | 99.4% | p99.4 |
| break_sqlalchemy_sink_1 | 6.9568 | 99.4% | p99.4 |
| break_numpy_random_1 | 4.5933 | 90.6% | p90.6 |
| break_requests_source_1 | 7.3802 | 100.0% | p100.0 |

## 3. Separation Metrics

| metric | value |
|---|---|
| max_normal (max of 159 normal scores) | 7.3732 |
| p99_normal | 5.8221 |
| p95_normal | 5.0307 |
| p90_normal | 4.5538 |
| min(break_scores) | 4.2043 |
| max(break_scores) | 7.3802 |
| **margin_vs_max** (min_break − max_normal) | **-3.1689** |
| **margin_vs_p99** (min_break − p99_normal) | **-1.6178** |

A positive `margin_vs_max` means clean separation: every break scores above every normal hunk.

## 4. ASCII Histogram of Normal Scores + Break Annotations

Normal distribution (159 hunks, bucket width 0.5):

```
[-1.0--0.5]: #####                                    (4)
[-0.5-0.0]:                                          (0)
[0.0-0.5]: ####                                     (3)
[0.5-1.0]: ###########                              (9)
[1.0-1.5]: ################                         (13)
[1.5-2.0]: #####################                    (17)
[2.0-2.5]: ############                             (10)
[2.5-3.0]: ######################################## (32)
[3.0-3.5]: #########################                (20)
[3.5-4.0]: ######################                   (18)
[4.0-4.5]: ##################                       (14)
[4.5-5.0]: ############                             (10)
[5.0-5.5]: ######                                   (5)
[5.5-6.0]: ##                                       (2)
[6.0-6.5]: #                                        (1)
[6.5-7.0]:                                          (0)
[7.0-7.5]: #                                        (1)
```

Break positions:

- `break_mimesis_alt_1`: 4.2043  <- falls in bucket 4.0-4.5
- `break_threading_provider_1`: 6.9589  <- falls in bucket 6.5-7.0
- `break_sqlalchemy_sink_1`: 6.9568  <- falls in bucket 6.5-7.0
- `break_numpy_random_1`: 4.5933  <- falls in bucket 4.5-5.0
- `break_requests_source_1`: 7.3802  <- falls in bucket 7.0-7.5

## 5. Verdict

**FULL OVERLAP / INVERSION**

min(break_scores)=4.2043 < p95_normal=5.0307. Some breaks score no better than the bottom 5% of normal hunks.

### Thresholds applied

| criterion | condition | value | result |
|---|---|---|---|
| Clean separation | min(break_scores) > max_normal | 4.2043 > 7.3732 | NO |
| Partial overlap | min(break_scores) in [p95_normal, max_normal] | 4.2043 in [5.0307, 7.3732] | NO |
| Full overlap | min(break_scores) < p95_normal | 4.2043 < 5.0307 | YES |

## 6. Phase 13 Final Recommendation

**Verdict: FULL OVERLAP / INVERSION**

BPE-tfidf not production-ready for a general style linter. Investigate semantic approaches (tree-sitter AST structural, or CodeBERT zero-shot context-conditional perplexity).

### Supporting numbers

- Normal distribution: n=159, max=7.3732, p99=5.8221, p95=5.0307
- Break score range: [4.2043, 7.3802]
- margin_vs_max = -3.1689 (negative — overlap)
- margin_vs_p99 = -1.6178

