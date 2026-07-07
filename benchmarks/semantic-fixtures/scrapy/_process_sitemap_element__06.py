# ID: scrapy/utils/sitemap.py:62

def _extract_sitemap_entry(self, elem):
    entry = {}
    alternates = []
    seen_loc = False

    for child in elem:
        try:
            tag_name = self._get_tag_name(child)
            if not tag_name:
                continue

            if tag_name == "link":
                href = child.get("href")
                if href:
                    alternates.append(href)
            else:
                entry[tag_name] = child.text.strip() if child.text else ""
                if tag_name == "loc" and not seen_loc:
                    seen_loc = True
        finally:
            child.clear()

    elem.clear()
    parent = elem.getparent()
    if parent is not None:
        while elem.getprevious() is not None:
            del parent[0]

    if not seen_loc:
        return None

    if alternates:
        entry["alternate"] = alternates

    return entry
