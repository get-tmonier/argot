# ID: wagtail/rich_text/rewriters.py:183
def get_tag_type_from_attrs(self, attrs):
    try:
        return attrs["linktype"]
    except KeyError:
        pass

    # No explicit linktype: infer the ones the link chooser supports from the href.
    href = attrs.get("href", None)
    if not href:
        return

    if href.startswith(("http:", "https:")):
        return "external"
    elif href.startswith("mailto:"):
        return "email"
    elif href.startswith("#"):
        return "anchor"
