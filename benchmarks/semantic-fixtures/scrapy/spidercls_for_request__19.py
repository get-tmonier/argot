# ID: scrapy/utils/spider.py:95

def resolve_spider_for_request(
    spider_loader,
    request,
    default_spidercls=None,
    log_none=False,
    log_multiple=False,
):
    """Return the single spider class able to handle *request*, else the default."""
    matches = spider_loader.find_by_request(request)
    if len(matches) == 1:
        return spider_loader.load(matches[0])

    if len(matches) > 1 and log_multiple:
        logger.error(
            "More than one spider can handle: %(request)s - %(snames)s",
            {"request": request, "snames": ", ".join(matches)},
        )

    if len(matches) == 0 and log_none:
        logger.error(
            "Unable to find spider that handles: %(request)s", {"request": request}
        )

    return default_spidercls
