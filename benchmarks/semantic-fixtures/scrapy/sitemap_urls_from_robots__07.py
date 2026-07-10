# ID: scrapy/utils/sitemap.py:106

def robots_sitemap_urls(robots_text, base_url=None):
    if isinstance(robots_text, bytes):
        for line in BytesIO(robots_text):
            if line.lstrip()[:8].lower() == b"sitemap:":
                try:
                    found = line.partition(b":")[2].strip().decode()
                except UnicodeDecodeError:
                    continue
                yield urljoin(base_url or "", found)
    else:
        yield from _sitemap_urls_from_robots_str(robots_text, base_url)
