# Landing manual responsive matrix

Run this recorded spot check before a landing release. Automated axe and
Lighthouse runs complement it; they do not establish rendered visual behavior.

| Viewport / setting | Routes | Check | Receipt |
| --- | --- | --- | --- |
| 320px | `/`, `/#audit`, `/#engine`, `/benchmarks/`, `/#proof`, `/docs/`, `/privacy/` | No horizontal overflow; every primary action is reachable; mobile navigation opens, escapes, and restores focus. | Navigation width is automated in `responsive.spec.ts`; keyboard spot check required before release |
| 375px | Same | Focus ring remains visible; skip link lands on main content. | Navigation width is automated in `responsive.spec.ts`; keyboard spot check required before release |
| 768px | Same | Navigation transition has no hidden or duplicate controls. | Navigation width is automated in `responsive.spec.ts`; visual spot check required before release |
| 1440px | Same | Header, section links, and long docs layouts remain usable. | Navigation width is automated in `responsive.spec.ts`; visual spot check required before release |
| Reduced motion | `/` | No essential content depends on animation; navigation remains operable. | Automated in `responsive.spec.ts`; visual spot check required before release |
| 200% zoom | `/`, `/docs/` | No clipped primary controls or unrecoverable overlap. | Pending release check |
