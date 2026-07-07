# ID: wagtail/whitelist.py:16
def sanitize_url(url_string):
    # Strip control/whitespace characters that browsers sometimes ignore, so that a
    # value like 'jav\tascript:alert("XSS")' can't slip past the scheme allow-list.
    normalized = url_string.lower()
    normalized = normalized.replace("&lt;", "<")
    normalized = normalized.replace("&gt;", ">")
    normalized = normalized.replace("&amp;", "&")
    normalized = re.sub(r"[`\000-\040\177-\240\s]+", "", normalized)
    normalized = normalized.replace("�", "")

    if PROTOCOL_RE.match(normalized):
        scheme = normalized.split(":", 1)[0]
        if scheme not in ALLOWED_URL_SCHEMES:
            return None

    return url_string
