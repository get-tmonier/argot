# PR-02 evidence work log

**Worktree:** `/Users/damienmeur/projects/argot-wt-pr-02-20260722t001`
**Branch:** `codex/pr-02-evidence-20260722t001`
**Base:** `98ef01c33f4193715a43da92794c083005284297`
**Date:** 2026-07-22

## Lease and exclusions

The lease is new evidence records under `docs/research/evidence/` and
`docs/execution/PUBLIC_CLAIMS.md`. Strategy, backlog, product code, landing,
README, benchmark implementation, and consumer copy are excluded. No GitHub
state is changed.

## Issue record

| Issue | Goal and leased artifact | Dependency / boundary | Validation planned or completed |
| --- | --- | --- | --- |
| EV-01 | Released capability matrix: `integration-capability-matrix.md` | No dependency; inventory, not a support promise. | Repository-manifest/source inspection, focused MCP test, official vendor links. |
| EV-02 | Reproducible combined-brief protocol: `accept-brief-protocol.md` | DR-02 must freeze exposure semantics before implementation/measurement. | Three-record dry-run contract and schema review; no full evaluation is claimed. |
| EV-03 | Fixed clean/one/many prototypes: `accept-brief-prototypes.md` | DR-02 owns exposure policy; DR-14 owns selection. | Fixed 80-column/12-line proxy protocol; it is not a human study (#273 tracks that study). |
| EV-05 | Wild-case receipt inventory: `caught-in-the-wild-inventory.md` | No dependency; no public page change. | Source inventory and accessible upstream-commit links; unavailable receipts are named. |
| EV-06 | Film claim/accessibility/provenance inventory: `launch-film-inventory.md` | Evaluated; acceptance criteria are not satisfied. The approved [DR-11 fallback (#163)](https://github.com/get-tmonier/argot/issues/163) selects removal from the launch path, pending implementation. | Repository-reference and remote-asset receipt review. |
| CL-01 | Claim dictionary: `PUBLIC_CLAIMS.md` | EV-01 complete; DR-09 numeric lineages remain unavailable. | Canonical D-register and current-reality cross-check; phrase-search worklist. |

## Claim safeguards applied

- No lifecycle is described as shipped automatic accept-time behavior.
- The DR-14 proxy is explicitly bounded: it may inform a layout choice but is
  not a passed human study; #273 remains the human-study follow-up.
- EV-05 treats reachable upstream commits as provenance fragments, never as
  finding hashes or case validation.
- EV-06 is evaluated but does not satisfy its acceptance criteria. Under the
  approved [DR-11 policy (#163)](https://github.com/get-tmonier/argot/issues/163),
  unavailable evidence selects removal from the launch path; this is not a
  shipping claim or an implemented change.
- Numeric detector and combined-brief claims remain unavailable until their
  selected manifest/evaluation gates are satisfied.

## Completion receipt

Run the commands listed in each evidence record after final review. Record
their exact result in the handoff; no claimed command result in this work log is
substituted for a retained terminal receipt.
