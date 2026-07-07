# ID: fastapi/sse.py:146
def encode_sse_event(*, data_str=None, event=None, id=None, retry=None, comment=None):
    """Build SSE wire-format bytes from pre-serialized data; always ends with a blank line."""
    wire_lines = []

    if comment is not None:
        for chunk in comment.splitlines():
            wire_lines.append(f": {chunk}")

    if event is not None:
        wire_lines.append(f"event: {event}")

    if data_str is not None:
        for chunk in data_str.splitlines():
            wire_lines.append(f"data: {chunk}")

    if id is not None:
        wire_lines.append(f"id: {id}")

    if retry is not None:
        wire_lines.append(f"retry: {retry}")

    wire_lines.append("")
    wire_lines.append("")
    return "\n".join(wire_lines).encode("utf-8")
