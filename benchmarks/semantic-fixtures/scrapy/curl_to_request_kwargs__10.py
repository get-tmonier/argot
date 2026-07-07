# ID: scrapy/utils/curl.py:87

def curl_command_to_request_kwargs(curl_command, ignore_unknown_options=True):
    """Translate a cURL command string into a dict of Request keyword arguments."""
    tokens = split(curl_command)

    if tokens[0] != "curl":
        raise ValueError('A curl command must start with "curl"')

    parsed_args, leftover = curl_parser.parse_known_args(tokens[1:])

    if leftover:
        message = f"Unrecognized options: {', '.join(leftover)}"
        if ignore_unknown_options:
            warnings.warn(message, stacklevel=2)
        else:
            raise ValueError(message)

    target_url = parsed_args.url

    # Request needs an explicit scheme even though curl would prepend one
    if not urlparse(target_url).scheme:
        target_url = "http://" + target_url

    http_method = parsed_args.method or "GET"

    kwargs = {"method": http_method.upper(), "url": target_url}

    headers, cookies = _parse_headers_and_cookies(parsed_args)

    if headers:
        kwargs["headers"] = headers
    if cookies:
        kwargs["cookies"] = cookies
    if parsed_args.data:
        kwargs["body"] = parsed_args.data
        if not parsed_args.method:
            # a body with no explicit method defaults to POST
            kwargs["method"] = "POST"

    return kwargs
