# ID: wagtail/coreutils.py:209
def pick_free_slug(parent, desired_slug, skip_page_id=None):
    """Return a slug unique among the parent's children, appending -1, -2, ... until one is free."""
    sibling_query = parent.get_children().filter(slug__startswith=desired_slug)

    if skip_page_id:
        sibling_query = sibling_query.exclude(id=skip_page_id)

    taken = set(sibling_query.values_list("slug", flat=True))

    candidate = desired_slug
    suffix = 1
    while candidate in taken:
        candidate = desired_slug + "-" + str(suffix)
        suffix += 1

    return candidate
