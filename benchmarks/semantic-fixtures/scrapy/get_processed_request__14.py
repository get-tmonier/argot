# ID: scrapy/spidermiddlewares/depth.py:81

def get_processed_request(self, request, response):
    if response is None:
        # start requests
        return request
    new_depth = response.meta["depth"] + 1
    request.meta["depth"] = new_depth
    if self.prio:
        request.priority -= new_depth * self.prio
    if self.maxdepth and new_depth > self.maxdepth:
        logger.debug(
            "Ignoring link (depth > %(maxdepth)d): %(requrl)s ",
            {"maxdepth": self.maxdepth, "requrl": request.url},
            extra={"spider": self.crawler.spider},
        )
        return None
    if self.verbose_stats:
        self.stats.inc_value(f"request_depth_count/{new_depth}")
    self.stats.max_value("request_depth_max", new_depth)
    return request
