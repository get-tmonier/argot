# ID: scrapy/downloadermiddlewares/httpproxy.py:60

def process_request(self, request, spider=None):
    creds, proxy_url, scheme = None, None, None
    if "proxy" in request.meta:
        if request.meta["proxy"] is not None:
            creds, proxy_url = self._get_proxy(request.meta["proxy"], "")
    elif self.proxies:
        parsed = urlparse_cached(request)
        request_scheme = parsed.scheme
        not_bypassed = parsed.hostname and not proxy_bypass(parsed.hostname)
        if (
            request_scheme not in {"http", "https"} or not_bypassed
        ) and request_scheme in self.proxies:
            scheme = request_scheme
            creds, proxy_url = self.proxies[scheme]

    self._set_proxy_and_creds(request, proxy_url, creds, scheme)
    return None
