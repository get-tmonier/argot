# Audit demo and proof receipts

`proof/audit.gif` is the rendered terminal recording consumed by the project
README. It starts with a no-setup `argot audit`, shows the finding in an
**authored** two-commit fixture, then names the deliberate next step: fit the
current repository before using `argot check` on new changes.

The final lines mention pre-commit and GitHub Actions as optional routes that
must be configured separately. This recording does not configure either route,
schedule a check, or claim an ongoing lifecycle.

## Files

- **`audit.tape`** — the [VHS](https://github.com/charmbracelet/vhs) script for the terminal recording.
- **`receipts.py`** — the introduced Django-style change in the small authored fixture. It exists solely to make the audit result deterministic; it is not presented as a real-world catch.
- **`rebuild-proof.sh`** — constructs the fixed two-commit repository, records the audit JSON/Markdown/HTML receipts, and optionally records the GIF.
- **[`proof/`](proof/)** — committed receipts, checksum manifest, and the accessible caption for the visual.

## Rebuild and verify

The bundle is pinned to the `argot 0.2.100` output contract and rebuilt with a
local development binary reporting that version. This default build
intentionally omits the semantic, architecture, and integrity feature layers,
so the committed receipt records those groups as unavailable rather than making
a claim about the released full-feature binary.

```sh
cargo build --bin argot         # produces target/debug/argot 0.2.100
RECORD_AUDIT=1 docs/demo/rebuild-proof.sh
docs/demo/check-proof.sh
```

Install [VHS](https://github.com/charmbracelet/vhs) first (`brew install vhs`)
when recording `audit.gif`. JSON, Markdown, and HTML receipts are
byte-for-byte checked; GIF bytes are intentionally not compared because
VHS/FFmpeg output is not stable across runners. See [`proof/README.md`](proof/README.md)
for the fixture history, version/context, checksum gate, and alt/caption.
