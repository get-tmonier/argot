# CycloneDX 1.5 SBOM from `cargo metadata` — Python-free (jq only; jq already
# drives the release pipeline's manifest parsing). Invoked by cargo-dist as an
# `extra-artifacts` build step (see dist-workspace.toml); the resulting
# bom.cdx.json is checksummed, attested, and attached to the GitHub release.
#
# Usage: cargo metadata --all-features --format-version 1 | jq -f scripts/gen-sbom.jq
(.packages | map({(.id): .}) | add) as $byid
| ([.workspace_members[] | $byid[.] | select(.name == "argot")] | first) as $root
| {
    bomFormat: "CycloneDX",
    specVersion: "1.5",
    version: 1,
    metadata: {
      timestamp: (now | todate),
      tools: [ { vendor: "argot", name: "gen-sbom.jq" } ],
      component: {
        type: "application",
        name: $root.name,
        version: $root.version,
        purl: ("pkg:cargo/" + $root.name + "@" + $root.version)
      }
    },
    components: (
      [ .packages[]
        | select(.name != $root.name)
        | { type: "library", name: .name, version: .version, purl: ("pkg:cargo/" + .name + "@" + .version) }
          + (if .description then { description: .description } else {} end)
          + (if .license then { licenses: [ { expression: .license } ] } else {} end)
      ] | sort_by(.name, .version)
    )
  }
