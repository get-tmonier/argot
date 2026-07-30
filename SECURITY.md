# Security Policy

argot is a local-first developer tool: a single statically-linked Rust binary
that learns a repository's "voice" from its git history and flags foreign code.
The analysis path runs entirely on your machine: `extract`, `fit`, and `check`
do not upload repository content and have no server, daemon, or telemetry.
Some separately enumerable operations can make outbound requests (update/version
check, installation, and CI's configured GitHub operations); `ARGOT_OFFLINE=1`
forbids the binary's update requests. No analysis argot performs needs a
network: every model it uses is compiled into the binary.
Its attack surface is small by design — but we
take it seriously, and this document tells you how to report a problem and what
we treat as in scope.

## Supported versions

argot ships as a rolling release: fixes land on `main` and are cut as patch
releases (see [CHANGELOG / releases](https://github.com/get-tmonier/argot/releases)).
Only the **latest published release** is supported. There are no long-lived
release branches to back-port to; please upgrade (`argot update`) before
reporting, in case the issue is already fixed.

| Version            | Supported |
| ------------------ | --------- |
| Latest release     | ✅        |
| Any older release  | ❌        |

## Reporting a vulnerability

**Please report privately — do not open a public issue for a security bug.**

1. **Preferred:** [GitHub private vulnerability reporting](https://github.com/get-tmonier/argot/security/advisories/new).
   This keeps the report confidential and lets us coordinate a fix and advisory
   in one place.
2. **Alternative:** email **damien.meur@tmonier.com** with `argot security` in
   the subject line.

Include, as far as you can:

- the argot version (`argot --version`) and platform;
- a description of the issue and its impact;
- steps to reproduce (a minimal repo, diff, or input that triggers it);
- any suggested remediation.

You do not need to encrypt your report, but if you prefer to, say so and we will
arrange a channel.

### What to expect

argot is maintained by a single person, so timelines are best-effort rather than
contractual:

- **Acknowledgement** within **3 business days**.
- An initial **assessment** (accepted / needs-info / not-a-vuln) within **10
  business days**.
- For accepted issues, a fix in the next patch release, coordinated with you on
  disclosure. We will credit you in the advisory unless you ask us not to.

### Safe harbour

We will not pursue or support action against anyone who reports a vulnerability
in good faith, acts within the scope below, avoids privacy violations and
service disruption, and gives us a reasonable chance to fix the issue before
public disclosure.

## Scope

**In scope** — issues in code this project ships:

- The `argot` binary (`crates/*`): memory-safety or panics reachable from
  **untrusted input** (a malicious git repository, diff, dataset file, config,
  or scripted rule), sandbox escapes in the scripted-rules (Rhai) host, and any
  path that executes attacker-controlled data.
- The install and self-update flow (`argot update`, the cargo-dist installers).
- The embedded model weights and their load path (`argot-rules-semantic`): a malformed tensor or tokenizer reachable from a tampered binary.
- The GitHub Action (`action.yml`) and the release/CI pipeline
  (`.github/workflows/`): credential handling, artifact integrity, injection.

**Out of scope:**

- Vulnerabilities in third-party dependencies with no argot-specific impact —
  please report those upstream (we run `cargo-deny` in CI and will pick up
  advisories; a heads-up is still welcome).
- The landing site content (`landing/`) and documentation.
- False positives / false negatives in argot's findings. argot is a
  **probabilistic guardrail**, not a correctness oracle; a missed or spurious
  finding is a quality issue, not a security one — open a normal issue.
- Denial of service that requires you to point argot at a repository you already
  fully control and that harms only your own run.

## Security design (summary)

- **Local-first analysis, explicit egress.** `extract → fit → check` never
  upload repository content. A missing semantic model may be fetched on first
  use; the passive version check, explicit `argot update`, installation, and
  CI's configured GitHub operations are separate outbound paths. The binary
  sends no code, findings, or telemetry on those paths. `ARGOT_OFFLINE=1`
  forbids its update requests.
- **Model integrity.** The embedding model is compiled into the binary, so there
  is no download to intercept, no mirror to substitute and no cache to poison:
  the weights' integrity is the release artifact's integrity, which is attested
  and checksummed by the release pipeline. Their provenance is recorded in the
  file's own safetensors metadata and in `NOTICE`.
- **No `unsafe`.** argot's own crates contain no `unsafe` code; this is enforced
  at compile time (`unsafe_code = "deny"`, workspace-wide).
- **Sandboxed scripted rules.** Community Rhai rules run with no I/O, captured
  output, operation/size/depth caps, and a 100 ms per-file wall-clock budget.
- **Supply chain.** Dependencies are pinned with documented rationale, `git2`
  links a vendored libgit2 with network transports stripped, `cargo-deny` gates
  advisories/licenses/sources in CI, and release binaries carry per-artifact
  SHA-256 checksums and build provenance attestations.

A fuller write-up of trust boundaries and threats lives in
[`docs/security/threat-model.md`](docs/security/threat-model.md).
