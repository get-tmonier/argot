# Phase 1c — mute + prevention surfaces (fastapi)

## Mute (intentional foreign → audit trail) — WORKS
- `import tenacity` → `! foreign · tenacity [cbc8047c9ecc]`.
- `argot mute cbc8047c9ecc --reason "RFC-42: tenacity is our chosen retry lib"`
  → "Muted [hash] in fastapi/_client.py — RFC-42…".
- Re-check → clean. `list-mutes` shows it active; `.argot/suppressions.yaml`
  records path+hash+reason. Clean, auditable, exactly as advertised.

## Good reminder: argot only flags GENUINELY foreign
- First tried `import httpx` for the mute demo → scored CLEAN. httpx is attested
  in fastapi (its tests use it), so it's in-voice. argot didn't cry wolf on a
  popular lib the repo actually uses — correct, and a nice honest data point.

## Prevention surfaces
- `argot describe-voice` → a repo-voice summary (callee groups + a "Red flags"
  line: "a hunk that calls something absent from its area's set, or imports
  outside the familiar set, is what argot flags"). A concrete artifact to hand an
  agent BEFORE it writes — prevention, not just detection.
- `argot mcp` starts the stdio MCP server (voice_context) — the proactive
  integration; feeds idioms to an agent in-editor.

## Skill fixes applied (from the dev-loop agent's report)
- Documented the hit JSON fields (reason_label, source, score/threshold) in
  argot-check SKILL.md, with "read severity, not the raw score/threshold".
