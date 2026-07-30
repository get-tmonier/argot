# argot threat model

This document records what argot defends against, what it deliberately does not,
and where the trust boundaries sit. It complements [`SECURITY.md`](../../SECURITY.md)
(how to report) with the *why* behind the design. It is a living document —
update it when a new network path, input surface, or execution sink is added.

## What argot is

A single statically-linked Rust binary that:

1. reads a repository's **git history** and **working-tree source files**,
2. learns a per-repo model (`argot fit`), and
3. flags diffs that are foreign to that model (`argot check`).

It has no server, no daemon, and no telemetry. Analysis runs in-process in one
CLI invocation. A detached update check is the only outbound path argot takes on its own; it
uploads no repository content. The embedding model is compiled into the binary,
so analysis makes no request at all.

## Assets

- **The user's source code and git history.** argot reads them; it must never
  exfiltrate them. The zero-network-by-default posture is the primary control.
- **The integrity of argot's output.** In CI, a check result influences review;
  a result forged by attacker-controlled input would mislead reviewers. argot is
  non-blocking by default, which bounds the impact.
- **The user's machine / CI runner.** argot must not let untrusted repository
  content achieve code execution or escape its sandbox.
- **The release artifacts.** A tampered binary or model would compromise every
  downstream user.

## Trust boundaries and input surfaces

| Surface | Trust | Handling |
| --- | --- | --- |
| Source file bytes (extract/check) | **Untrusted** | Parsed by tree-sitter on byte buffers — no code execution. Parse failures degrade to skipping the file. |
| Git diffs / blobs / commit metadata | **Untrusted** | Read via libgit2 (`git2`); errors propagate as `Result`, not panics. |
| `dataset.jsonl`, `scorer-config.json`, semantic index | **Untrusted** (rebuildable artifacts) | Deserialized with serde; a malformed or stale artifact is rejected loudly (the semantic index is bound to the model identity). |
| `argot.toml` / `.argotignore` / suppressions | **Untrusted** | Parsed with `toml` / `serde_yaml` (pure-Rust); parse errors are reported, not executed. |
| Scripted rules (`.argot/rules/*/check.rhai`) | **Untrusted, executable** | Run in a Rhai sandbox: no I/O, captured print, 1M-op + depth/size caps, 100 ms per-file wall clock. A runaway or misbehaving rule is disabled for the run. |
| Embedded model weights | **Trusted with the binary** | Compiled in via `include_bytes!`; no transport to attack. Their integrity is the release artifact's, which is attested and checksummed by the release pipeline. A malformed tensor or tokenizer fails the load loudly rather than producing a wrong vector space. |
| Self-update payload | **Untrusted transport, pinned source** | Version from the published `version.json`, installer from the release web-download URL (never the GitHub API); TLS-enforced. |

Everything to the left of these boundaries is assumed hostile: a repository
argot is pointed at may have been crafted to make it crash, hang, or misreport.

## Threats and mitigations

- **Code execution from repository content.** Mitigated by parsing (never
  evaluating) source with tree-sitter, and by sandboxing the only executable
  input — scripted rules — with no I/O and hard resource caps. argot's own
  crates contain **no `unsafe`**, enforced at compile time (`unsafe_code =
  "deny"`, workspace-wide).
- **Denial of service (panic / hang) on crafted input.** Production code paths
  propagate errors via `anyhow`/`thiserror` rather than `unwrap`; the scripted-
  rule host has a wall-clock budget. The GitHub Action is non-blocking, so a
  crash degrades the check rather than blocking the merge. Fuzz targets exercise
  the untrusted-byte parsers (`fuzz/`).
- **Data exfiltration.** `extract → fit → check` do not upload repository
  content. The passive version check, explicit update, installation, and CI's
  configured GitHub API/release operations are distinct network paths; the
  binary's update request sends no code or findings.
  `ARGOT_OFFLINE=1` disables it.
- **Update tampering (supply chain, runtime).** Version from the published
  `version.json`, installer from the release web-download URL rather than the
  mutable GitHub API, TLS on all fetches. The embedding model is no longer a
  transport-level asset: it ships inside the binary, so there is no fetch to
  intercept and no mirror to substitute.
- **Release supply chain.** Dependencies are version-pinned with documented
  rationale; `git2` links a vendored libgit2 with network transports stripped
  (no OpenSSL/libssh2). `cargo-deny` gates advisories, licenses, and sources in
  CI. Release binaries ship per-artifact SHA-256 checksums and build-provenance
  attestations; a CycloneDX SBOM is published per release. CI uses least-
  privilege token scopes, `persist-credentials: false`, and SHA-pinned
  third-party actions.
- **CI credential leakage.** Workflows declare minimal `permissions`; the
  release PAT is used only where a downstream trigger requires it; checkouts do
  not persist credentials.
- **PR self-certification (Action-specific).** The Action fits the voice model
  on the PR **base** ref, never the head — so a PR cannot train argot on its own
  new code and certify it in-voice. Caches are keyed on the base commit for the
  same reason.

## Out of scope / accepted risk

- Vulnerabilities in third-party dependencies with no argot-specific impact
  (tracked via `cargo-deny` + Dependabot; reported upstream).
- The correctness of argot's *findings* — false positives/negatives are a
  quality concern, not a security one. argot is a probabilistic guardrail.
- An attacker who already controls the user's machine, CI runner, or the
  repository argot is invoked on **and** its own run (self-inflicted DoS).
- The landing site and documentation content.

## Non-goals

argot is not a SAST tool, a secret scanner, or a policy engine. It does not
attempt to detect vulnerabilities *in the code it reads* — only code that is
stylistically foreign to the repository. Don't rely on it as a security gate.
