# Authored audit proof receipts

These files are **authored fixtures**, not a wild-case corpus. They exist to
make the behavioral example and audit card reproducible without claiming that
either came from another project.

`rebuild-proof.sh` constructs a two-commit Python repository with fixed Git
identity and timestamps. It stages and commits the authored Django-style
change, then records the one-commit audit in JSON, Markdown, and HTML. It
requires a local development binary reporting exactly `argot 0.2.100`; this
pins the output contract rather than silently accepting a changed renderer.

The proof binary is the repository's default development build. Its audit
receipt preserves the reported group status: semantic, architecture, and
integrity are not compiled into that build. This is an explicit development
feature-status record, not a claim about the released full-feature binary.

Rebuild and verify from the repository root:

```sh
cargo build --bin argot
RECORD_AUDIT=1 docs/demo/rebuild-proof.sh
docs/demo/check-proof.sh
```

`checksums.sha256` covers the deterministic JSON, Markdown, and HTML audit
receipts. The check script rebuilds them in a temporary directory and compares
both every artifact checksum and the checksum manifest. Fixed timestamps, Git
metadata, fixture contents, and the exact Argot version make those receipts
byte-for-byte reproducible.

VHS/FFmpeg does not emit byte-stable GIFs on this runner (including after
metadata-free frame extraction), so `audit.gif` is an approved dynamic visual
artifact. `check-proof.sh` still re-renders it and requires a non-empty output;
visual review is the validation for that recording.

`audit.gif` is an accessible visual companion to `audit.md`: **Alt/caption:**
“Authored two-commit fixture: `argot audit --commits 1` reports one foreign
token sequence in the introduced Django-style import. The recording then says
to run `argot init` before checking new changes and names separately configured
pre-commit or GitHub Action routes. Semantic, architecture, and integrity are
shown as unavailable in this development build.”
