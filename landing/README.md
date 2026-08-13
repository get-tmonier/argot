# @argot/landing

The marketing site and docs for argot — **https://argot.tmonier.com**.

Astro 7 (static) · Tailwind 4 · i18n (en/fr) · oxlint/oxfmt · `astro check`. Animations are pure CSS
keyframes + a single `IntersectionObserver` for scroll reveals — no runtime animation library.

This is a **standalone project** with its own `bun.lock` and `node_modules` — deliberately *not* a
member of the repo-root bun workspace, so it ships only its own dependencies and can't reach the
CLI/engine packages.

## Develop

```sh
just landing          # dev server (astro dev)
just landing-check    # oxlint + oxfmt + astro check
just landing-build    # static build → landing/dist
```

## Bumping dependencies

`vite` and `playwright-core` are in the tree as *peers* — astro owns vite (`@tailwindcss/vite` only
peer-depends on it), and `@axe-core/playwright` peer-depends on playwright-core. bun pins an
auto-installed peer in the lockfile and never re-resolves it on an incremental install, so editing a
few versions and patching the lock in place leaves the old peer next to the new one. Two copies of
vite (or of playwright-core) means two copies of their *types*, and `astro check` fails with
`Type 'Plugin<any>[]' is not assignable to type 'PluginOption'` or a `Page`-vs-`Page` mismatch —
which is how a Dependabot bump lands red on green code.

Bump by regenerating, not by patching:

```sh
rm bun.lock && bun install     # one copy of every peer
just landing-check && bun run test:routes && bun run test:a11y
```

## Structure

```
src/
  tokens/      design tokens (tokens.css → @theme → global.css)
  styles/      global.css (fonts, base, keyframes, .reveal)
  layouts/     Base.astro (SEO/JSON-LD/reveal), DocsLayout.astro (sidebar + prose)
  components/  Logo, Nav, Hero, VoiceField, Demo, CodeWindow, TerminalCard,
               Catches, Local, Features, Cta, Footer, HomePage
  i18n/        en.ts / fr.ts (typed SiteContent)
  content/     docs/*.md (the integrated documentation)
  pages/       index.astro, fr/index.astro, docs/
public/        favicon.svg, og.png, _headers, robots.txt
```

The hero backdrop (`VoiceField.astro`) is argot's visual thesis: a baseline of in-voice token dots
with a few divergent hunks that lift off the line and flare in the flag colours, swept by a scan bar —
the same divergence-from-the-norm that `argot check` flags.

## Deploy (Cloudflare Pages)

Connect the repo and set, in the Pages project:

- **Root directory:** `landing`
- **Build command:** `bun install --frozen-lockfile && bun run build`
- **Build output directory:** `dist`
- **Custom domain:** `argot.tmonier.com`

`--frozen-lockfile` makes the deploy reproducible: it installs exactly what's pinned in
`landing/bun.lock` and fails if the lockfile drifts from `package.json`, instead of mutating it
mid-build (bun's equivalent of `npm ci`).

**Rebuild-on-release:** `/version.json` is built from files *outside* this directory
(`../Cargo.toml`, `../skills/VERSION`, the embedder's model pin) and is what `argot update` and
the binary's daily notice resolve against. Because the Pages root directory is `landing/`, a
release commit that only bumps `Cargo.toml` would not trigger a rebuild — so the auto-release
workflow also stamps `src/data/release.json`, guaranteeing every release redeploys the site. Don't
add Pages "build watch paths" that could skip those commits.

`public/_headers` ships the security headers and long-cache rules; the sitemap is generated at
`/sitemap-index.xml`.
