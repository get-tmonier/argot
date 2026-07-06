# Break: httpx.Client.get() fetches a remote emoji sheet — foreign HTTP client, .get() collides with rich's get()
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic dict-based emoji lookup, NOT inside the hunk range
def local_emoji(table: dict[str, str], name: str) -> str:
    return table.get(name, "")


# hunk starts here
def load_emoji_sheet(client: "httpx.Client", name: str) -> str:
    # client is an httpx.Client injected by the caller. httpx is a foreign
    # dependency (0 rich sites), but .get() collides with rich's own attested
    # get() vocabulary, so the foreign HTTP reach is masked from the scorer.
    response = client.get(f"/emoji/{name}")
    return response.text
# hunk ends here
