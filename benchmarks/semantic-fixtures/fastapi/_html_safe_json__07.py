# ID: fastapi/openapi/docs.py:9
def json_dump_html_escaped(value):
    """Serialize a value to JSON, escaping HTML-significant characters for <script> embedding."""
    serialized = json.dumps(value)
    serialized = serialized.replace("<", "\\u003c")
    serialized = serialized.replace(">", "\\u003e")
    serialized = serialized.replace("&", "\\u0026")
    return serialized
