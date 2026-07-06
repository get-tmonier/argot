# Break: jinja2 Template.render() drives markdown output — foreign engine, leaf collides with rich's render()
"""Break fixture — not for import."""
from __future__ import annotations


# Decoy — idiomatic rich markdown heading, NOT inside the hunk range
def render_heading(text: str, level: int) -> str:
    return f"{'#' * level} {text}"


# hunk starts here
def render_block(template: "jinja2.Template", context: dict[str, object]) -> str:
    # template is a jinja2.Template built and injected by the caller. jinja2 is
    # a foreign dependency (0 rich sites), but .render() collides with rich's
    # own attested render() vocabulary, so the foreign reach is masked.
    heading = context.get("title", "")
    body = template.render(**context)
    return f"{heading}\n{body}"


def render_many(template: "jinja2.Template", rows: list[dict[str, object]]) -> list[str]:
    return [template.render(**row) for row in rows]
# hunk ends here
