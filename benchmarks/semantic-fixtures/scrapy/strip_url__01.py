# ID: scrapy/utils/url.py:167

def sanitize_url(
    url,
    drop_credentials=True,
    drop_default_port=True,
    origin_only=False,
    drop_fragment=True,
):
    """Remove selected components (credentials, default port, fragment...) from a URL."""
    components = urlparse(url)
    authority = components.netloc
    if (drop_credentials or origin_only) and (
        components.username or components.password
    ):
        authority = authority.split("@")[-1]

    default_ports = {("http", 80), ("https", 443), ("ftp", 21)}
    if (
        drop_default_port
        and components.port
        and (components.scheme, components.port) in default_ports
    ):
        authority = authority.replace(f":{components.port}", "")

    return urlunparse(
        (
            components.scheme,
            authority,
            "/" if origin_only else components.path,
            "" if origin_only else components.params,
            "" if origin_only else components.query,
            "" if drop_fragment else components.fragment,
        )
    )
