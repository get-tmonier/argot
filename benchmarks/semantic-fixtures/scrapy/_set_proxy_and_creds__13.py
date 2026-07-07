# ID: scrapy/downloadermiddlewares/httpproxy.py:81

def _apply_proxy_and_creds(self, request, proxy_url, creds, scheme):
    if scheme:
        request.meta["_scheme_proxy"] = True
    if proxy_url:
        request.meta["proxy"] = proxy_url
    elif request.meta.get("proxy") is not None:
        request.meta["proxy"] = None
    if creds:
        request.headers[b"Proxy-Authorization"] = b"Basic " + creds
        request.meta["_auth_proxy"] = proxy_url
    elif "_auth_proxy" in request.meta:
        if proxy_url != request.meta["_auth_proxy"]:
            if b"Proxy-Authorization" in request.headers:
                del request.headers[b"Proxy-Authorization"]
            del request.meta["_auth_proxy"]
    elif b"Proxy-Authorization" in request.headers:
        if proxy_url:
            request.meta["_auth_proxy"] = proxy_url
        else:
            del request.headers[b"Proxy-Authorization"]
