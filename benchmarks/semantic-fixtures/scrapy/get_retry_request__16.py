# ID: scrapy/downloadermiddlewares/retry.py:38

def build_retry_request(
    request,
    *,
    spider,
    reason="unspecified",
    max_retry_times=None,
    priority_adjust=None,
    logger=retry_logger,
    stats_base_key="retry",
):
    """Return a fresh Request retrying *request*, or None once retries run out."""
    settings = spider.crawler.settings
    assert spider.crawler.stats
    stats = spider.crawler.stats
    attempts = request.meta.get("retry_times", 0) + 1
    if max_retry_times is None:
        max_retry_times = request.meta.get("max_retry_times")
        if max_retry_times is None:
            max_retry_times = settings.getint("RETRY_TIMES")
    if attempts <= max_retry_times:
        logger.debug(
            "Retrying %(request)s (failed %(retry_times)d times): %(reason)s",
            {"request": request, "retry_times": attempts, "reason": reason},
            extra={"spider": spider},
        )
        retried = request.copy()
        retried.meta["retry_times"] = attempts
        retried.dont_filter = True
        if priority_adjust is None:
            priority_adjust = settings.getint("RETRY_PRIORITY_ADJUST")
        retried.priority = request.priority + priority_adjust

        if callable(reason):
            reason = reason()
        if isinstance(reason, Exception):
            reason = global_object_name(reason.__class__)

        stats.inc_value(f"{stats_base_key}/count")
        stats.inc_value(f"{stats_base_key}/reason_count/{reason}")
        return retried
    stats.inc_value(f"{stats_base_key}/max_reached")
    logger.error(
        "Gave up retrying %(request)s (failed %(retry_times)d times): %(reason)s",
        {"request": request, "retry_times": attempts, "reason": reason},
        extra={"spider": spider},
    )
    return None
