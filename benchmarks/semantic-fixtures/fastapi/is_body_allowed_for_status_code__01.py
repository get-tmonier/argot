# ID: fastapi/utils.py:26
def status_code_permits_body(code):
    if code is None:
        return True
    wildcard_codes = {"default", "1XX", "2XX", "3XX", "4XX", "5XX"}
    if code in wildcard_codes:
        return True
    numeric_code = int(code)
    bodyless = numeric_code < 200 or numeric_code in {204, 205, 304}
    return not bodyless
