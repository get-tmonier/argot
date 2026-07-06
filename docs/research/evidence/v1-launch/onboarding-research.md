# Onboarding patterns for AI-era dev CLIs — research brief

Captured 2026-07-06 to inform the v1 setup/onboarding redesign (README + landing
home + docs). Source study: ~15 tool landing pages/READMEs + the Evil Martians
"100 devtool landing pages 2025" study + the `npx skills` / `add-mcp` ecosystem.

## Winning first-glance anatomy

- **One command block, above the fold, as the centerpiece** — not a matrix. If OS
  matters, 2 tabs max (Bun, uv); most tools pick one PM (Biome/Turbo/Astro: npm only).
- **Copy button is table stakes** (Astro's "Copied!" state).
- **Two CTAs, visually unequal** — bold primary + lighter "View on GitHub"
  (Biome, Vitest, oxc). Secondary must be styled differently so it doesn't compete.
- **Show the tool doing its job, not just installing** — Ruff pairs `uvx ruff check`
  with install; Biome shows `npx @biomejs/biome format`. Payoff on screen.
- **Specific CTA verbs** beat generic "Get started" (Evil Martians: use "Start
  building" / "Download now"). Tagline = what + differentiator + speed in one line.

## AI-agent-native without looking like "just a plugin" (argot's core risk)

The shadcn model (a CLI an agent can drive — nearly identical to argot):
- **Hero never mentions AI/MCP/agents.** Leads with product identity + a visual.
  The CLI (`npx shadcn init`/`add`) is unambiguously the product.
- **MCP/skill = a natural-language front-end onto the CLI you already have**, given
  its own prominent docs section (peer to Installation/Theming) — prominent, but an
  entry point, not the identity.
- **Transparency reads as "real tool":** skills.sh states plainly that
  `npx skills add <owner>/<repo>` "installs SKILL.md files into your repository so
  Claude Code can reference them… picks them up automatically the next session."
  Say what the skill writes + that it drives the binary. Opacity reads as toy plugin.
- **Breadth = credibility** (skills: "68 more agents"; AGENTS.md: "20+ platforms").
- Keep the README **human-first / product-first**; the agent affordance is an added
  surface, not the headline (inverse of AGENTS.md's "README for agents" framing).

## Progressive disclosure

One obvious primary path fully inline; everything else demoted to a link/callout.
- **uv** is textbook: one-liner inline → one "next steps" sentence → pip/Homebrew
  shrunk into a linked "Tip."
- **shadcn three-tier** = argot's model: (1) recommended path boldest, (2) CLI,
  (3) manual, each progressively smaller. argot analog: (1) skill `npx skills add`,
  (2) copy-paste prompt, (3) manual CLI.
- Show the **30-second happy path** inline; link out OS/PM permutations + manual.
- "Show it running" before "show all options."

## Anti-patterns (2026)

- Install-method matrix in the hero at equal weight.
- Generic "Get started" as the only CTA.
- Leading the hero/README with "AI-powered"/MCP/agent branding → reads as a plugin.
- Opaque magic (not saying what the skill writes/runs).
- Burying the payoff behind philosophy/feature lists/alternatives.
- Auto-pulled social-proof walls (use one curated stat instead).
- No copy button (reads as dated).

## Recommended anatomy for argot

1. Tagline: "flags code foreign to your repo's patterns" + "single static Rust
   binary, no runtime deps." AI stays out of the tagline.
2. One controlled proof stat above the fold (99% catch / <1% over-fire, 27 repos).
3. One primary command block (copy button); show install *and* a first real run.
4. Two unequal CTAs; specific-verb primary + lighter "View on GitHub."
5. Three-tier setup (shadcn model), primary boldest:
   - **Let your agent set it up** — `npx skills add get-tmonier/argot` → framed as
     "installs a SKILL.md so Claude Code / Cursor can run argot for you"; name what
     it writes + that it drives the `argot` binary; cite agent breadth.
   - **Copy-paste prompt** — one fenced block for agents without the skills CLI.
   - **Manual CLI (for control)** — raw install + commands, demoted ("if you want
     control, here's the manual way").
6. argot is a Rust CLI guardrail an agent can *also* drive — product-first hero,
   agent path a prominent-but-secondary lane.

Sources: bun.sh, docs.astral.sh/uv, astral-sh/ruff, biomejs.dev, ui.shadcn.com
(+ /docs/mcp, /docs/installation), astro.build, turborepo.dev, tailwindcss.com,
orm.drizzle.team, oxc.rs, prisma quickstart, vercel-labs/skills + skills.sh,
neon.com/blog/add-mcp, agents.md, llmstxt.org, Evil Martians "100 devtool landing
pages 2025".
