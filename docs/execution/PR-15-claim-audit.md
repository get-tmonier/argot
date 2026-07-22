# PR-15 public claim audit

**Date:** 2026-07-22
**Scope:** public consumer surfaces: `README.md`, `landing/`, `action.yml`,
plugin/registry manifests, skills, and generated demo receipts. This report is
evidence only; it does not authorize copy changes outside the PR-15 lease.

## Method and receipt

```sh
rg -n -i 'automatic|before you accept|check.on.accept|no network|no model|100%|98%|94%|33 real|caught in the wild|safe' \
  README.md landing action.yml .claude-plugin server.json skills docs/demo
```

- Reviewed the current landing pages and the demo GIF manually: no visual
  current-tense automatic acceptance-lifecycle or absolute privacy/model claim
  appears in the inspected UI.
- Sampled `docs/demo/proof/{audit.json,authored-check.json}`: the generated
  receipts identify their historical tool version and do not market a current
  capability.
- The audit command exits nonzero only for a scan/tool failure, not because a
  search match needs copy work; matches are classified against
  [PUBLIC_CLAIMS.md](PUBLIC_CLAIMS.md).

## Classification

| Claim family | Result | Evidence / required qualifier |
| --- | --- | --- |
| Acceptance-time automatic checking | **Keep future-only.** | Public setup, CI, and plugin material describes manual, user-wired, or opt-in paths. No current automatic-before-accept claim was found. |
| Pre-commit | **Qualified.** | It is user-wired and runs staged `argot check`; it is not described as universally informational or acceptance-time automation. |
| GitHub Action | **Qualified.** | `action.yml` describes a non-blocking default and explicit `fail-on-hits` gate. |
| Claude hook, skills, and MCP | **Qualified.** | Plugin material confines the hook to opt-in fitted-repository pre-write foreign-import prompts; skills/MCP are on-demand. |
| Local analysis, telemetry, and network | **Qualified.** | Consumer copy says no default telemetry and keeps the model/update/download qualifiers; it does not promise no network under every invocation. |
| Authoritative model path | **Qualified.** | No generative or opinion-forming model decides findings; wording does not claim that no model is used anywhere. |
| Detector, combined-brief, speed, and corpus numbers | **Remove or keep unavailable.** | No new public current numeric capability claim is authorized without the named evidence/decision. Historical research values remain internal evidence, not consumer positioning. |
| Languages and supported runners | **Qualified.** | `action.yml` names the published runner targets; release readiness now exercises the offline journey on those targets in CI. |

## Explicit exceptions

| Path | Reason | Owner |
| --- | --- | --- |
| `docs/strategy/` | Historical/current-reality documents contain quoted forbidden examples specifically to prohibit them; they are not consumer capability copy. | Strategy (do not edit in PR-15) |
| `docs/research/` | Historical experiment figures and old fixture versions are evidence, not launch claims. | Research/evidence (do not edit in PR-15) |
| `docs/demo/proof/` | Versioned generated receipt provenance is intentionally preserved and does not assert current release behavior. | Demo/proof owner |

## Verdict

No forbidden current public claim was found in the audited consumer surfaces.
Automatic lifecycle language remains future tense because REL-03 is deferred;
this PR adds no automatic lifecycle behavior.
