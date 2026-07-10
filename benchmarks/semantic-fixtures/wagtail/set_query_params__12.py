# ID: wagtail/admin/utils.py:71
def merge_query_params(url: str, params: dict):
    """Return the URL with the given query parameters added or updated; a value of None removes that parameter."""
    scheme, netloc, path, query, fragment = urlsplit(url)

    query_map = parse_qs(query)
    query_map.update(params)
    query_map = {key: value for key, value in query_map.items() if value is not None}

    new_query = urlencode(query_map, doseq=True)
    return urlunsplit((scheme, netloc, path, new_query, fragment))
