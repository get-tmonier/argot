# Release migration notes

Use this source when preparing GitHub-generated release notes. It describes the
current compatibility surface; it is not a per-version changelog.

| Surface | Before | Now | Required action | Canonical guidance |
| --- | --- | --- | --- | --- |
| Pre-commit | A hook could be assumed informational. | The user-wired staged hook runs `argot check --staged`; error findings can fail the hook. | Review existing hook automation and use the explicit gate recipe only if that policy is intended. | [CI and pre-commit](../../landing/src/content/docs/ci.md) |
| Check JSON | Consumers could rely on fields by convention. | Check JSON declares `schema_version: 1`; finding semantics remain the contract. | Validate parsers against the v1 schema and ignore additive fields. | [Check output](../../landing/src/content/docs/check.md) · [schema](../schema/check-v1.schema.json) |
| Human output | Terminal text is explanatory, not a stable parser API. | Human output continues to include evidence, severity, and a mute hash. | Parse `--format json`, not terminal formatting. | [Check output](../../landing/src/content/docs/check.md) |
| GitHub Action metric | The visual score is advisory. | The Action remains non-blocking by default; gating is an explicit input. | Keep `fail-on-hits` unset unless the repository intentionally chooses a gate. | [CI and pre-commit](../../landing/src/content/docs/ci.md) |
| Claude plugin | Installed plugin updates depend on the plugin manifest version. | The release workflow keeps the plugin version aligned with the Cargo release. | Reinstall or update the plugin when a release includes plugin changes. | [Plugin](../../landing/src/content/docs/plugin.md) |
| Automatic lifecycle | No automatic acceptance-time lifecycle is shipped. | It remains a future retention target pending lifecycle evidence. | Do not represent it as currently enabled or configure it from these notes. | [Public claim dictionary](PUBLIC_CLAIMS.md) |

## Rollback and opt-out

- Remove a user-wired pre-commit hook with `pre-commit uninstall`; this does not
  change the repository’s source or Argot findings.
- Remove the Claude plugin with `/plugin uninstall argot@argot`.
- Disable optional Action gating by removing `fail-on-hits: true`; the Action
  otherwise remains an advisory workflow signal.

The release workflow runs `scripts/check-release-version.py` before packaging.
It requires the Cargo workspace, plugin, MCP/npm registry, site release data,
and release tag to agree; it does not add an automatic check-on-accept lifecycle.
