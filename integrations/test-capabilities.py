#!/usr/bin/env python3

import json
from datetime import date
from pathlib import Path
from urllib.parse import urlparse


path = Path(__file__).with_name("capabilities.json")
document = json.loads(path.read_text())
required = {
    "id", "type", "event", "coverage", "prerequisites", "invocation",
    "blocking_default", "tested", "tested_version", "canonical_guide",
}
allowed_invocations = {
    "automatic-when-configured", "passive", "invoked", "user-wired", "not-available",
}
expected_ids = {
    "claude-plugin-pre-write",
    "claude-plugin-mcp",
    "bundled-skills",
    "skills-installer",
    "standalone-mcp",
    "pre-commit",
    "pre-commit-gate",
    "github-action",
    "cli-and-review",
    "claude-end-of-turn-lifecycle",
}
allowed_guides = {
    "https://argot.tmonier.com/docs/agents/",
    "https://argot.tmonier.com/docs/ci/",
    "https://argot.tmonier.com/docs/getting-started/",
}

assert document["schema_version"] == 1
date.fromisoformat(document["evidence_date"])
assert document["capabilities"]
seen = set()
for capability in document["capabilities"]:
    assert set(capability) == required
    assert capability["id"] not in seen
    seen.add(capability["id"])
    assert capability["invocation"] in allowed_invocations
    guide = urlparse(capability["canonical_guide"])
    assert guide.scheme == "https" and guide.netloc == "argot.tmonier.com"
    assert capability["canonical_guide"] in allowed_guides
    assert isinstance(capability["prerequisites"], list)
    if capability["tested"]:
        assert capability["tested_version"]
        assert capability["invocation"] != "not-available"
    else:
        assert capability["tested_version"] is None
        assert capability["invocation"] == "not-available"

assert seen == expected_ids
assert next(
    capability for capability in document["capabilities"]
    if capability["id"] == "claude-end-of-turn-lifecycle"
)["tested"] is False
