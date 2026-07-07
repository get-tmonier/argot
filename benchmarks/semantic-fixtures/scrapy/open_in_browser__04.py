# ID: scrapy/utils/response.py:74

def preview_response_in_browser(response, _openfunc=webbrowser.open):
    """Write *response* to a temp file (fixing its base tag) and open it locally."""
    from scrapy.http import HtmlResponse, TextResponse

    content = response.body
    if isinstance(response, HtmlResponse):
        if b"<base" not in content:
            _remove_html_comments(content)
            replacement = rf'\0<base href="{response.url}">'
            content = re.sub(
                rb"<head(?:[^<>]*?>)", to_bytes(replacement), content, count=1
            )
        suffix = ".html"
    elif isinstance(response, TextResponse):
        suffix = ".txt"
    else:
        raise TypeError(
            f"Unsupported response type: {response.__class__.__name__}"
        )

    handle, path = tempfile.mkstemp(suffix)
    os.write(handle, content)
    os.close(handle)
    return _openfunc(f"file://{path}")
