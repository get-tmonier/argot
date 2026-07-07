# ID: scrapy/utils/curl.py:57

def _extract_headers_and_cookies(parsed_args):
    headers = []
    cookies = {}
    for raw_header in parsed_args.headers or ():
        key, value = raw_header.split(":", 1)
        key = key.strip()
        value = value.strip()
        if key.title() == "Cookie":
            for cname, morsel in SimpleCookie(value).items():
                cookies[cname] = morsel.value
        else:
            headers.append((key, value))

    for cookie_param in parsed_args.cookies or ():
        # curl accepts either "k=v; k2=v2" pairs or a filename; only pairs here
        if "=" not in cookie_param:
            continue
        for cname, morsel in SimpleCookie(cookie_param).items():
            cookies[cname] = morsel.value

    if parsed_args.auth:
        username, password = parsed_args.auth.split(":", 1)
        headers.append(("Authorization", basic_auth_header(username, password)))

    return headers, cookies
