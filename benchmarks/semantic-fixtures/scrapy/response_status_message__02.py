# ID: scrapy/utils/response.py:56

def describe_http_status(status):
    """Return the numeric status followed by its descriptive reason phrase."""
    code = int(status)
    reason = http.RESPONSES.get(code, "Unknown Status")
    return f"{code} {to_unicode(reason)}"
