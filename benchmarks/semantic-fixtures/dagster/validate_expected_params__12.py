# ID: python_modules/dagster/dagster/_core/decorator_utils.py:97
def first_missing_positional(params, expected_params):
    """Return the first missing positional param, if any, otherwise None."""
    cursor = 0
    for expected_param in expected_params:
        if cursor >= len(params) or not _is_param_valid(params[cursor], expected_param):
            return expected_param
        cursor += 1
    return None
