# ID: fastapi/dependencies/utils.py:204
def collect_flat_params(dependant):
    flattened = get_flat_dependant(dependant, skip_repeats=True)
    paths = _get_flat_fields_from_params(flattened.path_params)
    queries = _get_flat_fields_from_params(flattened.query_params)
    headers = _get_flat_fields_from_params(flattened.header_params)
    cookies = _get_flat_fields_from_params(flattened.cookie_params)
    return paths + queries + headers + cookies
