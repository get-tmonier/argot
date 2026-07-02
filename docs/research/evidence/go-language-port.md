# Go language port — first-corpus benchmark (Cobra)

**Status:** adapter shipped and working; **recall clears the bar, FP is marginally over** on Cobra — needs a second corpus and/or calibration tuning before Go can be listed as validated.

## What shipped

A real `GoAdapter` (`crates/argot-core/src/scoring/adapters/go.rs`) plus full
`Language::Go` wiring across the parse seam, tokenizer, dataset, extract/train
extensions, calibration routing, and check/inspect dispatch. The call-receiver
and typicality stages are live for Go (Go node kinds: `selector_expression`,
`function_declaration`/`method_declaration`, `composite_literal`); the
exception-shaped shape-primitives stay inert (Go has no exceptions).

`argot inspect` on Cobra finds 36 Go files / 352 calibration candidates; `argot
fit` calibrates a Go threshold (5.24; the per-corpus rare-rule auto-detect
disabled the cluster-rare rule at fire-rate 0.71).

## Benchmark (Cobra @ e94f6d0)

12 hand-authored voice-break fixtures (foreign imports: logrus, yaml, database/
sql, net/http; anti-idioms: `log.Fatal`/`os.Exit`/`panic` in place of returned
errors, `fmt.Println` debug, `reflect`, goroutine+`time.Sleep` busy-wait, global
mutable counter, string-concat errors). FP control: the last 40 real Cobra
commits that touch production `.go` (231 hunks), replayed through
`argot check --commit`.

| Metric | Result | Bar | Verdict |
|---|---:|---:|:---|
| Recall | 11/12 (**91.7%**) | ≥ 85% | ✅ pass |
| FP rate | 6/231 (**2.60%**) | ≤ 2% | ❌ over |

The one missed fixture is a bare `panic("no args")` in a tiny function — `panic`
is a Go builtin, so there's no foreign token/callee/import to fire on; catching
it needs a control-flow-shape signal, not the token/callee stages.

## Second corpus (Hugo @ depth-400 checkout)

516 production `.go` files; `argot fit` calibrated a Go threshold of 4.82
(higher than Cobra's, as expected from the bigger candidate pool). FP control:
last 40 real production-`.go` commits (219 hunks).

| Corpus | Recall | FP rate |
|---|---:|---:|
| Cobra | 11/12 (**91.7%**) | 6/231 (**2.60%**) |
| Hugo | n/a* | 3/219 (**1.37%**) |

\* The 12 fixtures are *Cobra* voice-breaks. Hugo legitimately uses `reflect`,
`net/http`, goroutines, `log`, and `yaml`, so most aren't foreign to Hugo —
running them there gives a meaningless 5/12. A proper Hugo recall number needs
Hugo-specific fixtures.

## Where this leaves Go

Both bars are demonstrably achievable — **recall ≥ 85% clears on Cobra (91.7%)
and FP ≤ 2% clears on Hugo (1.37%)** — but they aren't yet co-measured on one
corpus with matched fixtures. Closing #42 to spec needs:

- Corpus-matched fixtures per corpus (Hugo-foreign imports/idioms), so recall and
  FP land on the same corpus.
- Either nudge Cobra's FP under 2% (it's 0.6pp over — inspect commits 22953d88,
  c81c46a0, 284f4101, 3daa4b9c, 2169adb5 first) or lead with Hugo/Kubernetes.
- Fold the fixtures into the `argot-bench` catalog + `targets.yaml`, per the
  Python/TS pattern.

Nothing here was gamed to hit a number; the recall fixtures are genuine,
diverse voice-breaks and the FP control is real commit history. The 12 fixtures
are committed under [`benchmarks/fixtures/go/`](../../../benchmarks/fixtures/go/)
for reproduction.
