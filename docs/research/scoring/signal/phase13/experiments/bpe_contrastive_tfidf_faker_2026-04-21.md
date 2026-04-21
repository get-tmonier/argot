# Phase 13 — BPE Contrastive TF-IDF: Faker Calibration Run (2026-04-21)

**Goal:** Determine whether the scorer produces false positives on ordinary faker code.

## 1. Summary Stats (max_score, n=159)

| stat | value |
|---|---|
| min | -0.7986 |
| p10 | 1.0493 |
| p25 | 1.6502 |
| p50 | 2.6347 |
| p75 | 3.7575 |
| p90 | 4.5538 |
| p99 | 5.8221 |
| max | 7.3732 |
| mean | 2.8235 |
| stdev | 1.4497 |
| % above 5 | 5.7% |
| % above 6 | 1.3% |

## 2. Histogram (bucket width 1.0)

| score_band | count | pct |
|---|---|---|
| [-1, 0) | 4 | 2.5% |
| [0, 1) | 12 | 7.5% |
| [1, 2) | 30 | 18.9% |
| [2, 3) | 42 | 26.4% |
| [3, 4) | 38 | 23.9% |
| [4, 5) | 24 | 15.1% |
| [5, 6) | 7 | 4.4% |
| [6, 7) | 1 | 0.6% |
| [7, 8) | 1 | 0.6% |

## 3. Reference Comparison

| corpus | min | p50 | max | notes |
|---|---|---|---|---|
| FastAPI breaks | 5.9 | 8.7 | 11.1 | style violations |
| FastAPI controls | 0.8 | 1.9 | 4.5 | ordinary FastAPI code |
| rich breaks | 4.98 | 6.7 | 7.79 | style violations |
| rich controls | 2.93 | 4.16 | 5.60 | ordinary rich code |
| **faker (this run)** | **-0.80** | **2.63** | **7.37** | **ordinary faker code (n=159)** |

## 4. Top-10 Highest-Scoring Hunks

| name | file_path | max_score | mean_score | top_3_tokens |
|---|---|---|---|---|
| faker_hunk_0047 | faker/providers/date_time/__init__.py | 7.3732 | 1.6163 | exception, Ġ'%, ĠOSError |
| faker_hunk_0149 | faker/providers/__init__.py | 6.0009 | 1.3584 | Ġerror, Ġerror, Ġconstraints |
| faker_hunk_0026 | faker/providers/automotive/ko_KR/__init__.py | 5.6926 | -0.5392 | Ġ''., join, Ġself |
| faker_hunk_0002 | faker/providers/ssn/ar_DZ/__init__.py | 5.6140 | 0.0747 | Ġkey, Ġkey, Ġover |
| faker_hunk_0097 | faker/providers/automotive/__init__.py | 5.4778 | 1.1739 | Ġinput, Ġwhen, def |
| faker_hunk_0143 | faker/providers/python/__init__.py | 5.4778 | 2.0694 | Ġinput, Ġissubclass, Ġ'{ |
| faker_hunk_0082 | faker/providers/file/__init__.py | 5.2991 | 1.8695 | Ġdirectory, Ġdirectory, Ġpath |
| faker_hunk_0081 | faker/providers/internet/__init__.py | 5.0760 | 1.1588 | ĠHTTP, Ġstatus, Ġstatus |
| faker_hunk_0095 | faker/providers/internet/__init__.py | 5.0256 | 1.4602 | path, path, Ġurl |
| faker_hunk_0046 | faker/providers/date_time/__init__.py | 4.8610 | 0.3406 | ĠPython, Ġargs, Ġobject |

## 5. Bottom-10 Lowest-Scoring Hunks

| name | file_path | max_score | mean_score | top_3_tokens |
|---|---|---|---|---|
| faker_hunk_0056 | faker/providers/address/uk_UA/__init__.py | -0.7986 | -5.6866 | ĠĠĠĠĠĠĠ, ĠĠĠĠĠĠĠ, ĠĠĠĠĠĠĠ |
| faker_hunk_0055 | faker/providers/address/uk_UA/__init__.py | -0.7986 | -5.4488 | ĠĠĠĠĠĠĠ, ĠĠĠĠĠĠĠ, ĠĠĠĠĠĠĠ |
| faker_hunk_0054 | faker/providers/address/uk_UA/__init__.py | -0.7986 | -5.4294 | ĠĠĠĠĠĠĠ, ĠĠĠĠĠĠĠ, ĠĠĠĠĠĠĠ |
| faker_hunk_0014 | faker/providers/company/fr_FR/__init__.py | -0.7986 | -0.7986 | ĠĠĠĠĠĠĠ, ĠĠĠĠĠĠĠ, ĠĠĠĠĠĠĠ |
| faker_hunk_0120 | faker/providers/address/es_AR/__init__.py | 0.0512 | -1.8774 | ĠĠĠ, ĠPer, rown |
| faker_hunk_0112 | faker/providers/address/es_AR/__init__.py | 0.0512 | -1.8774 | ĠĠĠ, ĠPer, rown |
| faker_hunk_0152 | faker/providers/person/tr_TR/__init__.py | 0.4607 | -1.5098 | ĠDo, ĠDo, ĠĠĠ |
| faker_hunk_0077 | faker/providers/person/pt_BR/__init__.py | 0.6239 | -1.9756 | end, arg, arc |
| faker_hunk_0035 | faker/providers/currency/__init__.py | 0.6785 | -0.2395 | SCR, Ġ"\, Ġ"\ |
| faker_hunk_0099 | faker/providers/phone_number/en_US/__init__.py | 0.7144 | -0.5224 | Ġ"$, Ġ"$, Ġ"$ |

## 6. Bimodality Check

Distribution appears unimodal — no gap of ≥ 2 units found.

## 7. Verdict + Production-Readiness Recommendation

**Partial false positives**: 5.7% of hunks above 5.

**Production recommendation:** 5.7% false-positive rate on ordinary code. Acceptable if break recall is high, but monitor in production.

### Thresholds applied
- Well-calibrated: p99 < 5 AND no hunks above 6
- Partial false positives: 5–15% of hunks above 5
- Systemic hallucination: > 15% of hunks above 5 OR distribution overlaps break range (5–10)
- Bimodal diagnostic: two clear clusters — identify what drives each

