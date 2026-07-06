# Issue #92 — false-alarm root cause + the call_receiver fix it specifies

**Date:** 2026-07-03 · **Branch:** `bench/92-temporal-holdout`. The guardrail's
co-headline is false-alarm rate (a guardrail that cries wolf is worse than none).
This pins where the false alarms come from and what the fix must be.

## The two FP drivers (temporal-holdout, existing-file)

| corpus | FP | by reason |
|---|---|---|
| bat | 11.54% (39/338) | **call_receiver 33**, import 4, convention 1, bpe 1 |
| jellyfin | 9.73% (43/442) | **call_receiver 21**, **convention 21**, import 1 |

The **novel-pattern import signal barely false-alarms** (4 / 1) — the actionable
guardrail signal is already clean. The noise is `call_receiver` (new callees) and
`convention` (structural). Both are *secondary* to the foreign-import signal.

## Refuted: cluster-bonus is not the driver

Hypothesis: the `call_receiver` FP is cluster-bonus (+5 for a globally-attested
callee absent from the file's cluster — i.e. new code calling an existing repo
fn). Tested `CR_CLUSTER_BONUS = 0`:

- **bat FP unchanged: 11.54% (still 33 call_receiver).** The FP is **alpha**
  (unattested callee), not cluster-bonus.
- jellyfin dropped only 9.73→7.47% (cr 21→11).
- **Cost: −2 catches** (98→94%: `laravel_foreign_firebasejwt_1`, redis
  `foreign_libevent_1` were riding on cluster-bonus). Net negative. Reverted.

## Root cause (the load-bearing finding)

**A call into a foreign library and a call to a new function the dev/LLM just
wrote are the *same signal* — an unattested callee.** No threshold/bonus tuning
separates them. The only reliable discriminator is **foreign-import association**:

| case | unattested callee | foreign association | fire? |
|---|---|---|---|
| Doctrine / MongoDB / viper | yes | foreign receiver/FQN (`\Doctrine\ORM`, `viper.` ← foreign pkg) | ✅ |
| libevent (C) | yes | foreign `#include` in the file | ✅ |
| bat's new `helper()` | yes | none — repo-internal/bare, no foreign import | ❌ suppress |

## The fix this specifies

An unattested callee fires the novel-pattern signal **only** when tied to a
foreign import: (a) its receiver is a foreign namespace/FQN, or (b) it resolves
through a foreign import (`viper` ← `github.com/spf13/viper`), or (c) the hunk
carries a foreign import/`#include`. A callee with a **repo-internal receiver**
(`self`/`this`/a local) or a **bare** name with **no foreign import** is treated
as new legitimate code and suppressed. This keeps every gated foreign catch (all
have a foreign FQN or import) and removes the `call_receiver` cry-wolf on the
codebase's own new functions.

This is a `call_receiver` redesign (per-language foreign-receiver detection +
import↔callee association), validated by re-benching **catch and FP together**
across corpora — the next focused pass.
