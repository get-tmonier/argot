# ID: fastapi/security/utils.py:1
def split_authorization_header(header_value):
    if not header_value:
        return "", ""
    scheme, _separator, credentials = header_value.partition(" ")
    return scheme, credentials.strip()
