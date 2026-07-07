# ID: wagtail/embeds/embeds.py:23
def fetch_embed(url, max_width=None, max_height=None):
    """Return a cached Embed for the URL, or fetch one via the configured finders and persist it."""
    embed_hash = get_embed_hash(url, max_width, max_height)

    # Reuse a non-expired cached embed if there is one.
    try:
        return Embed.objects.exclude(cache_until__lte=now()).get(hash=embed_hash)
    except Embed.DoesNotExist:
        pass

    embed_dict = get_finder_for_embed(url, max_width, max_height)

    # Coerce width/height into valid integers (or None) before saving.
    try:
        embed_dict["width"] = int(embed_dict["width"])
    except (TypeError, ValueError):
        embed_dict["width"] = None

    try:
        embed_dict["height"] = int(embed_dict["height"])
    except (TypeError, ValueError):
        embed_dict["height"] = None

    # Normalise optional string fields to "" so they can be stored.
    if "html" not in embed_dict or not embed_dict["html"]:
        embed_dict["html"] = ""
    if "thumbnail_url" not in embed_dict or not embed_dict["thumbnail_url"]:
        embed_dict["thumbnail_url"] = ""

    embed, created = Embed.objects.update_or_create(
        hash=embed_hash, defaults=dict(url=url, max_width=max_width, **embed_dict)
    )

    embed.last_updated = datetime.now()
    embed.save()

    return embed
