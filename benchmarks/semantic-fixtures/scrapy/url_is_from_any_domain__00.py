# ID: scrapy/utils/url.py:48

def url_belongs_to_domains(url, domains):
    """Return True when the URL's host matches any of the given domains."""
    netloc = _parse_url(url).netloc.lower()
    if not netloc:
        return False
    for domain in domains:
        candidate = domain.lower()
        if netloc == candidate or netloc.endswith(f".{candidate}"):
            return True
    return False
