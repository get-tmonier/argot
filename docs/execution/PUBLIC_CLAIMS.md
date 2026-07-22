# Public claim dictionary

**Issue:** CL-01 · **Source date:** 2026-07-22

This is an operational source for later consumer-copy owners. It authorizes no
consumer change. `Current` means supported by current repository reality;
`qualified` requires the stated boundary; `unavailable` must not be used until
its named evidence/decision exists; and `internal` is not positioning.

## Canonical D-register

| ID | Canonical decision |
| --- | --- |
| D1 | Behavioral invariant is the foundational belief |
| D2 | Audit installs; check-on-accept retains |
| D3 | Build and market separate acquisition and retention engines |
| D4 | Frame retention as awareness, not defect detection |
| D5 | North Star is audit-to-habit conversion |
| D6 | Conviction on the foundation; options on the destination |
| D7 | Fully local individual core remains free, no account/payment |
| D8 | Pursue onboarding that runs the check at the nearest acceptance lifecycle without a manual step |
| D9 | No future-specific work before its evidence gate is crossed |
| D10 | “Voice” is secondary brand/visual language, never the explanation |
| D11 | Keep the four positioning layers separate |
| D12 | No generative/opinion-forming model in the authoritative analytical core |
| D13 | Local-first, no default telemetry; enumerated default egress |
| D14 | Signal quality is existential; no default-gating detector above the defined noise threshold |

## Claim ledger

| Claim | Status / evidence owner | Allowed wording | Forbidden wording |
| --- | --- | --- | --- |
| Product job | qualified · D1/D4/current reality | “Surfaces repository-grounded divergence while you decide whether to accept a change”; name the current invocation. | “Automatically checks every change before you accept it.” |
| Audit | qualified · audit evidence | “`argot audit` needs no prior Argot fit/config and evaluates the base-to-HEAD net diff.” | Product-wide zero setup, commit-by-commit replay, or a complete historical census. |
| Audit attribution | qualified · audit evidence | “Supported commit-marker attribution is a floor, not a census.” | Detects who wrote code or what AI introduced. |
| Automatic acceptance lifecycle | unavailable · lifecycle evidence/DR-07 | “A retention target” only. | Current or universal automatic-before-accept checking. |
| Claude pre-write hook | qualified · EV-01 | “In a fitted Claude Code repository, the opt-in plugin can ask before a write introduces a foreign dependency.” | Full check-on-accept, every detector, or blocking behavior. |
| Skills and MCP | qualified · EV-01 | “Six on-demand skills” and “optional agent-invoked MCP context/checks.” | Automatic checking across named agents or guaranteed invocation. |
| Pre-commit | qualified · current manifest/DR-06 | “A user-wired staged check that currently uses `argot check --staged`.” | Informational/never failing, or acceptance-time checking. |
| GitHub Action | qualified · current action | “A workflow-configured PR/push signal; non-blocking by default, with an opt-in gate.” | A default merge gate or acceptance-time check. |
| Local analysis and telemetry | qualified · D13/current reality | “Source, history, and findings are analyzed locally; Argot has no default telemetry.” | Nothing ever leaves the machine. |
| Network | qualified · privacy inventory | “Enumerate model, update/version, review/download, and explicit-update paths.” | No network without the applicable qualifier. |
| Authoritative model path | qualified · D12 | “No generative or opinion-forming model decides findings; local statistical/graph/scripted and embedding evidence remains inspectable.” | No model or no LLM anywhere. |
| Clean check | qualified · check contract | “No configured findings on the scanned change.” | Correct, fully idiomatic, or 100% in voice. |
| Detector numbers | unavailable · DR-09/benchmark manifest | Only the selected manifest wording with its revision, corpus, denominator, and detector scope. | Product-wide 98%, 94%, or mixed-lineage figures. |
| Combined brief/noise | unavailable · EV-02/DR-03/BM-09 | “Not yet measured for the combined brief.” | Applying a detector-specific rate to the full brief. |
| Speed | qualified · timing evidence | Named hardware/repository/cold-warm bounds only. | Universal “60 seconds” or “two minutes.” |
| Languages and platforms | qualified · adapter/distribution evidence | Name the tested adapters or supported targets with their source. | All languages or universal static portability. |
| Open source and free local core | current · LICENSE/D7 | “MIT-licensed open source” and “the individual local core remains free.” | Future pricing promises. |
| Account/cloud | current · current reality/D7 | “No account or cloud required for core audit/check.” | No network ever. |
| Portable configuration | qualified · parity evidence | “Portable, user-owned configuration” only after the named surface-parity evidence passes. | Cross-surface parity before that evidence exists. |
| Voice | internal · D10 | Brand or compatibility term only. | The product explanation or a conformity score. |
| Wild corpus | unavailable · EV-05/DR-10 | Exact count only from qualifying committed receipts. | “33 repositories” or a commit SHA presented as a finding hash. |
| Launch film | removal selected, not shipped · EV-06/[DR-11 #163](https://github.com/get-tmonier/argot/issues/163) | No launch-film capability wording. Removal from the launch path is a policy outcome pending landing implementation, not a shipped state. | Safety, automatic-lifecycle, no-model, or no-network absolutes; that removal has already shipped. |

## Consumer worklist

Before changing copy, inspect affected consumers:

```sh
rg -n -i '70\+|automatic|accept|in.voice|100%|no model|no network|33 real|caught in the wild|safe' \
  README.md landing docs AGENTS.md action.yml
```

The result is a worklist, not authorization to edit every match. Numeric claims
wait for DR-09’s manifest selection; current-tense lifecycle copy waits for
released, measured lifecycle evidence.
